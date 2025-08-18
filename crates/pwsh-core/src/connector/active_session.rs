use crate::{
    connector::http::{HttpBuilder, HttpRequest, HttpResponse},
    pipeline::ParameterValue,
    powershell::PowerShell,
    runspace_pool::{RunspacePool, pool::AcceptResponsResult},
};

#[derive(Debug)]
pub enum UserEvent {
    PipelineCreated { powershell: PowerShell },
}

#[derive(Debug)]
pub enum SessionStepResult {
    SendBack(Vec<HttpRequest<String>>),
    SendBackError(crate::PwshCoreError),
    UserEvent(UserEvent),
    OperationSuccess,
}

impl SessionStepResult {
    pub fn priority(&self) -> u8 {
        match self {
            SessionStepResult::SendBack(_) => 0,
            SessionStepResult::SendBackError(_) => 2,
            SessionStepResult::UserEvent(_) => 3,
            SessionStepResult::OperationSuccess => 4,
        }
    }
}

#[derive(Debug)]
pub enum PowershellOperations {
    AddScript(String),
    AddCommand(String),
    AddParameter { name: String, value: ParameterValue },
    AddArgument(String),
}

#[derive(Debug)]
pub enum UserOperation {
    CreatePipeline,
    OperatePipeline {
        powershell: PowerShell,
        operation: PowershellOperations,
    },
    InvokePipeline {
        powershell: PowerShell,
    },
}

/// ActiveSession manages post-connection operations
#[derive(Debug)]
pub struct ActiveSession {
    runspace_pool: RunspacePool,
    http_builder: HttpBuilder,
}

impl ActiveSession {
    pub fn new(runspace_pool: RunspacePool, http_builder: HttpBuilder) -> Self {
        Self {
            runspace_pool,
            http_builder,
        }
    }

    /// Handle a client-initiated operation
    pub fn accept_client_operation(
        &mut self,
        operation: UserOperation,
    ) -> Result<SessionStepResult, crate::PwshCoreError> {
        match operation {
            UserOperation::CreatePipeline => {
                Ok(SessionStepResult::UserEvent(UserEvent::PipelineCreated {
                    powershell: self.runspace_pool.init_pipeline(),
                }))
            }
            UserOperation::OperatePipeline {
                powershell,
                operation,
            } => {
                match operation {
                    PowershellOperations::AddScript(script) => {
                        self.runspace_pool.add_script(powershell, script)?;
                    }
                    PowershellOperations::AddCommand(command) => {
                        self.runspace_pool.add_command(powershell, command)?;
                    }
                    PowershellOperations::AddParameter { name, value } => {
                        self.runspace_pool.add_parameter(powershell, name, value)?;
                    }
                    PowershellOperations::AddArgument(arg) => {
                        self.runspace_pool.add_switch_parameter(powershell, arg)?;
                    }
                }
                Ok(SessionStepResult::OperationSuccess)
            }
            UserOperation::InvokePipeline { powershell } => {
                let command_request = self.runspace_pool.invoke_pipeline_request(powershell);
                match command_request {
                    Ok(request) => {
                        let response = self.http_builder.post("/wsman", request);
                        Ok(SessionStepResult::SendBack(vec![response]))
                    }
                    Err(e) => Ok(SessionStepResult::SendBackError(e)),
                }
            }
        }
    }

    /// Handle a server response
    pub fn accept_server_response(
        &mut self,
        response: HttpResponse<String>,
    ) -> Result<SessionStepResult, crate::PwshCoreError> {
        let body = response.body.ok_or(crate::PwshCoreError::InvalidState(
            "Expected a body in server response",
        ))?;

        match self.runspace_pool.accept_response(body)? {
            AcceptResponsResult::ReceiveResponse { desired_streams } => {
                let receive_request = self.runspace_pool.fire_receive(desired_streams)?;
                let response = self.http_builder.post("/wsman", receive_request);
                Ok(SessionStepResult::SendBack(vec![response]))
            }
            AcceptResponsResult::NewPipeline(pipeline) => {
                Ok(SessionStepResult::UserEvent(UserEvent::PipelineCreated {
                    powershell: pipeline,
                }))
            }
        }
    }
}
