use crate::{
    PwshCoreError,
    connector::{
        connection_pool::{ConnectionId, ConnectionPool, ConnectionPoolAccept, TrySend},
        http::HttpResponseTargeted,
    },
    host::{HostCall, HostCallScope, Submission},
    pipeline::PipelineSpec,
    powershell::PipelineHandle,
    runspace_pool::{DesiredStream, RunspacePool, pool::AcceptResponsResult},
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
    /// disconnect the runspace pool shell (MS-WSMV Disconnect)
    Disconnect,
    /// reconnect a previously disconnected runspace pool shell (MS-WSMV Reconnect)
    Reconnect,
}

impl UserOperation {
    pub fn operation_type(&self) -> &str {
        match self {
            Self::InvokeWithSpec { .. } => "InvokeWithSpec",
            Self::KillPipeline { .. } => "KillPipeline",
            Self::SubmitHostResponse { .. } => "SubmitHostResponse",
            Self::CancelHostCall { .. } => "CancelHostCall",
            Self::Disconnect => "Disconnect",
            Self::Reconnect => "Reconnect",
        }
    }
}

/// Outcome of a transport-level failure on an in-flight connection,
/// correlated against the disconnect/reconnect bookkeeping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportErrorDisposition {
    /// Unexpected failure; the session cannot continue.
    Fatal,
    /// Failure on a dying/stale connection during disconnect; ignore it.
    Tolerated,
    /// The Disconnect request itself failed; the pool reverted to Opened.
    DisconnectAborted,
    /// The Reconnect request itself failed; the pool reverted to Disconnected.
    ReconnectAborted,
}

/// Manages post-connect PSRP operations. Produces `TrySend` for the caller to send.
#[derive(Debug)]
pub struct ActiveSession {
    runspace_pool: RunspacePool,
    connection_pool: ConnectionPool,
    /// Connection carrying an in-flight Disconnect request, so a fault answering
    /// it can be told apart from the dying in-flight Receive.
    disconnect_conn_id: Option<ConnectionId>,
    /// Connection carrying an in-flight Reconnect request, so late responses from
    /// pre-disconnect traffic are not mistaken for the ReconnectResponse.
    reconnect_conn_id: Option<ConnectionId>,
    /// Connections that were in flight when a Disconnect was issued (e.g. the dying
    /// long-poll Receive). Their one straggler completion/error is ignored regardless of
    /// the current pool state — including after a reconnect returns the pool to Opened —
    /// so a late stale response cannot kill the session.
    retired_conn_ids: std::collections::HashSet<ConnectionId>,
}

impl ActiveSession {
    pub(crate) fn new(runspace_pool: RunspacePool, connection_pool: ConnectionPool) -> Self {
        info!("ActiveSession: created new session");
        Self {
            runspace_pool,
            connection_pool,
            disconnect_conn_id: None,
            reconnect_conn_id: None,
            retired_conn_ids: std::collections::HashSet::new(),
        }
    }

    /// Mark connections as retired: their next completion or transport error is a doomed
    /// straggler (e.g. the long-poll Receive that was in flight when a Disconnect was
    /// issued) and must be ignored in any state. Called by the session loop with the
    /// connections in flight at disconnect time.
    pub fn retire_connections(&mut self, conns: impl IntoIterator<Item = ConnectionId>) {
        self.retired_conn_ids.extend(conns);
    }

    /// Current runspace pool state (used by session loops to observe
    /// disconnect/reconnect transitions).
    pub fn runspace_pool_state(&self) -> crate::runspace_pool::RunspacePoolState {
        self.runspace_pool.state
    }

    /// Server-assigned shell id of the runspace pool, if the shell was created.
    pub fn shell_id(&self) -> Option<String> {
        self.runspace_pool.shell_id().map(ToOwned::to_owned)
    }

