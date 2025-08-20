use std::sync::Arc;

use protocol_powershell_remoting::HostInfo;
use protocol_winrm::ws_management::WsMan;
use tracing::{info, instrument, warn};

use crate::{
    connector::http::{HttpBuilder, HttpRequest, HttpResponse, ServerAddress},
    runspace_pool::{
        DesiredStream, ExpectShellCreated, RunspacePool, RunspacePoolCreator, RunspacePoolState,
        pool::AcceptResponsResult,
    },
};

pub use active_session::{ActiveSession, ActiveSessionOutput, UserOperation};
pub mod active_session;
pub mod http;

#[derive(Debug, Clone)]
pub enum Authentication {
    Basic { username: String, password: String },
    // TODO: Add SSPI
}

#[derive(Debug, Clone)]
pub enum Scheme {
    Http,
    Https,
}

pub struct ConnectorConfig {
    pub server: (ServerAddress, u16),
    pub scheme: Scheme,
    pub authentication: Authentication,
    pub host_info: HostInfo,
}

impl ConnectorConfig {
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
    SendBack(HttpRequest<String>),
    SendBackError(crate::PwshCoreError),
    Connected {
        active_session: ActiveSession,
        next_receive_request: HttpRequest<String>,
    },
}

impl ConnectorStepResult {
    pub fn name(&self) -> &'static str {
        match self {
            ConnectorStepResult::SendBack(_) => "SendBack",
            ConnectorStepResult::SendBackError(_) => "SendBackError",
            ConnectorStepResult::Connected { .. } => "Connected",
        }
    }
}

impl ConnectorStepResult {
    pub fn priority(&self) -> u8 {
        match self {
            ConnectorStepResult::SendBack(_) => 0,
            ConnectorStepResult::SendBackError(_) => 1,
            ConnectorStepResult::Connected { .. } => 2,
        }
    }
}

#[derive(Default, Debug)]
pub enum ConnectorState {
    Idle,
    #[default]
    Taken,
    Connecting {
        expect_shell_created: ExpectShellCreated,
        http_builder: HttpBuilder,
    },
    ConnectReceiveCycle {
        runspace_pool: RunspacePool,
        http_builder: HttpBuilder,
    },
    Connected,
    Failed,
}

impl ConnectorState {
    fn state_name(&self) -> &'static str {
        match self {
            ConnectorState::Idle => "Idle",
            ConnectorState::Taken => "Taken",
            ConnectorState::Connecting { .. } => "Connecting",
            ConnectorState::ConnectReceiveCycle { .. } => "ConnectReceiveCycle",
            ConnectorState::Connected => "Connected",
            ConnectorState::Failed => "Failed",
        }
    }
}

pub struct Connector {
    state: ConnectorState,
    config: ConnectorConfig,
}

impl Connector {
    pub fn new(config: ConnectorConfig) -> Self {
        Self {
            state: ConnectorState::Idle,
            config,
        }
    }

    pub fn set_state(&mut self, state: ConnectorState) {
        info!(state = state.state_name(), "Setting connector state");
        self.state = state;
    }

