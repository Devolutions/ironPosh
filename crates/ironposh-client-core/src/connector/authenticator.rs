use std::fmt::Debug;

use base64::Engine;
use sspi::builders::{
    InitializeSecurityContext, WithContextRequirements, WithCredentialsHandle, WithOutput,
    WithTargetDataRepresentation,
};
use sspi::generator::{Generator, GeneratorState};
use sspi::{
    BufferType, ClientRequestFlags, CredentialUse, Credentials, DataRepresentation, Error,
    InitializeSecurityContextResult, Negotiate, NegotiateConfig, NetworkRequest, Ntlm,
    SecurityBuffer, SecurityStatus, Sspi, SspiImpl,
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

/// Caller-owned “furniture” the generator borrows.
/// Holds provider, credential handle, in/out buffers, and the ISC builder for the current round.
#[derive(Debug)]
pub struct AuthFurniture<'a, P: Sspi> {
    pub provider: P,
    // Box<T> provides a stable heap address; we keep borrows within the same `AuthFurniture`.
    cred: Box<P::CredentialsHandle>,
    out: [SecurityBuffer; 1],
    // Keep the builder + input buffer alive for the duration of the suspension (generator borrows them).
    inbuf: Option<[SecurityBuffer; 1]>,
    isc: Option<SecurityContextBuilder<'a, P>>,
}

impl<'a> AuthFurniture<'a, Ntlm> {
    pub fn new_ntlm(id: ClientAuthIdentity) -> Result<Self, PwshCoreError> {
        Self::new_with_identity(Ntlm::new(), id)
    }
}

impl<'a> AuthFurniture<'a, Negotiate> {
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

impl<'a, P> AuthFurniture<'a, P>
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
            isc: None,
        })
    }
}

impl<'a, P> AuthFurniture<'a, P>
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
            isc: None,
        })
    }
}

impl<'a, P> AuthFurniture<'a, P>
where
    P: Sspi,
{
    /// Prepare for the next `InitializeSecurityContext` round.
    /// We only clear here, right before wiring a new round.
    fn clear_for_next_round(&mut self) {
        self.inbuf = None;
        self.out[0].buffer.clear();
        self.isc = None;
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
pub enum SspiAuthenticatorState {
    /// Kick off or continue an ISC round; may or may not need the generator depending on input.
    SeeIfNeedsGenerator,
    /// We yielded a `NetworkRequest` and are waiting for the caller to resume the generator.
    ResolvingGenerator,
    /// A round completed; builder still holds `&mut out`. We will extract the token in the next call.
    ProcessInitializedContextPending {
        init_sec_context_res: InitializeSecurityContextResult,
    },
    /// Finished.
    Done,
}

impl Default for SspiAuthenticatorState {
    fn default() -> Self {
        SspiAuthenticatorState::SeeIfNeedsGenerator
    }
}

#[derive(Debug)]
pub struct SspiStepRequest<'a> {
    pub(super) generator: Generator<
        'a,
        NetworkRequest,
        Result<Vec<u8>, Error>,
        Result<InitializeSecurityContextResult, Error>,
    >,
}

#[derive(Debug)]
pub enum GeneratorResumeResult<'a> {
    DoItAgainBro {
        packet: NetworkRequest,
        generator_holder: SspiStepRequest<'a>,
    },
    NahYouGood,
}

#[derive(Debug)]
pub enum AuthenticatorStepResult<'a> {
    /// Nothing to send this turn (e.g., waiting for a 401 with Negotiate or no token emitted).
    Continue,
    /// Send `Authorization: Negotiate <token>` and call `step` again with the server response.
    ContinueWithToken { token: Token },
    /// Send the packet (e.g., to a KDC) and call `resolve_generator` with the raw response.
    SendToHereAndContinue {
        packet: NetworkRequest,
        generator_holder: SspiStepRequest<'a>,
    },
    /// Authentication done; may require sending a final token with the last request.
    Done { token: Option<Token> },
}

#[derive(Debug, Default)]
pub struct SspiAuthenticator {
    state: SspiAuthenticatorState,
}

impl SspiAuthenticator {
    pub fn new() -> Self {
        Self {
            state: SspiAuthenticatorState::SeeIfNeedsGenerator,
        }
    }

