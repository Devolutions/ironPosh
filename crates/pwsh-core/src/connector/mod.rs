use std::sync::Arc;

use protocol_powershell_remoting::HostInfo;
use protocol_winrm::ws_management::WsMan;
use tracing::{info, instrument, warn};

use crate::{
    connector::http::{HttpBuilder, HttpRequest, ServerAddress},
    runspace_pool::{
        ExpectShellCreated, PowerShell, RunspacePool, RunspacePoolCreator, RunspacePoolState,
        pool::AcceptResponsResult,
    },
};
pub mod http;

#[derive(Debug, Clone)]
pub enum Authentication {
    Basic { username: String, password: String },
    // TODO: Add SSPI
}

#[derive(Debug, Clone)]
pub enum Scheme {
    Http,
    Https,
}

pub struct ConnectorConfig {
    pub server: (ServerAddress, u16),
    pub scheme: Scheme,
    pub authentication: Authentication,
}

impl ConnectorConfig {
    pub fn wsman_to(&self, query: Option<&str>) -> String {
        let query = query
            .map(|q| format!("?{}", q.trim_start_matches('?')))
            .unwrap_or_default();

        match &self.scheme {
            Scheme::Http => format!("http://{}:{}/wsman{}", self.server.0, self.server.1, query),
            Scheme::Https => format!("https://{}:{}/wsman{}", self.server.0, self.server.1, query),
        }
    }
}

#[derive(Debug)]
pub enum UserEvent {}

#[derive(Debug)]
pub enum StepResult {
    SendBack(HttpRequest<String>),
    PipelineCreated(PowerShell),
    SendBackError(crate::PwshCoreError),
    UserEvent(UserEvent),
    ReadyForOperation {
        user_operation_issuer: UserOperationIssuer,
    },
}

#[derive(Debug)]
pub enum UserOperation {
    CreatePipeline,
}

pub struct UserOperationCertificate {
    user_operation: UserOperation,
}

#[derive(Debug)]
pub struct UserOperationIssuer {
    __marker: std::marker::PhantomData<()>,
}

impl UserOperationIssuer {
    pub fn issue_operation(
        self,
        operation: UserOperation,
    ) -> Result<UserOperationCertificate, crate::PwshCoreError> {
        // consume self, and return a certificate for the operation
        Ok(UserOperationCertificate {
            user_operation: operation,
        })
    }
}

#[derive(Default, Debug)]
pub enum ConnectorState {
    Idle,
    #[default]
    Taken,
    Connecting {
        expect_shell_created: ExpectShellCreated,
        http_builder: HttpBuilder,
    },
    ConnectReceiveCycle {
        runspace_pool: RunspacePool,
        http_builder: HttpBuilder,
    },
    ReadyForOperations {
        runspace_pool: RunspacePool,
        http_builder: HttpBuilder,
    },
    Failed,
}

impl ConnectorState {
    fn state_name(&self) -> &'static str {
        match self {
            ConnectorState::Idle => "Idle",
            ConnectorState::Taken => "Taken",
            ConnectorState::Connecting { .. } => "Connecting",
            ConnectorState::ConnectReceiveCycle { .. } => "ConnectReceiveCycle",
            ConnectorState::ReadyForOperations { .. } => "ReadyForOperations",
            ConnectorState::Failed => "Failed",
        }
    }
}

pub struct Connector {
    state: ConnectorState,
    config: ConnectorConfig,
}

impl Connector {
    pub fn new(config: ConnectorConfig) -> Self {
        Self {
            state: ConnectorState::Idle,
            config,
        }
    }

    pub fn set_state(&mut self, state: ConnectorState) {
        info!(state = state.state_name(), "Setting connector state");
        self.state = state;
    }

