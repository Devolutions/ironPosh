use crate::{
    connector::http::{HttpBuilder, HttpRequest, HttpResponse},
    host::{HostCallMethodWithParams, HostCallRequest, HostCallResponse},
    pipeline::ParameterValue,
    powershell::PowerShell,
    runspace_pool::{RunspacePool, pool::AcceptResponsResult},
};

#[derive(Debug, PartialEq, Eq)]
pub enum UserEvent {
    PipelineCreated { powershell: PowerShell },
}

#[derive(Debug)]
pub enum ActiveSessionOutput {
    SendBack(Vec<HttpRequest<String>>),
    SendBackError(crate::PwshCoreError),
    UserEvent(UserEvent),
    HostCall(HostCallRequest),
    OperationSuccess,
}

impl PartialEq for ActiveSessionOutput {
    fn eq(&self, other: &Self) -> bool {
        self.priority() == other.priority()
    }
}

impl Eq for ActiveSessionOutput {}

impl PartialOrd for ActiveSessionOutput {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ActiveSessionOutput {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority().cmp(&other.priority())
    }
}

impl ActiveSessionOutput {
    pub fn priority(&self) -> u8 {
        match self {
            ActiveSessionOutput::HostCall { .. } => 1,
            ActiveSessionOutput::SendBack(_) => 2,
            ActiveSessionOutput::SendBackError(_) => 3,
            ActiveSessionOutput::UserEvent(_) => 4,
            ActiveSessionOutput::OperationSuccess => 5,
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
    ) -> Result<ActiveSessionOutput, crate::PwshCoreError> {
        match operation {
            UserOperation::CreatePipeline => {
                Ok(ActiveSessionOutput::UserEvent(UserEvent::PipelineCreated {
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
                Ok(ActiveSessionOutput::OperationSuccess)
            }
            UserOperation::InvokePipeline { powershell } => {
                let command_request = self.runspace_pool.invoke_pipeline_request(powershell);
                match command_request {
                    Ok(request) => {
                        let response = self.http_builder.post("/wsman", request);
                        Ok(ActiveSessionOutput::SendBack(vec![response]))
                    }
                    Err(e) => Ok(ActiveSessionOutput::SendBackError(e)),
                }
            }
        }
    }

    /// Handle a server response
    pub fn accept_server_response(
        &mut self,
        response: HttpResponse<String>,
    ) -> Result<Vec<ActiveSessionOutput>, crate::PwshCoreError> {
        let body = response.body.ok_or(crate::PwshCoreError::InvalidState(
            "Expected a body in server response",
        ))?;

        let results = self.runspace_pool.accept_response(body)?;
        let mut step_output = Vec::new();
        for result in results {
            match result {
                AcceptResponsResult::ReceiveResponse { desired_streams } => {
                    let receive_request = self.runspace_pool.fire_receive(desired_streams)?;
                    let response = self.http_builder.post("/wsman", receive_request);
                    step_output.push(ActiveSessionOutput::SendBack(vec![response]));
                }
                AcceptResponsResult::NewPipeline(pipeline) => {
                    step_output.push(ActiveSessionOutput::UserEvent(UserEvent::PipelineCreated {
                        powershell: pipeline,
                    }));
                }
                AcceptResponsResult::HostCall(host_call) => {
                    step_output.push(ActiveSessionOutput::HostCall(host_call));
                }
            }
        }

        step_output.sort();
        Ok(step_output)
    }

}
