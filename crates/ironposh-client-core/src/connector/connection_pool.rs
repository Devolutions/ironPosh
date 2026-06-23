use std::collections::HashMap;
use tracing::{debug, error, info, instrument};

use crate::{
    PwshCoreError,
    connector::{
        Scheme, WinRmConfig,
        auth_sequence::{
            AuthSequence, AuthSequenceConfig, Authenticated, PostConAuthSequence,
            SecurityContextBuilderHolder, SspiAuthSequence,
        },
        encryption::{EncryptionOptions, EncryptionProvider},
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

    #[cfg(any(test, feature = "test-helpers"))]
    pub fn test_new(id: u32) -> Self {
        Self { id }
    }
}

// ============================= ConnectionState =============================
#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionState {
    /// SSPI only. Retains the queued SOAP so a TLS channel-binding challenge can
    /// restart auth on a fresh connection with the binding applied.
    PreAuth {
        queued_xml: String,
    },
    Idle {
        enc: EncryptionOptions,
    },
    Pending {
        enc: EncryptionOptions,
        queued_xml: String,
    },
    Closed,
}

#[derive(Debug)]
pub enum ConnectionPoolAccept {
    /// Plaintext SOAP envelope (after decrypt / state transition)
    Body(String),
    /// The previous request could not be accepted; caller must send these.
    SendBack(Vec<TrySend>),
}

// =============================== TrySend API ===============================
#[expect(clippy::large_enum_variant)]
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

impl TrySend {
    pub fn get_connection_id(&self) -> ConnectionId {
        match self {
            Self::JustSend { conn_id, .. } => *conn_id,
            Self::AuthNeeded { auth_sequence } => auth_sequence.conn_id,
        }
    }
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
            Self::JustSend { request, conn_id } => JustSendOut { request, conn_id },
            other @ Self::AuthNeeded { .. } => panic!("expected JustSend, got {other:?}"),
        }
    }
}

// ============================== ConnectionPool =============================
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    server: (ServerAddress, u16),
    scheme: Scheme,
}

