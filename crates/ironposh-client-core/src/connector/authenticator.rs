use std::fmt::Debug;

use base64::Engine;
use sspi::builders::{
    InitializeSecurityContext, WithContextRequirements, WithCredentialsHandle, WithOutput,
    WithTargetDataRepresentation,
};
use sspi::generator::{Generator, GeneratorState};
use sspi::{
    BufferType, ClientRequestFlags, CredentialUse, Credentials, DataRepresentation,
    EncryptionFlags, Error, InitializeSecurityContextResult, Kerberos, KerberosConfig, Negotiate,
    NegotiateConfig, NetworkRequest, Ntlm, SecurityBuffer, SecurityBufferFlags, SecurityBufferRef,
    SecurityStatus, Sspi, SspiImpl,
};
use tracing::{debug, instrument};

use crate::PwshCoreError;
use crate::connector::http::HttpResponse;
use crate::credentials::ClientAuthIdentity;

pub type SecurityContextBuilder<'a, P> = InitializeSecurityContext<
    'a,
    <P as SspiImpl>::CredentialsHandle,
    WithCredentialsHandle,
    WithContextRequirements,
    WithTargetDataRepresentation,
    WithOutput,
>;

#[derive(Debug)]
pub struct SspiConfig {
    target_name: String,
}

impl SspiConfig {
    pub fn new(mut target: String) -> Self {
        if !target.trim().starts_with("HTTP/") {
            target = format!("HTTP/{target}");
        }
        Self {
            target_name: target,
        }
    }
}

/// Caller-owned "Context" the generator borrows.
///
/// Holds provider, credential handle, in/out buffers, and the ISC builder for the current round.
/// The reason we need to hold the builder is that, during the generator suspension, the generator holds
/// a mutable borrow to the future that holds both the builder and the mut ref of provider, and we need to keep the
/// context around during the suspension.
#[derive(Debug)]
pub struct SspiContext<P: Sspi> {
    pub(crate) provider: P,
    // Box<T> provides a stable heap address; we keep borrows within the same `AuthFurniture`.
    cred: Box<P::CredentialsHandle>,
    out: [SecurityBuffer; 1],
    // Keep the builder + input buffer alive for the duration of the suspension (generator borrows them).
    inbuf: Option<[SecurityBuffer; 1]>,
    sspi_auth_config: SspiConfig,
}

impl SspiContext<Ntlm> {
    pub fn new_ntlm(id: ClientAuthIdentity, config: SspiConfig) -> Result<Self, PwshCoreError> {
        Self::new_with_identity(Ntlm::new(), id, config)
    }
}

impl SspiContext<Negotiate> {
    pub fn new_negotiate(
        id: ClientAuthIdentity,
        config: NegotiateConfig,
        sspi_config: SspiConfig,
    ) -> Result<Self, PwshCoreError> {
        Self::new_with_credential(
            Negotiate::new_client(config)?,
            &Credentials::AuthIdentity(id.into_inner()),
            sspi_config,
        )
    }
}

impl SspiContext<Kerberos> {
    pub fn new_kerberos(
        id: ClientAuthIdentity,
        kerberos_config: KerberosConfig,
        sspi_config: SspiConfig,
    ) -> Result<Self, PwshCoreError> {
        Self::new_with_credential(
            Kerberos::new_client_from_config(kerberos_config)?,
            &Credentials::AuthIdentity(id.into_inner()),
            sspi_config,
        )
    }
}

impl<P> SspiContext<P>
where
    P: Sspi + SspiImpl<AuthenticationData = sspi::Credentials>,
{
    pub fn new_with_credential(
        mut provider: P,
        id: &Credentials,
        config: SspiConfig,
    ) -> Result<Self, PwshCoreError> {
        let acq = provider
            .acquire_credentials_handle()
            .with_credential_use(CredentialUse::Outbound)
            .with_auth_data(id);
        let cred = acq.execute(&mut provider)?.credentials_handle;

        Ok(Self {
            provider,
            cred: Box::new(cred),
            out: [SecurityBuffer::new(Vec::new(), BufferType::Token)],
            inbuf: None,
            sspi_auth_config: config,
        })
    }
}

impl<P> SspiContext<P>
where
    P: Sspi + SspiImpl<AuthenticationData = sspi::AuthIdentity>,
{
    pub fn new_with_identity(
        mut provider: P,
        id: ClientAuthIdentity,
        config: SspiConfig,
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
            sspi_auth_config: config,
        })
    }
}

