use std::fmt::Debug;

use sspi::{NegotiateConfig, ntlm::NtlmConfig};

use crate::{
    PwshCoreError,
    connector::{
        authenticator::{
            SecContextMaybeInit, SecurityContextBuilder, SspiAuthenticator, SspiConext, SspiConfig,
            Token,
        },
        config::{Authentication, SspiAuthConfig},
        encryption::EncryptionProvider,
        http::{HttpBody, HttpBuilder, HttpRequest, HttpResponse},
    },
};

#[derive(Debug)]
pub enum AuthContext {
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

impl AuthContext {
    pub fn new(sspi_config: SspiAuthConfig) -> Result<Self, crate::PwshCoreError> {
        match sspi_config {
            SspiAuthConfig::NTLM {
                identity,
                target: target_name,
            } => {
                SspiConext::new_ntlm(identity, SspiConfig::new(target_name)).map(AuthContext::Ntlm)
            }

            SspiAuthConfig::Kerberos {
                identity,
                kerberos_config,
                target: target_name,
            } => SspiConext::new_kerberos(
                identity,
                kerberos_config.into(),
                SspiConfig::new(target_name),
            )
            .map(AuthContext::Kerberos),

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

                SspiConext::new_negotiate(identity, config, sspi_config).map(AuthContext::Negotiate)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub require_encryption: bool,
    pub sspi_config: SspiAuthConfig,
}

// Add From trait implementation for Authentication -> AuthConfig
impl From<Authentication> for AuthConfig {
    fn from(auth: Authentication) -> Self {
        match auth {
            Authentication::Basic { .. } => {
                // For basic auth, we'll use NTLM as a fallback
                // This is a simplification - in practice you might want to handle this differently
                todo!("Basic authentication conversion not implemented")
            }
            Authentication::Sspi(sspi_config) => AuthConfig {
                require_encryption: true, // Default to requiring encryption
                sspi_config,
            },
        }
    }
}

pub struct AuthSequence {
    context: AuthContext,
    http_builder: HttpBuilder,
    require_encryption: bool,
}

pub enum SecCtxInited {
    Continue(HttpRequest),
    Done(Option<Token>),
}

impl Debug for AuthSequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthSequence")
            .field("context", &"AnyAuthContext { ... }")
            .field("http_builder", &self.http_builder)
            .finish()
    }
}

impl AuthSequence {
    pub(crate) fn new(
        auth_config: AuthConfig,
        http_builder: HttpBuilder,
    ) -> Result<Self, crate::PwshCoreError> {
        let AuthConfig {
            sspi_config,
            require_encryption,
        } = auth_config;

        let context = AuthContext::new(sspi_config)?;
        Ok(AuthSequence {
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
            AuthContext::Ntlm(auth_context) => SspiAuthenticator::try_init_sec_context(
                response,
                auth_context,
                sec_ctx_holder.as_mut_ntlm(),
                self.require_encryption,
            )?,
            AuthContext::Kerberos(auth_context) => SspiAuthenticator::try_init_sec_context(
                response,
                auth_context,
                sec_ctx_holder.as_mut_kerberos(),
                self.require_encryption,
            )?,
            AuthContext::Negotiate(auth_context) => SspiAuthenticator::try_init_sec_context(
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
            AuthContext::Ntlm(auth_context) => {
                SspiAuthenticator::process_initialized_sec_context(auth_context, sec_context)
            }
            AuthContext::Kerberos(auth_context) => {
                SspiAuthenticator::process_initialized_sec_context(auth_context, sec_context)
            }
            AuthContext::Negotiate(auth_context) => {
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
        let AuthSequence {
            context,
            http_builder,
            require_encryption,
        } = self;

        Authenticated {
            encryption_provider: EncryptionProvider::new(context, require_encryption),
            http_builder,
        }
    }
}

pub struct Authenticated {
    pub(crate) encryption_provider: EncryptionProvider,
    pub(crate) http_builder: HttpBuilder,
}
