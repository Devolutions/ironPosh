use std::{fmt::Debug, sync::Arc};

use ironposh_psrp::HostInfo;
use ironposh_winrm::ws_management::WsMan;

// I'm lasy for now, just re-export from sspi
pub use sspi::{generator::NetworkRequest, network_client::NetworkProtocol};

use tracing::{info, instrument, warn};

use crate::{
    connector::{
        auth_sequence::AuthSequenceConfig,
        config::AuthenticatorConfig,
        conntion_pool::{ConnectionPool, ConnectionPoolConfig, TrySend},
        http::{HttpResponseTargeted, ServerAddress},
    },
    runspace_pool::{
        DesiredStream, ExpectShellCreated, RunspacePool, RunspacePoolCreator, RunspacePoolState,
        pool::AcceptResponsResult,
    },
};

pub use active_session::{ActiveSession, ActiveSessionOutput, UserOperation};
pub mod active_session;
pub mod auth_sequence;
pub mod authenticator;
pub mod config;
pub mod conntion_pool;
pub mod encryption;
pub mod http;

#[derive(Debug, Clone)]
pub enum Scheme {
    Http,
    Https,
}

#[derive(Debug, Clone)]
pub struct WinRmConfig {
    pub server: (ServerAddress, u16),
    pub scheme: Scheme,
    pub authentication: AuthenticatorConfig,
    pub host_info: HostInfo,
}

impl WinRmConfig {
    pub fn wsman_to(&self, query: Option<&str>) -> String {
        let query = query
            .map(|q| format!("?{}", q.trim_start_matches('?')))
            .unwrap_or_default();

        match &self.scheme {
            Scheme::Http => format!("http://{}:{}/wsman{}", self.server.0, self.server.1, query),
            Scheme::Https => format!("https://{}:{}/wsman{}", self.server.0, self.server.1, query),
        }
    }
}

#[derive(Debug)]
pub enum ConnectorStepResult {
    SendBack {
        try_send: TrySend,
    },
    Connected {
        /// use box to avoid large enum variant
        active_session: Box<ActiveSession>,
        send_this_one_async_or_you_stuck: TrySend,
    },
}

impl ConnectorStepResult {
    pub fn name(&self) -> &'static str {
        match self {
            ConnectorStepResult::SendBack { .. } => "SendBack",
            ConnectorStepResult::Connected { .. } => "Connected",
        }
    }
}

impl ConnectorStepResult {
    pub fn priority(&self) -> u8 {
        match self {
            ConnectorStepResult::SendBack { .. } => 1,
            ConnectorStepResult::Connected { .. } => 3,
        }
    }
}

#[derive(Default, Debug)]
pub enum ConnectorState {
    #[default]
    Idle,
    Connecting {
        expect_shell_created: ExpectShellCreated,
        connection_pool: ConnectionPool,
    },
    ConnectReceiveCycle {
        runspace_pool: RunspacePool,
        connection_pool: ConnectionPool,
    },
    Connected,
}

impl ConnectorState {
    fn state_name(&self) -> &'static str {
        match self {
            ConnectorState::Idle => "Idle",
            ConnectorState::Connecting { .. } => "Connecting",
            ConnectorState::ConnectReceiveCycle { .. } => "ConnectReceiveCycle",
            ConnectorState::Connected => "Connected",
        }
    }
}

#[derive(Debug)]
pub struct Connector {
    state: ConnectorState,
    config: WinRmConfig,
}

impl Connector {
    pub fn new(config: WinRmConfig) -> Self {
        Self {
            state: ConnectorState::Idle,
            config,
        }
    }

    fn set_state(&mut self, state: ConnectorState) {
        info!(state = state.state_name(), "Setting connector state");
        self.state = state;
    }

