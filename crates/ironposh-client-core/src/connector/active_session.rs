use crate::{
    connector::{
        conntion_pool::{ConnectionPool, TrySend},
        http::HttpResponseTargeted,
    },
    host::{HostCallRequest, HostCallResponse, HostCallScope},
    pipeline::PipelineCommand,
    powershell::PipelineHandle,
    runspace_pool::{RunspacePool, pool::AcceptResponsResult, DesiredStream},
};
use ironposh_psrp::{PipelineOutput, PsValue, PsPrimitiveValue};
use tracing::{debug, error, info, instrument};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub enum UserEvent {
    PipelineCreated { powershell: PipelineHandle },
    PipelineFinished { powershell: PipelineHandle },
    PipelineOutput { powershell: PipelineHandle, output: PipelineOutput },
}

impl UserEvent {
    pub fn pipeline_id(&self) -> uuid::Uuid {
        match self {
            UserEvent::PipelineCreated { powershell }
            | UserEvent::PipelineFinished { powershell }
            | UserEvent::PipelineOutput { powershell, .. } => powershell.id(),
        }
    }
}

#[derive(Debug)]
pub enum ActiveSessionOutput {
    SendBack(Vec<TrySend>),
    SendBackError(crate::PwshCoreError),
    UserEvent(UserEvent),
    HostCall(HostCallRequest),
    OperationSuccess,
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
impl PartialEq for ActiveSessionOutput {
    fn eq(&self, other: &Self) -> bool { self.priority() == other.priority() }
}
impl Eq for ActiveSessionOutput {}
impl PartialOrd for ActiveSessionOutput {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}
impl Ord for ActiveSessionOutput {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.priority().cmp(&other.priority()) }
}

#[derive(Debug)]
pub enum PowershellOperations {
    AddCommand { command: PipelineCommand },
}

#[derive(Debug)]
pub enum UserOperation {
    CreatePipeline { uuid: uuid::Uuid },
    OperatePipeline { powershell: PipelineHandle, operation: PowershellOperations },
    InvokePipeline { powershell: PipelineHandle },
    /// reply to a server-initiated host call
    SubmitHostResponse { response: Box<HostCallResponse> },
    /// cancel a pending host call (timeout / user cancelled)
    CancelHostCall { scope: HostCallScope, call_id: i64, reason: Option<String> },
}

/// Manages post-connect PSRP operations. Produces `TrySend` for the caller to send.
#[derive(Debug)]
pub struct ActiveSession {
    runspace_pool: RunspacePool,
    connection_pool: ConnectionPool,
    /// Track pending host calls to validate responses
    pending_host_calls: HashMap<(HostCallScope, i64), ()>,
}

impl ActiveSession {
    pub(crate) fn new(runspace_pool: RunspacePool, connection_pool: ConnectionPool) -> Self {
        Self {
            runspace_pool,
            connection_pool,
            pending_host_calls: HashMap::new(),
        }
    }

    /// Client-initiated operation → produce network work (`TrySend`) or a user-level event.
    #[instrument(skip_all)]
    pub fn accept_client_operation(
        &mut self,
        operation: UserOperation,
    ) -> Result<ActiveSessionOutput, crate::PwshCoreError> {
        info!(?operation, "ActiveSession: client operation");
        match operation {
            UserOperation::CreatePipeline { uuid } => {
                let handle = self.runspace_pool.init_pipeline(uuid)?;
                Ok(ActiveSessionOutput::UserEvent(UserEvent::PipelineCreated { powershell: handle }))
            }

            UserOperation::OperatePipeline { powershell, operation } => {
                match operation {
                    PowershellOperations::AddCommand { command } => {
                        self.runspace_pool.add_command(powershell, command)?;
                    }
                }
                Ok(ActiveSessionOutput::OperationSuccess)
            }

            UserOperation::InvokePipeline { powershell } => {
                // 1) Build the Invoke request
                let invoke_xml = self.runspace_pool.invoke_pipeline_request(powershell)?;

                // 2) Send invoke
                let send_invoke = self.connection_pool.send(&invoke_xml)?;

                // 3) Queue a receive for the pipeline streams
                let recv_xml = self.runspace_pool
                    .fire_receive(DesiredStream::pipeline_streams(powershell.id()))?;
                let send_recv = self.connection_pool.send(&recv_xml)?;

                Ok(ActiveSessionOutput::SendBack(vec![send_invoke, send_recv]))
            }

            UserOperation::SubmitHostResponse { response } => {
                let HostCallResponse {
                    call_scope,
                    call_id,
                    method_id,
                    method_name,
                    method_result: result,
                    method_exception: error,
                } = *response;

                // Validate pending
                let key = (call_scope.clone(), call_id);
                if !self.pending_host_calls.contains_key(&key) {
                    return Err(crate::PwshCoreError::InvalidState(
                        "Host call not found or already completed",
                    ));
                }
                self.pending_host_calls.remove(&key);

                match call_scope {
                    HostCallScope::Pipeline { command_id } => self.send_pipeline_host_response(
                        command_id, call_id, method_id, method_name, result, error,
                    ),
                    HostCallScope::RunspacePool => self.send_runspace_pool_host_response(
                        call_id, method_id, method_name, result, error,
                    ),
                }
            }

            UserOperation::CancelHostCall { scope, call_id, reason: _ } => {
                let key = (scope.clone(), call_id);
                self.pending_host_calls.remove(&key);

                // send an error response back
                let err = Some(PsValue::Primitive(PsPrimitiveValue::Str(
                    format!("Host call {call_id} was cancelled"),
                )));
                match scope {
                    HostCallScope::Pipeline { command_id } => self.send_pipeline_host_response(
                        command_id, call_id, 0, "Cancelled".to_string(), None, err,
                    ),
                    HostCallScope::RunspacePool => self.send_runspace_pool_host_response(
                        call_id, 0, "Cancelled".to_string(), None, err,
                    ),
                }
            }
        }
    }

