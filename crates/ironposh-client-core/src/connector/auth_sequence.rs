use std::fmt::Debug;

use sspi::{NegotiateConfig, ntlm::NtlmConfig};

use crate::{
    PwshCoreError, SspiAuthConfig,
    connector::{
        authenticator::{
            AuthContext, SecContextMaybeInit, SecurityContextBuilder, SspiAuthenticator, Token,
        },
        http::{HttpBuilder, HttpRequest, HttpResponse},
    },
};

pub enum AnyAuthContext {
    Ntlm(AuthContext<sspi::ntlm::Ntlm>),
    Kerberos(AuthContext<sspi::kerberos::Kerberos>),
    Negotiate(AuthContext<sspi::negotiate::Negotiate>),
}

pub struct SecurityContextBuilderHolder<'ctx> {
    ntlm: Option<SecurityContextBuilder<'ctx, sspi::ntlm::Ntlm>>,
    kerberos: Option<SecurityContextBuilder<'ctx, sspi::kerberos::Kerberos>>,
    negotiate: Option<SecurityContextBuilder<'ctx, sspi::negotiate::Negotiate>>,
}

impl<'ctx> SecurityContextBuilderHolder<'ctx> {
    pub fn new() -> Self {
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

impl AnyAuthContext {
    pub fn new(sspi_config: SspiAuthConfig) -> Result<Self, crate::PwshCoreError> {
        match &sspi_config {
            SspiAuthConfig::NTLM { identity, .. } => {
                AuthContext::new_ntlm(identity.clone(), sspi_config).map(AnyAuthContext::Ntlm)
            }
            SspiAuthConfig::Kerberos {
                identity,
                kerberos_config,
                target_name: _,
            } => AuthContext::new_kerberos(
                identity.clone(),
                sspi::KerberosConfig {
                    kdc_url: kerberos_config.kdc_url.clone(),
                    client_computer_name: kerberos_config.client_computer_name.clone(),
                },
                sspi_config,
            )
            .map(AnyAuthContext::Kerberos),
            SspiAuthConfig::Negotiate {
                identity,
                kerberos_config,
                target_name: _,
            } => {
                if let Some(kerberos_config) = kerberos_config {
                    let client_computer_name = kerberos_config
                        .client_computer_name
                        .clone()
                        .unwrap_or("IronWinRMClient".to_string());

                    AuthContext::new_negotiate(
                        identity.clone(),
                        NegotiateConfig::from_protocol_config(
                            Box::new(sspi::KerberosConfig {
                                kdc_url: kerberos_config.kdc_url.clone(),
                                client_computer_name: kerberos_config.client_computer_name.clone(),
                            }),
                            client_computer_name,
                        ),
                        sspi_config,
                    )
                    .map(AnyAuthContext::Negotiate)
                } else {
                    let client_computer_name = "IronWinRMClient".to_string();
                    AuthContext::new_negotiate(
                        identity.clone(),
                        NegotiateConfig::from_protocol_config(
                            Box::new(NtlmConfig::default()),
                            client_computer_name,
                        ),
                        sspi_config,
                    )
                    .map(AnyAuthContext::Negotiate)
                }
            }
        }
    }
}

pub struct AuthSequence {
    context: AnyAuthContext,
    http_builder: HttpBuilder,
}

pub enum SecCtxInited {
    Continue(HttpRequest<String>),
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
    pub fn new(
        sspi_config: SspiAuthConfig,
        http_builder: HttpBuilder,
    ) -> Result<Self, crate::PwshCoreError> {
        let context = AnyAuthContext::new(sspi_config)?;
        Ok(AuthSequence {
            context,
            http_builder,
        })
    }

    pub fn try_init_sec_context<'ctx, 'builder, 'generator>(
        self: &'ctx mut Self,
        response: Option<&HttpResponse<String>>,
        sec_ctx_holder: &'builder mut SecurityContextBuilderHolder<'ctx>,
    ) -> Result<SecContextMaybeInit<'generator>, PwshCoreError>
    where
        'ctx: 'builder,
        'builder: 'generator,
    {
        Ok(match &mut self.context {
            AnyAuthContext::Ntlm(auth_context) => SspiAuthenticator::try_init_sec_context(
                response,
                auth_context,
                sec_ctx_holder.as_mut_ntlm(),
            )?,
            AnyAuthContext::Kerberos(auth_context) => SspiAuthenticator::try_init_sec_context(
                response,
                auth_context,
                sec_ctx_holder.as_mut_kerberos(),
            )?,
            AnyAuthContext::Negotiate(auth_context) => SspiAuthenticator::try_init_sec_context(
                response,
                auth_context,
                sec_ctx_holder.as_mut_negotiate(),
            )?,
        })
    }

    pub fn resume<'a>(
        generator_holder: crate::connector::authenticator::GeneratorHolder<'a>,
        kdc_response: Vec<u8>,
    ) -> Result<SecContextMaybeInit<'a>, PwshCoreError> {
        SspiAuthenticator::resume(generator_holder, kdc_response)
    }

    pub fn process_initialized_sec_context(
        &mut self,
        sec_context: crate::connector::authenticator::SecContextInit,
    ) -> Result<SecCtxInited, PwshCoreError> {
        let res = match &mut self.context {
            AnyAuthContext::Ntlm(auth_context) => {
                SspiAuthenticator::process_initialized_sec_context(auth_context, sec_context)
            }
            AnyAuthContext::Kerberos(auth_context) => {
                SspiAuthenticator::process_initialized_sec_context(auth_context, sec_context)
            }
            AnyAuthContext::Negotiate(auth_context) => {
                SspiAuthenticator::process_initialized_sec_context(auth_context, sec_context)
            }
        }?;

        match res {
            super::authenticator::ActionReqired::TryInitSecContextAgain { token } => {
                self.http_builder.with_auth_header(token.0);
                Ok(SecCtxInited::Continue(
                    self.http_builder.post("/wsman", String::new()),
                ))
            }
            super::authenticator::ActionReqired::Done { token } => Ok(SecCtxInited::Done(token)),
        }
    }
}
