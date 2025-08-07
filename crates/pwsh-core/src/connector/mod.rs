use std::sync::Arc;

use protocol_winrm::ws_management::WsMan;

use crate::{
    connector::http::{HttpBuilder, HttpRequest, ServerAddress},
    powershell::{ExpectShellCreated, RunspacePool},
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
    pub fn wsman_to(&self, query: &str) -> String {
        match &self.scheme {
            Scheme::Http => format!("http://{}:{}/wsman?{}", self.server.0, self.server.1, query),
            Scheme::Https => format!(
                "https://{}:{}/wsman?{}",
                self.server.0, self.server.1, query
            ),
        }
    }
}

#[derive(Debug)]
pub enum UserEvent {}

#[derive(Debug)]
pub enum StepResult {
    SendBack(HttpRequest<String>),
    SendBackError(crate::PwshCoreError),
    UserEvent(UserEvent),
    ReadyForOperation,
}

#[derive(Default)]
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
        self.state = state;
    }

    pub fn step(
        &mut self,
        request: Option<HttpRequest<String>>,
    ) -> Result<StepResult, crate::PwshCoreError> {
        let state = std::mem::take(&mut self.state);

        let response = match state {
            ConnectorState::Taken => {
                return Err(crate::PwshCoreError::UnlikelyToHappen(
                    "Connector should not be in Taken state when stepping",
                ));
            }
            ConnectorState::Idle => {
                debug_assert!(request.is_none(), "Request should be None in Idle state");
                let runspace_pool = RunspacePool::builder()
                    .connection(Arc::new(
                        WsMan::builder()
                            .to(self.config.wsman_to("PSVersion=7.4.10"))
                            .build(),
                    ))
                    .build();

                let http_builder = HttpBuilder::new(
                    self.config.server.0.clone(),
                    self.config.server.1,
                    self.config.scheme.clone(),
                    self.config.authentication.clone(),
                );

                let (xml_body, expect_shell_created) = runspace_pool.open()?;

                let response = http_builder.post("/wsman", xml_body);

                self.set_state(ConnectorState::Connecting {
                    expect_shell_created,
                    http_builder,
                });

                // Now we expect the shell create response
                StepResult::SendBack(response)
            }
            ConnectorState::Connecting {
                expect_shell_created,
                http_builder,
            } => {
                let request = request.ok_or({
                    crate::PwshCoreError::InvalidState("Expected a request in Connecting state")
                })?;

                let body = request.body.ok_or({
                    crate::PwshCoreError::InvalidState("Expected a body in Connecting state")
                })?;

                let runspace_pool = expect_shell_created.accept(body)?;

                let receive_request = runspace_pool.fire_receive();

                let response = http_builder.post("/wsman", receive_request);

                self.set_state(ConnectorState::ConnectReceiveCycle {
                    runspace_pool,
                    http_builder,
                });

                StepResult::SendBack(response)
            }
            ConnectorState::ConnectReceiveCycle {
                mut runspace_pool,
                http_builder,
            } => {
                let request = request.ok_or({
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
                runspace_pool.accept_receive_response(body)?;

                todo!()

                // let receive_response = runspace_pool.fire_receive();

                // let response = http_builder.post("/wsman", receive_response);

                // self.set_state(ConnectorState::Taken);

                // StepResult::SendBack(response)
            }
        };

        Ok(response)
    }
}
