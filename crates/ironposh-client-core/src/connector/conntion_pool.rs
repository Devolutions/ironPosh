use std::collections::HashMap;

use crate::{
    Authentication, PwshCoreError,
    connector::{
        Scheme, WinRmConfig,
        auth_sequence::{
            self, AuthConfig as SspiAuthCfg, AuthSequence, Authenticated,
            SecurityContextBuilderHolder,
        },
        authenticator::Token,
        encryption::EncryptionProvider,
        http::{HttpBody, HttpBuilder, HttpRequest, HttpRequestAction, ServerAddress},
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
            other => panic!("expected JustSend, got {:?}", other),
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
    pub fn send(&mut self, unencrypted_xml: &str) -> Result<TrySend, PwshCoreError> {
        if let Some((id, mut enc)) = self.take_idle() {
            let body = enc.encrypt(unencrypted_xml)?;
            let req = self.http_builder().post(body);
            self.connections.insert(
                id,
                ConnectionState::Pending {
                    encryption_provider: enc,
                },
            );
            return Ok(TrySend::JustSend {
                request: req,
                conn_id: id,
            });
        }

        // No idle socket → allocate PreAuth and hand out an auth FSM the caller will drive
        let id = self.alloc_pre_auth();
        let seq = AuthSequence::new(self.sspi_cfg.clone(), self.http_builder())?;

        let post = PostConAuthSequence {
            auth_sequence: seq,
            queued_xml: unencrypted_xml.to_owned(),
            conn_id: id,
        };

        Ok(TrySend::AuthNeeded {
            auth_sequence: post,
        })
    }

    /// Mark a connection Pending→Idle after its HTTP response has been fully processed.
    pub fn on_response_mark_idle(&mut self, id: &ConnectionId) {
        if let Some(state) = self.connections.remove(id) {
            if let ConnectionState::Pending {
                encryption_provider: enc,
            } = state
            {
                self.connections.insert(
                    *id,
                    ConnectionState::Idle {
                        encryption_provider: enc,
                    },
                );
            } else {
                // keep original if not pending
                self.connections.insert(*id, state);
            }
        }
    }

    /// Decrypt a response body using the per-connection provider.
    pub fn decrypt(&mut self, id: &ConnectionId, body: HttpBody) -> Result<String, PwshCoreError> {
        match self.connections.get_mut(id) {
            Some(ConnectionState::Idle {
                encryption_provider: enc,
            }) => enc.decrypt(body),
            Some(ConnectionState::Pending {
                encryption_provider: enc,
            }) => enc.decrypt(body),
            Some(ConnectionState::PreAuth) => Err(PwshCoreError::InvalidState(
                "PreAuth has no decryptor".into(),
            )),
            Some(ConnectionState::Closed) | None => Err(PwshCoreError::InvalidState(
                "Closed/unknown connection".into(),
            )),
        }
    }

    /// After the caller finishes the per-connection auth FSM, install the
    /// provider and return the first sealed request for this connection.
    pub fn auth_complete_and_send(
        &mut self,
        id: ConnectionId,
        authenticated: Authenticated, // decryptor + http_builder
        last_token: Option<Vec<u8>>,  // if your handshake wants an Authorization header
        xml_to_send: String,
    ) -> Result<TrySend, PwshCoreError> {
        // install provider
        let Authenticated {
            encryption_provider: decryptor,
            mut http_builder,
        } = authenticated;
        if let Some(tok) = last_token {
            http_builder.with_auth_header(String::from_utf8_lossy(&tok).to_string());
        }
        // mark this connection Pending with its provider
        self.connections.insert(
            id,
            ConnectionState::Pending {
                encryption_provider: decryptor,
            },
        );
        // seal the queued xml on this connection
        // (we temporarily borrow the provider to encrypt, then put it back as Pending)
        let enc = match self.connections.remove(&id) {
            Some(ConnectionState::Pending {
                encryption_provider: enc,
            }) => enc,
            other => {
                // unexpected; restore original
                if let Some(st) = other {
                    self.connections.insert(id, st);
                }
                return Err(PwshCoreError::InvalidState(
                    "auth_complete_and_send on non-Pending".into(),
                ));
            }
        };
        let mut enc2 = enc;
        let body = enc2.encrypt(&xml_to_send)?;
        let req = http_builder.post(body);
        self.connections.insert(
            id,
            ConnectionState::Pending {
                encryption_provider: enc2,
            },
        );
        Ok(TrySend::JustSend {
            request: req,
            conn_id: id,
        })
    }

    // -------- internals --------
    fn alloc_pre_auth(&mut self) -> ConnectionId {
        let id = ConnectionId::new(self.next_id);
        self.next_id += 1;
        self.connections.insert(id, ConnectionState::PreAuth);
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
            }) => Some((key, encryption_provider)),
            Some(other) => {
                self.connections.insert(key, other);
                None
            }
            None => None,
        }
    }

    pub(crate) fn mark_authenticated(
        &mut self,
        AuthenticatedHttpChannel {
            conn_id,
            encryption_provider,
            ..
        }: AuthenticatedHttpChannel,
    ) -> Result<(), PwshCoreError> {
        let Some(state) = self.connections.get_mut(&conn_id) else {
            return Err(PwshCoreError::InvalidState("unknown connection".into()));
        };

        if !matches!(state, ConnectionState::PreAuth) {
            return Err(PwshCoreError::InvalidState(
                "mark_authenticated on non-PreAuth".into(),
            ));
        }

        *state = ConnectionState::Idle {
            encryption_provider,
        };

        Ok(())
    }
}

// ============================ PostConAuthSequence ==========================
#[derive(Debug)]
pub struct PostConAuthSequence {
    auth_sequence: AuthSequence,
    queued_xml: String,
    conn_id: ConnectionId,
}

pub struct AuthenticatedHttpChannel {
    pub(crate) encryption_provider: EncryptionProvider,
    pub(crate) conn_id: ConnectionId,
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
