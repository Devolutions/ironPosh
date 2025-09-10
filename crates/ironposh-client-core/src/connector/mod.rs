use std::{fmt::Debug, sync::Arc};

use base64::Engine;
use ironposh_psrp::HostInfo;
use ironposh_winrm::ws_management::WsMan;

// I'm lasy for now, just re-export from sspi
pub use sspi::{generator::NetworkRequest, network_client::NetworkProtocol};

use tracing::{info, instrument, warn};

use crate::{
    connector::{
        // auth_sequence::{AnyContext, AuthSequence, SecContextProcessResult, TryInitSecContext},,
        auth_sequence::{AuthSequence, EncryptionProvider},
        authenticator::Token,
        config::Authentication,
        http::{HttpBody, HttpBuilder, HttpRequest, HttpResponse, ServerAddress, ENCRYPTION_BUNDARY},
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
    sequence_number: u32,
}

impl Connector {
    pub fn new(config: ConnectorConfig) -> Self {
        Self {
            state: ConnectorState::Idle,
            config,
            encryption_provider: None,
            sequence_number: 0,
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
                            sequence: AuthSequence::new(sspi_auth, http_builder)?,
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

                // Decrypt body if encryption is enabled
                let body_string = if self.encryption_provider.is_some() {
                    match body {
                        HttpBody::Encrypted(mut encrypted_data) => {
                            let decrypted = self.decrypt(&mut encrypted_data)?;
                            String::from_utf8(decrypted).map_err(|e| {
                                crate::PwshCoreError::InternalError(format!(
                                    "Failed to decode decrypted body: {}",
                                    e
                                ))
                            })?
                        }
                        HttpBody::Xml(xml) => xml,
                        _ => {
                            return Err(crate::PwshCoreError::InvalidState(
                                "Expected encrypted or XML body in Connecting state",
                            ));
                        }
                    }
                } else {
                    match body {
                        HttpBody::Xml(xml) => xml,
                        _ => {
                            return Err(crate::PwshCoreError::InvalidState(
                                "Expected XML body in Connecting state",
                            ));
                        }
                    }
                };

                let mut runspace_pool = expect_shell_created.accept(body_string)?;

                let receive_request =
                    runspace_pool.fire_receive(DesiredStream::runspace_pool_streams())?;

                let body = if self.encryption_provider.is_some() {
                    self.encrypt(receive_request)?
                } else {
                    HttpBody::Xml(receive_request)
                };

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

                let soap_xml = if self.encryption_provider.is_some() {
                    match body {
                        HttpBody::Encrypted(mut encrypted_data) => {
                            let decrypted = self.decrypt(&mut encrypted_data)?;
                            String::from_utf8(decrypted).map_err(|e| {
                                crate::PwshCoreError::InternalError(format!(
                                    "Failed to decode decrypted body: {}",
                                    e
                                ))
                            })?
                        }
                        HttpBody::Xml(xml) => xml,
                        _ => {
                            return Err(crate::PwshCoreError::InvalidState(
                                "Expected encrypted or XML body in ConnectReceiveCycle state",
                            ));
                        }
                    }
                } else {
                    match body {
                        HttpBody::Xml(xml) => xml,
                        _ => {
                            return Err(crate::PwshCoreError::InvalidState(
                                "Expected XML body in ConnectReceiveCycle state",
                            ));
                        }
                    }
                };

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
                    let next_http_request =
                        http_builder.post("/wsman", HttpBody::Xml(next_receive_request));
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

    fn encrypt(&mut self, data: String) -> Result<HttpBody, crate::PwshCoreError> {
        let next_sequence_number = self.next_sequence_number();
        let Some(encryption_provider) = &mut self.encryption_provider else {
            return Err(crate::PwshCoreError::InvalidState(
                "No encryptor available for encryption",
            ));
        };

        // Length of the ORIGINAL SOAP (bytes, UTF-8) BEFORE sealing
        let plain_len = data.as_bytes().len();

        // This buffer will be sealed in-place by the provider
        let mut data_bytes = data.into_bytes();

        // `token` is the RFC4121 per-message header (e.g., ~16 bytes), **without** the length prefix
        let token = encryption_provider.wrap(&mut data_bytes, next_sequence_number)?;

        // Required 4-byte little-endian length prefix of the per-message header
        let token_len_le = (token.len() as u32).to_le_bytes();

        let mut body = Vec::new();
        // Part 1 (metadata only)
        // Part 1 (metadata only)
        body.extend_from_slice(format!("--{}\r\n", ENCRYPTION_BUNDARY).as_bytes());
        body.extend_from_slice(b"\tContent-Type: application/HTTP-SPNEGO-session-encrypted\r\n");
        body.extend_from_slice(
            format!(
                "OriginalContent: type=application/soap+xml;charset=UTF-8;Length={}\r\n",
                plain_len
            )
            .as_bytes(),
        );

        // Part 2 (binary)
        body.extend_from_slice(b"--Encrypted Boundary\r\n");
        body.extend_from_slice(b"\tContent-Type: application/octet-stream\r\n");
        body.extend_from_slice(b"\r\n");

        // GSS wrap: 4‑byte LE length + token + sealed payload
        body.extend_from_slice(&token_len_le);
        body.extend_from_slice(&token);
        body.extend_from_slice(&data_bytes);

        body.extend_from_slice(b"--Encrypted Boundary--\r\n");

        Ok(HttpBody::Encrypted(body))
    }

    fn decrypt(&mut self, data: &mut [u8]) -> Result<Vec<u8>, crate::PwshCoreError> {
        let next_sequence_number = self.next_sequence_number();
        let Some(decryptor) = &mut self.encryption_provider else {
            return Err(crate::PwshCoreError::InvalidState(
                "No decryptor available for decryption",
            ));
        };

        decryptor.unwrap(data, next_sequence_number)
    }

    fn next_sequence_number(&mut self) -> u32 {
        let result = self.sequence_number;
        // For now, always return 0
        self.sequence_number += 1;
        result
    }
}
