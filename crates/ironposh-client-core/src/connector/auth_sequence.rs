use std::fmt::Debug;

use sspi::{NegotiateConfig, ntlm::NtlmConfig};

use crate::{
    PwshCoreError,
    connector::{
        Scheme,
        authenticator::{
            SecContextMaybeInit, SecurityContextBuilder, SspiAuthenticator, SspiConext, SspiConfig,
            Token,
        },
        config::{AuthenticatorConfig, SspiAuthConfig},
        conntion_pool::ConnectionId,
        encryption::EncryptionProvider,
        http::{HttpBody, HttpBuilder, HttpRequest, HttpResponse},
    },
};

#[derive(Debug)]
pub enum SspiAuthContext {
    Ntlm(SspiConext<sspi::ntlm::Ntlm>),
    Kerberos(SspiConext<sspi::kerberos::Kerberos>),
    Negotiate(SspiConext<sspi::negotiate::Negotiate>),
}

pub struct SecurityContextBuilderHolder<'ctx> {
    ntlm: Option<SecurityContextBuilder<'ctx, sspi::ntlm::Ntlm>>,
    kerberos: Option<SecurityContextBuilder<'ctx, sspi::kerberos::Kerberos>>,
    negotiate: Option<SecurityContextBuilder<'ctx, sspi::negotiate::Negotiate>>,
}

impl<'ctx> SecurityContextBuilderHolder<'ctx> {
    pub(crate) fn new() -> Self {
        SecurityContextBuilderHolder {
            ntlm: None,
            kerberos: None,
            negotiate: None,
        }
    }

    pub fn as_mut_ntlm(&mut self) -> &mut Option<SecurityContextBuilder<'ctx, sspi::ntlm::Ntlm>> {
        &mut self.ntlm
    }

    pub fn as_mut_kerberos(
        &mut self,
    ) -> &mut Option<SecurityContextBuilder<'ctx, sspi::kerberos::Kerberos>> {
        &mut self.kerberos
    }

    pub fn as_mut_negotiate(
        &mut self,
    ) -> &mut Option<SecurityContextBuilder<'ctx, sspi::negotiate::Negotiate>> {
        &mut self.negotiate
    }

    pub fn clear(&mut self) {
        self.ntlm = None;
        self.kerberos = None;
        self.negotiate = None;
    }
}

impl SspiAuthContext {
    pub fn new(sspi_config: SspiAuthConfig) -> Result<Self, crate::PwshCoreError> {
        match sspi_config {
            SspiAuthConfig::NTLM {
                identity,
                target: target_name,
            } => SspiConext::new_ntlm(identity, SspiConfig::new(target_name))
                .map(SspiAuthContext::Ntlm),

            SspiAuthConfig::Kerberos {
                identity,
                kerberos_config,
                target: target_name,
            } => SspiConext::new_kerberos(
                identity,
                kerberos_config.into(),
                SspiConfig::new(target_name),
            )
            .map(SspiAuthContext::Kerberos),

            SspiAuthConfig::Negotiate {
                identity,
                kerberos_config,
                target: target_name,
            } => {
                let sspi_config = SspiConfig::new(target_name);

                let client_computer_name = whoami::fallible::hostname().map_err(|e| {
                    crate::PwshCoreError::InternalError(format!(
                        "Failed to get local hostname: {e}"
                    ))
                })?;

                let config = match kerberos_config {
                    Some(kerberos_config) => {
                        let kerberos_config: sspi::kerberos::config::KerberosConfig =
                            kerberos_config.into();

                        NegotiateConfig::from_protocol_config(
                            Box::new(kerberos_config),
                            client_computer_name,
                        )
                    }
                    None => {
                        let ntlm_config = NtlmConfig::new(client_computer_name.clone());

                        NegotiateConfig::from_protocol_config(
                            Box::new(ntlm_config),
                            client_computer_name,
                        )
                    }
                };

                SspiConext::new_negotiate(identity, config, sspi_config)
                    .map(SspiAuthContext::Negotiate)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthSequenceConfig {
    pub authenticator_config: AuthenticatorConfig,
}

impl AuthSequenceConfig {
    pub fn new(config: AuthenticatorConfig, _require_encryption: bool) -> Self {
        // require_encryption is now embedded in the AuthenticatorConfig::Sspi variant
        AuthSequenceConfig {
            authenticator_config: config,
        }
    }
}

pub struct SspiAuthSequence {
    context: SspiAuthContext,
    http_builder: HttpBuilder,
    require_encryption: bool,
}

pub enum SecCtxInited {
    Continue(HttpRequest),
    Done(Option<Token>),
}

impl Debug for SspiAuthSequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthSequence")
            .field("context", &"AnyAuthContext { ... }")
            .field("http_builder", &self.http_builder)
            .finish()
    }
}

impl SspiAuthSequence {
    pub(crate) fn new(
        sspi_auth_config: SspiAuthConfig,
        require_encryption: bool,
        http_builder: HttpBuilder,
    ) -> Result<Self, crate::PwshCoreError> {
        let context = SspiAuthContext::new(sspi_auth_config)?;
        Ok(SspiAuthSequence {
            context,
            http_builder,
            require_encryption,
        })
    }

    pub fn try_init_sec_context<'ctx, 'builder, 'generator>(
        &'ctx mut self,
        response: Option<&HttpResponse>,
        sec_ctx_holder: &'builder mut SecurityContextBuilderHolder<'ctx>,
    ) -> Result<SecContextMaybeInit<'generator>, PwshCoreError>
    where
        'ctx: 'builder,
        'builder: 'generator,
    {
        Ok(match &mut self.context {
            SspiAuthContext::Ntlm(auth_context) => SspiAuthenticator::try_init_sec_context(
                response,
                auth_context,
                sec_ctx_holder.as_mut_ntlm(),
                self.require_encryption,
            )?,
            SspiAuthContext::Kerberos(auth_context) => SspiAuthenticator::try_init_sec_context(
                response,
                auth_context,
                sec_ctx_holder.as_mut_kerberos(),
                self.require_encryption,
            )?,
            SspiAuthContext::Negotiate(auth_context) => SspiAuthenticator::try_init_sec_context(
                response,
                auth_context,
                sec_ctx_holder.as_mut_negotiate(),
                self.require_encryption,
            )?,
        })
    }

    pub fn resume<'a>(
        generator_holder: crate::connector::authenticator::GeneratorHolder<'a>,
        kdc_response: Vec<u8>,
    ) -> Result<SecContextMaybeInit<'a>, PwshCoreError> {
        SspiAuthenticator::resume(generator_holder, kdc_response)
    }

    pub(crate) fn process_initialized_sec_context(
        &mut self,
        sec_context: crate::connector::authenticator::SecContextInit,
    ) -> Result<SecCtxInited, PwshCoreError> {
        let res = match &mut self.context {
            SspiAuthContext::Ntlm(auth_context) => {
                SspiAuthenticator::process_initialized_sec_context(auth_context, sec_context)
            }
            SspiAuthContext::Kerberos(auth_context) => {
                SspiAuthenticator::process_initialized_sec_context(auth_context, sec_context)
            }
            SspiAuthContext::Negotiate(auth_context) => {
                SspiAuthenticator::process_initialized_sec_context(auth_context, sec_context)
            }
        }?;

        match res {
            super::authenticator::ActionReqired::TryInitSecContextAgain { token } => {
                self.http_builder.with_auth_header(token.0);
                Ok(SecCtxInited::Continue(
                    self.http_builder.post(HttpBody::empty()),
                ))
            }
            super::authenticator::ActionReqired::Done { token } => Ok(SecCtxInited::Done(token)),
        }
    }

    pub fn when_finish(self) -> Authenticated {
        let SspiAuthSequence {
            context,
            http_builder,
            require_encryption,
        } = self;

        Authenticated {
            encryption_provider: EncryptionProvider::new(context, require_encryption),
            http_builder,
        }
    }

    /// Start SSPI authentication sequence
    pub(crate) fn start(self, xml: &str, conn_id: ConnectionId) -> StartAuth {
        StartAuth::AuthNeeded {
            post: PostConAuthSequence {
                auth_sequence: self,
                queued_xml: xml.to_owned(),
                conn_id,
            },
        }
    }
}

