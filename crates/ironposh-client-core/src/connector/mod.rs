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
        connection_pool::{ConnectionPool, ConnectionPoolAccept, ConnectionPoolConfig, TrySend},
        http::{HttpResponseTargeted, ServerAddress},
    },
    runspace_pool::{
        DesiredStream, ExpectShellConnected, ExpectShellCreated, RunspacePool, RunspacePoolCreator,
        RunspacePoolState, pool::AcceptResponsResult,
    },
};

pub use active_session::{ActiveSession, ActiveSessionOutput, UserOperation};
pub mod active_session;
pub mod auth_sequence;
pub mod authenticator;
pub mod config;
pub mod connection_pool;
pub mod encryption;
pub mod http;

/// Internal scheme type for URL building
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Http,
    Https,
}

/// Transport security configuration - invalid states are unrepresentable.
///
/// This enum enforces correct security settings:
/// - `Http`: HTTP with SSPI message sealing (required for security over unencrypted transport)
/// - `Https`: HTTPS with TLS encryption (SSPI sealing not needed, TLS handles it)
/// - `HttpInsecure`: HTTP without SSPI sealing - **DANGEROUS**, use only for testing/debugging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportSecurity {
    /// HTTP transport with SSPI message sealing enabled.
    /// This is the secure way to use HTTP - all post-auth messages are encrypted via SSPI.
    Http,

    /// HTTPS transport with TLS encryption.
    /// SSPI message sealing is not needed since TLS provides encryption and integrity.
    Https,

    /// HTTP transport WITHOUT SSPI message sealing.
    /// **WARNING: This is insecure!** Post-auth messages are sent unencrypted.
    /// Only use for testing, debugging, or legacy compatibility.
    HttpInsecure,
}

impl TransportSecurity {
    /// Get the underlying HTTP scheme for URL construction
    pub fn scheme(&self) -> Scheme {
        match self {
            Self::Http | Self::HttpInsecure => Scheme::Http,
            Self::Https => Scheme::Https,
        }
    }

    /// Whether SSPI message sealing (wrap/unwrap) should be used
    pub fn requires_sspi_sealing(&self) -> bool {
        matches!(self, Self::Http)
    }

    /// Whether this transport configuration is considered secure
    pub fn is_secure(&self) -> bool {
        !matches!(self, Self::HttpInsecure)
    }
}

#[derive(Debug, Clone)]
pub struct WinRmConfig {
    pub server: (ServerAddress, u16),
    pub transport: TransportSecurity,
    pub authentication: AuthenticatorConfig,
    pub host_info: HostInfo,
    /// WS-Management OperationTimeout in seconds (fractional values allowed, e.g. `0.5`).
    ///
    /// - `None` — use the server/protocol default (180 s).
    /// - `Some(t)` — set an explicit timeout of `t` seconds. Fractional values are
    ///   serialised as `PT{t:.3}S` in the SOAP header (e.g. `PT0.500S`).
    ///
    /// For serial/single-connection mode, a short timeout (0.5 s–5 s) is recommended
    /// so inbound Receives don't block outbound sends for too long.
    ///
    /// # Type note
    ///
    /// Changed from `Option<u32>` to `Option<f64>` to support sub-second timeouts
    /// (required by serial mode's 500 ms Receive slices).
    pub operation_timeout_secs: Option<f64>,
    /// TLS behaviour for HTTPS transports. Ignored for plain-HTTP transports.
    pub tls: config::TlsOptions,
    /// PowerShell session configuration (JEA endpoint) name.
    /// `None` → `Microsoft.PowerShell`. Becomes the shell resource URI
    /// `http://schemas.microsoft.com/powershell/{name}`.
    pub configuration_name: Option<String>,
}

impl WinRmConfig {
    pub fn wsman_to(&self, query: Option<&str>) -> String {
        let query = query
            .map(|q| format!("?{}", q.trim_start_matches('?')))
            .unwrap_or_default();

        match self.transport.scheme() {
            Scheme::Http => format!("http://{}:{}/wsman{}", self.server.0, self.server.1, query),
            Scheme::Https => format!("https://{}:{}/wsman{}", self.server.0, self.server.1, query),
        }
    }

    /// Shell resource URI for the configured PowerShell session configuration
    /// (JEA endpoint). Defaults to `Microsoft.PowerShell` when no
    /// `configuration_name` is set.
    pub fn shell_resource_uri(&self) -> String {
        format!(
            "http://schemas.microsoft.com/powershell/{}",
            self.configuration_name
                .as_deref()
                .unwrap_or("Microsoft.PowerShell")
        )
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
            Self::SendBack { .. } => "SendBack",
            Self::Connected { .. } => "Connected",
        }
    }
}

