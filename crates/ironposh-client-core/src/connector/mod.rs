use std::{fmt::Debug, sync::Arc};

use base64::Engine;
use ironposh_psrp::HostInfo;
use ironposh_winrm::ws_management::WsMan;
use sspi::{KerberosConfig, NegotiateConfig, ntlm::NtlmConfig};
use tracing::{info, instrument, warn};

use crate::{
    connector::{
        auth_sequence::{AnyContext, AuthSequence, SecContextProcessResult, TryInitSecContext},
        authenticator::{AuthFurniture, GeneratorHolder, SecContextInit, SspiAuthenticator, Token},
        config::{Authentication, SspiAuthConfig},
        http::{HttpBuilder, HttpRequest, HttpResponse, ServerAddress},
    },
    runspace_pool::{
        pool::AcceptResponsResult, DesiredStream, ExpectShellCreated, RunspacePool, RunspacePoolCreator, RunspacePoolState
    },
};

pub use active_session::{ActiveSession, ActiveSessionOutput, UserOperation};
pub mod active_session;
pub mod auth_sequence;
pub mod authenticator;
pub mod config;
pub mod http;

#[derive(Debug, Clone)]
pub enum Scheme {
    Http,
    Https,
}

#[derive(Debug, Clone)]
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

fn new_ctx_and_seq<'conn, 'auth>(
    connector: &'conn mut Connector,
    config: SspiAuthConfig,
) -> Result<(AnyContext<'auth>, AuthSequence<'conn>), crate::PwshCoreError> {
    let new = match config {
        SspiAuthConfig::NTLM { identity } => {
            let context = AuthFurniture::new_ntlm(identity)?;
            let sequnece = AuthSequence::new(connector);
            (AnyContext::Ntlm(context), sequnece)
        }
        SspiAuthConfig::Kerberos {
            identity,
            kerberos_config,
        } => {
            let context = AuthFurniture::new_kerberos(
                identity.clone(),
                KerberosConfig {
                    client_computer_name: kerberos_config.client_computer_name,
                    kdc_url: kerberos_config.kdc_url,
                },
            )?;
            let sequnece = AuthSequence::new(connector);

            (AnyContext::Kerberos(context), sequnece)
        }
        SspiAuthConfig::Negotiate {
            identity,
            kerberos_config,
        } => {
            let ntlm_config = NtlmConfig::default();
            let kerberos_config = kerberos_config.clone().map(|k| KerberosConfig {
                client_computer_name: k.client_computer_name,
                kdc_url: k.kdc_url,
            });

            let negotiate_config = match kerberos_config {
                Some(k) => {
                    let client_computer_name = k
                        .client_computer_name
                        .as_deref()
                        .unwrap_or("ironposh-client")
                        .to_string();
                    NegotiateConfig::from_protocol_config(Box::new(k), client_computer_name)
                }
                None => {
                    let client_computer_name = ntlm_config
                        .client_computer_name
                        .as_deref()
                        .unwrap_or("ironposh-client")
                        .to_string();
                    NegotiateConfig::from_protocol_config(
                        Box::new(ntlm_config),
                        client_computer_name,
                    )
                }
            };

            let context = AuthFurniture::new_negotiate(identity.clone(), negotiate_config)?;
            let sequnece = AuthSequence::new(connector);
            (AnyContext::Negotiate(context), sequnece)
        }
    };

    Ok(new)
}

enum InnerConnectorStepResult {
    SendBack(HttpRequest<String>),
    SendBackError(crate::PwshCoreError),
    Borrowed {
        http_builder: HttpBuilder,
        sspi_auth: SspiAuthConfig,
    },
    Connected {
        /// use box to avoid large enum variant
        active_session: Box<ActiveSession>,
        next_receive_request: HttpRequest<String>,
    },
}

#[derive(Debug)]
pub enum ConnectorStepResult<'conn, 'auth> {
    SendBack(HttpRequest<String>),
    SendBackError(crate::PwshCoreError),
    Borrowed {
        context: AnyContext<'auth>,
        sequence: AuthSequence<'conn>,
        http_builder: HttpBuilder,
    },
    Connected {
        /// use box to avoid large enum variant
        active_session: Box<ActiveSession>,
        next_receive_request: HttpRequest<String>,
    },
}

impl<'conn, 'auth> ConnectorStepResult<'conn, 'auth> {
    pub fn name(&self) -> &'static str {
        match self {
            ConnectorStepResult::SendBack(_) => "SendBack",
            ConnectorStepResult::SendBackError(_) => "SendBackError",
            ConnectorStepResult::Connected { .. } => "Connected",
            ConnectorStepResult::Borrowed { .. } => "Borrowed",
        }
    }
}

impl<'conn, 'auth> ConnectorStepResult<'conn, 'auth> {
    pub fn priority(&self) -> u8 {
        match self {
            ConnectorStepResult::SendBack(_) => 0,
            ConnectorStepResult::SendBackError(_) => 1,
            ConnectorStepResult::Connected { .. } => 2,
            ConnectorStepResult::Borrowed { .. } => 3,
        }
    }
}

