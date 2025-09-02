use sspi::{NetworkRequest, Sspi};

use crate::{
    connector::{
        authenticator::{
            AuthFurniture, AuthenticatorStepResult, SspiAuthenticator, SspiStepRequest, Token,
        },
        http::{HttpBuilder, HttpResponse},
    },
};

pub struct AuthSequence<'conn, 'auth, P: Sspi> {
    pub(crate) connector: &'conn mut super::Connector,
    pub(crate) authenticator: SspiAuthenticator,
    pub(crate) http_builder: super::http::HttpBuilder,
    context: AuthFurniture<'auth, P>,
    state: AuthSequenceState<'auth>,
}

// Baiscally, the generator should live as long as the AuthContext, which is owned by AuthSequence
#[derive(Debug, Default)]
pub enum AuthSequenceState<'a> {
    #[default]
    Initialized,
    WaitingForGeneratorResponse {
        generator: SspiStepRequest<'a>,
    },
    Resumed,
    Complete,
}

pub enum AuthSequenceStepResult {
    Continue,
    ContinueWithToken { token: Token },
    NeedsKdcResponse { packet: NetworkRequest },
    Done { token: Option<Token> },
}

pub enum ResumeResult {
    NeedsKdcResponse { packet: NetworkRequest },
    ContinueStep,
}

impl<'conn, 'auth, P> AuthSequence<'conn, 'auth, P>
where
    P: Sspi,
{
    pub fn step(
        &'auth mut self,
        server_response: Option<HttpResponse<String>>,
    ) -> Result<AuthSequenceStepResult, crate::PwshCoreError> {
        let state = std::mem::take(&mut self.state);

        let (next_state, result) = match state {
            AuthSequenceState::Initialized => {
                let step_result = self
                    .authenticator
                    .try_init_sec_context(server_response.as_ref(), &mut self.context)?;

                match step_result {
                    AuthenticatorStepResult::Continue => (
                        AuthSequenceState::Initialized,
                        AuthSequenceStepResult::Continue,
                    ),
                    AuthenticatorStepResult::ContinueWithToken { token } => (
                        AuthSequenceState::Initialized,
                        AuthSequenceStepResult::ContinueWithToken { token },
                    ),
                    AuthenticatorStepResult::SendToHereAndContinue {
                        packet,
                        generator_holder,
                    } => (
                        AuthSequenceState::WaitingForGeneratorResponse {
                            generator: generator_holder,
                        },
                        AuthSequenceStepResult::NeedsKdcResponse { packet },
                    ),
                    AuthenticatorStepResult::Done { token } => (
                        AuthSequenceState::Complete,
                        AuthSequenceStepResult::Done { token },
                    ),
                }
            }
            AuthSequenceState::WaitingForGeneratorResponse { .. } => {
                return Err(crate::PwshCoreError::InvalidState(
                    "AuthSequence is already waiting for KDC response, should use resume()",
                ));
            }
            AuthSequenceState::Resumed  => {
                todo!()   
            }
            AuthSequenceState::Complete => {
                return Err(crate::PwshCoreError::InvalidState(
                    "AuthSequence is already complete",
                ));
            }
        };

        self.state = next_state;

        Ok(result)
    }

    pub fn resume(
        &'auth mut self,
        kdc_response: Vec<u8>,
    ) -> Result<ResumeResult, crate::PwshCoreError> {
        let state = std::mem::take(&mut self.state);
        let AuthSequenceState::WaitingForGeneratorResponse { generator } = state else {
            return Err(crate::PwshCoreError::InvalidState(
                "AuthSequence is not waiting for KDC response",
            ));
        };

        match self.authenticator.resume(generator, kdc_response)? {
            crate::connector::authenticator::GeneratorResumeResult::DoItAgainBro {
                packet,
                generator_holder,
            } => {
                self.state = AuthSequenceState::WaitingForGeneratorResponse {
                    generator: generator_holder,
                };
                return Ok(ResumeResult::NeedsKdcResponse { packet });
            }
            crate::connector::authenticator::GeneratorResumeResult::NahYouGood => {
                self.state = AuthSequenceState::Resumed;
                return Ok(ResumeResult::ContinueStep);
            }
        }
    }

    pub fn destruct_me(self) -> (&'conn mut super::Connector, HttpBuilder) {
        todo!()
    }
}
