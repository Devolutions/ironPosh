use std::collections::HashMap;
use tracing::{error, info, instrument};

use crate::{
    Authentication, PwshCoreError,
    connector::{
        Scheme, WinRmConfig,
        auth_sequence::{
            AuthConfig as SspiAuthCfg, AuthSequence, Authenticated, SecurityContextBuilderHolder,
        },
        encryption::EncryptionProvider,
        http::{
            HttpBody, HttpBuilder, HttpRequest, HttpRequestAction, HttpResponseTargeted,
            ServerAddress,
        },
    },
};

// ============================== ConnectionId ===============================
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ConnectionId {
    id: u32,
}

impl ConnectionId {
    fn new(id: u32) -> Self {
        Self { id }
    }

    pub fn inner(&self) -> u32 {
        self.id
    }
}

// ============================= ConnectionState =============================
#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionState {
    PreAuth,
    Idle {
        encryption_provider: EncryptionProvider,
    },
    Pending {
        encryption_provider: EncryptionProvider,
    },
    Closed,
}

// =============================== TrySend API ===============================
#[derive(Debug)]
pub enum TrySend {
    /// We had an Idle socket: body was sealed inside the pool and the conn was
    /// moved to Pending. Caller must send this on `conn_id`.
    JustSend {
        request: HttpRequest,
        conn_id: ConnectionId,
    },

    /// No Idle socket: the pool allocated a PreAuth socket and returns a
    /// per-connection auth sequence you must drive. When it finishes, call
    /// `ConnectionPool::auth_complete_and_send(...)` to seal the queued XML
    /// for this connection and get a `JustSend` back.
    AuthNeeded { auth_sequence: PostConAuthSequence },
}

// === Helper: unwrap a TrySend to JustSend during Connected handoff ===
pub trait TrySendExt {
    fn expect_just_send(self) -> JustSendOut;
}

pub struct JustSendOut {
    pub request: HttpRequest,
    pub conn_id: ConnectionId,
}

impl TrySendExt for TrySend {
    fn expect_just_send(self) -> JustSendOut {
        match self {
            TrySend::JustSend { request, conn_id } => JustSendOut { request, conn_id },
            other => panic!("expected JustSend, got {other:?}"),
        }
    }
}

// ============================== ConnectionPool =============================
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    pub server: (ServerAddress, u16),
    pub scheme: Scheme,
    pub authentication: Authentication,
    pub require_encryption: bool,
}

impl From<&WinRmConfig> for ConnectionPoolConfig {
    fn from(w: &WinRmConfig) -> Self {
        Self {
            server: w.server.clone(),
            scheme: w.scheme.clone(),
            authentication: w.authentication.clone(),
            require_encryption: w.require_encryption,
        }
    }
}

#[derive(Debug, Clone)]
struct ServerConfig {
    server: ServerAddress,
    port: u16,
    scheme: Scheme,
}

#[derive(Debug)]
pub struct ConnectionPool {
    connections: HashMap<ConnectionId, ConnectionState>,
    sspi_cfg: SspiAuthCfg,
    next_id: u32,
    sever_config: ServerConfig,
}

impl ConnectionPool {
    pub fn new(cfg: ConnectionPoolConfig, sspi_cfg: SspiAuthCfg) -> Self {
        Self {
            connections: HashMap::new(),
            sspi_cfg,
            sever_config: ServerConfig {
                server: cfg.server.0,
                port: cfg.server.1,
                scheme: cfg.scheme,
            },
            next_id: 1,
        }
    }

    fn http_builder(&self) -> HttpBuilder {
        HttpBuilder::new(
            self.sever_config.server.clone(),
            self.sever_config.port,
            self.sever_config.scheme.clone(),
        )
    }