impl<P> SspiContext<P>
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
    fn take_input(&mut self, response: Option<&HttpResponse>) -> Result<(), PwshCoreError> {
        if let Some(resp) = response {
            let server_token = parse_negotiate_token(&resp.headers)
                .ok_or(PwshCoreError::Auth("no Negotiate token"))?;
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
    #[instrument(skip(context, sec_ctx_holder))]
    pub fn try_init_sec_context<'ctx, 'builder, 'generator, P>(
        response: Option<&HttpResponse>,
        context: &'ctx mut SspiContext<P>,
        sec_ctx_holder: &'builder mut Option<SecurityContextBuilder<'ctx, P>>,
        require_encryption: bool,
    ) -> Result<SecContextMaybeInit<'generator>, PwshCoreError>
    where
        P: Sspi + SspiImpl,
        'ctx: 'builder,
        'builder: 'generator,
        <P as SspiImpl>::CredentialsHandle: Debug,
    {
        context.clear_for_next_round();
        context.take_input(response)?;

        let flag = if require_encryption {
            debug!("encryption required for this session");
            ClientRequestFlags::CONFIDENTIALITY | ClientRequestFlags::INTEGRITY
        } else {
            debug!("encryption NOT required for this session");
            ClientRequestFlags::empty()
        };

        // Build the builder; wire inputs/outputs.
        let mut isc: SecurityContextBuilder<P> = context
            .provider
            .initialize_security_context()
            .with_credentials_handle(&mut *context.cred)
            .with_context_requirements(
                ClientRequestFlags::ALLOCATE_MEMORY | ClientRequestFlags::MUTUAL_AUTH | flag,
            )
            .with_target_data_representation(DataRepresentation::Native)
            .with_target_name(&context.sspi_auth_config.target_name)
            .with_output(&mut context.out);

        if let Some(input_buffer) = &mut context.inbuf {
            isc = isc.with_input(input_buffer);
        }

        debug!(?isc, "calling SSPI InitializeSecurityContext");

        *sec_ctx_holder = Some(isc);

        // Produce the generator for this round.
        let mut generator = context
            .provider
            .initialize_security_context_impl(sec_ctx_holder.as_mut().unwrap())?;

        match generator.start() {
            GeneratorState::Suspended(request) => {
                debug!("SSPI generator suspended, need to send packet to server");
                // We have to suspend to send the packet to the server.
                Ok(SecContextMaybeInit::RunGenerator {
                    packet: request,
                    generator_holder: GeneratorHolder { generator },
                })
            }
            GeneratorState::Completed(init_sec_context_res) => {
                debug!("SSPI InitializeSecurityContext completed immediately");
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
    #[instrument(skip_all)]
    pub fn resume<'a>(
        generator_holder: GeneratorHolder<'a>,
        kdc_response: Vec<u8>,
    ) -> Result<SecContextMaybeInit<'a>, PwshCoreError> {
        let mut generator = generator_holder.generator;

        debug!(
            kdc_response_length = kdc_response.len(),
            "resuming SSPI generator with KDC response"
        );

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

    #[instrument(skip_all)]
    pub fn process_initialized_sec_context<P>(
        furniture: &mut SspiContext<P>,
        sec_context: &SecContextInit,
    ) -> Result<ActionReqired, PwshCoreError>
    where
        P: Sspi + SspiImpl,
    {
        let produced = std::mem::take(&mut furniture.out[0].buffer);
        let token = token_header_from(&produced).map(Token);

        debug!(status=?sec_context.init_sec_context_res.status, "SSPI InitializeSecurityContext completed");

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

    #[instrument(skip_all)]
    pub fn wrap<P: Sspi + SspiImpl>(
        provider: &mut P,
        data: &mut [u8],
        sequence_number: u32,
    ) -> Result<Vec<u8>, PwshCoreError> {
        let size_result = provider.query_context_sizes()?;
        debug!(?size_result, "SSPI QueryContextSizes");
        let mut token_buffer = vec![0u8; size_result.security_trailer as usize];
        let sec_token_buffer = SecurityBufferRef::token_buf(&mut token_buffer);
        let sec_data_buffer =
            SecurityBufferRef::data_buf(data).with_flags(SecurityBufferFlags::NONE);

        let mut buffers = [sec_token_buffer, sec_data_buffer];

        let res =
            provider.encrypt_message(EncryptionFlags::empty(), &mut buffers, sequence_number)?;

        debug!(token=?buffers[0],token_len=?buffers[0].buf_len(), data_len=?buffers[1].buf_len(), "SSPI EncryptMessage");

        if res != SecurityStatus::Ok {
            return Err(PwshCoreError::Auth("SSPI EncryptMessage failed"));
        }

        Ok(token_buffer)
    }

    #[instrument(skip(provider, token, encrypted_data), fields(token_len = token.len(), data_len = encrypted_data.len(), sequence_number = sequence_number))]
    pub fn unwrap<P: Sspi + SspiImpl>(
        provider: &mut P,
        token: &[u8],
        encrypted_data: &mut [u8],
        sequence_number: u32,
    ) -> Result<Vec<u8>, PwshCoreError> {
        debug!("SSPI unwrap called with separate token and data buffers");

        // Create a mutable copy of the token for the security buffer
        let mut token_buffer = token.to_vec();

        // Create security buffers: one for the token (signature) and one for the data
        let sec_token_buffer = SecurityBufferRef::token_buf(&mut token_buffer);
        let sec_data_buffer = SecurityBufferRef::data_buf(encrypted_data);

        let mut buffers = [sec_token_buffer, sec_data_buffer];

        debug!(
            buffer_count = buffers.len(),
            token_buffer_type = ?buffers[0].buffer_type(),
            data_buffer_type = ?buffers[1].buffer_type(),
            "Calling SSPI decrypt_message with token and data buffers"
        );

        let result = provider.decrypt_message(&mut buffers, sequence_number);

        match result {
            Ok(_) => {
                let decrypted_buffer = buffers[1].data().to_vec();
                debug!(
                    decrypted_len = decrypted_buffer.len(),
                    "SSPI decrypt_message succeeded with token/data buffers"
                );
                Ok(decrypted_buffer)
            }
            Err(e) => {
                debug!(
                    error = %e,
                    "SSPI decrypt_message failed with token/data buffers"
                );
                Err(e.into())
            }
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
                && let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(rest.trim())
            {
                return Some(bytes);
            }
        }
    }
    None
}
