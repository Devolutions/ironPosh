use sspi::{NetworkProtocol, NetworkRequest, Sspi};
use url::Url;

use crate::connector::{
    authenticator::{self, AuthFurniture, GeneratorHolder, SspiAuthenticator, Token},
    http::{HttpBuilder, HttpRequest, HttpResponse},
};

#[derive(Debug)]
pub struct AuthSequence<'conn> {
    /// Holding connector to ensure it's not used while auth sequence is active
    pub(crate) _connector: &'conn mut super::Connector,
}

impl<'conn> AuthSequence<'conn> {
    pub(crate) fn new(connector: &'conn mut super::Connector) -> Self {
        Self {
            _connector: connector,
        }
    }
}

pub enum KerberoRequestProtocol {
    Http,
    Https,
    Tcp,
    Udp,
}

pub struct KerberoRequestPacket {
    pub protocol: KerberoRequestProtocol,
    pub url: Url,
    pub data: Vec<u8>,
}

impl KerberoRequestPacket {
    fn new(request: NetworkRequest) -> Self {
        Self {
            protocol: match request.protocol {
                NetworkProtocol::Http => KerberoRequestProtocol::Http,
                NetworkProtocol::Https => KerberoRequestProtocol::Https,
                NetworkProtocol::Tcp => KerberoRequestProtocol::Tcp,
                NetworkProtocol::Udp => KerberoRequestProtocol::Udp,
            },
            url: request.url,
            data: request.data,
        }
    }
}

pub enum TryInitSecContext<'g> {
    RunGenerator {
        packet: KerberoRequestPacket,
        generator_holder: GeneratorHolder<'g>,
    },
    Initialized {
        init_sec_context_res: authenticator::SecContextInit,
    },
}

pub enum SecContextProcessResult {
    TryInitAgain { request: HttpRequest<String> },
    Done { token: Option<Token> },
}

pub enum AnyContext {
    Ntlm(AuthFurniture<sspi::ntlm::Ntlm>),
    Kerberos(AuthFurniture<sspi::kerberos::Kerberos>),
    Negotiate(AuthFurniture<sspi::negotiate::Negotiate>),
}

impl std::fmt::Debug for AnyContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnyContext::Ntlm(_) => write!(f, "AnyContext::Ntlm"),
            AnyContext::Kerberos(_) => write!(f, "AnyContext::Kerberos"),
            AnyContext::Negotiate(_) => write!(f, "AnyContext::Negotiate"),
        }
    }
}

impl<'conn, 'ctx, 'builder, 'generator> AuthSequence<'conn>
where
    'ctx: 'builder,
    'builder: 'generator,
{
    pub fn try_init_sec_context(
        &self,
        context: &'ctx mut AnyContext<'builder>,
        response: Option<HttpResponse<String>>,
    ) -> Result<TryInitSecContext<'generator>, crate::PwshCoreError> {
        let sec_context_init = match context {
            AnyContext::Ntlm(ctx) => {
                SspiAuthenticator::try_init_sec_context(response.as_ref(), ctx)
            }
            AnyContext::Kerberos(ctx) => {
                SspiAuthenticator::try_init_sec_context(response.as_ref(), ctx)
            }
            AnyContext::Negotiate(ctx) => {
                SspiAuthenticator::try_init_sec_context(response.as_ref(), ctx)
            }
        }?;

        match sec_context_init {
            crate::connector::authenticator::SecContextMaybeInit::RunGenerator {
                packet,
                generator_holder,
            } => Ok(TryInitSecContext::RunGenerator {
                packet: KerberoRequestPacket::new(packet),
                generator_holder,
            }),
            crate::connector::authenticator::SecContextMaybeInit::Initialized(sec_context_init) => {
                Ok(TryInitSecContext::Initialized {
                    init_sec_context_res: sec_context_init,
                })
            }
        }
    }

    pub fn resume(
        &self,
        kdc_response: Vec<u8>,
        generator_holder: GeneratorHolder<'builder>,
    ) -> Result<TryInitSecContext<'builder>, crate::PwshCoreError> {
        match SspiAuthenticator::resume(generator_holder, kdc_response)? {
            authenticator::SecContextMaybeInit::RunGenerator {
                packet,
                generator_holder,
            } => Ok(TryInitSecContext::RunGenerator {
                packet: KerberoRequestPacket::new(packet),
                generator_holder,
            }),
            authenticator::SecContextMaybeInit::Initialized(init_sec_context_res) => {
                Ok(TryInitSecContext::Initialized {
                    init_sec_context_res,
                })
            }
        }
    }

    pub fn process_initialized_sec_context(
        &self,
        context: &'ctx mut AnyContext<'builder>,
        http_builder: &mut HttpBuilder,
        sec_context_init: authenticator::SecContextInit,
    ) -> Result<SecContextProcessResult, crate::PwshCoreError> {
        todo!();
        // match SspiAuthenticator::process_initialized_sec_context(context, sec_context_init)? {
        //     authenticator::ActionReqired::TryInitSecContextAgain { token } => {
        //         http_builder.with_auth_header(token.0.to_owned());
        //         let request = http_builder.post("/wsman", String::new());
        //         Ok(SecContextProcessResult::TryInitAgain { request })
        //     }
        //     authenticator::ActionReqired::Done { token } => {
        //         Ok(SecContextProcessResult::Done { token })
        //     }
        // }
    }

    pub fn destruct_me(self) -> (&'conn mut super::Connector, HttpBuilder) {
        todo!()
    }
}