#[derive(Default, Debug)]
pub enum ConnectorState {
    Idle,
    #[default]
    Taken,
    AuthenticateInProgress {},
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
            ConnectorState::AuthenticateInProgress { .. } => "Authenticate",
            ConnectorState::ConnectReceiveCycle { .. } => "ConnectReceiveCycle",
            ConnectorState::Connected => "Connected",
            ConnectorState::Failed => "Failed",
        }
    }
}

#[derive(Debug)]
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

    pub fn authenticate(
        &mut self,
        last_token: Option<Token>,
        mut http_builder: HttpBuilder,
    ) -> Result<HttpRequest<String>, crate::PwshCoreError> {
        match self.state {
            ConnectorState::AuthenticateInProgress {} => {}
            _ => {
                return Err(crate::PwshCoreError::InvalidState(
                    "Connector is not in Authenticate state",
                ));
            }
        };

        if let Some(token) = last_token {
            http_builder.with_auth_header(token.0);
        }

        let connection = Arc::new(WsMan::builder().to(self.config.wsman_to(None)).build());

        let runspace_pool = RunspacePoolCreator::builder()
            .host_info(self.config.host_info.clone())
            .build()
            .into_runspace_pool(connection);

        let (xml_body, expect_shell_created) = runspace_pool.open()?;

        let request = http_builder.post("/wsman", xml_body);

        self.set_state(ConnectorState::Connecting {
            expect_shell_created,
            http_builder,
        });

        Ok(request)
    }

    #[instrument(skip(self, server_response), name = "Connector::step")]
    pub fn step<'conn, 'auth>(
        &'conn mut self,
        server_response: Option<HttpResponse<String>>,
    ) -> Result<ConnectorStepResult<'conn, 'auth>, crate::PwshCoreError> {
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

                let mut http_builder = HttpBuilder::new(
                    self.config.server.0.clone(),
                    self.config.server.1,
                    self.config.scheme.clone(),
                );

                // if matches!(self.config.authentication, Authentication::Basic { .. }) {
                match self.config.authentication.clone() {
                    Authentication::Basic { username, password } => {
                        let auth_header = format!(
                            "Basic {}",
                            base64::engine::general_purpose::STANDARD
                                .encode(format!("{}:{}", username, password))
                        );

                        http_builder.with_auth_header(auth_header);

                        let connection =
                            Arc::new(WsMan::builder().to(self.config.wsman_to(None)).build());

                        let runspace_pool = RunspacePoolCreator::builder()
                            .host_info(self.config.host_info.clone())
                            .build()
                            .into_runspace_pool(connection);

                        let (xml_body, expect_shell_created) = runspace_pool.open()?;

                        let response = http_builder.post("/wsman", xml_body);

                        let new_state = ConnectorState::Connecting {
                            expect_shell_created,
                            http_builder,
                        };

                        (new_state, InnerConnectorStepResult::SendBack(response))
                    }
                    Authentication::Sspi(sspi_auth) => (
                        ConnectorState::AuthenticateInProgress {},
                        InnerConnectorStepResult::Borrowed {
                            http_builder,
                            sspi_auth,
                        },
                    ),
                }
            }
            ConnectorState::AuthenticateInProgress { .. } => {
                return Err(crate::PwshCoreError::InvalidState(
                    "Connector is already in AuthenticateInProgress state, should use AuthSequence",
                ));
            }
            ConnectorState::Connecting {
                expect_shell_created,
                mut http_builder,
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

                (new_state, InnerConnectorStepResult::SendBack(response))
            }
            ConnectorState::ConnectReceiveCycle {
                mut runspace_pool,
                mut http_builder,
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
                    (new_state, InnerConnectorStepResult::SendBack(response))
                } else if let RunspacePoolState::Opened = runspace_pool.state {
                    info!("Connection established successfully - returning ActiveSession");
                    let next_receive_request = runspace_pool.fire_receive(desired_streams)?;
                    let next_http_request = http_builder.post("/wsman", next_receive_request);
                    let active_session = ActiveSession::new(runspace_pool, http_builder);
                    (
                        ConnectorState::Connected,
                        InnerConnectorStepResult::Connected {
                            active_session: Box::new(active_session),
                            next_receive_request: next_http_request,
                        },
                    )
                } else {
                    warn!("Unexpected RunspacePool state: {:?}", runspace_pool.state);
                    (
                        ConnectorState::Failed,
                        InnerConnectorStepResult::SendBackError(
                            crate::PwshCoreError::InvalidState("Unexpected RunspacePool state"),
                        ),
                    )
                }
            }
        };

        self.set_state(new_state);

        let response = match response {
            InnerConnectorStepResult::SendBack(req) => ConnectorStepResult::SendBack(req),
            InnerConnectorStepResult::SendBackError(err) => ConnectorStepResult::SendBackError(err),
            InnerConnectorStepResult::Borrowed {
                http_builder,
                sspi_auth,
            } => {
                // Avoid &mut self borrow issue by creating the auth sequence here
                let (context, sequence) = new_ctx_and_seq(self, sspi_auth)?;
                ConnectorStepResult::Borrowed {
                    context,
                    sequence,
                    http_builder,
                }
            }
            InnerConnectorStepResult::Connected {
                active_session,
                next_receive_request,
            } => ConnectorStepResult::Connected {
                active_session,
                next_receive_request,
            },
        };

        Ok(response)
    }
}