    /// Encrypts and builds a request on an Idle connection, or returns
    /// an AuthNeeded with a per-connection auth sequence for a fresh socket.
    #[instrument(skip(self, unencrypted_xml), fields(xml_length = unencrypted_xml.len()))]
    pub fn send(&mut self, unencrypted_xml: &str) -> Result<TrySend, PwshCoreError> {
        info!("ConnectionPool: processing send request");
        info!(unencrypted_soap = %unencrypted_xml, "outgoing unencrypted SOAP before encryption");

        if let Some((id, mut enc)) = self.take_idle() {
            info!(
                conn_id = id.inner(),
                "found idle connection, encrypting and sending"
            );
            let body = enc.encrypt(unencrypted_xml)?;
            let req = self.http_builder().post(body);
            self.connections.insert(
                id,
                ConnectionState::Pending {
                    encryption_provider: enc,
                },
            );
            info!(conn_id = id.inner(), "connection moved to Pending state");
            return Ok(TrySend::JustSend {
                request: req,
                conn_id: id,
            });
        }

        // No idle socket → allocate PreAuth and hand out an auth FSM the caller will drive
        let id = self.alloc_pre_auth();
        info!(
            conn_id = id.inner(),
            "no idle connection, allocated new PreAuth connection for authentication"
        );
        let seq = AuthSequence::new(self.sspi_cfg.clone(), self.http_builder())?;

        let post = PostConAuthSequence {
            auth_sequence: seq,
            queued_xml: unencrypted_xml.to_owned(),
            conn_id: id,
        };

        info!(
            conn_id = id.inner(),
            "returning AuthNeeded for new connection"
        );
        Ok(TrySend::AuthNeeded {
            auth_sequence: post,
        })
    }

    #[instrument(skip(self, response), fields(
        conn_id = response.connection_id.inner(),
        status_code = response.response.status_code,
        body_length = response.response.body.len(),
        has_auth = response.authenticated.is_some()
    ))]
    pub fn accept(&mut self, response: HttpResponseTargeted) -> Result<String, PwshCoreError> {
        info!("ConnectionPool: processing server response");

        let HttpResponseTargeted {
            response,
            connection_id,
            authenticated: encryption,
        } = response;

        let Some(state) = self.connections.get_mut(&connection_id) else {
            error!(conn_id = connection_id.inner(), "unknown connection ID");
            return Err(PwshCoreError::InvalidState("Unknown connection"));
        };

        let in_progress_state = std::mem::replace(state, ConnectionState::Closed);
        info!(conn_id = connection_id.inner(), state = ?in_progress_state, "processing connection state");

        match in_progress_state {
            ConnectionState::PreAuth => {
                info!(conn_id = connection_id.inner(), "handling PreAuth response");
                let encryption_provider = encryption.ok_or_else(|| {
                    error!(
                        conn_id = connection_id.inner(),
                        "expected encryption provider after auth but none found"
                    );
                    PwshCoreError::InvalidState("Expected encryption provider after auth")
                })?;

                let AuthenticatedHttpChannel {
                    mut encryption_provider,
                    conn_id: _,
                } = encryption_provider;

                let body = encryption_provider.decrypt(response.body)?;
                if response.status_code >= 400 {
                    error!(
                        conn_id = connection_id.inner(),
                        status_code = response.status_code,
                        decrypted_error_body = %body,
                        "server returned error response with decrypted body"
                    );
                } else {
                    info!(
                        conn_id = connection_id.inner(),
                        decrypted_length = body.len(),
                        "decrypted PreAuth response, moving to Idle"
                    );
                }
                *state = ConnectionState::Idle {
                    encryption_provider,
                };
                Ok(body)
            }
            ConnectionState::Pending {
                mut encryption_provider,
            } => {
                info!(conn_id = connection_id.inner(), "handling Pending response");
                let body = encryption_provider.decrypt(response.body)?;
                if response.status_code >= 400 {
                    error!(
                        conn_id = connection_id.inner(),
                        status_code = response.status_code,
                        decrypted_error_body = %body,
                        "server returned error response with decrypted body"
                    );
                } else {
                    info!(
                        conn_id = connection_id.inner(),
                        decrypted_length = body.len(),
                        "decrypted Pending response, moving to Idle"
                    );
                }
                *state = ConnectionState::Idle {
                    encryption_provider,
                };
                Ok(body)
            }
            ConnectionState::Closed => {
                error!(conn_id = connection_id.inner(), "connection already closed");
                Err(PwshCoreError::InvalidState("Connection already closed"))
            }
            ConnectionState::Idle { .. } => {
                error!(
                    conn_id = connection_id.inner(),
                    "connection was idle when response received"
                );
                Err(PwshCoreError::InvalidState("Connection was idle"))
            }
        }
    }

    /// Decrypt a response body using the per-connection provider.
    fn decrypt(&mut self, id: &ConnectionId, body: HttpBody) -> Result<String, PwshCoreError> {
        match self.connections.get_mut(id) {
            Some(ConnectionState::Idle {
                encryption_provider: enc,
            }) => enc.decrypt(body),
            Some(ConnectionState::Pending {
                encryption_provider: enc,
            }) => enc.decrypt(body),
            Some(ConnectionState::PreAuth) => {
                Err(PwshCoreError::InvalidState("PreAuth has no decryptor"))
            }
            Some(ConnectionState::Closed) | None => {
                Err(PwshCoreError::InvalidState("Closed/unknown connection"))
            }
        }
    }

    // -------- internals --------
    fn alloc_pre_auth(&mut self) -> ConnectionId {
        let id = ConnectionId::new(self.next_id);
        self.next_id += 1;
        self.connections.insert(id, ConnectionState::PreAuth);
        info!(
            conn_id = id.inner(),
            total_connections = self.connections.len(),
            "allocated new PreAuth connection"
        );
        id
    }

    /// Remove one Idle connection from the pool, returning its provider.
    fn take_idle(&mut self) -> Option<(ConnectionId, EncryptionProvider)> {
        let key = self
            .connections
            .iter()
            .find_map(|(id, st)| matches!(st, ConnectionState::Idle { .. }).then_some(*id))?;

        match self.connections.remove(&key) {
            Some(ConnectionState::Idle {
                encryption_provider,
            }) => {
                info!(
                    conn_id = key.inner(),
                    remaining_connections = self.connections.len(),
                    "took idle connection from pool"
                );
                Some((key, encryption_provider))
            }
            Some(other) => {
                self.connections.insert(key, other);
                None
            }
            None => None,
        }
    }
}

