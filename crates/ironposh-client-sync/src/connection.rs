use anyhow::Context;
use ironposh_client_core::connector::{
    conntion_pool::TrySend,
    http::{HttpRequest, HttpResponse},
    Connector, ConnectorStepResult, WinRmConfig,
};
use tracing::{info, instrument};

use crate::auth_handler::AuthHandler;

pub trait HttpClient {
    fn send_request(
        &self,
        request: HttpRequest,
        connection_id: u32,
    ) -> Result<HttpResponse, anyhow::Error>;
}

pub struct RemotePowershell {
    active_session: ironposh_client_core::connector::active_session::ActiveSession,
    next_request: TrySend,
}

impl RemotePowershell {
    /// Establish connection to the PowerShell remote server
    pub fn open(config: WinRmConfig, client: &mut dyn HttpClient) -> Result<Self, anyhow::Error> {
        let mut connector = Connector::new(config);
        let mut response = None;
        let mut authenticate_cert = None;

        let (active_session, next_request) = loop {
            if let Some(cert) = authenticate_cert.take() {
                connector.accept_authenticated_http_channel(cert)?;
            }

            let step_result = connector.step(response.take())?;

            match step_result {
                ConnectorStepResult::SendBack { try_send } => match try_send {
                    ironposh_client_core::connector::conntion_pool::TrySend::JustSend {
                        request,
                        conn_id,
                    } => {
                        let res = client.send_request(request, conn_id.inner())?;
                        response = Some((res, conn_id));
                    }

                    ironposh_client_core::connector::conntion_pool::TrySend::AuthNeeded {
                        auth_sequence,
                    } => {
                        let (http_authenticated, auth_response) = AuthHandler::handle_auth_sequence(client, auth_sequence)?;
                        
                        authenticate_cert = Some(http_authenticated);
                        if let Some(auth_resp) = auth_response {
                            response = Some(auth_resp);
                        }
                    }
                },
                ConnectorStepResult::Connected {
                    active_session,
                    next_receive_request,
                } => {
                    break (*active_session, next_receive_request);
                }
            }
        };

        Ok(Self {
            active_session,
            next_request,
        })
    }

    /// Extract the components for use in the main event loop
    pub fn into_components(
        self,
    ) -> (
        ironposh_client_core::connector::active_session::ActiveSession,
        ironposh_client_core::connector::conntion_pool::TrySend,
    ) {
        (self.active_session, self.next_request)
    }
}