impl From<&WinRmConfig> for ConnectionPoolConfig {
    fn from(w: &WinRmConfig) -> Self {
        Self {
            server: w.server.clone(),
            scheme: w.transport.scheme(),
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
    auth_seq_conf: AuthSequenceConfig,
    next_id: u32,
    sever_config: ServerConfig,
    /// `SEC_CHANNEL_BINDINGS` bytes (`tls-server-end-point`) learned from the
    /// server's TLS certificate after the first HTTPS challenge. Once set, every
    /// auth sequence this pool starts includes it (EPA). `None` over plain HTTP
    /// or before the first challenge.
    channel_binding: Option<Vec<u8>>,
}

impl ConnectionPool {
    pub fn new(cfg: ConnectionPoolConfig, sspi_cfg: AuthSequenceConfig) -> Self {
        Self {
            connections: HashMap::new(),
            auth_seq_conf: sspi_cfg,
            sever_config: ServerConfig {
                server: cfg.server.0,
                port: cfg.server.1,
                scheme: cfg.scheme,
            },
            next_id: 1,
            channel_binding: None,
        }
    }

    fn http_builder(&self) -> HttpBuilder {
        HttpBuilder::new(
            self.sever_config.server.clone(),
            self.sever_config.port,
            self.sever_config.scheme,
        )
    }

    /// Encrypts and builds a request on an Idle connection, or returns
    /// an AuthNeeded with a per-connection auth sequence for a fresh socket.
    #[instrument(skip(self, unencrypted_xml), fields(xml_length = unencrypted_xml.len()))]
    pub fn send(&mut self, unencrypted_xml: &str) -> Result<TrySend, PwshCoreError> {
        info!("ConnectionPool: processing send request");
        info!(unencrypted_soap = %unencrypted_xml, "outgoing unencrypted SOAP before encryption");

        if let Some((id, mut enc_opt)) = self.take_idle() {
            info!(
                conn_id = id.inner(),
                "found idle connection, preparing request"
            );

            let req = match &mut enc_opt {
                EncryptionOptions::Sspi {
                    encryption_provider,
                } => {
                    debug!(
                        conn_id = id.inner(),
                        "using SSPI encryption provider to encrypt outgoing XML"
                    );
                    // Over HTTP this seals the body; over HTTPS (unsealed) it returns
                    // the SOAP plain. The connection was authenticated once during the
                    // handshake and is now trusted (connection-oriented auth, RFC 4559),
                    // so no Authorization header is needed on this reused connection.
                    let body = encryption_provider.encrypt(unencrypted_xml)?;
                    self.http_builder().post(body)
                }
                EncryptionOptions::IncludeHeader { header } => {
                    debug!(
                        header,
                        conn_id = id.inner(),
                        "using Basic auth header to prepare outgoing XML"
                    );

                    self.http_builder()
                        .with_auth_header(header.clone())
                        .post(HttpBody::Xml(unencrypted_xml.to_owned()))
                }
            };

            self.connections.insert(
                id,
                ConnectionState::Pending {
                    enc: enc_opt,
                    queued_xml: unencrypted_xml.to_owned(),
                },
            );

            info!(
                ?req,
                conn_id = id.inner(),
                "connection moved to Pending state"
            );

            return Ok(TrySend::JustSend {
                request: req,
                conn_id: id,
            });
        }

        // No idle connection: open a fresh one and authenticate it. Over HTTPS
        // (unsealed) the very first operation rides the SPNEGO challenge legs, so the
        // handshake itself delivers it; the connection is then authenticated and every
        // subsequent operation is sent plain on the reused (idle) connection above.
        let id = self.alloc_new();
        info!(
            conn_id = id.inner(),
            "no idle connection, allocated new PreAuth connection for authentication"
        );

        // Build an engine (SSPI or Basic) from cfg and a fresh HttpBuilder.
        let seq = AuthSequence::new(
            &self.auth_seq_conf,
            self.http_builder(),
            self.channel_binding.clone(),
        )?;

        let (try_send, next_state) = match seq {
            AuthSequence::Sspi(sspi_auth_sequence) => {
                let try_send = sspi_auth_sequence.start(unencrypted_xml, id);
                let next_state = ConnectionState::PreAuth {
                    queued_xml: unencrypted_xml.to_owned(),
                };

                (try_send, next_state)
            }
            AuthSequence::Basic(mut basic_auth_sequence) => {
                let auth_header = basic_auth_sequence.get_auth_header();
                let try_send = basic_auth_sequence.start(unencrypted_xml, id);
                let next_state = ConnectionState::Pending {
                    enc: EncryptionOptions::IncludeHeader {
                        header: auth_header,
                    },
                    queued_xml: unencrypted_xml.to_owned(),
                };

                (try_send, next_state)
            }
        };

        self.connections.insert(id, next_state);

        Ok(try_send)
    }

    #[expect(clippy::too_many_lines)]
    #[instrument(skip(self, response), fields(
        conn_id = response.connection_id.inner(),
        status_code = response.response.status_code,
        body_length = response.response.body.len(),
        has_auth = response.authenticated.is_some()
    ))]
    pub fn accept(
        &mut self,
        response: HttpResponseTargeted,
    ) -> Result<ConnectionPoolAccept, PwshCoreError> {
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
            ConnectionState::PreAuth { queued_xml } => {
                info!(conn_id = connection_id.inner(), "handling PreAuth response");

                // EPA / channel binding: a server that enforces Extended
                // Protection (e.g. a DC over HTTPS) rejects the first auth leg
                // with 401 because the SSPI token carried no channel binding.
                // Now that the TLS handshake has surfaced the server certificate,
                // restart auth on a fresh connection with the `tls-server-end-point`
                // binding applied. We require only a TLS cert and that we have not
                // yet tried a binding (`channel_binding.is_none()` also bounds this
                // to a single retry); the `WWW-Authenticate` header is NOT required,
                // because some EPA rejections come back as a bare 401 + `Connection:
                // close` with no re-challenge. Without this, such a recoverable 401
                // would fall through to the terminal-401 guard below and fail hard.
                if response.status_code == 401
                    && self.channel_binding.is_none()
                    && response.peer_cert_der.is_some()
                {
                    *state = ConnectionState::Closed;

                    let cert_der = response
                        .peer_cert_der
                        .as_deref()
                        .expect("peer_cert_der checked above");
                    self.channel_binding = Some(
                        crate::connector::authenticator::tls_server_end_point_channel_bindings(
                            cert_der,
                        ),
                    );
                    info!(
                        conn_id = connection_id.inner(),
                        "TLS channel-binding challenge; restarting auth with EPA"
                    );

                    let id = self.alloc_new();
                    let seq = crate::connector::auth_sequence::AuthSequence::new(
                        &self.auth_seq_conf,
                        self.http_builder(),
                        self.channel_binding.clone(),
                    )?;
                    let try_send = match seq {
                        crate::connector::auth_sequence::AuthSequence::Sspi(sspi_auth_sequence) => {
                            let ts = sspi_auth_sequence.start(&queued_xml, id);
                            self.connections
                                .insert(id, ConnectionState::PreAuth { queued_xml });
                            ts
                        }
                        crate::connector::auth_sequence::AuthSequence::Basic(
                            mut basic_auth_sequence,
                        ) => {
                            let header = basic_auth_sequence.get_auth_header();
                            let ts = basic_auth_sequence.start(&queued_xml, id);
                            self.connections.insert(
                                id,
                                ConnectionState::Pending {
                                    enc: EncryptionOptions::IncludeHeader { header },
                                    queued_xml,
                                },
                            );
                            ts
                        }
                    };

                    return Ok(ConnectionPoolAccept::SendBack(vec![try_send]));
                }

                if let Some(encryption_provider) = encryption {
                    let AuthenticatedHttpChannel {
                        mut encryption_provider,
                        conn_id: _,
                    } = encryption_provider;

                    let body = encryption_provider.decrypt(response.body)?;
                    if response.status_code == 401 {
                        // Recoverable 401 challenges are consumed earlier (in the http
                        // client's auth loop and the channel-binding restart above). A
                        // 401 reaching here is a terminal rejection — e.g. unsealed SSPI
                        // refused over plain HTTP, or auth that simply failed. Surface it
                        // so the handshake fails fast instead of treating the empty body
                        // as success and stalling forever.
                        return reject_terminal_401(
                            connection_id,
                            response.status_code,
                            "server rejected authentication (HTTP 401)",
                        );
                    }
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
                        enc: EncryptionOptions::Sspi {
                            encryption_provider,
                        },
                    };

                    Ok(ConnectionPoolAccept::Body(body))
                } else {
                    // Unreachable
                    error!(
                        conn_id = connection_id.inner(),
                        "PreAuth response missing encryption provider"
                    );

                    Err(PwshCoreError::InvalidState(
                        "PreAuth response missing encryption provider",
                    ))
                }
            }
            ConnectionState::Pending {
                enc:
                    EncryptionOptions::Sspi {
                        mut encryption_provider,
                    },
                queued_xml,
            } => {
                info!(conn_id = connection_id.inner(), "handling Pending response");

                // If we get challenged again (401) while we believed we had an established
                // HTTP-SPNEGO-session-encrypted channel, we can no longer trust this
                // SSPI context for this logical connection. Retry the same queued SOAP on
                // a fresh authenticated channel.
                if response.status_code == 401
                    && response.body.is_empty()
                    && response
                        .headers
                        .iter()
                        .any(|(k, _)| k.eq_ignore_ascii_case("www-authenticate"))
                {
                    error!(
                        conn_id = connection_id.inner(),
                        status_code = response.status_code,
                        "server challenged encrypted Pending request; retrying on fresh connection"
                    );

                    // Keep the old connection closed and restart auth on a new one.
                    *state = ConnectionState::Closed;

                    let id = self.alloc_new();
                    info!(
                        conn_id = id.inner(),
                        "retry: allocated new PreAuth connection for authentication"
                    );

                    let seq = crate::connector::auth_sequence::AuthSequence::new(
                        &self.auth_seq_conf,
                        self.http_builder(),
                        self.channel_binding.clone(),
                    )?;

                    let (try_send, next_state) = match seq {
                        crate::connector::auth_sequence::AuthSequence::Sspi(sspi_auth_sequence) => {
                            let try_send = sspi_auth_sequence.start(&queued_xml, id);
                            let next_state = ConnectionState::PreAuth {
                                queued_xml: queued_xml.clone(),
                            };
                            (try_send, next_state)
                        }
                        crate::connector::auth_sequence::AuthSequence::Basic(
                            mut basic_auth_sequence,
                        ) => {
                            let auth_header = basic_auth_sequence.get_auth_header();
                            let try_send = basic_auth_sequence.start(&queued_xml, id);
                            let next_state = ConnectionState::Pending {
                                enc: EncryptionOptions::IncludeHeader {
                                    header: auth_header,
                                },
                                queued_xml,
                            };
                            (try_send, next_state)
                        }
                    };

                    self.connections.insert(id, next_state);

                    return Ok(ConnectionPoolAccept::SendBack(vec![try_send]));
                }

                let body = encryption_provider.decrypt(response.body)?;
                if response.status_code == 401 {
                    // The recoverable re-challenge case is handled above; a 401 here
                    // is a terminal auth rejection. Fail fast rather than stalling.
                    return reject_terminal_401(
                        connection_id,
                        response.status_code,
                        "server rejected authentication (HTTP 401)",
                    );
                }
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
                    enc: EncryptionOptions::Sspi {
                        encryption_provider,
                    },
                };
                Ok(ConnectionPoolAccept::Body(body))
            }
            ConnectionState::Pending { enc, queued_xml: _ } => {
                info!(
                    conn_id = connection_id.inner(),
                    "handling Pending response without encryption (Basic auth)"
                );

                if response.status_code == 401 {
                    // Basic credentials rejected (or Basic disabled on the listener).
                    // Terminal — fail fast instead of returning an empty body and
                    // stalling the handshake.
                    return reject_terminal_401(
                        connection_id,
                        response.status_code,
                        "server rejected Basic authentication (HTTP 401)",
                    );
                }
                if response.status_code >= 400 {
                    error!(
                        conn_id = connection_id.inner(),
                        status_code = response.status_code,
                        raw_error_body = ?response.body,
                        "server returned error response with raw body"
                    );
                }

                let string_body = response.body.as_str()?;

                *state = ConnectionState::Idle { enc };
                Ok(ConnectionPoolAccept::Body(string_body.to_owned()))
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

    // -------- internals --------
    fn alloc_new(&mut self) -> ConnectionId {
        let id = ConnectionId::new(self.next_id);
        self.next_id += 1;
        self.connections.insert(
            id,
            ConnectionState::PreAuth {
                queued_xml: String::new(),
            },
        );
        info!(
            conn_id = id.inner(),
            total_connections = self.connections.len(),
            "allocated new PreAuth connection"
        );
        id
    }

    /// Drop a connection's state from the pool without processing a response.
    ///
    /// Used to clean up a connection retired at disconnect time (e.g. the dying long-poll
    /// Receive, or a dropped reauth retry): its straggler is ignored rather than fed through
    /// [`Self::accept`], so without this its pool entry would stay `Pending` forever and leak
    /// across repeated disconnect/reconnect cycles.
    pub(crate) fn discard(&mut self, conn_id: ConnectionId) {
        if self.connections.remove(&conn_id).is_some() {
            debug!(
                conn_id = conn_id.inner(),
                remaining_connections = self.connections.len(),
                "discarded retired connection from pool"
            );
        }
    }

    /// Remove one Idle connection from the pool, returning its provider.
    fn take_idle(&mut self) -> Option<(ConnectionId, EncryptionOptions)> {
        let key = self
            .connections
            .iter()
            .find_map(|(id, st)| matches!(st, ConnectionState::Idle { .. }).then_some(*id))?;

        match self.connections.remove(&key) {
            Some(ConnectionState::Idle { enc }) => {
                info!(
                    conn_id = key.inner(),
                    remaining_connections = self.connections.len(),
                    ?enc,
                    "took idle connection from pool"
                );
                Some((key, enc))
            }
            Some(other) => {
                self.connections.insert(key, other);
                None
            }
            None => None,
        }
    }
}

