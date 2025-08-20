use crate::{
    connector::http::{HttpBuilder, HttpRequest, HttpResponse},
    host::{self, HostCallRequest, HostCallType},
    pipeline::{ParameterValue, PipelineCommand},
    powershell::{PipelineHandle, PipelineOutputType},
    runspace_pool::{RunspacePool, pool::AcceptResponsResult},
};
use protocol_powershell_remoting::{PipelineOutput, PsValue};
use tracing::{debug, error, instrument};

#[derive(Debug, PartialEq, Eq)]
pub enum UserEvent {
    PipelineCreated { powershell: PipelineHandle },
}

#[derive(Debug)]
pub enum ActiveSessionOutput {
    SendBack(Vec<HttpRequest<String>>),
    SendBackError(crate::PwshCoreError),
    UserEvent(UserEvent),
    HostCall(HostCallRequest),
    PipelineOutput {
        output: PipelineOutput,
        handle: PipelineHandle,
    },
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
            ActiveSessionOutput::PipelineOutput { .. } => 5,
            ActiveSessionOutput::OperationSuccess => 6,
        }
    }
}

#[derive(Debug)]
pub enum PowershellOperations {
    AddCommand { command: PipelineCommand },
    AddArgument(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostCallScope {
    Pipeline { command_id: uuid::Uuid },
    RunspacePool,
}

impl From<HostCallType> for HostCallScope {
    fn from(host_call_type: HostCallType) -> Self {
        match host_call_type {
            HostCallType::Pipeline { id } => HostCallScope::Pipeline { command_id: id },
            HostCallType::RunspacePool => HostCallScope::RunspacePool,
        }
    }
}

#[derive(Debug)]
pub enum UserOperation {
    CreatePipeline,
    OperatePipeline {
        powershell: PipelineHandle,
        operation: PowershellOperations,
    },
    InvokePipeline {
        powershell: PipelineHandle,
        output_type: PipelineOutputType,
    },
    /// Reply to a server-initiated host call (PipelineHostCall or RunspacePoolHostCall)
    SubmitHostResponse {
        scope: HostCallScope,
        call_id: i64,
        method_id: i32,
        method_name: String,
        result: Option<PsValue>,
        error: Option<PsValue>,
    },
    /// Allow UI to abort a pending prompt cleanly (timeout, user cancelled)
    CancelHostCall {
        scope: HostCallScope,
        call_id: i64,
        reason: Option<String>,
    },
}

/// ActiveSession manages post-connection operations
#[derive(Debug)]
pub struct ActiveSession {
    runspace_pool: RunspacePool,
    http_builder: HttpBuilder,
    /// Tracks pending host calls by (scope, call_id) to validate responses
    pending_host_calls: std::collections::HashMap<(HostCallScope, i64), ()>,
}

impl ActiveSession {
    pub fn new(runspace_pool: RunspacePool, http_builder: HttpBuilder) -> Self {
        Self {
            runspace_pool,
            http_builder,
            pending_host_calls: std::collections::HashMap::new(),
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
                    PowershellOperations::AddCommand { command } => {
                        self.runspace_pool.add_command(powershell, command)?;
                    }
                    PowershellOperations::AddArgument(arg) => {
                        self.runspace_pool.add_switch_parameter(powershell, arg)?;
                    }
                }
                Ok(ActiveSessionOutput::OperationSuccess)
            }
            UserOperation::InvokePipeline {
                powershell,
                output_type,
            } => {
                let command_request = self
                    .runspace_pool
                    .invoke_pipeline_request(powershell, output_type);
                match command_request {
                    Ok(request) => {
                        let response = self.http_builder.post("/wsman", request);
                        Ok(ActiveSessionOutput::SendBack(vec![response]))
                    }
                    Err(e) => Ok(ActiveSessionOutput::SendBackError(e)),
                }
            }
            UserOperation::SubmitHostResponse {
                scope,
                call_id,
                method_id,
                method_name,
                result,
                error,
            } => {
                // Validate that this host call is actually pending
                let key = (scope.clone(), call_id);
                if !self.pending_host_calls.contains_key(&key) {
                    return Err(crate::PwshCoreError::InvalidState(
                        "Host call not found or already completed",
                    ));
                }

                // Remove from pending calls
                self.pending_host_calls.remove(&key);

                // Create the appropriate host response message based on scope
                match scope {
                    HostCallScope::Pipeline { command_id } => self.send_pipeline_host_response(
                        command_id,
                        call_id,
                        method_id,
                        method_name,
                        result,
                        error,
                    ),
                    HostCallScope::RunspacePool => self.send_runspace_pool_host_response(
                        call_id,
                        method_id,
                        method_name,
                        result,
                        error,
                    ),
                }
            }
            UserOperation::CancelHostCall {
                scope,
                call_id,
                reason: _reason,
            } => {
                // Remove from pending calls if it exists
                let key = (scope.clone(), call_id);
                self.pending_host_calls.remove(&key);

                // For cancellation, send an error response
                let error_msg = format!("Host call {call_id} was cancelled");
                let error = Some(PsValue::Primitive(
                    protocol_powershell_remoting::PsPrimitiveValue::Str(error_msg),
                ));

                match scope {
                    HostCallScope::Pipeline { command_id } => self.send_pipeline_host_response(
                        command_id,
                        call_id,
                        0,
                        "Cancelled".to_string(),
                        None,
                        error,
                    ),
                    HostCallScope::RunspacePool => self.send_runspace_pool_host_response(
                        call_id,
                        0,
                        "Cancelled".to_string(),
                        None,
                        error,
                    ),
                }
            }
        }
    }