// ============================ PostConAuthSequence ==========================
#[derive(Debug)]
pub struct PostConAuthSequence {
    auth_sequence: AuthSequence,
    queued_xml: String,
    conn_id: ConnectionId,
}

#[derive(Debug)]
pub struct AuthenticatedHttpChannel {
    pub(crate) encryption_provider: EncryptionProvider,
    pub(crate) conn_id: ConnectionId,
}

impl AuthenticatedHttpChannel {
    /// Gets the connection ID for this authenticated channel
    pub fn connection_id(&self) -> ConnectionId {
        self.conn_id
    }

    /// Extracts the encryption provider and connection ID, consuming the channel
    pub fn into_parts(self) -> (EncryptionProvider, ConnectionId) {
        (self.encryption_provider, self.conn_id)
    }
}

pub enum SecContextInited {
    Continue {
        request: HttpRequestAction,
        sequence: PostConAuthSequence,
    },
    SendRequest {
        request: HttpRequestAction,
        authenticated_http_channel_cert: AuthenticatedHttpChannel,
    },
}

impl PostConAuthSequence {
    pub fn prepare(&mut self) -> (&mut AuthSequence, SecurityContextBuilderHolder) {
        (&mut self.auth_sequence, SecurityContextBuilderHolder::new())
    }

    pub fn process_sec_ctx_init(
        mut self,
        sec_context: crate::connector::authenticator::SecContextInit,
    ) -> Result<SecContextInited, PwshCoreError> {
        match self
            .auth_sequence
            .process_initialized_sec_context(sec_context)?
        {
            super::auth_sequence::SecCtxInited::Continue(http_request) => {
                Ok(SecContextInited::Continue {
                    request: HttpRequestAction {
                        connection_id: self.conn_id,
                        request: http_request,
                    },
                    sequence: self,
                })
            }
            super::auth_sequence::SecCtxInited::Done(mut token) => {
                let PostConAuthSequence {
                    auth_sequence,
                    queued_xml,
                    conn_id,
                } = self;

                let authenticated = auth_sequence.when_finish();

                let Authenticated {
                    mut encryption_provider,
                    mut http_builder,
                } = authenticated;

                let body = encryption_provider.encrypt(&queued_xml)?;

                if let Some(token) = token.take() {
                    http_builder.with_auth_header(token.0);
                }

                let request = HttpRequestAction {
                    connection_id: conn_id,
                    request: http_builder.post(body),
                };

                Ok(SecContextInited::SendRequest {
                    request,
                    authenticated_http_channel_cert: AuthenticatedHttpChannel {
                        encryption_provider,
                        conn_id,
                    },
                })
            }
        }
    }
}