impl ConnectorStepResult {
    pub fn priority(&self) -> u8 {
        match self {
            Self::SendBack { .. } => 1,
            Self::Connected { .. } => 3,
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
    /// Waiting for the ConnectResponse of a WSMan Connect to an existing
    /// disconnected shell (reattach path).
    ConnectingExisting {
        expect_shell_connected: ExpectShellConnected,
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
            Self::Idle => "Idle",
            Self::Connecting { .. } => "Connecting",
            Self::ConnectingExisting { .. } => "ConnectingExisting",
            Self::ConnectReceiveCycle { .. } => "ConnectReceiveCycle",
            Self::Connected => "Connected",
        }
    }
}

#[derive(Debug)]
pub struct Connector {
    state: ConnectorState,
    config: WinRmConfig,
    /// When set, the connector attaches to this existing disconnected shell
    /// (WSMan Connect) instead of creating a new one. The shell id is also
    /// used as the pool RPID (shell id == pool RPID in this codebase).
    connect_shell_id: Option<uuid::Uuid>,
}

impl Connector {
    pub fn new(config: WinRmConfig) -> Self {
        Self {
            state: ConnectorState::Idle,
            config,
            connect_shell_id: None,
        }
    }

    /// Create a connector that attaches to an existing disconnected shell
    /// (browser-refresh / new-process reattach) instead of creating one.
    pub fn new_connect(config: WinRmConfig, shell_id: uuid::Uuid) -> Self {
        Self {
            state: ConnectorState::Idle,
            config,
            connect_shell_id: Some(shell_id),
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
                let require_sspi_sealing = self.config.transport.requires_sspi_sealing();

                let auth_sequence_config =
                    AuthSequenceConfig::new(authenticator_cfg, require_sspi_sealing);

                let mut connection_pool = ConnectionPool::new(pool_cfg, auth_sequence_config);

                let operation_timeout = self.config.operation_timeout_secs.unwrap_or(180.0);
                let ws_man = Arc::new(
                    WsMan::builder()
                        .to(self.config.wsman_to(None))
                        .operation_timeout(operation_timeout)
                        .resource_uri(self.config.shell_resource_uri())
                        .build(),
                );

                if let Some(shell_id) = self.connect_shell_id {
                    // Reattach path: WSMan Connect to an existing disconnected
                    // shell. The provided shell id doubles as the pool RPID.
                    let runspace_pool = RunspacePoolCreator::builder()
                        .id(shell_id)
                        .host_info(self.config.host_info.clone())
                        .build()
                        .into_connect_runspace_pool(ws_man);

                    let (xml_body, expect_shell_connected) = runspace_pool.connect()?;
                    info!(shell_id = %shell_id, shell_connect_xml = %xml_body, "outgoing unencrypted shell connect SOAP");

                    let try_send = connection_pool.send(&xml_body)?;

                    let new_state = ConnectorState::ConnectingExisting {
                        expect_shell_connected,
                        connection_pool,
                    };

                    (new_state, ConnectorStepResult::SendBack { try_send })
                } else {
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
            }
            ConnectorState::ConnectingExisting {
                expect_shell_connected,
                mut connection_pool,
            } => {
                // Expect the response to the Connect POST on some conn
                let targeted_response = server_response.ok_or(
                    crate::PwshCoreError::InvalidState("Expected response in ConnectingExisting"),
                )?;

                match connection_pool.accept(targeted_response)? {
                    ConnectionPoolAccept::Body(xml) => {
                        // The ConnectResponse carries the full negotiation
                        // (SESSION_CAPABILITY + RUNSPACEPOOL_INIT_DATA): the
                        // pool is Opened right away. Fire the initial Receive
                        // and hand off to the ActiveSession like the normal path.
                        let runspace_pool = expect_shell_connected.accept(&xml)?;
                        let next_receive_xml =
                            runspace_pool.fire_receive(DesiredStream::runspace_pool_streams())?;
                        info!(connect_receive_xml = %next_receive_xml, "outgoing unencrypted post-connect receive SOAP");
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
                    }
                    ConnectionPoolAccept::SendBack(reqs) => {
                        let [try_send] = <[TrySend; 1]>::try_from(reqs).map_err(|_| {
                            crate::PwshCoreError::InvalidState(
                                "Expected single SendBack during ConnectingExisting retry",
                            )
                        })?;
                        let new_state = ConnectorState::ConnectingExisting {
                            expect_shell_connected,
                            connection_pool,
                        };
                        (new_state, ConnectorStepResult::SendBack { try_send })
                    }
                }
            }
            ConnectorState::Connecting {
                expect_shell_created,
                mut connection_pool,
            } => {
                // Expect the response to the OpenShell POST on some conn
                let targeted_response = server_response.ok_or(
                    crate::PwshCoreError::InvalidState("Expected response in Connecting"),
                )?;

                match connection_pool.accept(targeted_response)? {
                    ConnectionPoolAccept::Body(xml) => {
                        // Advance runspace handshake
                        let runspace_pool = expect_shell_created.accept(&xml)?;
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
                    ConnectionPoolAccept::SendBack(reqs) => {
                        let [try_send] = <[TrySend; 1]>::try_from(reqs).map_err(|_| {
                            crate::PwshCoreError::InvalidState(
                                "Expected single SendBack during Connecting retry",
                            )
                        })?;
                        let new_state = ConnectorState::Connecting {
                            expect_shell_created,
                            connection_pool,
                        };
                        (new_state, ConnectorStepResult::SendBack { try_send })
                    }
                }
            }
            ConnectorState::ConnectReceiveCycle {
                mut runspace_pool,
                mut connection_pool,
            } => {
                let targeted_response = server_response.ok_or(
                    crate::PwshCoreError::InvalidState("Expected response in ConnectReceiveCycle"),
                )?;

                match connection_pool.accept(targeted_response)? {
                    ConnectionPoolAccept::Body(xml) => {
                        let results = runspace_pool.accept_response(&xml)?;
                        let Some(AcceptResponsResult::ReceiveResponse { desired_streams }) =
                            results
                                .into_iter()
                                .find(|r| matches!(r, AcceptResponsResult::ReceiveResponse { .. }))
                        else {
                            return Err(crate::PwshCoreError::InvalidState(
                                "Expected ReceiveResponse",
                            ));
                        };

                        if runspace_pool.state == RunspacePoolState::NegotiationSent {
                            let receive_xml = runspace_pool.fire_receive(desired_streams)?;
                            let try_send = connection_pool.send(&receive_xml)?;
                            let new_state = ConnectorState::ConnectReceiveCycle {
                                runspace_pool,
                                connection_pool,
                            };
                            (new_state, ConnectorStepResult::SendBack { try_send })
                        } else if runspace_pool.state == RunspacePoolState::Opened {
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
                    ConnectionPoolAccept::SendBack(reqs) => {
                        let [try_send] = <[TrySend; 1]>::try_from(reqs).map_err(|_| {
                            crate::PwshCoreError::InvalidState(
                                "Expected single SendBack during ConnectReceiveCycle retry",
                            )
                        })?;
                        let new_state = ConnectorState::ConnectReceiveCycle {
                            runspace_pool,
                            connection_pool,
                        };
                        (new_state, ConnectorStepResult::SendBack { try_send })
                    }
                }
            }
        };

        self.set_state(new_state);

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_configuration_name(configuration_name: Option<String>) -> WinRmConfig {
        let size = ironposh_psrp::Size {
            width: 80,
            height: 25,
        };
        let host_data = ironposh_psrp::HostDefaultData::builder()
            .buffer_size(size.clone())
            .window_size(size.clone())
            .max_window_size(size.clone())
            .max_physical_window_size(size)
            .build();

        WinRmConfig {
            server: (ServerAddress::parse("127.0.0.1").unwrap(), 5985),
            transport: TransportSecurity::HttpInsecure,
            authentication: AuthenticatorConfig::Basic {
                username: "user".into(),
                password: "pass".into(),
            },
            host_info: HostInfo::builder().host_default_data(host_data).build(),
            operation_timeout_secs: None,
            tls: config::TlsOptions::default(),
            configuration_name,
        }
    }

    #[test]
    fn shell_resource_uri_defaults_to_microsoft_powershell() {
        let config = config_with_configuration_name(None);
        assert_eq!(
            config.shell_resource_uri(),
            "http://schemas.microsoft.com/powershell/Microsoft.PowerShell"
        );
    }

    #[test]
    fn shell_resource_uri_uses_configuration_name() {
        let config = config_with_configuration_name(Some("MyJEAEndpoint".to_owned()));
        assert_eq!(
            config.shell_resource_uri(),
            "http://schemas.microsoft.com/powershell/MyJEAEndpoint"
        );
    }
}
