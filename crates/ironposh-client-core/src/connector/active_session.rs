use crate::{
    connector::{
        conntion_pool::{ConnectionPool, TrySend},
        http::HttpResponseTargeted,
    },
    host::{HostCall, HostCallScope, Submission},
    pipeline::PipelineCommand,
    powershell::PipelineHandle,
    runspace_pool::{DesiredStream, RunspacePool, pool::AcceptResponsResult},
};
use ironposh_psrp::{PipelineOutput, PsPrimitiveValue, PsValue};
use tracing::{error, info, instrument, warn};

#[derive(Debug, PartialEq, Eq)]
pub enum UserEvent {
    PipelineCreated {
        powershell: PipelineHandle,
    },
    PipelineFinished {
        powershell: PipelineHandle,
    },
    PipelineOutput {
        powershell: PipelineHandle,
        output: PipelineOutput,
    },
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
    HostCall(HostCall),
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

#[derive(Debug)]
pub enum PowershellOperations {
    AddCommand { command: PipelineCommand },
}

#[derive(Debug)]
pub enum UserOperation {
    CreatePipeline {
        uuid: uuid::Uuid,
    },
    OperatePipeline {
        powershell: PipelineHandle,
        operation: PowershellOperations,
    },
    InvokePipeline {
        powershell: PipelineHandle,
    },
    /// reply to a server-initiated host call
    SubmitHostResponse {
        submission: Submission,
        scope: HostCallScope,
        call_id: i64,
    },
    /// cancel a pending host call (timeout / user cancelled)
    CancelHostCall {
        scope: HostCallScope,
        call_id: i64,
        reason: Option<String>,
    },
}

/// Manages post-connect PSRP operations. Produces `TrySend` for the caller to send.
#[derive(Debug)]
pub struct ActiveSession {
    runspace_pool: RunspacePool,
    connection_pool: ConnectionPool,
}

impl ActiveSession {
    pub(crate) fn new(runspace_pool: RunspacePool, connection_pool: ConnectionPool) -> Self {
        info!("ActiveSession: created new session");
        Self {
            runspace_pool,
            connection_pool,
        }
    }