pub struct Authenticated {
    pub(crate) encryption_provider: EncryptionProvider,
    pub(crate) http_builder: HttpBuilder,
}

// ============================================================================
// NEW ENUM-BASED AUTH SEQUENCE IMPLEMENTATION
// ============================================================================

/// Outcome for the caller (pool) when starting a send on a fresh connection.
pub enum StartAuth {
    /// Build and send a request right now (Basic).
    JustSend { request: HttpRequest },
    /// SSPI handshake required; the caller must drive `PostConAuthSequence`.
    AuthNeeded { post: PostConAuthSequence },
}

/// The post-connection state machine used for SSPI rounds.
#[derive(Debug)]
pub struct PostConAuthSequence {
    pub auth_sequence: SspiAuthSequence, 
    pub queued_xml: String,
    pub conn_id: ConnectionId,
}

/// Drives auth for a newly created connection.
#[derive(Debug)]
pub enum AuthSequence {
    Sspi(SspiAuthSequence),
    Basic(BasicAuthSequence),
}

/// Basic engine (new, zero-round)
#[derive(Debug)]
pub struct BasicAuthSequence {
    username: String,
    password: String,
    http_builder: HttpBuilder,
}

impl BasicAuthSequence {
    /// No handshake. Build a request with the Basic header and raw XML body.
    pub fn start(mut self, xml: &str, _conn_id: ConnectionId) -> StartAuth {
        self.http_builder.with_basic(&self.username, &self.password);
        let req = self.http_builder.post(HttpBody::Xml(xml.to_owned()));
        StartAuth::JustSend { request: req }
    }
}

impl AuthSequence {
    pub fn new(cfg: &AuthSequenceConfig, http: HttpBuilder) -> Result<Self, PwshCoreError> {
        match &cfg.authenticator_config {
            AuthenticatorConfig::Sspi {
                sspi,
                require_encryption,
            } => {
                let sspi_auth = SspiAuthSequence::new(sspi.clone(), *require_encryption, http)?;
                Ok(AuthSequence::Sspi(sspi_auth))
            }
            AuthenticatorConfig::Basic { username, password } => {
                Ok(AuthSequence::Basic(BasicAuthSequence {
                    username: username.clone(),
                    password: password.clone(),
                    http_builder: http,
                }))
            }
        }
    }

    pub(crate) fn start(self, xml: &str, conn_id: ConnectionId) -> StartAuth {
        match self {
            AuthSequence::Sspi(s) => s.start(xml, conn_id),
            AuthSequence::Basic(b) => b.start(xml, conn_id),
        }
    }
}
