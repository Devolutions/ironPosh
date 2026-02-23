use crate::{
    connector::{
        conntion_pool::{ConnectionPool, ConnectionPoolAccept, TrySend},
        http::HttpResponseTargeted,
    },
    host::{HostCall, HostCallScope, Submission},
    pipeline::PipelineSpec,
    powershell::PipelineHandle,
    runspace_pool::{DesiredStream, RunspacePool, pool::AcceptResponsResult},
    PwshCoreError,
};
use ironposh_psrp::{ErrorRecord, PipelineOutput, PsPrimitiveValue, PsValue};
use tracing::{error, info, instrument, warn};

#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq, Eq)]
pub enum UserEvent {
    PipelineCreated {
        pipeline: PipelineHandle,
    },
    PipelineFinished {
        pipeline: PipelineHandle,
    },
    PipelineOutput {
        pipeline: PipelineHandle,
        output: PipelineOutput,
    },
    ErrorRecord {
        error_record: ErrorRecord,
        handle: PipelineHandle,
    },
    PipelineRecord {
        pipeline: PipelineHandle,
        record: crate::psrp_record::PsrpRecord,
    },
}

impl UserEvent {
    pub fn pipeline_id(&self) -> uuid::Uuid {
        match self {
            Self::PipelineCreated {
                pipeline: powershell,
            }
            | Self::PipelineFinished {
                pipeline: powershell,
            }
            | Self::PipelineOutput {
                pipeline: powershell,
                ..
            } => powershell.id(),
            Self::ErrorRecord { handle, .. } => handle.id(),
            Self::PipelineRecord { pipeline, .. } => pipeline.id(),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum ActiveSessionOutput {
    SendBack(Vec<TrySend>),
    SendBackError(crate::PwshCoreError),
    UserEvent(UserEvent),
    HostCall(HostCall),
    /// Sequential: send the request first, wait for response,
    /// THEN issue a Receive for the given streams.
    /// Used when send+receive must be serialized (single-connection mode).
    SendAndThenReceive {
        send_request: TrySend,
        then_receive_streams: Vec<DesiredStream>,
    },
    /// Indicates a Receive is needed for these streams, but does NOT allocate
    /// a connection. The session loop calls `fire_receive()` when ready to send.
    PendingReceive {
        desired_streams: Vec<DesiredStream>,
    },
    OperationSuccess,
    Ignore,
}

impl ActiveSessionOutput {
    pub fn priority(&self) -> u8 {
        match self {
            Self::HostCall { .. } => 1,
            Self::SendBack(_) | Self::SendAndThenReceive { .. } | Self::PendingReceive { .. } => 2,
            Self::SendBackError(_) => 3,
            Self::UserEvent(_) => 4,
            Self::OperationSuccess => 5,
            Self::Ignore => 6,
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

#[expect(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum UserOperation {
    InvokeWithSpec {
        uuid: uuid::Uuid,
        spec: PipelineSpec,
    },
    KillPipeline {
        pipeline: PipelineHandle,
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

impl UserOperation {
    pub fn operation_type(&self) -> &str {
        match self {
            Self::InvokeWithSpec { .. } => "InvokeWithSpec",
            Self::KillPipeline { .. } => "KillPipeline",
            Self::SubmitHostResponse { .. } => "SubmitHostResponse",
            Self::CancelHostCall { .. } => "CancelHostCall",
        }
    }
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

    /// Generate a Receive TrySend for the given streams.
    /// Used by the serial session loop to issue Receives after processing sends.
    pub fn fire_receive(
        &mut self,
        desired_streams: Vec<DesiredStream>,
    ) -> Result<TrySend, PwshCoreError> {
        let recv_xml = self.runspace_pool.fire_receive(desired_streams)?;
        self.connection_pool.send(&recv_xml)
    }

    /// Client-initiated operation → produce network work (`TrySend`) or a user-level event.
    #[instrument(skip_all, fields(operation_type = operation.operation_type()))]
    pub fn accept_client_operation(
        &mut self,
        operation: UserOperation,
    ) -> Result<ActiveSessionOutput, crate::PwshCoreError> {
        info!("ActiveSession: processing client operation");
        match operation {
            UserOperation::InvokeWithSpec { uuid, spec } => {
                info!(pipeline_uuid = %uuid, "invoking pipeline with spec");

                // Single operation: create, populate, and invoke pipeline
                let invoke_xml = self.runspace_pool.invoke_spec(uuid, spec)?;
                info!(xml_length = invoke_xml.len(), "built invoke XML request");
                info!(unencrypted_invoke_xml = %invoke_xml, "outgoing unencrypted invoke SOAP");

                // Send the invoke request
                let send_invoke = self.connection_pool.send(&invoke_xml)?;
                info!(invoke_request = ?send_invoke, "queued invoke request");

                Ok(ActiveSessionOutput::SendBack(vec![send_invoke]))
            }

            UserOperation::KillPipeline { pipeline } => {
                info!(pipeline_id = %pipeline.id(), "killing pipeline");

                // 1) Build the Signal request
                let kill_xml = self.runspace_pool.kill_pipeline(&pipeline);
                let kill_xml = match kill_xml {
                    Ok(kill_xml) => kill_xml,
                    Err(e) => {
                        error!(error = ?e, "failed to build kill XML");
                        return Ok(ActiveSessionOutput::Ignore);
                    }
                };

                info!(xml_length = kill_xml.len(), "built kill XML request");

                // 2) Send signal
                let ts_send = self.connection_pool.send(&kill_xml)?;
                info!(signal_request = ?ts_send, "queued signal request");

                Ok(ActiveSessionOutput::SendBack(vec![ts_send]))
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
        let xml_body = match self.connection_pool.accept(response)? {
            ConnectionPoolAccept::Body(xml_body) => xml_body,
            ConnectionPoolAccept::SendBack(reqs) => {
                return Ok(vec![ActiveSessionOutput::SendBack(reqs)]);
            }
        };

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
        let results = self.runspace_pool.accept_response(&xml_body).map_err(|e| {
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
                    info!(streams = ?desired_streams, "deferring receive to session loop");
                    outs.push(ActiveSessionOutput::PendingReceive { desired_streams });
                }
                AcceptResponsResult::SendThenReceive {
                    send_xml,
                    desired_streams,
                } => {
                    info!(
                        send_xml_length = send_xml.len(),
                        "queued send-then-receive (key exchange / control)"
                    );
                    let ts_send = self.connection_pool.send(&send_xml)?;

                    outs.push(ActiveSessionOutput::SendAndThenReceive {
                        send_request: ts_send,
                        then_receive_streams: desired_streams,
                    });
                }
                AcceptResponsResult::PipelineCreated(pipeline) => {
                    outs.push(ActiveSessionOutput::UserEvent(UserEvent::PipelineCreated {
                        pipeline,
                    }));
                }
                AcceptResponsResult::PipelineFinished(pipeline) => {
                    info!(pipeline_id= %pipeline.id(),"pipeline finished");
                    outs.push(ActiveSessionOutput::UserEvent(
                        UserEvent::PipelineFinished { pipeline },
                    ));
                }
                AcceptResponsResult::HostCall(host_call) => {
                    info!(call_id=host_call.call_id(),method= %host_call.method_name(),"received host call");
                    outs.push(ActiveSessionOutput::HostCall(host_call));
                }
                AcceptResponsResult::PipelineOutput { output, handle } => {
                    info!(pipeline_id= %handle.id(),output_type= ?output,"pipeline output received");
                    outs.push(ActiveSessionOutput::UserEvent(UserEvent::PipelineOutput {
                        pipeline: handle,
                        output,
                    }));
                }
                AcceptResponsResult::ErrorRecord {
                    error_record,
                    handle,
                } => {
                    info!(pipeline_id= %handle.id(),error_record = ?error_record, "ErrorRecord received");
                    outs.push(ActiveSessionOutput::UserEvent(UserEvent::ErrorRecord {
                        error_record,
                        handle,
                    }));
                }
                AcceptResponsResult::PipelineRecord { record, handle } => {
                    outs.push(ActiveSessionOutput::UserEvent(UserEvent::PipelineRecord {
                        pipeline: handle,
                        record,
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

        let mut result = result;
        let mut error = error;
        if let Some(v) = result.as_mut() {
            self.runspace_pool.encrypt_secure_strings_in_value(v)?;
        }
        if let Some(v) = error.as_mut() {
            self.runspace_pool.encrypt_secure_strings_in_value(v)?;
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
            .send_pipeline_host_response(command_id, &host_resp)?;
        info!(send_xml_length = send_xml.len(), "built host response XML");
        info!(unencrypted_host_response_xml = %send_xml, "outgoing unencrypted pipeline host response SOAP");

        // 2) Send, then receive for this pipeline's streams
        let ts_send = self.connection_pool.send(&send_xml)?;
        info!(send_request = ?ts_send, "queued host response send-then-receive");

        Ok(ActiveSessionOutput::SendAndThenReceive {
            send_request: ts_send,
            then_receive_streams: DesiredStream::pipeline_streams(command_id),
        })
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

        let mut result = result;
        let mut error = error;
        if let Some(v) = result.as_mut() {
            self.runspace_pool.encrypt_secure_strings_in_value(v)?;
        }
        if let Some(v) = error.as_mut() {
            self.runspace_pool.encrypt_secure_strings_in_value(v)?;
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
            .send_runspace_pool_host_response(&host_resp)?;
        info!(
            send_xml_length = send_xml.len(),
            "built pool host response XML"
        );
        info!(unencrypted_pool_host_response_xml = %send_xml, "outgoing unencrypted pool host response SOAP");

        // 2) Send, then receive for pool streams
        let ts_send = self.connection_pool.send(&send_xml)?;
        info!(send_request = ?ts_send, "queued pool host response send-then-receive");

        Ok(ActiveSessionOutput::SendAndThenReceive {
            send_request: ts_send,
            then_receive_streams: DesiredStream::runspace_pool_streams(),
        })
    }
}