    #[instrument(skip(self, server_response), name = "Connector::step")]
    pub fn step(
        &mut self,
        server_response: Option<HttpResponseTargeted>,
    ) -> Result<ConnectorStepResult, crate::PwshCoreError> {
        let state = std::mem::take(&mut self.state);

        let (new_state, response) = match state {
            ConnectorState::Connected => {
                return Err(crate::PwshCoreError::InvalidState(
                    "Connector is in invalid state for step()",
                ));
            }
            ConnectorState::Idle => {
                debug_assert!(
                    server_response.is_none(),
                    "Request should be None in Idle state"
                );

                // Create pool with SSPI cfg derived from WinRmConfig
                let pool_cfg = ConnectionPoolConfig::from(&self.config);

                let authenticator_cfg = self.config.authentication.clone();

                let auth_sequence_config = AuthSequenceConfig::new(authenticator_cfg);

                let mut connection_pool = ConnectionPool::new(pool_cfg, auth_sequence_config);

                let ws_man = Arc::new(WsMan::builder().to(self.config.wsman_to(None)).build());

                let runspace_pool = RunspacePoolCreator::builder()
                    .host_info(self.config.host_info.clone())
                    .build()
                    .into_runspace_pool(ws_man);

                let (xml_body, expect_shell_created) = runspace_pool.open()?;
                info!(shell_creation_xml = %xml_body, "outgoing unencrypted shell creation SOAP");

                let try_send = connection_pool.send(&xml_body)?;

                let new_state = ConnectorState::Connecting {
                    expect_shell_created,
                    connection_pool,
                };

                (new_state, ConnectorStepResult::SendBack { try_send })
            }
            ConnectorState::Connecting {
                expect_shell_created,
                mut connection_pool,
            } => {
                // Expect the response to the OpenShell POST on some conn
                let targeted_response = server_response.ok_or(
                    crate::PwshCoreError::InvalidState("Expected response in Connecting"),
                )?;
                let xml = connection_pool.accept(targeted_response)?;

                // Advance runspace handshake
                let mut runspace_pool = expect_shell_created.accept(xml)?;
                let receive_xml =
                    runspace_pool.fire_receive(DesiredStream::runspace_pool_streams())?;
                info!(connecting_receive_xml = %receive_xml, "outgoing unencrypted connecting receive SOAP");
                let try_send = connection_pool.send(&receive_xml)?;

                let new_state = ConnectorState::ConnectReceiveCycle {
                    runspace_pool,
                    connection_pool,
                };

                (new_state, ConnectorStepResult::SendBack { try_send })
            }
            ConnectorState::ConnectReceiveCycle {
                mut runspace_pool,
                mut connection_pool,
            } => {
                let targeted_response = server_response.ok_or(
                    crate::PwshCoreError::InvalidState("Expected response in ConnectReceiveCycle"),
                )?;
                let xml = connection_pool.accept(targeted_response)?;

                let results = runspace_pool.accept_response(xml)?;
                let Some(AcceptResponsResult::ReceiveResponse { desired_streams }) = results
                    .into_iter()
                    .find(|r| matches!(r, AcceptResponsResult::ReceiveResponse { .. }))
                else {
                    return Err(crate::PwshCoreError::InvalidState(
                        "Expected ReceiveResponse",
                    ));
                };

                if let RunspacePoolState::NegotiationSent = runspace_pool.state {
                    let receive_xml = runspace_pool.fire_receive(desired_streams)?;
                    let try_send = connection_pool.send(&receive_xml)?;
                    let new_state = ConnectorState::ConnectReceiveCycle {
                        runspace_pool,
                        connection_pool,
                    };
                    (new_state, ConnectorStepResult::SendBack { try_send })
                } else if let RunspacePoolState::Opened = runspace_pool.state {
                    // Hand off to ActiveSession: it should carry the pool forward
                    let next_receive_xml = runspace_pool.fire_receive(desired_streams)?;
                    let next_req = connection_pool.send(&next_receive_xml)?;
                    let active_session = ActiveSession::new(runspace_pool, connection_pool);
                    let new_state = ConnectorState::Connected;
                    (
                        new_state,
                        ConnectorStepResult::Connected {
                            active_session: Box::new(active_session),
                            send_this_one_async_or_you_stuck: next_req,
                        },
                    )
                } else {
                    return Err(crate::PwshCoreError::InvalidState(
                        "Unexpected RunspacePool state",
                    ));
                }
            }
        };

        self.set_state(new_state);

        Ok(response)
    }
}
