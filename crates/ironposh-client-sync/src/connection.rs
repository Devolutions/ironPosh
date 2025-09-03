use ironposh_client_core::connector::{
    auth_sequence::{
        AuthSequence, KerberoRequestPacket, SecContextProcessResult, TryInitSecContext,
    },
    http::{HttpRequest, HttpResponse},
    Connector, ConnectorConfig, ConnectorStepResult,
};

pub trait HttpClient {
    fn send_request(
        &self,
        request: HttpRequest<String>,
    ) -> Result<HttpResponse<String>, anyhow::Error>;
}

pub struct RemotePowershell {
    active_session: ironposh_client_core::connector::active_session::ActiveSession,
    client: Box<dyn HttpClient>,
    next_request: ironposh_client_core::connector::http::HttpRequest<String>,
}

impl RemotePowershell {
    /// Establish connection to the PowerShell remote server
    pub fn open(
        config: ConnectorConfig,
        client: impl HttpClient + 'static,
    ) -> Result<Self, anyhow::Error> {
        let mut connector = Connector::new(config);
        let mut response = None;

        let (active_session, next_request) = loop {
            let step_result = connector.step(response.take())?;

            match step_result {
                ConnectorStepResult::SendBack(http_request) => {
                    response = Some(client.send_request(http_request)?);
                }
                ConnectorStepResult::SendBackError(e) => {
                    anyhow::bail!("Connection failed: {}", e);
                }
                ConnectorStepResult::Connected {
                    active_session,
                    next_receive_request,
                } => {
                    break (*active_session, next_receive_request);
                }
                ConnectorStepResult::Borrowed {
                    mut context,
                    sequence,
                    mut http_builder,
                } => {
                    let mut auth_response = None;

                    loop {
                        let init_sec_res = {
                            let result = sequence
                                .try_init_sec_context(&mut context, auth_response.take())?;

                             match result {
                                TryInitSecContext::RunGenerator {
                                    packet,
                                    generator_holder,
                                } => {
                                    let mut packet = Some(packet);
                                    let mut generator = Some(generator_holder);

                                    loop {
                                        let inner_packet = packet.take().unwrap();
                                        let kdc_response = send_packet(inner_packet);
                                        let resume_result = sequence
                                            .resume(kdc_response, generator.take().unwrap())?;

                                        match resume_result {
                                            TryInitSecContext::RunGenerator {
                                                packet: packet1,
                                                generator_holder: generator1,
                                            } => {
                                                packet = Some(packet1);
                                                generator = Some(generator1);
                                            }
                                            TryInitSecContext::Initialized {
                                                init_sec_context_res,
                                            } => {
                                                break init_sec_context_res;
                                            }
                                        }
                                    }
                                }
                                TryInitSecContext::Initialized {
                                    init_sec_context_res,
                                } => init_sec_context_res,
                            }
                        };

                        match sequence.process_initialized_sec_context(
                            &mut context,
                            &mut http_builder,
                            init_sec_res,
                        )? {
                            SecContextProcessResult::TryInitAgain { request } => {
                                auth_response = Some(client.send_request(request)?);
                            }
                            SecContextProcessResult::Done { token } => {
                                let (connector_ref, http_builder) = sequence.destruct_me();
                                let request = connector_ref.authenticate(token, http_builder)?;
                                response = Some(client.send_request(request)?);
                                break;
                            }
                        }
                    }
                }
            }
        };

        Ok(Self {
            active_session,
            client: Box::new(client),
            next_request,
        })
    }

    /// Extract the components for use in the main event loop
    pub fn into_components(
        self,
    ) -> (
        ironposh_client_core::connector::active_session::ActiveSession,
        ironposh_client_core::connector::http::HttpRequest<String>,
    ) {
        (self.active_session, self.next_request)
    }
}

fn send_packet(packet: KerberoRequestPacket) -> Vec<u8> {
    todo!()
}