    /// Server-supplied ApplicationPrivateData, if delivered during open/connect.
    pub fn application_private_data(&self) -> Option<&ironposh_psrp::ApplicationPrivateData> {
        self.runspace_pool.application_private_data()
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

    /// Fire a Receive covering the currently-active streams (the pool stream when no
    /// pipelines are running). Used to resume polling after a failed Disconnect reverts
    /// the pool to Opened, since the pre-disconnect Receive was retired.
    pub fn fire_active_receive(&mut self) -> Result<TrySend, PwshCoreError> {
        let desired = self.runspace_pool.compute_active_desired_streams();
        self.fire_receive(desired)
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
                // A pipeline can only run against an Opened pool. While the pool is
                // disconnected or in a disconnect/reconnect transition, invoking would
                // enqueue a command against an unusable shell whose response the routing
                // then drops. Reject it without sending anything, but emit a terminal
                // PipelineFinished for this id so the caller's result stream closes
                // instead of hanging forever (the consumer registered it on send).
                if self.runspace_pool.state != crate::runspace_pool::RunspacePoolState::Opened {
                    warn!(
                        pipeline_uuid = %uuid,
                        state = ?self.runspace_pool.state,
                        "rejecting pipeline invocation while the runspace pool is not Opened"
                    );
                    return Ok(ActiveSessionOutput::UserEvent(
                        UserEvent::PipelineFinished {
                            pipeline: PipelineHandle::new(uuid),
                        },
                    ));
                }
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

            UserOperation::Disconnect => {
                info!("disconnecting runspace pool");
                let disconnect_xml = match self.runspace_pool.fire_disconnect() {
                    Ok(xml) => xml,
                    Err(e @ PwshCoreError::InvalidState(_)) => {
                        // Mistimed operation (e.g. already disconnecting) — non-fatal.
                        warn!(error = %e, "ignoring mistimed Disconnect operation");
                        return Ok(ActiveSessionOutput::Ignore);
                    }
                    Err(e) => return Err(e),
                };
                let ts_send = self.connection_pool.send(&disconnect_xml)?;
                self.disconnect_conn_id = Some(ts_send.get_connection_id());
                Ok(ActiveSessionOutput::SendBack(vec![ts_send]))
            }

            UserOperation::Reconnect => {
                info!("reconnecting runspace pool");
                let reconnect_xml = match self.runspace_pool.fire_reconnect() {
                    Ok(xml) => xml,
                    Err(e @ PwshCoreError::InvalidState(_)) => {
                        // Mistimed operation (e.g. still connected) — non-fatal.
                        warn!(error = %e, "ignoring mistimed Reconnect operation");
                        return Ok(ActiveSessionOutput::Ignore);
                    }
                    Err(e) => return Err(e),
                };
                let ts_send = self.connection_pool.send(&reconnect_xml)?;
                self.reconnect_conn_id = Some(ts_send.get_connection_id());
                Ok(ActiveSessionOutput::SendBack(vec![ts_send]))
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

        let conn_id = response.connection_id();

        // 0) Drop the one doomed straggler from a connection retired at disconnect time
        //    (e.g. the long-poll Receive that was in flight). This must run in ALL states,
        //    including after a reconnect returns the pool to Opened, so a late stale fault
        //    cannot reach the normal PSRP path and kill the session.
        if self.retired_conn_ids.remove(&conn_id) {
            warn!(
                conn_id = conn_id.inner(),
                "ignoring straggler response from a connection retired at disconnect"
            );
            return Ok(vec![ActiveSessionOutput::Ignore]);
        }

        // 1) Decrypt & state-transition inside the pool, get plaintext SOAP
        let xml_body = match self.connection_pool.accept(response)? {
            ConnectionPoolAccept::Body(xml_body) => xml_body,
            ConnectionPoolAccept::SendBack(reqs) => {
                use crate::runspace_pool::RunspacePoolState;
                // A reauth retry (e.g. 401) moves the operation to a fresh connection.
                // During a disconnect/reconnect, follow the tracked conn id to the retry's
                // connection so the eventual response is still recognized; drop retries for
                // non-tracked transitional traffic (the dying in-flight Receive), which the
                // fail-closed routing would otherwise ignore anyway.
                match self.runspace_pool.state {
                    RunspacePoolState::Disconnecting
                        if self.disconnect_conn_id == Some(conn_id) =>
                    {
                        if let Some(retry) = reqs.first() {
                            self.disconnect_conn_id = Some(retry.get_connection_id());
                        }
                    }
                    RunspacePoolState::Connecting if self.reconnect_conn_id == Some(conn_id) => {
                        if let Some(retry) = reqs.first() {
                            self.reconnect_conn_id = Some(retry.get_connection_id());
                        }
                    }
                    RunspacePoolState::Disconnecting
                    | RunspacePoolState::Disconnected
                    | RunspacePoolState::Connecting => {
                        warn!(
                            conn_id = conn_id.inner(),
                            "dropping reauth retry for non-tracked traffic during disconnect/reconnect"
                        );
                        return Ok(vec![ActiveSessionOutput::Ignore]);
                    }
                    _ => {}
                }
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

        // 2) While a disconnect/reconnect is in progress, responses are routed to
        //    the dedicated pool accept methods instead of the PSRP receive path.
        match self.runspace_pool.state {
            crate::runspace_pool::RunspacePoolState::Disconnecting => {
                return self.accept_response_while_disconnecting(&xml_body, conn_id);
            }
            crate::runspace_pool::RunspacePoolState::Disconnected => {
                // Late traffic from connections that were in flight when the shell
                // was disconnected (e.g. the long-poll Receive). Drop it.
                warn!(
                    body_length = xml_body.len(),
                    "dropping server traffic while runspace pool is disconnected"
                );
                return Ok(vec![ActiveSessionOutput::Ignore]);
            }
            crate::runspace_pool::RunspacePoolState::Connecting => {
                return self.accept_response_while_connecting(&xml_body, conn_id);
            }
            _ => {}
        }

        // 3) Feed PSRP
        let results = self.runspace_pool.accept_response(&xml_body).map_err(|e| {
            error!("RunspacePool.accept_response failed: {:#}", e);
            e
        })?;

        info!(result_count = results.len(), "PSRP processed response");

        // 4) Translate PSRP results to outputs
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

    /// Classify a transport-level error (e.g. TCP reset) on an in-flight connection.
    ///
    /// While a Disconnect is in flight the dying long-poll Receive (or other
    /// stale connections) may fail at the transport level instead of answering
    /// with a SOAP fault; those failures are tolerated. A failure on the
    /// connection carrying the Disconnect/Reconnect itself aborts that
    /// operation so the pool does not stay stuck in a transitional state.
    pub fn handle_transport_error(&mut self, conn_id: ConnectionId) -> TransportErrorDisposition {
        use crate::runspace_pool::RunspacePoolState;

        // A doomed straggler from a connection retired at disconnect time (e.g. the dying
        // long-poll Receive) is tolerated in any state, including after a reconnect has
        // returned the pool to Opened.
        if self.retired_conn_ids.remove(&conn_id) {
            warn!(
                conn_id = conn_id.inner(),
                "tolerating straggler transport error from a connection retired at disconnect"
            );
            return TransportErrorDisposition::Tolerated;
        }

        match self.runspace_pool.state {
            RunspacePoolState::Disconnecting if self.disconnect_conn_id == Some(conn_id) => {
                self.disconnect_conn_id = None;
                self.runspace_pool.abort_disconnect();
                error!(
                    conn_id = conn_id.inner(),
                    "transport error on the Disconnect connection; reverting runspace pool to Opened"
                );
                TransportErrorDisposition::DisconnectAborted
            }
            state @ (RunspacePoolState::Disconnecting | RunspacePoolState::Disconnected) => {
                warn!(
                    conn_id = conn_id.inner(),
                    ?state,
                    "tolerating transport error on dying connection during disconnect"
                );
                TransportErrorDisposition::Tolerated
            }
            RunspacePoolState::Connecting if self.reconnect_conn_id == Some(conn_id) => {
                self.reconnect_conn_id = None;
                self.runspace_pool.abort_reconnect();
                error!(
                    conn_id = conn_id.inner(),
                    "transport error on the Reconnect connection; reverting runspace pool to Disconnected"
                );
                TransportErrorDisposition::ReconnectAborted
            }
            state @ RunspacePoolState::Connecting => {
                // A non-reconnect connection failing while reconnecting is the dying
                // pre-disconnect Receive (the response path ignores its traffic too);
                // tolerate it and keep waiting for the tracked ReconnectResponse.
                warn!(
                    conn_id = conn_id.inner(),
                    ?state,
                    "tolerating transport error on dying connection during reconnect"
                );
                TransportErrorDisposition::Tolerated
            }
            state => {
                error!(
                    conn_id = conn_id.inner(),
                    ?state,
                    "fatal transport error on connection"
                );
                TransportErrorDisposition::Fatal
            }
        }
    }

    /// Handle a server response that arrives while a Disconnect is in flight.
    ///
    /// Besides the DisconnectResponse itself, the long-poll Receive that was in
    /// flight when the Disconnect was issued typically completes with a fault or
    /// unrelated body once the server tears the shell down — tolerate that
    /// traffic instead of failing the session. A fault arriving on the connection
    /// that carries the Disconnect itself means the Disconnect failed: abort it
    /// so the pool does not stay stuck in `Disconnecting` forever.
    fn accept_response_while_disconnecting(
        &mut self,
        xml_body: &str,
        conn_id: ConnectionId,
    ) -> Result<Vec<ActiveSessionOutput>, crate::PwshCoreError> {
        // Fail closed: only the tracked disconnect connection may complete or abort the
        // disconnect. Traffic on any other connection (the dying in-flight Receive) is
        // tolerated and ignored — it must never be able to flip the pool to Disconnected
        // (premature completion) nor abort it.
        if self.disconnect_conn_id != Some(conn_id) {
            warn!(
                conn_id = conn_id.inner(),
                disconnect_conn_id = self.disconnect_conn_id.map(|id| id.inner()),
                body_length = xml_body.len(),
                "ignoring non-disconnect traffic while disconnecting (expected in-flight Receive teardown)"
            );
            return Ok(vec![ActiveSessionOutput::Ignore]);
        }

        match self.runspace_pool.accept_disconnect_response(xml_body) {
            Ok(()) => {
                self.disconnect_conn_id = None;
                Ok(vec![ActiveSessionOutput::OperationSuccess])
            }
            Err(PwshCoreError::InvalidResponse(reason)) => {
                // The Disconnect request itself received the wrong shape of response:
                // revert to Opened so the session loop surfaces the failed disconnect
                // instead of remaining stuck in Disconnecting.
                self.disconnect_conn_id = None;
                self.runspace_pool.abort_disconnect();
                error!(
                    reason = %reason,
                    body = %xml_body,
                    conn_id = conn_id.inner(),
                    "Disconnect request returned an invalid response; reverting runspace pool to Opened"
                );
                // Pool is Opened again but the pre-disconnect Receive was retired; re-arm.
                Ok(vec![ActiveSessionOutput::PendingReceive {
                    desired_streams: self.runspace_pool.compute_active_desired_streams(),
                }])
            }
            Err(PwshCoreError::SoapFault { code, reason }) => {
                // The Disconnect request faulted: revert the pool to Opened. The session
                // loop observes the Disconnecting → Opened transition and surfaces the
                // failure to the user.
                self.disconnect_conn_id = None;
                self.runspace_pool.abort_disconnect();
                error!(
                    %code,
                    %reason,
                    conn_id = conn_id.inner(),
                    "Disconnect request faulted; reverting runspace pool to Opened"
                );
                // Pool is Opened again but the pre-disconnect Receive was retired; re-arm.
                Ok(vec![ActiveSessionOutput::PendingReceive {
                    desired_streams: self.runspace_pool.compute_active_desired_streams(),
                }])
            }
            Err(e) => Err(e),
        }
    }

    /// Handle a server response that arrives while a Reconnect is in flight.
    ///
    /// Late traffic from the pre-disconnect Receive can race ahead of the
    /// ReconnectResponse. Only the tracked reconnect connection is allowed to
    /// complete the reconnect; everything else is discarded like disconnected
    /// state traffic.
    fn accept_response_while_connecting(
        &mut self,
        xml_body: &str,
        conn_id: ConnectionId,
    ) -> Result<Vec<ActiveSessionOutput>, crate::PwshCoreError> {
        // Fail closed: only the tracked reconnect connection may complete the reconnect.
        // Anything else — including the case where no reconnect connection is tracked — is
        // late pre-disconnect traffic and must never be allowed to flip the pool to Opened
        // against a dead shell.
        if self.reconnect_conn_id != Some(conn_id) {
            warn!(
                conn_id = conn_id.inner(),
                reconnect_conn_id = self.reconnect_conn_id.map(|id| id.inner()),
                body_length = xml_body.len(),
                "dropping non-reconnect traffic while runspace pool is reconnecting"
            );
            return Ok(vec![ActiveSessionOutput::Ignore]);
        }

        match self.runspace_pool.accept_reconnect_response(xml_body) {
            Ok(()) => {
                self.reconnect_conn_id = None;
                // The pre-disconnect Receive is gone; schedule a fresh Receive covering
                // surviving pipelines (plus the pool stream when none) so the session loop
                // resumes the receive loop.
                Ok(vec![
                    ActiveSessionOutput::OperationSuccess,
                    ActiveSessionOutput::PendingReceive {
                        desired_streams: self.runspace_pool.compute_active_desired_streams(),
                    },
                ])
            }
            Err(PwshCoreError::InvalidResponse(reason)) => {
                // The Reconnect request itself received the wrong shape of response: revert
                // to Disconnected so the session loop surfaces ReconnectFailed instead of
                // remaining stuck in Connecting.
                self.reconnect_conn_id = None;
                self.runspace_pool.abort_reconnect();
                error!(
                    reason = %reason,
                    body = %xml_body,
                    conn_id = conn_id.inner(),
                    "Reconnect request returned an invalid response; reverting runspace pool to Disconnected"
                );
                Ok(vec![ActiveSessionOutput::Ignore])
            }
            Err(PwshCoreError::SoapFault { code, reason }) => {
                // The Reconnect request faulted (e.g. the shell is gone): revert to
                // Disconnected. The session loop observes Connecting → Disconnected and
                // surfaces ReconnectFailed to the user.
                self.reconnect_conn_id = None;
                self.runspace_pool.abort_reconnect();
                error!(
                    %code,
                    %reason,
                    conn_id = conn_id.inner(),
                    "Reconnect request faulted; reverting runspace pool to Disconnected"
                );
                Ok(vec![ActiveSessionOutput::Ignore])
            }
            Err(e) => Err(e),
        }
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