    /// Client-initiated operation → produce network work (`TrySend`) or a user-level event.
    #[instrument(skip_all, fields(operation_type = %match &operation {
        UserOperation::CreatePipeline { uuid } => format!("CreatePipeline({uuid})"),
        UserOperation::OperatePipeline { powershell, .. } => format!("OperatePipeline({})", powershell.id()),
        UserOperation::InvokePipeline { powershell } => format!("InvokePipeline({})", powershell.id()),
        UserOperation::SubmitHostResponse { .. } => "SubmitHostResponse".to_string(),
        UserOperation::CancelHostCall { .. } => "CancelHostCall".to_string(),
    }))]
    pub fn accept_client_operation(
        &mut self,
        operation: UserOperation,
    ) -> Result<ActiveSessionOutput, crate::PwshCoreError> {
        info!("ActiveSession: processing client operation");
        match operation {
            UserOperation::CreatePipeline { uuid } => {
                info!(pipeline_uuid = %uuid, "creating new pipeline");
                let handle = self.runspace_pool.init_pipeline(uuid)?;
                info!(pipeline_id = %handle.id(), "pipeline created successfully");
                Ok(ActiveSessionOutput::UserEvent(UserEvent::PipelineCreated {
                    powershell: handle,
                }))
            }

            UserOperation::OperatePipeline {
                powershell,
                operation,
            } => {
                info!(pipeline_id = %powershell.id(), "operating on pipeline");
                match operation {
                    PowershellOperations::AddCommand { command } => {
                        info!(command = ?command, "adding command to pipeline");
                        self.runspace_pool.add_command(powershell, command)?;
                        info!("command added successfully");
                    }
                }
                Ok(ActiveSessionOutput::OperationSuccess)
            }

            UserOperation::InvokePipeline { powershell } => {
                info!(pipeline_id = %powershell.id(), "invoking pipeline");

                // 1) Build the Invoke request
                let invoke_xml = self.runspace_pool.invoke_pipeline_request(powershell)?;
                info!(xml_length = invoke_xml.len(), "built invoke XML request");
                info!(unencrypted_invoke_xml = %invoke_xml, "outgoing unencrypted invoke SOAP");

                // 2) Send invoke
                let send_invoke = self.connection_pool.send(&invoke_xml)?;
                info!(invoke_request = ?send_invoke, "queued invoke request");

                info!("returning 2 TrySend requests for pipeline invoke");
                Ok(ActiveSessionOutput::SendBack(vec![send_invoke]))
            }

            UserOperation::SubmitHostResponse {
                submission, scope, ..
            } => {
                match submission {
                    Submission::Send(response) => match scope {
                        HostCallScope::Pipeline { command_id } => self.send_pipeline_host_response(
                            command_id,
                            response.call_id,
                            response.method_id,
                            response.method_name,
                            response.method_result,
                            response.method_exception,
                        ),
                        HostCallScope::RunspacePool => self.send_runspace_pool_host_response(
                            response.call_id,
                            response.method_id,
                            response.method_name,
                            response.method_result,
                            response.method_exception,
                        ),
                    },
                    Submission::NoSend => {
                        // Void method - no response needed
                        Ok(ActiveSessionOutput::OperationSuccess)
                    }
                }
            }

            UserOperation::CancelHostCall {
                scope,
                call_id,
                reason: _,
            } => {
                // send an error response back
                let err = Some(PsValue::Primitive(PsPrimitiveValue::Str(format!(
                    "Host call {call_id} was cancelled"
                ))));
                match scope {
                    HostCallScope::Pipeline { command_id } => self.send_pipeline_host_response(
                        command_id,
                        call_id,
                        0,
                        "Cancelled".to_string(),
                        None,
                        err,
                    ),
                    HostCallScope::RunspacePool => self.send_runspace_pool_host_response(
                        call_id,
                        0,
                        "Cancelled".to_string(),
                        None,
                        err,
                    ),
                }
            }
        }
    }

    /// Server response → plaintext XML via pool → PSRP accept → outputs (events / more sends)
    #[instrument(skip(self, response), fields(
        conn_id = response.connection_id().inner(),
        status_code = response.response().status_code,
        body_length = response.response().body.len(),
        has_auth = response.authenticated.is_some()
    ))]
    pub fn accept_server_response(
        &mut self,
        response: HttpResponseTargeted,
    ) -> Result<Vec<ActiveSessionOutput>, crate::PwshCoreError> {
        info!("ActiveSession: processing server response");

        // 1) Decrypt & state-transition inside the pool, get plaintext SOAP
        let xml_body = self.connection_pool.accept(response)?;

        // Log the full decrypted response for error analysis when needed
        if xml_body.contains("<s:Fault") || xml_body.contains("HTTP 5") || xml_body.len() < 500 {
            warn!(
                decrypted_xml = %xml_body,
                decrypted_xml_length = xml_body.len(),
                "decrypted server response (full content logged for debugging)"
            );
        } else {
            info!(
                decrypted_xml_length = xml_body.len(),
                "decrypted server response"
            );
        }

        // 2) Feed PSRP
        let results = self.runspace_pool.accept_response(xml_body).map_err(|e| {
            error!("RunspacePool.accept_response failed: {:#}", e);
            e
        })?;
        info!(result_count = results.len(), "PSRP processed response");

        // 3) Translate PSRP results to outputs
        let mut outs = Vec::new();
        for (idx, res_accepted) in results.into_iter().enumerate() {
            info!(index = idx, "processing PSRP result");
            match res_accepted {
                AcceptResponsResult::ReceiveResponse { desired_streams } => {
                    info!(streams = ?desired_streams, "creating receive request for streams");
                    let recv_xml = self.runspace_pool.fire_receive(desired_streams)?;
                    let ts = self.connection_pool.send(&recv_xml)?;
                    info!(try_send = ?ts, "queued receive request");
                    outs.push(ActiveSessionOutput::SendBack(vec![ts]));
                }
                AcceptResponsResult::PipelineCreated(powershell) => {
                    let recv_xml = self
                        .runspace_pool
                        .fire_receive(DesiredStream::pipeline_streams(powershell.id()))?;

                    info!(
                        recv_xml_length = recv_xml.len(),
                        "built receive XML request"
                    );
                    info!(unencrypted_recv_xml = %recv_xml, "outgoing unencrypted receive SOAP");
                    let send_recv = self.connection_pool.send(&recv_xml)?;

                    outs.push(ActiveSessionOutput::UserEvent(UserEvent::PipelineCreated {
                        powershell,
                    }));

                    outs.push(ActiveSessionOutput::SendBack(vec![send_recv]));
                }
                AcceptResponsResult::PipelineFinished(powershell) => {
                    info!(pipeline_id = %powershell.id(), "pipeline finished");
                    outs.push(ActiveSessionOutput::UserEvent(
                        UserEvent::PipelineFinished { powershell },
                    ));
                }
                AcceptResponsResult::HostCall(host_call) => {
                    info!(call_id = host_call.call_id(), method = %host_call.method_name(), "received host call");
                    outs.push(ActiveSessionOutput::HostCall(host_call));
                }
                AcceptResponsResult::PipelineOutput { output, handle } => {
                    info!(pipeline_id = %handle.id(), output_type = ?output, "pipeline output received");
                    outs.push(ActiveSessionOutput::UserEvent(UserEvent::PipelineOutput {
                        powershell: handle,
                        output,
                    }));
                }
            }
        }

        outs.sort();
        info!(output_count = outs.len(), "returning ActiveSession outputs");
        Ok(outs)
    }

    /// Build + send a pipeline host response, then queue a receive for that pipeline.
    #[instrument(skip(self, result, error), fields(command_id = %command_id, call_id, method_name = %method_name))]
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
            info!("void method, no response to send");
            return Ok(ActiveSessionOutput::OperationSuccess);
        }

        info!("building pipeline host response");
        let host_resp = PipelineHostResponse::builder()
            .call_id(call_id)
            .method_id(method_id)
            .method_name(method_name)
            .method_result_opt(result)
            .method_exception_opt(error)
            .build();

        // 1) Fragment to XML
        let send_xml = self
            .runspace_pool
            .send_pipeline_host_response(command_id, host_resp)?;
        info!(send_xml_length = send_xml.len(), "built host response XML");
        info!(unencrypted_host_response_xml = %send_xml, "outgoing unencrypted pipeline host response SOAP");

        // 2) Send
        let ts_send = self.connection_pool.send(&send_xml)?;
        info!(send_request = ?ts_send, "queued host response send");

        // 3) Queue receive for this pipeline's streams
        let recv_xml = self
            .runspace_pool
            .fire_receive(DesiredStream::pipeline_streams(command_id))?;
        info!(unencrypted_pipeline_recv_xml = %recv_xml, "outgoing unencrypted pipeline receive SOAP");
        let ts_recv = self.connection_pool.send(&recv_xml)?;
        info!(recv_request = ?ts_recv, "queued host response receive");

        Ok(ActiveSessionOutput::SendBack(vec![ts_send, ts_recv]))
    }

    /// Build + send a runspace-pool host response, then queue a receive for pool streams.
    #[instrument(skip(self, result, error), fields(call_id, method_name = %method_name))]
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
            info!("void method, no response to send");
            return Ok(ActiveSessionOutput::OperationSuccess);
        }

        info!("building runspace pool host response");
        let host_resp = RunspacePoolHostResponse::builder()
            .call_id(call_id)
            .method_id(method_id)
            .method_name(method_name)
            .method_result_opt(result)
            .method_exception_opt(error)
            .build();

        // 1) Fragment to XML
        let send_xml = self
            .runspace_pool
            .send_runspace_pool_host_response(host_resp)?;
        info!(
            send_xml_length = send_xml.len(),
            "built pool host response XML"
        );
        info!(unencrypted_pool_host_response_xml = %send_xml, "outgoing unencrypted pool host response SOAP");

        // 2) Send
        let ts_send = self.connection_pool.send(&send_xml)?;
        info!(send_request = ?ts_send, "queued pool host response send");

        // 3) Queue receive for pool streams
        let recv_xml = self
            .runspace_pool
            .fire_receive(DesiredStream::runspace_pool_streams())?;
        info!(unencrypted_pool_recv_xml = %recv_xml, "outgoing unencrypted pool receive SOAP");
        let ts_recv = self.connection_pool.send(&recv_xml)?;
        info!(recv_request = ?ts_recv, "queued pool host response receive");

        Ok(ActiveSessionOutput::SendBack(vec![ts_send, ts_recv]))
    }
}