    /// Handle a server response
    #[instrument(skip(self, response))]
    pub fn accept_server_response(
        &mut self,
        response: HttpResponse<String>,
    ) -> Result<Vec<ActiveSessionOutput>, crate::PwshCoreError> {
        let body = response.body.ok_or(crate::PwshCoreError::InvalidState(
            "Expected a body in server response",
        ))?;

        debug!("Response body length: {}", body.len());

        let results = self.runspace_pool.accept_response(body).map_err(|e| {
            error!("RunspacePool.accept_response failed: {:#}", e);
            e
        })?;

        let mut step_output = Vec::new();
        debug!(?results, "RunspacePool accept_response results");

        for (index, result) in results.into_iter().enumerate() {
            debug!("Processing result {}: {:?}", index, result);

            match result {
                AcceptResponsResult::ReceiveResponse { desired_streams } => {
                    debug!(
                        "Creating receive request for streams: {:?}",
                        desired_streams
                    );
                    let receive_request = self
                        .runspace_pool
                        .fire_receive(desired_streams)
                        .map_err(|e| {
                            error!("Failed to create receive request: {:#}", e);
                            e
                        })?;
                    let response = self.http_builder.post("/wsman", receive_request);
                    step_output.push(ActiveSessionOutput::SendBack(vec![response]));
                }
                AcceptResponsResult::NewPipeline(pipeline) => {
                    debug!("New pipeline created: {:?}", pipeline);
                    step_output.push(ActiveSessionOutput::UserEvent(UserEvent::PipelineCreated {
                        powershell: pipeline,
                    }));
                }
                AcceptResponsResult::HostCall(host_call) => {
                    debug!(host_call = ?host_call, "Received host call request");
                    // Track this host call as pending
                    let scope: HostCallScope = host_call.call_type.clone().into();
                    let key = (scope, host_call.call_id);
                    self.pending_host_calls.insert(key, ());

                    step_output.push(ActiveSessionOutput::HostCall(host_call));
                }
                AcceptResponsResult::PipelineOutput { output, handle } => {
                    debug!("Pipeline output: {:?}", output);
                    step_output.push(ActiveSessionOutput::PipelineOutput {
                        output,
                        handle: handle.clone(),
                    });
                }
            }
        }

        step_output.sort();
        debug!("Returning {} step outputs", step_output.len());
        Ok(step_output)
    }

    /// Send a pipeline host response back to the server
    fn send_pipeline_host_response(
        &mut self,
        command_id: uuid::Uuid,
        call_id: i64,
        method_id: i32,
        method_name: String,
        result: Option<PsValue>,
        error: Option<PsValue>,
    ) -> Result<ActiveSessionOutput, crate::PwshCoreError> {
        // Only send a response if we have a result or error to report
        // Void methods (like Write, WriteLine, WriteProgress) don't need responses
        if result.is_none() && error.is_none() {
            return Ok(ActiveSessionOutput::OperationSuccess);
        }

        use protocol_powershell_remoting::PipelineHostResponse;

        let host_response = PipelineHostResponse::builder()
            .call_id(call_id)
            .method_id(method_id)
            .method_name(method_name)
            .method_result_opt(result)
            .method_exception_opt(error)
            .build();

        // Fragment and send via RunspacePool
        let request = self
            .runspace_pool
            .send_pipeline_host_response(command_id, host_response)?;
        let http_response = self.http_builder.post("/wsman", request);

        // Queue a receive after sending the response
        let receive_request = self.runspace_pool.fire_receive(
            crate::runspace_pool::DesiredStream::pipeline_streams(command_id),
        )?;
        let receive_http_response = self.http_builder.post("/wsman", receive_request);

        Ok(ActiveSessionOutput::SendBack(vec![
            http_response,
            receive_http_response,
        ]))
    }

    /// Send a runspace pool host response back to the server
    fn send_runspace_pool_host_response(
        &mut self,
        call_id: i64,
        method_id: i32,
        method_name: String,
        result: Option<PsValue>,
        error: Option<PsValue>,
    ) -> Result<ActiveSessionOutput, crate::PwshCoreError> {
        // Only send a response if we have a result or error to report
        // Void methods (like Write, WriteLine, WriteProgress) don't need responses
        if result.is_none() && error.is_none() {
            return Ok(ActiveSessionOutput::OperationSuccess);
        }

        use protocol_powershell_remoting::RunspacePoolHostResponse;

        let host_response = RunspacePoolHostResponse::builder()
            .call_id(call_id)
            .method_id(method_id)
            .method_name(method_name)
            .method_result_opt(result)
            .method_exception_opt(error)
            .build();

        // Fragment and send via RunspacePool
        let request = self
            .runspace_pool
            .send_runspace_pool_host_response(host_response)?;
        let http_response = self.http_builder.post("/wsman", request);

        // Queue a receive after sending the response
        let receive_request = self
            .runspace_pool
            .fire_receive(crate::runspace_pool::DesiredStream::runspace_pool_streams())?;
        let receive_http_response = self.http_builder.post("/wsman", receive_request);

        Ok(ActiveSessionOutput::SendBack(vec![
            http_response,
            receive_http_response,
        ]))
    }
}
