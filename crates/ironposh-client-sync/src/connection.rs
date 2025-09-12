use ironposh_client_core::connector::{
    conntion_pool::TrySend,
    http::{HttpRequest, HttpResponse, HttpResponseTargeted},
    Connector, ConnectorStepResult, WinRmConfig,
};

pub trait HttpClient {
    fn send_request(&self, try_send: TrySend) -> Result<HttpResponseTargeted, anyhow::Error>;
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

        let (active_session, next_request) = loop {
            let step_result = connector.step(response.take())?;

            match step_result {
                ConnectorStepResult::SendBack { try_send } => {
                    response = Some(client.send_request(try_send)?);
                }
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
