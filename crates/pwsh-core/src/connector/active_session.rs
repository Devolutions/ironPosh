use crate::{
    connector::http::{HttpBuilder, HttpRequest, HttpResponse},
    pipeline::ParameterValue,
    powershell::PowerShell,
    runspace_pool::{RunspacePool, pool::AcceptResponsResult},
};

#[derive(Debug)]
pub enum SessionStepResult {
    SendBack(HttpRequest<String>),
    PipelineCreated(PowerShell),
    SendBackError(crate::PwshCoreError),
    UserEvent(super::UserEvent),
    OperationSuccess,
}

impl SessionStepResult {
    pub fn priority(&self) -> u8 {
        match self {
            SessionStepResult::SendBack(_) => 0,
            SessionStepResult::PipelineCreated(_) => 1,
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
                let xml_body = self.runspace_pool.fire_create_pipeline()?;
                let response = self.http_builder.post("/wsman", xml_body);
                Ok(SessionStepResult::SendBack(response))
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
                let request = self.runspace_pool.invoke_pipeline_request(powershell);
                match request {
                    Ok(request) => {
                        let response = self.http_builder.post("/wsman", request);
                        Ok(SessionStepResult::SendBack(response))
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
            AcceptResponsResult::ReceiveResponse => {
                let receive_request = self.runspace_pool.fire_receive()?;
                let response = self.http_builder.post("/wsman", receive_request);
                Ok(SessionStepResult::SendBack(response))
            }
            AcceptResponsResult::NewPipeline(pipeline) => {
                Ok(SessionStepResult::PipelineCreated(pipeline))
            }
        }
    }
}
