use std::fmt::Debug;

use base64::Engine;
use sspi::builders::{
    InitializeSecurityContext, WithContextRequirements, WithCredentialsHandle, WithOutput,
    WithTargetDataRepresentation,
};
use sspi::generator::{Generator, GeneratorState};
use sspi::{
    BufferType, ClientRequestFlags, CredentialUse, Credentials, DataRepresentation, Error,
    InitializeSecurityContextResult, Kerberos, KerberosConfig, Negotiate, NegotiateConfig,
    NetworkRequest, Ntlm, SecurityBuffer, SecurityStatus, Sspi, SspiImpl,
};

use crate::PwshCoreError;
use crate::connector::http::HttpResponse;
use crate::credentials::ClientAuthIdentity;

type SecurityContextBuilder<'a, P> = InitializeSecurityContext<
    'a,
    <P as SspiImpl>::CredentialsHandle,
    WithCredentialsHandle,
    WithContextRequirements,
    WithTargetDataRepresentation,
    WithOutput,
>;

/// Caller-owned "Context" the generator borrows.
/// Holds provider, credential handle, in/out buffers, and the ISC builder for the current round.
#[derive(Debug)]
pub struct AuthContext<P: Sspi> {
    pub(crate) provider: P,
    // Box<T> provides a stable heap address; we keep borrows within the same `AuthFurniture`.
    cred: Box<P::CredentialsHandle>,
    out: [SecurityBuffer; 1],
    // Keep the builder + input buffer alive for the duration of the suspension (generator borrows them).
    inbuf: Option<[SecurityBuffer; 1]>,
}

impl AuthContext<Ntlm> {
    pub fn new_ntlm(id: ClientAuthIdentity) -> Result<Self, PwshCoreError> {
        Self::new_with_identity(Ntlm::new(), id)
    }
}

impl AuthContext<Negotiate> {
    pub fn new_negotiate(
        id: ClientAuthIdentity,
        config: NegotiateConfig,
    ) -> Result<Self, PwshCoreError> {
        Self::new_with_credential(
            Negotiate::new_client(config)?,
            Credentials::AuthIdentity(id.into_inner()),
        )
    }
}

impl AuthContext<Kerberos> {
    pub fn new_kerberos(
        id: ClientAuthIdentity,
        config: KerberosConfig,
    ) -> Result<Self, PwshCoreError> {
        Self::new_with_credential(
            Kerberos::new_client_from_config(config)?,
            Credentials::AuthIdentity(id.into_inner()),
        )
    }
}

impl<P> AuthContext<P>
where
    P: Sspi + SspiImpl<AuthenticationData = sspi::Credentials>,
{
    pub fn new_with_credential(mut provider: P, id: Credentials) -> Result<Self, PwshCoreError> {
        let acq = provider
            .acquire_credentials_handle()
            .with_credential_use(CredentialUse::Outbound)
            .with_auth_data(&id);
        let cred = acq.execute(&mut provider)?.credentials_handle;

        Ok(Self {
            provider,
            cred: Box::new(cred),
            out: [SecurityBuffer::new(Vec::new(), BufferType::Token)],
            inbuf: None,
        })
    }
}

impl<P> AuthContext<P>
where
    P: Sspi + SspiImpl<AuthenticationData = sspi::AuthIdentity>,
{
    pub fn new_with_identity(
        mut provider: P,
        id: ClientAuthIdentity,
    ) -> Result<Self, PwshCoreError> {
        let id: sspi::AuthIdentity = id.into_inner();
        let acq = provider
            .acquire_credentials_handle()
            .with_credential_use(CredentialUse::Outbound)
            .with_auth_data(&id);
        let cred = acq.execute(&mut provider)?.credentials_handle;

        Ok(Self {
            provider,
            cred: Box::new(cred),
            out: [SecurityBuffer::new(Vec::new(), BufferType::Token)],
            inbuf: None,
        })
    }
}