    #[instrument(skip(self, server_response, user_request), name = "Connector::step")]
    pub fn step(
        &mut self,
        server_response: Option<HttpRequest<String>>,
        user_request: Option<UserOperationCertificate>,
    ) -> Result<Vec<StepResult>, crate::PwshCoreError> {
        let state = std::mem::take(&mut self.state);

        let (new_state, response) = match state {
            ConnectorState::Taken => {
                return Err(crate::PwshCoreError::UnlikelyToHappen(
                    "Connector should not be in Taken state when stepping",
                ));
            }
            ConnectorState::Failed => {
                warn!("Connector is in Failed state, cannot proceed");
                return Err(crate::PwshCoreError::InvalidState(
                    "Connector is in Failed state",
                ));
            }
            ConnectorState::Idle => {
                debug_assert!(
                    server_response.is_none(),
                    "Request should be None in Idle state"
                );
                let connection = Arc::new(WsMan::builder().to(self.config.wsman_to(None)).build());
                let runspace_pool = RunspacePoolCreator::builder()
                    .host_info(HostInfo::builder().build())
                    .build()
                    .into_runspace_pool(connection);

                let http_builder = HttpBuilder::new(
                    self.config.server.0.clone(),
                    self.config.server.1,
                    self.config.scheme.clone(),
                    self.config.authentication.clone(),
                );

                let (xml_body, expect_shell_created) = runspace_pool.open()?;

                let response = http_builder.post("/wsman", xml_body);

                let new_state = ConnectorState::Connecting {
                    expect_shell_created,
                    http_builder,
                };

                // Now we expect the shell create response
                (new_state, vec![StepResult::SendBack(response)])
            }
            ConnectorState::Connecting {
                expect_shell_created,
                http_builder,
            } => {
                info!("Processing Connecting state");
                let request = server_response.ok_or({
                    crate::PwshCoreError::InvalidState("Expected a request in Connecting state")
                })?;

                let body = request.body.ok_or({
                    crate::PwshCoreError::InvalidState("Expected a body in Connecting state")
                })?;

                let mut runspace_pool = expect_shell_created.accept(body)?;

                let receive_request = runspace_pool.fire_receive()?;

                let response = http_builder.post("/wsman", receive_request);

                let new_state = ConnectorState::ConnectReceiveCycle {
                    runspace_pool,
                    http_builder,
                };

                (new_state, vec![StepResult::SendBack(response)])
            }
            ConnectorState::ConnectReceiveCycle {
                mut runspace_pool,
                http_builder,
            } => 'receive_cycle: {
                let request = server_response.ok_or({
                    crate::PwshCoreError::InvalidState(
                        "Expected a request in ConnectReceiveCycle state",
                    )
                })?;

                let body = request.body.ok_or({
                    crate::PwshCoreError::InvalidState(
                        "Expected a body in ConnectReceiveCycle state",
                    )
                })?;

                // Ok, we definately need to change control flow here
                let AcceptResponsResult::ReceiveResponse = runspace_pool.accept_response(body)?
                else {
                    return Err(crate::PwshCoreError::InvalidState(
                        "Unexpected response type in ConnectReceiveCycle state from RunspacePool",
                    ));
                };

                if let RunspacePoolState::NegotiationSent = runspace_pool.state {
                    let receive_request = runspace_pool.fire_receive()?;
                    let response = http_builder.post("/wsman", receive_request);
                    let new_state = ConnectorState::ConnectReceiveCycle {
                        runspace_pool,
                        http_builder,
                    };
                    break 'receive_cycle (new_state, vec![StepResult::SendBack(response)]);
                }

                if let RunspacePoolState::Opened = runspace_pool.state {
                    info!("Runspace pool is opened, ready for operations");
                    let response = http_builder.post("/wsman", String::new());
                    break 'receive_cycle (
                        ConnectorState::ReadyForOperations {
                            runspace_pool,
                            http_builder,
                        },
                        vec![StepResult::SendBack(response)],
                    );
                }

                warn!(
                    "Unexpected RunspacePool state: {:?}, continuing with next step",
                    runspace_pool.state
                );

                (ConnectorState::Failed, vec![])
            }
            ConnectorState::ReadyForOperations {
                mut runspace_pool,
                http_builder,
            } => {
                let mut responses = vec![];
                if let Some(user_operation_certificate) = user_request {
                    let UserOperationCertificate { user_operation } = user_operation_certificate;

                    match user_operation {
                        UserOperation::CreatePipeline => {
                            let xml_body = runspace_pool.fire_create_pipeline()?;
                            let response = http_builder.post("/wsman", xml_body);
                            responses.push(StepResult::SendBack(response));
                        }
                    }
                }

                if let Some(request) = server_response {
                    let body = request.body.ok_or({
                        crate::PwshCoreError::InvalidState(
                            "Expected a body in ReadyForOperations state",
                        )
                    })?;

                    match runspace_pool.accept_response(body)? {
                        AcceptResponsResult::ReceiveResponse => {
                            let receive_request = runspace_pool.fire_receive()?;
                            let response = http_builder.post("/wsman", receive_request);
                            responses.push(StepResult::SendBack(response));
                        }
                        AcceptResponsResult::NewPipeline(pipeline) => {
                            responses.push(StepResult::PipelineCreated(pipeline));
                        }
                    }
                }

                let new_state = ConnectorState::ReadyForOperations {
                    runspace_pool,
                    http_builder,
                };

                (new_state, responses)
            }
        };

        self.set_state(new_state);
        Ok(response)
    }
}
