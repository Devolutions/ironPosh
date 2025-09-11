use std::{fmt::Debug, sync::Arc};

use base64::Engine;
use ironposh_psrp::HostInfo;
use ironposh_winrm::ws_management::WsMan;

// I'm lasy for now, just re-export from sspi
pub use sspi::{generator::NetworkRequest, network_client::NetworkProtocol};

use tracing::{debug, info, instrument, warn};

use crate::{
    PwshCoreError,
    connector::{
        // auth_sequence::{AnyContext, AuthSequence, SecContextProcessResult, TryInitSecContext},,
        auth_sequence::{AuthConfig, AuthSequence},
        authenticator::Token,
        config::Authentication,
        encryption::EncryptionProvider,
        http::{HttpBody, HttpBuilder, HttpRequest, HttpResponse, ServerAddress},
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
pub mod encryption;
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
    pub require_encryption: bool,
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
    SendBack(HttpRequest),
    SendBackError(crate::PwshCoreError),
    Auth {
        sequence: AuthSequence,
    },
    Connected {
        /// use box to avoid large enum variant
        active_session: Box<ActiveSession>,
        next_receive_request: HttpRequest,
    },
}

impl ConnectorStepResult {
    pub fn name(&self) -> &'static str {
        match self {
            ConnectorStepResult::SendBack(_) => "SendBack",
            ConnectorStepResult::SendBackError(_) => "SendBackError",
            ConnectorStepResult::Connected { .. } => "Connected",
            ConnectorStepResult::Auth { .. } => "Auth",
        }
    }
}

impl ConnectorStepResult {
    pub fn priority(&self) -> u8 {
        match self {
            ConnectorStepResult::Auth { .. } => 0,
            ConnectorStepResult::SendBack(_) => 1,
            ConnectorStepResult::SendBackError(_) => 2,
            ConnectorStepResult::Connected { .. } => 3,
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
    encryption_provider: Option<EncryptionProvider>,
}

impl Connector {
    pub fn new(config: ConnectorConfig) -> Self {
        Self {
            state: ConnectorState::Idle,
            config,
            encryption_provider: None,
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
        decryptor: EncryptionProvider,
    ) -> Result<HttpRequest, crate::PwshCoreError> {
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
        self.encryption_provider = Some(decryptor);

        let connection = Arc::new(WsMan::builder().to(self.config.wsman_to(None)).build());

        let runspace_pool = RunspacePoolCreator::builder()
            .host_info(self.config.host_info.clone())
            .build()
            .into_runspace_pool(connection);

        let (xml_body, expect_shell_created) = runspace_pool.open()?;

        let body = if self.encryption_provider.is_some() {
            self.encrypt(xml_body)?
        } else {
            HttpBody::Xml(xml_body)
        };

        let request = http_builder.post("/wsman", body);

        self.set_state(ConnectorState::Connecting {
            expect_shell_created,
            http_builder,
        });

        Ok(request)
    }

    #[instrument(skip(self, server_response), name = "Connector::step")]
    pub fn step(
        &mut self,
        server_response: Option<HttpResponse>,
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
                                .encode(format!("{username}:{password}"))
                        );

                        http_builder.with_auth_header(auth_header);

                        let connection =
                            Arc::new(WsMan::builder().to(self.config.wsman_to(None)).build());

                        let runspace_pool = RunspacePoolCreator::builder()
                            .host_info(self.config.host_info.clone())
                            .build()
                            .into_runspace_pool(connection);

                        let (xml_body, expect_shell_created) = runspace_pool.open()?;

                        let response = http_builder.post("/wsman", HttpBody::Xml(xml_body));

                        let new_state = ConnectorState::Connecting {
                            expect_shell_created,
                            http_builder,
                        };

                        (new_state, ConnectorStepResult::SendBack(response))
                    }
                    Authentication::Sspi(sspi_auth) => (
                        ConnectorState::AuthenticateInProgress {},
                        ConnectorStepResult::Auth {
                            sequence: AuthSequence::new(
                                AuthConfig {
                                    sspi_config: sspi_auth,
                                    require_encryption: self.config.require_encryption,
                                },
                                http_builder,
                            )?,
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

                debug!(
                    status_code = response.status_code,
                    headers_count = response.headers.len(),
                    "Received response in Connecting state"
                );

                let body = response.body.ok_or({
                    crate::PwshCoreError::InvalidState("Expected a body in Connecting state")
                })?;

                debug!(
                    body_type = ?body,
                    "Processing response body"
                );

                let body_string = self.decrypt(body)?;
                debug!(decrypted_body = %body_string, "Decrypted body");

                let mut runspace_pool = expect_shell_created.accept(body_string)?;

                let receive_request =
                    runspace_pool.fire_receive(DesiredStream::runspace_pool_streams())?;

                let body = self.encrypt(receive_request)?;

                let response = http_builder.post("/wsman", body);

                let new_state = ConnectorState::ConnectReceiveCycle {
                    runspace_pool,
                    http_builder,
                };

                (new_state, ConnectorStepResult::SendBack(response))
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

                let soap_xml = self.decrypt(body)?;

                let accept_response_results = runspace_pool.accept_response(soap_xml)?;

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
                    let response = http_builder.post("/wsman", HttpBody::Xml(receive_request));
                    let new_state = ConnectorState::ConnectReceiveCycle {
                        runspace_pool,
                        http_builder,
                    };
                    (new_state, ConnectorStepResult::SendBack(response))
                } else if let RunspacePoolState::Opened = runspace_pool.state {
                    info!("Connection established successfully - returning ActiveSession");
                    let next_receive_request = runspace_pool.fire_receive(desired_streams)?;
                    let body = self.encrypt(next_receive_request)?;
                    let next_http_request = http_builder.post("/wsman", body);
                    let active_session = ActiveSession::new(runspace_pool, http_builder);
                    (
                        ConnectorState::Connected,
                        ConnectorStepResult::Connected {
                            active_session: Box::new(active_session),
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

    #[instrument(skip(self, data))]
    pub fn encrypt(&mut self, data: String) -> Result<HttpBody, PwshCoreError> {
        debug!(to_be_encrypted = data, "Starting encryption process");
        let enc = self.encryption_provider.as_mut().ok_or_else(|| {
            crate::PwshCoreError::InvalidState("No encryption provider available")
        })?;

        enc.encrypt(data)
    }

    #[instrument(skip(self, data))]
    fn decrypt(&mut self, data: HttpBody) -> Result<String, crate::PwshCoreError> {
        debug!(
            body_type = ?data,
            "Starting decryption process"
        );

        let decryptor =
            self.encryption_provider
                .as_mut()
                .ok_or(crate::PwshCoreError::InvalidState(
                    "No decryptor available for decryption",
                ))?;

        let result = decryptor.decrypt(data);

        match &result {
            Ok(decrypted) => {
                debug!(
                    decrypted_len = decrypted.len(),
                    "Decryption completed successfully"
                );
            }
            Err(e) => {
                debug!(
                    error = %e,
                    "Decryption failed"
                );
            }
        }

        result
    }
}