impl<P> AuthContext<P>
where
    P: Sspi,
{
    /// Prepare for the next `InitializeSecurityContext` round.
    /// We only clear here, right before wiring a new round.
    fn clear_for_next_round(&mut self) {
        self.inbuf = None;
        self.out[0].buffer.clear();
    }

    /// Parse the server's negotiate token (if present) and set `inbuf`.
    fn take_input(&mut self, response: Option<&HttpResponse<String>>) -> Result<(), PwshCoreError> {
        if let Some(resp) = response {
            let server_token = parse_negotiate_token(&resp.headers)
                .ok_or_else(|| PwshCoreError::Auth("no Negotiate token"))?;
            self.inbuf = Some([SecurityBuffer::new(server_token, BufferType::Token)]);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct GeneratorHolder<'g> {
    pub(super) generator: Generator<
        'g,
        NetworkRequest,
        Result<Vec<u8>, Error>,
        Result<InitializeSecurityContextResult, Error>,
    >,
}

#[derive(Debug, Default)]
pub struct SspiAuthenticator {}

#[derive(Debug)]
pub struct SecContextInit {
    init_sec_context_res: InitializeSecurityContextResult,
}

pub enum SecContextMaybeInit<'g> {
    RunGenerator {
        packet: NetworkRequest,
        generator_holder: GeneratorHolder<'g>,
    },
    Initialized(SecContextInit),
}

pub enum ActionReqired {
    TryInitSecContextAgain { token: Token },
    Done { token: Option<Token> },
}

impl SspiAuthenticator {
    /// Drive one step of the SSPI handshake.
    ///
    /// We mutate `self.state` in place (no `mem::take`), so early returns don't
    /// strand the state as `Taken`. This avoids hard-to-debug invalid-state errors.
    pub fn try_init_sec_context<'ctx, 'builder, 'generator, P>(
        response: Option<&HttpResponse<String>>,
        context: &'ctx mut AuthContext<P>,
        sec_ctx_holder: &'builder mut Option<SecurityContextBuilder<'ctx, P>>,
    ) -> Result<SecContextMaybeInit<'generator>, PwshCoreError>
    where
        P: Sspi + SspiImpl,
        'ctx: 'builder,
        'builder: 'generator,
    {
        context.clear_for_next_round();
        context.take_input(response)?;

        // Build the builder; wire inputs/outputs.
        let mut isc: SecurityContextBuilder<P> = context
            .provider
            .initialize_security_context()
            .with_credentials_handle(&mut *context.cred)
            .with_context_requirements(
                // TODO: expose these flags to callers for tuning.
                ClientRequestFlags::CONFIDENTIALITY | ClientRequestFlags::ALLOCATE_MEMORY,
            )
            .with_target_data_representation(DataRepresentation::Native)
            .with_output(&mut context.out);

        if let Some(input_buffer) = &mut context.inbuf {
            isc = isc.with_input(input_buffer);
        }

        *sec_ctx_holder = Some(isc);

        // Produce the generator for this round.
        let mut generator = context
            .provider
            .initialize_security_context_impl(sec_ctx_holder.as_mut().unwrap())?;

        match generator.start() {
            GeneratorState::Suspended(request) => {
                // We have to suspend to send the packet to the server.
                Ok(SecContextMaybeInit::RunGenerator {
                    packet: request,
                    generator_holder: GeneratorHolder { generator },
                })
            }
            GeneratorState::Completed(init_sec_context_res) => {
                // Do NOT touch `out` here; `isc` still holds &mut out.
                // Defer token extraction to next call.
                let init_sec_context_res = init_sec_context_res?;

                Ok(SecContextMaybeInit::Initialized(SecContextInit {
                    init_sec_context_res,
                }))
            }
        }
    }

    /// Resume a previously-suspended generator with the raw KDC (or similar) response.
    ///
    /// We set the state to `ProcessInitializedContextPending` on completion, and only in the
    /// *next* call to `step` will we drop the builder and extract the token.
    pub fn resume<'a>(
        generator_holder: GeneratorHolder<'a>,
        kdc_response: Vec<u8>,
    ) -> Result<SecContextMaybeInit<'a>, PwshCoreError> {
        let mut generator = generator_holder.generator;

        match generator.resume(Ok(kdc_response)) {
            GeneratorState::Suspended(request) => Ok(SecContextMaybeInit::RunGenerator {
                packet: request,
                generator_holder: GeneratorHolder { generator },
            }),

            GeneratorState::Completed(res) => {
                let init_sec_context_res = res?;
                Ok(SecContextMaybeInit::Initialized(SecContextInit {
                    init_sec_context_res,
                }))
            }
        }
    }

    pub fn process_initialized_sec_context<'a, P: Sspi>(
        furniture: &'a mut AuthContext<P>,
        sec_context: SecContextInit,
    ) -> Result<ActionReqired, PwshCoreError>
    where
        P: Sspi + SspiImpl,
    {
        let produced = std::mem::take(&mut furniture.out[0].buffer);
        let token = token_header_from(&produced).map(Token);

        match sec_context.init_sec_context_res.status {
            SecurityStatus::ContinueNeeded => Ok(ActionReqired::TryInitSecContextAgain {
                token: token.ok_or(PwshCoreError::Auth(
                    "SSPI ContinueNeeded but no token produced",
                ))?,
            }),
            SecurityStatus::Ok => Ok(ActionReqired::Done { token }),
            _ => Err(PwshCoreError::Auth(
                "SSPI InitializeSecurityContext status needs to be handled",
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token(pub(crate) String);

/// Create an `Authorization` header value if a token exists.
fn token_header_from(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        None
    } else {
        Some(format!(
            "Negotiate {}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ))
    }
}

/// Parse the "WWW-Authenticate: Negotiate <b64>" header case-insensitively.
///
/// If multiple `WWW-Authenticate` headers are present, we take the first `Negotiate` one.
fn parse_negotiate_token(headers: &[(String, String)]) -> Option<Vec<u8>> {
    for (key, value) in headers {
        if key.eq_ignore_ascii_case("www-authenticate") {
            // Try case-insensitive "Negotiate ".
            if let Some(rest) = value
                .strip_prefix("Negotiate ")
                .or_else(|| value.strip_prefix("negotiate "))
            {
                if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(rest.trim()) {
                    return Some(bytes);
                }
            }
        }
    }
    None
}