    /// Server response → plaintext XML via pool → PSRP accept → outputs (events / more sends)
    #[instrument(skip(self, response))]
    pub fn accept_server_response(
        &mut self,
        response: HttpResponseTargeted,
    ) -> Result<Vec<ActiveSessionOutput>, crate::PwshCoreError> {
        // 1) Decrypt & state-transition inside the pool, get plaintext SOAP
        let xml_body = self.connection_pool.accept(response)?;

        // 2) Feed PSRP
        let results = self.runspace_pool.accept_response(xml_body).map_err(|e| {
            error!("RunspacePool.accept_response failed: {:#}", e);
            e
        })?;

        // 3) Translate PSRP results to outputs
        let mut outs = Vec::new();
        for (idx, r) in results.into_iter().enumerate() {
            debug!("processing result {idx}: {r:?}");
            match r {
                AcceptResponsResult::ReceiveResponse { desired_streams } => {
                    let recv_xml = self.runspace_pool.fire_receive(desired_streams)?;
                    let ts = self.connection_pool.send(&recv_xml)?;
                    outs.push(ActiveSessionOutput::SendBack(vec![ts]));
                }
                AcceptResponsResult::PipelineCreated(powershell) => {
                    outs.push(ActiveSessionOutput::UserEvent(UserEvent::PipelineCreated { powershell }));
                }
                AcceptResponsResult::PipelineFinished(powershell) => {
                    outs.push(ActiveSessionOutput::UserEvent(UserEvent::PipelineFinished { powershell }));
                }
                AcceptResponsResult::HostCall(host_call) => {
                    // mark pending so only legitimate replies are accepted
                    let scope = host_call.call_type.clone();
                    let key = (scope, host_call.call_id);
                    self.pending_host_calls.insert(key, ());
                    outs.push(ActiveSessionOutput::HostCall(host_call));
                }
                AcceptResponsResult::PipelineOutput { output, handle } => {
                    outs.push(ActiveSessionOutput::UserEvent(UserEvent::PipelineOutput {
                        powershell: handle,
                        output,
                    }));
                }
            }
        }

        outs.sort();
        Ok(outs)
    }

    /// Build + send a pipeline host response, then queue a receive for that pipeline.
    fn send_pipeline_host_response(
        &mut self,
        command_id: uuid::Uuid,
        call_id: i64,
        method_id: i32,
        method_name: String,
        result: Option<PsValue>,
        error: Option<PsValue>,
    ) -> Result<ActiveSessionOutput, crate::PwshCoreError> {
        use ironposh_psrp::PipelineHostResponse;

        // void methods: nothing to send
        if result.is_none() && error.is_none() {
            return Ok(ActiveSessionOutput::OperationSuccess);
        }

        let host_resp = PipelineHostResponse::builder()
            .call_id(call_id)
            .method_id(method_id)
            .method_name(method_name)
            .method_result_opt(result)
            .method_exception_opt(error)
            .build();

        // 1) Fragment to XML
        let send_xml = self.runspace_pool.send_pipeline_host_response(command_id, host_resp)?;

        // 2) Send
        let ts_send = self.connection_pool.send(&send_xml)?;

        // 3) Queue receive for this pipeline's streams
        let recv_xml = self.runspace_pool
            .fire_receive(DesiredStream::pipeline_streams(command_id))?;
        let ts_recv = self.connection_pool.send(&recv_xml)?;

        Ok(ActiveSessionOutput::SendBack(vec![ts_send, ts_recv]))
    }

    /// Build + send a runspace-pool host response, then queue a receive for pool streams.
    fn send_runspace_pool_host_response(
        &mut self,
        call_id: i64,
        method_id: i32,
        method_name: String,
        result: Option<PsValue>,
        error: Option<PsValue>,
    ) -> Result<ActiveSessionOutput, crate::PwshCoreError> {
        use ironposh_psrp::RunspacePoolHostResponse;

        // void methods: nothing to send
        if result.is_none() && error.is_none() {
            return Ok(ActiveSessionOutput::OperationSuccess);
        }

        let host_resp = RunspacePoolHostResponse::builder()
            .call_id(call_id)
            .method_id(method_id)
            .method_name(method_name)
            .method_result_opt(result)
            .method_exception_opt(error)
            .build();

        // 1) Fragment to XML
        let send_xml = self.runspace_pool.send_runspace_pool_host_response(host_resp)?;

        // 2) Send
        let ts_send = self.connection_pool.send(&send_xml)?;

        // 3) Queue receive for pool streams
        let recv_xml = self.runspace_pool
            .fire_receive(DesiredStream::runspace_pool_streams())?;
        let ts_recv = self.connection_pool.send(&recv_xml)?;

        Ok(ActiveSessionOutput::SendBack(vec![ts_send, ts_recv]))
    }
}
