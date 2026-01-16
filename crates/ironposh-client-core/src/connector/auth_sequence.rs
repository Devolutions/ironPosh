use std::fmt::Debug;

use base64::Engine;
use sspi::{NegotiateConfig, ntlm::NtlmConfig};

use crate::{
    PwshCoreError,
    connector::{
        authenticator::{
            SecContextMaybeInit, SecurityContextBuilder, SspiAuthenticator, SspiConfig,
            SspiContext, Token,
        },
        config::{AuthenticatorConfig, SspiAuthConfig},
        conntion_pool::{ConnectionId, TrySend},
        encryption::EncryptionProvider,
        http::{HttpBody, HttpBuilder, HttpRequest, HttpResponse},
    },
};

#[expect(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum SspiAuthContext {
    Ntlm(SspiContext<sspi::ntlm::Ntlm>),
    Kerberos(SspiContext<sspi::kerberos::Kerberos>),
    Negotiate(SspiContext<sspi::negotiate::Negotiate>),
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
            } => SspiContext::new_ntlm(identity, SspiConfig::new(target_name))
                .map(SspiAuthContext::Ntlm),

            SspiAuthConfig::Kerberos {
                identity,
                kerberos_config,
                target: target_name,
            } => SspiContext::new_kerberos(
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

                let config = if let Some(kerberos_config) = kerberos_config {
                    let kerberos_config: sspi::kerberos::config::KerberosConfig =
                        kerberos_config.into();

                    NegotiateConfig::from_protocol_config(
                        Box::new(kerberos_config),
                        client_computer_name,
                    )
                } else {
                    let ntlm_config = NtlmConfig::new(client_computer_name.clone());

                    NegotiateConfig::from_protocol_config(
                        Box::new(ntlm_config),
                        client_computer_name,
                    )
                };

                SspiContext::new_negotiate(identity, config, sspi_config)
                    .map(SspiAuthContext::Negotiate)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthSequenceConfig {
    pub authenticator_config: AuthenticatorConfig,
    /// Whether SSPI message sealing is required (derived from TransportSecurity)
    pub require_sspi_sealing: bool,
}

impl AuthSequenceConfig {
    pub fn new(config: AuthenticatorConfig, require_sspi_sealing: bool) -> Self {
        Self {
            authenticator_config: config,
            require_sspi_sealing,
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

#[expect(clippy::missing_fields_in_debug)]
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
        Ok(Self {
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

    pub fn resume(
        generator_holder: crate::connector::authenticator::GeneratorHolder<'_>,
        kdc_response: Vec<u8>,
    ) -> Result<SecContextMaybeInit<'_>, PwshCoreError> {
        SspiAuthenticator::resume(generator_holder, kdc_response)
    }

    pub(crate) fn process_initialized_sec_context(
        &mut self,
        sec_context: &crate::connector::authenticator::SecContextInit,
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
        let Self {
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
    pub(crate) fn start(self, xml: &str, conn_id: ConnectionId) -> TrySend {
        TrySend::AuthNeeded {
            auth_sequence: PostConAuthSequence {
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

/// The post-connection state machine used for SSPI rounds.
#[derive(Debug)]
pub struct PostConAuthSequence {
    pub auth_sequence: SspiAuthSequence,
    pub queued_xml: String,
    pub conn_id: ConnectionId,
}

/// Drives auth for a newly created connection.
#[expect(clippy::large_enum_variant)]
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
    pub fn get_auth_header(&self) -> String {
        let creds = format!("{}:{}", self.username, self.password);
        let b64 = base64::engine::general_purpose::STANDARD.encode(creds.as_bytes());
        format!("Basic {b64}")
    }

    pub fn start(&mut self, xml: &str, connection_id: ConnectionId) -> TrySend {
        self.http_builder.with_auth_header(self.get_auth_header());
        let request = self.http_builder.post(HttpBody::Xml(xml.to_owned()));
        TrySend::JustSend {
            request,
            conn_id: connection_id,
        }
    }
}

impl AuthSequence {
    pub fn new(cfg: &AuthSequenceConfig, http: HttpBuilder) -> Result<Self, PwshCoreError> {
        match &cfg.authenticator_config {
            AuthenticatorConfig::Sspi(sspi) => {
                let sspi_auth =
                    SspiAuthSequence::new(sspi.clone(), cfg.require_sspi_sealing, http)?;
                Ok(Self::Sspi(sspi_auth))
            }
            AuthenticatorConfig::Basic { username, password } => {
                Ok(Self::Basic(BasicAuthSequence {
                    username: username.clone(),
                    password: password.clone(),
                    http_builder: http,
                }))
            }
        }
    }
}
