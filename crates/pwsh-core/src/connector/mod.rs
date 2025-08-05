use std::{net::IpAddr, sync::Arc};

use protocol_winrm::{cores::S, ws_management::WsMan};

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
    pub(crate) server: (ServerAddress, u16),
    pub(crate) scheme: Scheme,
    pub(crate) authentication: Authentication,
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

pub enum ConnectorState {
    Idel,
    Connecting {
        expect_shell_created: ExpectShellCreated,
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
            state: ConnectorState::Idel,
            config,
        }
    }

    pub fn step(
        &mut self,
        request: Option<HttpRequest<String>>,
    ) -> Result<StepResult, crate::PwshCoreError> {
        let response = match &self.state {
            ConnectorState::Idel => {
                debug_assert!(request.is_none(), "Request should be None in Idel state");
                let runspace_pool = RunspacePool::builder()
                    .connection(Arc::new(WsMan::builder().build()))
                    .build();

                let http_builder = HttpBuilder::new(
                    self.config.server.0.clone(),
                    self.config.server.1,
                    self.config.scheme.clone(),
                    self.config.authentication.clone(),
                );

                let (xml_body, expect_shell_created) = runspace_pool.open()?;

                let response = http_builder.post("/wsman", xml_body);

                self.state = ConnectorState::Connecting {
                    expect_shell_created,
                    http_builder,
                };

                // Now we expect the shell create response
                StepResult::SendBack(response)
            }
            ConnectorState::Connecting {
                expect_shell_created,
                http_builder,
            } => {
                todo!()
            }
        };

        Ok(response)
    }
}
