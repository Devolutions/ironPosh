use std::fmt::Debug;

use base64::Engine;
use sspi::{
    BufferType, ClientRequestFlags, CredentialUse,
    DataRepresentation, Ntlm, SecurityBuffer, SecurityStatus, Sspi, SspiImpl,
};

use crate::connector::http::{HttpRequest, HttpResponse};
use crate::credentials::ClientAuthIdentity;

#[derive(Debug, Default)]
pub enum SspiAuthenticatorState<P: Sspi> {
    #[default]
    Taken,
    PreAuthentication {
        prov: P,
        identity: ClientAuthIdentity,
    },
    InAuthentication {
        prov: P,
        cred: P::CredentialsHandle,
    },
    Established {},
}

pub struct SspiAuthenticator<P: Sspi> {
    state: SspiAuthenticatorState<P>,
}

impl<P> Debug for SspiAuthenticator<P>
where
    P: Sspi,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SspiAuthenticator").finish()
    }
}

#[derive(Debug)]
pub enum AuthenticaterStepResult {
    SendBackAndContinue {
        token: String,
    },
    SendToHereAndContinue {
        request: HttpRequest<String>,
        to: String,
    },
    Done {
        // Sometimes we need to send last token with the final request
        token: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub enum Authentication {
    Basic { username: String, password: String },
    // TODO: I should make user have choice of what sspi provider to use
    Sspi { identity: ClientAuthIdentity },
}

// Convenience type aliases
pub type NtlmAuthenticator = SspiAuthenticator<sspi::ntlm::Ntlm>;
pub type NegotiateAuthenticator = SspiAuthenticator<sspi::negotiate::Negotiate>;

impl NtlmAuthenticator {
    pub fn new_ntlm(identity: ClientAuthIdentity) -> Self {
        let provider = Ntlm::new();
        Self {
            state: SspiAuthenticatorState::PreAuthentication {
                prov: provider,
                identity,
            },
        }
    }
}

impl<ISspi> SspiAuthenticator<ISspi>
where
    ISspi: Sspi + SspiImpl<AuthenticationData = sspi::AuthIdentity>,
{

    pub fn step(
        &mut self,
        response: Option<&HttpResponse<String>>,
    ) -> Result<AuthenticaterStepResult, crate::PwshCoreError> {
        let state = std::mem::take(&mut self.state);

        let (next_state, action) = match state {
            SspiAuthenticatorState::Taken => {
                return Err(crate::PwshCoreError::InvalidState(
                    "Authenticator already taken",
                ));
            }

            SspiAuthenticatorState::PreAuthentication { mut prov, identity } => {
                debug_assert!(
                    response.is_none(),
                    "Response should be None in PreAuthentication state"
                );

                let mut acq = prov
                    .acquire_credentials_handle()
                    .with_credential_use(CredentialUse::Outbound);

                // Convert our ClientAuthIdentity to sspi::AuthIdentity for the SSPI layer
                let sspi_identity: sspi::AuthIdentity = identity.into_inner();
                acq = acq.with_auth_data(&sspi_identity);

                let mut cred = acq.execute(&mut prov)?.credentials_handle;

                // Produce initial token
                let mut out = [SecurityBuffer::new(Vec::new(), BufferType::Token)];
                let mut isc = prov
                    .initialize_security_context()
                    .with_credentials_handle(&mut cred)
                    .with_context_requirements(ClientRequestFlags::ALLOCATE_MEMORY)
                    .with_target_data_representation(DataRepresentation::Native)
                    .with_target_name("HTTP/host.example.com")
                    .with_output(&mut out);

                prov.initialize_security_context_impl(&mut isc)?
                    .resolve_to_result()?;
                let client_token_b64 =
                    base64::engine::general_purpose::STANDARD.encode(&out[0].buffer);

                let next = SspiAuthenticatorState::InAuthentication { prov, cred };
                (
                    next,
                    AuthenticaterStepResult::SendBackAndContinue {
                        token: format!("Negotiate {client_token_b64}"),
                    },
                )
            }

            SspiAuthenticatorState::InAuthentication { mut prov, mut cred } => {
                // Expect a 401 with WWW-Authenticate: Negotiate <b64>
                let resp = response.ok_or(crate::PwshCoreError::InvalidState(
                    "response required in InAuthentication state",
                ))?;

                let server_tok = parse_negotiate_token(&resp.headers)
                    .ok_or_else(|| crate::PwshCoreError::Auth("no Negotiate token"))?;

                let mut input_buffer = [SecurityBuffer::new(server_tok, BufferType::Token)];
                let mut output_buffer = [SecurityBuffer::new(Vec::new(), BufferType::Token)];
                let mut isc = prov
                    .initialize_security_context()
                    .with_credentials_handle(&mut cred)
                    .with_context_requirements(
                        ClientRequestFlags::CONFIDENTIALITY | ClientRequestFlags::ALLOCATE_MEMORY,
                    )
                    .with_target_data_representation(DataRepresentation::Native)
                    .with_output(&mut output_buffer)
                    .with_input(&mut input_buffer);

                let result = prov
                    .initialize_security_context_impl(&mut isc)?
                    .resolve_to_result()?; // Handle Kerberos generator herstatus here

                let client_token_b64 =
                    base64::engine::general_purpose::STANDARD.encode(&output_buffer[0].buffer);

                match result.status {
                    SecurityStatus::ContinueNeeded => {
                        let next = SspiAuthenticatorState::InAuthentication { prov, cred };
                        (
                            next,
                            AuthenticaterStepResult::SendBackAndContinue {
                                token: format!("Negotiate {client_token_b64}"),
                            },
                        )
                    }
                    SecurityStatus::Ok => {
                        let next = SspiAuthenticatorState::Established {};
                        (
                            next,
                            AuthenticaterStepResult::Done {
                                token: if output_buffer[0].buffer.is_empty() {
                                    None
                                } else {
                                    Some(format!("Negotiate {client_token_b64}"))
                                },
                            },
                        )
                    }
                    _ => {
                        todo!(" handle other kind of issue")
                    }
                }
            }

            SspiAuthenticatorState::Established {} => {
                todo!("Maybe we shouldn't do anything here")
                // (next, AuthenticaterStepResult::Done { token: None })
            }
        };

        self.state = next_state;
        Ok(action)
    }
}

// Helper function to parse negotiate token from headers
fn parse_negotiate_token(headers: &Vec<(String, String)>) -> Option<Vec<u8>> {
    for (key, value) in headers {
        if key.to_lowercase() == "www-authenticate" && value.starts_with("Negotiate ") {
            let token_b64 = &value[10..]; // Skip "Negotiate "
            return base64::engine::general_purpose::STANDARD
                .decode(token_b64)
                .ok();
        }
    }
    None
}