/// Surface a terminal `401` (auth rejected after the recoverable re-challenge /
/// channel-binding paths) as an error, so the handshake fails fast instead of
/// treating the empty body as a successful response and stalling. Shared by the
/// PreAuth/SSPI, Pending/SSPI, and Pending/Basic arms of [`ConnectionPool::accept`].
fn reject_terminal_401(
    conn_id: ConnectionId,
    status_code: u16,
    detail: &'static str,
) -> Result<ConnectionPoolAccept, PwshCoreError> {
    error!(
        conn_id = conn_id.inner(),
        status_code, "authentication rejected by server (terminal 401)"
    );
    Err(PwshCoreError::Auth(detail))
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
    /// HTTPS (unsealed): the operation SOAP already rode the auth challenge legs
    /// and the server processed it, so there is NO separate request to send — the
    /// operation response is the last HTTP response already received during auth.
    AlreadyComplete {
        authenticated_http_channel_cert: AuthenticatedHttpChannel,
    },
}

impl PostConAuthSequence {
    pub fn prepare<'b>(&mut self) -> (&mut SspiAuthSequence, SecurityContextBuilderHolder<'b>) {
        (&mut self.auth_sequence, SecurityContextBuilderHolder::new())
    }

    pub fn process_sec_ctx_init(
        mut self,
        sec_context: &crate::connector::authenticator::SecContextInit,
    ) -> Result<SecContextInited, PwshCoreError> {
        // When sealing is off (HTTPS), the operation SOAP must ride the auth
        // challenge legs (the server rejects a token-less operation request).
        // Clone it so we can hand a copy to each leg without borrow conflicts.
        let operation_body =
            (!self.auth_sequence.require_encryption()).then(|| self.queued_xml.clone());

        match self
            .auth_sequence
            .process_initialized_sec_context(sec_context, operation_body.as_deref())?
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
                let Self {
                    auth_sequence,
                    queued_xml,
                    conn_id,
                } = self;

                let sealing = auth_sequence.require_encryption();
                let authenticated = auth_sequence.when_finish();

                let Authenticated {
                    mut encryption_provider,
                    mut http_builder,
                } = authenticated;

                if !sealing && token.is_none() {
                    // HTTPS with no final client token (e.g. Kerberos): the
                    // operation already rode the auth legs and the server
                    // processed it; the operation response is the last auth
                    // response. Nothing more to send.
                    return Ok(SecContextInited::AlreadyComplete {
                        authenticated_http_channel_cert: AuthenticatedHttpChannel {
                            encryption_provider,
                            conn_id,
                        },
                    });
                }

                // Send the operation as the final request: sealed body over HTTP,
                // plain body over HTTPS. Attach the final client token if present
                // — e.g. the NTLM AUTHENTICATE message, which must accompany the
                // operation request that completes the exchange.
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
