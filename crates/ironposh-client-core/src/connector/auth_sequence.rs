use crate::connector::{
    authenticator::NtlmAuthenticator,
    http::{HttpBuilder, HttpResponse},
};

#[derive(Debug)]
pub struct AuthSequence<'connector> {
    pub(crate) connector: &'connector mut super::Connector,
    pub(crate) authenticator: NtlmAuthenticator,
    pub(crate) http_builder: super::http::HttpBuilder,
}

pub enum AuthSequenceStepResult {
    SendBackAndContinue {
        request: super::http::HttpRequest<String>,
    },
    SendToHereAndContinue {
        request: super::http::HttpRequest<String>,
        to: String,
    },
    DestructMe,
}

impl<'connector> AuthSequence<'connector> {
    pub fn step(
        &mut self,
        server_response: Option<HttpResponse<String>>,
    ) -> Result<AuthSequenceStepResult, crate::PwshCoreError> {
        let result = match self.authenticator.step(server_response.as_ref())? {
            crate::connector::authenticator::AuthenticaterStepResult::SendBackAndContinue {
                token,
            } => {
                self.http_builder.with_auth_header(token);
                let request = self.http_builder.post("/wsman", String::new());

                AuthSequenceStepResult::SendBackAndContinue { request }
            }
            crate::connector::authenticator::AuthenticaterStepResult::SendToHereAndContinue {
                request,
                to,
            } => todo!("Handle kerberos here"),
            crate::connector::authenticator::AuthenticaterStepResult::Done { token } => {
                if let Some(token) = token {
                    self.http_builder.with_auth_header(token);
                }
                AuthSequenceStepResult::DestructMe
            }
        };

        Ok(result)
    }

    pub fn destruct_me(self) -> (&'connector mut super::Connector, HttpBuilder) {
        let AuthSequence {
            connector,
            http_builder,
            ..
        } = self;
        // I think we should return some sort of decryptor/encryptor here
        // And update connector's state
        (connector, http_builder)
    }
}