    #[instrument(skip(self, server_response), name = "Connector::step")]
    pub fn step(
        &mut self,
        server_response: Option<HttpResponse<String>>,
    ) -> Result<ConnectorStepResult, crate::PwshCoreError> {
        let state = std::mem::take(&mut self.state);

        let (new_state, response) = match state {
            ConnectorState::Taken => {
                return Err(crate::PwshCoreError::UnlikelyToHappen(
                    "Connector should not be in Taken state when stepping",
                ));
            }
            ConnectorState::Failed => {
                warn!("Connector is in Failed state, cannot proceed");
                return Err(crate::PwshCoreError::InvalidState(
                    "Connector is in Failed state",
                ));
            }
            ConnectorState::Connected => {
                warn!("Connector is already connected, cannot step further");
                return Err(crate::PwshCoreError::InvalidState(
                    "Connector is already connected",
                ));
            }
            ConnectorState::Idle => {
                debug_assert!(
                    server_response.is_none(),
                    "Request should be None in Idle state"
                );
                let connection = Arc::new(WsMan::builder().to(self.config.wsman_to(None)).build());
                let runspace_pool = RunspacePoolCreator::builder()
                    .host_info(self.config.host_info.clone())
                    .build()
                    .into_runspace_pool(connection);

                let http_builder = HttpBuilder::new(
                    self.config.server.0.clone(),
                    self.config.server.1,
                    self.config.scheme.clone(),
                    self.config.authentication.clone(),
                );

                let (xml_body, expect_shell_created) = runspace_pool.open()?;

                let response = http_builder.post("/wsman", xml_body);

                let new_state = ConnectorState::Connecting {
                    expect_shell_created,
                    http_builder,
                };

                (new_state, ConnectorStepResult::SendBack(response))
            }
            ConnectorState::Connecting {
                expect_shell_created,
                http_builder,
            } => {
                info!("Processing Connecting state");
                let response = server_response.ok_or({
                    crate::PwshCoreError::InvalidState("Expected a response in Connecting state")
                })?;

                let body = response.body.ok_or({
                    crate::PwshCoreError::InvalidState("Expected a body in Connecting state")
                })?;

                let mut runspace_pool = expect_shell_created.accept(body)?;

                let receive_request =
                    runspace_pool.fire_receive(DesiredStream::runspace_pool_streams())?;

                let response = http_builder.post("/wsman", receive_request);

                let new_state = ConnectorState::ConnectReceiveCycle {
                    runspace_pool,
                    http_builder,
                };

                (new_state, ConnectorStepResult::SendBack(response))
            }
            ConnectorState::ConnectReceiveCycle {
                mut runspace_pool,
                http_builder,
            } => {
                let response = server_response.ok_or({
                    crate::PwshCoreError::InvalidState(
                        "Expected a response in ConnectReceiveCycle state",
                    )
                })?;

                let body = response.body.ok_or({
                    crate::PwshCoreError::InvalidState(
                        "Expected a body in ConnectReceiveCycle state",
                    )
                })?;

                let accept_response_results = runspace_pool.accept_response(body)?;
                let Some(AcceptResponsResult::ReceiveResponse { desired_streams }) =
                    accept_response_results
                        .into_iter()
                        .find(|r| matches!(r, AcceptResponsResult::ReceiveResponse { .. }))
                else {
                    return Err(crate::PwshCoreError::InvalidState(
                        "Expected ReceiveResponse in ConnectReceiveCycle state",
                    ));
                };

                if let RunspacePoolState::NegotiationSent = runspace_pool.state {
                    let receive_request = runspace_pool.fire_receive(desired_streams)?;
                    let response = http_builder.post("/wsman", receive_request);
                    let new_state = ConnectorState::ConnectReceiveCycle {
                        runspace_pool,
                        http_builder,
                    };
                    (new_state, ConnectorStepResult::SendBack(response))
                } else if let RunspacePoolState::Opened = runspace_pool.state {
                    info!("Connection established successfully - returning ActiveSession");
                    let next_receive_request = runspace_pool.fire_receive(desired_streams)?;
                    let next_http_request = http_builder.post("/wsman", next_receive_request);
                    let active_session = ActiveSession::new(runspace_pool, http_builder);
                    (
                        ConnectorState::Connected,
                        ConnectorStepResult::Connected {
                            active_session,
                            next_receive_request: next_http_request,
                        },
                    )
                } else {
                    warn!("Unexpected RunspacePool state: {:?}", runspace_pool.state);
                    (
                        ConnectorState::Failed,
                        ConnectorStepResult::SendBackError(crate::PwshCoreError::InvalidState(
                            "Unexpected RunspacePool state",
                        )),
                    )
                }
            }
        };

        self.set_state(new_state);
        Ok(response)
    }
}