    /// Drive one step of the SSPI handshake.
    ///
    /// We mutate `self.state` in place (no `mem::take`), so early returns don't
    /// strand the state as `Taken`. This avoids hard-to-debug invalid-state errors.
    pub fn step<'a, P>(
        &mut self,
        response: Option<&HttpResponse<String>>,
        furniture: &'a mut AuthFurniture<'a, P>,
    ) -> Result<AuthenticatorStepResult<'a>, PwshCoreError>
    where
        P: Sspi + SspiImpl,
    {
        match &mut self.state {
            SspiAuthenticatorState::SeeIfNeedsGenerator => {
                // Starting/continuing an ISC round: clear buffers *now*.
                furniture.clear_for_next_round();
                furniture.take_input(response)?;

                // Build the builder; wire inputs/outputs.
                let mut isc = furniture
                    .provider
                    .initialize_security_context()
                    .with_credentials_handle(&mut *furniture.cred)
                    .with_context_requirements(
                        // TODO: expose these flags to callers for tuning.
                        ClientRequestFlags::CONFIDENTIALITY | ClientRequestFlags::ALLOCATE_MEMORY,
                    )
                    .with_target_data_representation(DataRepresentation::Native)
                    .with_output(&mut furniture.out);

                if let Some(input_buffer) = &mut furniture.inbuf {
                    isc = isc.with_input(input_buffer);
                }
                furniture.isc = Some(isc);

                // Produce the generator for this round.
                let mut generator = furniture.provider.initialize_security_context_impl(
                    furniture
                        .isc
                        .as_mut()
                        .expect("If this happens just go code JavaScript :D"),
                )?;

                match generator.start() {
                    GeneratorState::Suspended(request) => {
                        // We need an external round-trip (e.g., to KDC).
                        self.state = SspiAuthenticatorState::ResolvingGenerator;
                        Ok(AuthenticatorStepResult::SendToHereAndContinue {
                            packet: request,
                            generator_holder: SspiStepRequest { generator },
                        })
                    }
                    GeneratorState::Completed(init_sec_context_res) => {
                        // Do NOT touch `out` here; `isc` still holds &mut out.
                        // Defer token extraction to next call.
                        let init_sec_context_res = init_sec_context_res?;
                        self.state = SspiAuthenticatorState::ProcessInitializedContextPending {
                            init_sec_context_res,
                        };
                        Ok(AuthenticatorStepResult::Continue)
                    }
                }
            }
            SspiAuthenticatorState::ResolvingGenerator => Err(PwshCoreError::InvalidState(
                "Call resolve_generator if you received SendToHereAndContinue",
            )),
            SspiAuthenticatorState::ProcessInitializedContextPending {
                init_sec_context_res,
            } => {
                // Now it's safe to drop the builder and read the output token.
                furniture.isc = None; // releases &mut borrow on `out` inside the builder

                let produced = std::mem::take(&mut furniture.out[0].buffer);
                let header = token_header_from(&produced);

                match init_sec_context_res.status {
                    SecurityStatus::ContinueNeeded => {
                        // Another round needed; next call will rebuild ISC and clear buffers.
                        self.state = SspiAuthenticatorState::SeeIfNeedsGenerator;
                        if let Some(token) = header {
                            Ok(AuthenticatorStepResult::ContinueWithToken {
                                token: Token(token),
                            })
                        } else {
                            Ok(AuthenticatorStepResult::Continue)
                        }
                    }
                    SecurityStatus::Ok => {
                        self.state = SspiAuthenticatorState::Done;
                        Ok(AuthenticatorStepResult::Done {
                            token: header.map(Token),
                        })
                    }
                    _ => Err(PwshCoreError::Auth("SSPI InitializeSecurityContext failed")),
                }
            }
            SspiAuthenticatorState::Done => Err(PwshCoreError::InvalidState(
                "Authenticator is already in Done state",
            )),
        }
    }

    /// Resume a previously-suspended generator with the raw KDC (or similar) response.
    ///
    /// We set the state to `ProcessInitializedContextPending` on completion, and only in the
    /// *next* call to `step` will we drop the builder and extract the token.
    pub fn resume<'a>(
        &mut self,
        generator_holder: SspiStepRequest<'a>,
        kdc_response: Vec<u8>,
    ) -> Result<GeneratorResumeResult<'a>, PwshCoreError> {
        debug_assert!(matches!(
            self.state,
            SspiAuthenticatorState::ResolvingGenerator
        ));
        let mut generator = generator_holder.generator;

        match generator.resume(Ok(kdc_response)) {
            GeneratorState::Suspended(request) => Ok(GeneratorResumeResult::DoItAgainBro {
                packet: request,
                generator_holder: SspiStepRequest { generator },
            }),
            GeneratorState::Completed(res) => {
                let init_sec_context_res = res?;
                self.state = SspiAuthenticatorState::ProcessInitializedContextPending {
                    init_sec_context_res,
                };
                Ok(GeneratorResumeResult::NahYouGood)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token(pub(crate) String);

impl Token {
    fn from_string(s: String) -> Self {
        Token(s)
    }
}

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
