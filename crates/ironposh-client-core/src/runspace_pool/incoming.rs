//! Inbound server-response / PSRP message handling for [`RunspacePool`].
//!
//! This file is a continuation of the `impl RunspacePool` block that lives in
//! [`super::pool`]. It groups the methods that accept and dispatch inbound
//! server traffic: the WSMan Disconnect/Reconnect responses, the SOAP-fault
//! helpers, the main `accept_response` entry point, and the per-message PSRP
//! handlers it fans out to (`handle_pwsh_responses` and friends). This is
//! groundwork for a future event-driven `on_message` API.
//!
//! This is a pure file-organization split: the methods here are behavior- and
//! signature-identical to their previous definitions in `pool.rs`.

use std::collections::HashSet;

use base64::Engine;
use ironposh_psrp::{
    ApplicationPrivateData, ErrorRecord, PipelineOutput, PsValue, RunspacePoolStateMessage,
    SessionCapability, fragmentation,
};
use ironposh_winrm::{soap::SoapEnvelope, ws_management::WsAction};
use ironposh_xml::mapping::FromXml;
use rsa::pkcs1v15::Pkcs1v15Encrypt;
use tracing::{debug, error, info, instrument, trace, warn};
use uuid::Uuid;

use crate::{
    PwshCoreError, host::HostCall, powershell::PipelineHandle, runspace::win_rs::WinRunspace,
    runspace_pool::PsInvocationState,
};

use super::enums::RunspacePoolState;
use super::pool::{AcceptResponsResult, DesiredStream, RunspacePool};

impl RunspacePool {
    /// Accept the server's DisconnectResponse.
    /// Valid only in `Disconnecting` state; transitions the pool to `Disconnected`.
    #[instrument(skip(self, soap_envelope), fields(envelope_length = soap_envelope.len()))]
    pub fn accept_disconnect_response(
        &mut self,
        soap_envelope: &str,
    ) -> Result<(), crate::PwshCoreError> {
        if self.state != RunspacePoolState::Disconnecting {
            return Err(crate::PwshCoreError::InvalidState(
                "RunspacePool must be in Disconnecting state to accept a disconnect response",
            ));
        }

        let parsed = ironposh_xml::parser::parse(soap_envelope)?;
        let soap_envelope = SoapEnvelope::from_xml(parsed.root_element())
            .map_err(crate::PwshCoreError::XmlParsingError)?;

        Self::fault_to_error(&soap_envelope)?;

        // Real Windows servers answer shell Disconnect with an empty Body and
        // identify the operation via the `a:Action` header only; the
        // documented `rsp:DisconnectResponse` body element is accepted too.
        if soap_envelope.body.as_ref().disconnect_response.is_none()
            && !Self::header_action_is(&soap_envelope, &WsAction::DisconnectResponse)
        {
            return Err(crate::PwshCoreError::InvalidResponse(
                "No DisconnectResponse found in response".into(),
            ));
        }

        self.state = RunspacePoolState::Disconnected;
        info!(runspace_pool_id = %self.id, "runspace pool disconnected");
        Ok(())
    }

    /// Accept the server's ReconnectResponse.
    /// Valid only in `Connecting` state; transitions the pool back to `Opened`.
    /// The caller is responsible for resuming the receive loop afterwards.
    #[instrument(skip(self, soap_envelope), fields(envelope_length = soap_envelope.len()))]
    pub fn accept_reconnect_response(
        &mut self,
        soap_envelope: &str,
    ) -> Result<(), crate::PwshCoreError> {
        if self.state != RunspacePoolState::Connecting {
            return Err(crate::PwshCoreError::InvalidState(
                "RunspacePool must be in Connecting state to accept a reconnect response",
            ));
        }

        let parsed = ironposh_xml::parser::parse(soap_envelope)?;
        let soap_envelope = SoapEnvelope::from_xml(parsed.root_element())
            .map_err(crate::PwshCoreError::XmlParsingError)?;

        Self::fault_to_error(&soap_envelope)?;

        // Real Windows servers answer shell Reconnect with an empty Body and
        // identify the operation via the `a:Action` header only; the
        // documented `rsp:ReconnectResponse` body element is accepted too.
        if soap_envelope.body.as_ref().reconnect_response.is_none()
            && !Self::header_action_is(&soap_envelope, &WsAction::ReconnectResponse)
        {
            return Err(crate::PwshCoreError::InvalidResponse(
                "No ReconnectResponse found in response".into(),
            ));
        }

        self.state = RunspacePoolState::Opened;
        // The Receive that was in flight before the disconnect is gone; the caller
        // must fire a fresh pool-stream Receive to resume the receive loop.
        self.desired_stream_is_pooling = false;
        info!(runspace_pool_id = %self.id, "runspace pool reconnected");
        Ok(())
    }

    /// Whether the envelope's `a:Action` header equals the given WSMan action.
    fn header_action_is(soap_envelope: &SoapEnvelope<'_>, action: &WsAction) -> bool {
        soap_envelope
            .header
            .as_ref()
            .and_then(|header| header.as_ref().action.as_ref())
            .is_some_and(|tag| tag.as_ref().as_ref() == action.as_str())
    }

    /// Surface a WSMan SOAP fault as a `SoapFault` error.
    pub(super) fn fault_to_error(
        soap_envelope: &SoapEnvelope<'_>,
    ) -> Result<(), crate::PwshCoreError> {
        if let Some(fault_tag) = soap_envelope.body.as_ref().fault.as_ref() {
            let fault = fault_tag.as_ref();
            let code = fault
                .code
                .as_ref()
                .and_then(|c| c.as_ref().value.as_ref())
                .map_or("unknown", |v| <&str>::from(v.as_ref()))
                .to_string();
            let reason = fault.reason_text().unwrap_or("unknown").to_string();
            error!(target: "accept_response", %code, %reason, "received SOAP fault");
            return Err(PwshCoreError::SoapFault { code, reason });
        }
        Ok(())
    }

    #[expect(clippy::too_many_lines)]
    #[instrument(skip(self, soap_envelope), fields(envelope_length = soap_envelope.len()))]
    pub(crate) fn accept_response(
        &mut self,
        soap_envelope: &str,
    ) -> Result<Vec<AcceptResponsResult>, crate::PwshCoreError> {
        debug!(target: "soap", "parsing SOAP envelope");

        let parsed = ironposh_xml::parser::parse(soap_envelope).map_err(|e| {
            error!(target: "xml", error = %e, xml = soap_envelope, "failed to parse XML");
            e
        })?;

        let soap_envelope = SoapEnvelope::from_xml(parsed.root_element()).map_err(|e| {
            error!(target: "soap", error = %e, "failed to parse SOAP envelope");
            crate::PwshCoreError::XmlParsingError(e)
        })?;

        let mut result = Vec::new();

        if soap_envelope.body.as_ref().receive_response.is_some() {
            debug!(target: "receive", "processing receive response");

            let (streams, command_state) = WinRunspace::accept_receive_response(&soap_envelope)
                .map_err(|e| {
                    error!(target: "receive", error = %e, "failed to accept receive response");
                    e
                })?;

            let streams_ids = streams
                .iter()
                .filter_map(|stream| stream.command_id().copied())
                .collect::<Vec<_>>();

            let is_there_a_stream_has_no_command_id =
                streams.iter().any(|stream| stream.command_id().is_none());

            if is_there_a_stream_has_no_command_id {
                debug!(
                    target: "receive",
                    "stream without command_id found, should be runspace pool stream"
                );
                self.desired_stream_is_pooling = false;
            }

            debug!(
                target: "receive",
                stream_count = streams.len(),
                stream_command_ids = ?streams_ids,
                "processing streams"
            );

            let handle_results = self.handle_pwsh_responses(streams).map_err(|e| {
                error!(target: "pwsh", error = %e, "failed to handle PowerShell responses");
                e
            })?;

            let already_scheduled_receive = handle_results
                .iter()
                .any(|r| matches!(r, AcceptResponsResult::SendThenReceive { .. }));

            debug!(
                target: "pwsh",
                response_count = handle_results.len(),
                already_scheduled_receive,
                "handled PowerShell responses"
            );

            result.extend(handle_results);

            if let Some(command_state) = command_state
                && command_state.is_done()
            {
                debug!(
                    target: "pipeline",
                    pipeline_id = ?command_state.command_id,
                    "command state done received, removing pipeline"
                );
                // If command state is done, we can remove the pipeline from the pool
                let pipeline = self.pipelines.remove(&command_state.command_id);
                if pipeline.is_some() {
                    result.push(AcceptResponsResult::PipelineFinished(PipelineHandle {
                        id: command_state.command_id,
                    }));
                }
            }

            let desired_streams = if !streams_ids.is_empty() {
                // find the intersetction of streams.id and self.pipelines.keys()
                let next_desired_streams = streams_ids.into_iter().filter(|stream| {
                    self.pipelines
                        .keys()
                        .any(|pipeline_id| pipeline_id == stream)
                });

                // keep unique stream with the same id
                let mut stream_set = HashSet::new();

                for stream in next_desired_streams {
                    stream_set.insert(stream);
                }

                stream_set
                    .into_iter()
                    .map(|stream| DesiredStream::new("stdout", stream.into()))
                    .collect::<Vec<_>>()
            } else if !self.desired_stream_is_pooling {
                self.desired_stream_is_pooling = true;
                DesiredStream::runspace_pool_streams()
            } else {
                vec![]
            };

            if !already_scheduled_receive && !desired_streams.is_empty() {
                result.push(AcceptResponsResult::ReceiveResponse { desired_streams });
            }
        }

        if soap_envelope.body.as_ref().command_response.is_some() {
            let pipeline_id = self.shell.accept_commannd_response(&soap_envelope)?;

            self.pipelines
                .get_mut(&pipeline_id)
                .ok_or_else(|| {
                    crate::PwshCoreError::InvalidResponse(
                        "Pipeline not found for command response".into(),
                    )
                })?
                .set_state(PsInvocationState::Running);

            result.push(AcceptResponsResult::ReceiveResponse {
                desired_streams: vec![DesiredStream::stdout_for_command(pipeline_id)],
            });

            result.push(AcceptResponsResult::PipelineCreated(PipelineHandle {
                id: pipeline_id,
            }));
        }

        if soap_envelope.body.as_ref().signal_response.is_some() {
            let pipeline_id = self.shell.accept_signal_response(&soap_envelope)?;
            match pipeline_id {
                None => {
                    // Don't know what to do with it
                }
                Some(id) => match self.pipelines.remove(&id) {
                    None => {
                        warn!(
                            target: "signal",
                            pipeline_id = ?id,
                            "received signal response for unknown pipeline"
                        );
                    }
                    Some(_) => {
                        result.push(AcceptResponsResult::PipelineFinished(PipelineHandle { id }));
                    }
                },
            }
        }

        // Handle SOAP faults (e.g. operation timeout heartbeats)
        if let Some(fault_tag) = soap_envelope.body.as_ref().fault.as_ref() {
            let fault = fault_tag.as_ref();
            if fault.is_timeout() {
                info!(
                    target: "accept_response",
                    "received WS-Management timeout fault (heartbeat), re-issuing Receive"
                );
                // Normal heartbeat - re-issue Receive for active streams
                let desired_streams = self.compute_active_desired_streams();
                if !desired_streams.is_empty() {
                    result.push(AcceptResponsResult::ReceiveResponse { desired_streams });
                }
            } else if fault.is_invalid_selectors() {
                // Common cancel race: we had a Receive(CommandId=...) in flight while the
                // server already tore down the command. Treat this as non-fatal and
                // stop polling pipelines so the session remains usable.
                let reason = fault.reason_text().unwrap_or("unknown");
                warn!(
                    target: "accept_response",
                    reason = %reason,
                    pipeline_count = self.pipelines.len(),
                    "received WS-Management InvalidSelectors fault; dropping active pipelines and continuing"
                );

                let finished: Vec<Uuid> = self.pipelines.keys().copied().collect();
                self.pipelines.clear();

                for id in finished {
                    result.push(AcceptResponsResult::PipelineFinished(PipelineHandle { id }));
                }

                let desired_streams = self.compute_active_desired_streams();
                if !desired_streams.is_empty() {
                    result.push(AcceptResponsResult::ReceiveResponse { desired_streams });
                }
            } else if let Some(stopping) = self.stopping_pipelines_for_fault() {
                // A non-timeout fault can answer a Receive we issued for a pipeline the
                // server already tore down after our Ctrl+C. If a pipeline is Stopping,
                // treat the fault as its completion instead of killing the session.
                let reason = fault.reason_text().unwrap_or("unknown");
                warn!(
                    target: "accept_response",
                    reason = %reason,
                    stopping_count = stopping.len(),
                    "non-timeout SOAP fault while a pipeline is stopping; finishing it and continuing"
                );

                for id in stopping {
                    self.pipelines.remove(&id);
                    result.push(AcceptResponsResult::PipelineFinished(PipelineHandle { id }));
                }

                let desired_streams = self.compute_active_desired_streams();
                if !desired_streams.is_empty() {
                    result.push(AcceptResponsResult::ReceiveResponse { desired_streams });
                }
            } else {
                // Real fault - propagate as error
                let code = fault
                    .code
                    .as_ref()
                    .and_then(|c| c.as_ref().value.as_ref())
                    .map_or("unknown", |v| <&str>::from(v.as_ref()))
                    .to_string();
                let reason = fault.reason_text().unwrap_or("unknown").to_string();
                error!(
                    target: "accept_response",
                    %code,
                    %reason,
                    "received non-timeout SOAP fault"
                );
                return Err(PwshCoreError::SoapFault { code, reason });
            }
        }

        debug!(
            target: "accept_response",
            result_count = result.len(),
            result_types = ?result.iter().map(std::mem::discriminant).collect::<Vec<_>>(),
            "accept response results"
        );

        Ok(result)
    }

    /// Pipelines currently in the `Stopping` state, if any. Used to decide
    /// whether a non-timeout SOAP fault is answering a Receive for a pipeline we
    /// are already tearing down (Ctrl+C), in which case the fault is expected and
    /// non-fatal.
    fn stopping_pipelines_for_fault(&self) -> Option<Vec<Uuid>> {
        let stopping: Vec<Uuid> = self
            .pipelines
            .iter()
            .filter(|(_, p)| p.state() == PsInvocationState::Stopping)
            .map(|(id, _)| *id)
            .collect();
        (!stopping.is_empty()).then_some(stopping)
    }

    /// Fire create pipeline for a specific pipeline handle (used by service API)
    #[expect(clippy::too_many_lines)]
    #[instrument(skip(self, responses))]
    fn handle_pwsh_responses(
        &mut self,
        responses: Vec<crate::runspace::win_rs::Stream>,
    ) -> Result<Vec<AcceptResponsResult>, crate::PwshCoreError> {
        let mut result = Vec::new();

        for (stream_index, stream) in responses.into_iter().enumerate() {
            debug!(
                target: "stream",
                stream_index,
                stream_name = ?stream.name(),
                pipeline_id = ?stream.command_id(),
                "processing stream"
            );

            let messages = match self.defragmenter.defragment(stream.value()).map_err(|e| {
                error!(target: "defragment", stream_index, error = %e, "failed to defragment stream");
                e
            })? {
                fragmentation::DefragmentResult::Incomplete => {
                    debug!(target: "defragment", stream_index, "stream incomplete, continuing");
                    continue;
                }
                fragmentation::DefragmentResult::Complete(power_shell_remoting_messages) => {
                    debug!(
                        target: "defragment",
                        stream_index,
                        message_count = power_shell_remoting_messages.len(),
                        "stream complete"
                    );
                    power_shell_remoting_messages
                }
            };

            for (msg_index, message) in messages.into_iter().enumerate() {
                let ps_value = message.parse_ps_message().map_err(|e| {
                    error!(
                        target: "ps_message",
                        stream_index,
                        ?message,
                        error = %e,
                        "failed to parse PS message"
                    );
                    e
                })?;

                info!(
                    target: "ps_message",
                    message_type = ?message.message_type,
                    stream_index,
                    msg_index,
                    "parsed PS message"
                );

                match message.message_type {
                    ironposh_psrp::MessageType::PublicKeyRequest => {
                        debug!(
                            target: "key_exchange",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling PublicKeyRequest message"
                        );

                        // Validate the payload (best-effort).
                        if let Err(e) = ironposh_psrp::PublicKeyRequest::try_from(ps_value.clone())
                        {
                            warn!(
                                target: "key_exchange",
                                error = %e,
                                payload = ?ps_value,
                                "unexpected PublicKeyRequest payload"
                            );
                        }

                        let public_key_b64 = self.build_public_key_blob_base64()?;
                        let public_key_msg = ironposh_psrp::PublicKey {
                            public_key: public_key_b64,
                        };
                        let send_xml = self.send_runspace_pool_message(&public_key_msg)?;

                        result.push(AcceptResponsResult::SendThenReceive {
                            send_xml,
                            desired_streams: DesiredStream::runspace_pool_streams(),
                        });
                    }
                    ironposh_psrp::MessageType::EncryptedSessionKey => {
                        debug!(
                            target: "key_exchange",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling EncryptedSessionKey message"
                        );

                        let PsValue::Object(obj) = ps_value else {
                            return Err(crate::PwshCoreError::InvalidResponse(
                                "Expected EncryptedSessionKey as PsValue::Object".into(),
                            ));
                        };

                        let encrypted = ironposh_psrp::EncryptedSessionKey::try_from(obj)?;
                        let decoded = base64::engine::general_purpose::STANDARD
                            .decode(encrypted.encrypted_session_key)
                            .map_err(|e| {
                                crate::PwshCoreError::InvalidResponse(
                                    format!("Invalid base64 EncryptedSessionKey: {e}").into(),
                                )
                            })?;

                        if decoded.len() < 12 + 256 {
                            return Err(crate::PwshCoreError::InvalidResponse(
                                format!(
                                    "EncryptedSessionKey blob too short: {} bytes",
                                    decoded.len()
                                )
                                .into(),
                            ));
                        }

                        let encrypted_bytes = &decoded[12..12 + 256];
                        let state = self.ensure_key_exchange_state()?;

                        let decrypted = state
                            .private_key
                            .decrypt(Pkcs1v15Encrypt, encrypted_bytes)
                            .or_else(|e| {
                                // Some stacks may provide a representation that requires reversing.
                                // Try best-effort before failing hard.
                                let mut reversed = encrypted_bytes.to_vec();
                                reversed.reverse();
                                state
                                    .private_key
                                    .decrypt(Pkcs1v15Encrypt, &reversed)
                                    .map_err(|_e2| e)
                            })
                            .map_err(|e| {
                                crate::PwshCoreError::InternalError(format!(
                                    "failed to decrypt EncryptedSessionKey: {e}"
                                ))
                            })?;

                        if decrypted.len() != 32 {
                            return Err(crate::PwshCoreError::InvalidResponse(
                                format!(
                                    "Unexpected decrypted PSRP session key length: {} bytes",
                                    decrypted.len()
                                )
                                .into(),
                            ));
                        }

                        info!(
                            target: "key_exchange",
                            session_key_len = decrypted.len(),
                            "stored decrypted PSRP session key"
                        );
                        state.session_key = Some(decrypted);

                        self.psrp_key_exchange_pending = false;
                        while let Some(host_call) = self.pending_host_calls.pop_front() {
                            debug!(
                                target: "key_exchange",
                                host_call = ?host_call,
                                "releasing deferred host call after key exchange"
                            );
                            result.push(AcceptResponsResult::HostCall(host_call));
                        }
                    }
                    ironposh_psrp::MessageType::SessionCapability => {
                        debug!(target: "session", "handling SessionCapability message");
                        self.handle_session_capability(ps_value).map_err(|e| {
                            error!(target: "session", error = %e, "failed to handle SessionCapability");
                            e
                        })?;
                    }
                    ironposh_psrp::MessageType::ApplicationPrivateData => {
                        debug!(target: "session", "handling ApplicationPrivateData message");
                        self.handle_application_private_data(ps_value)
                            .map_err(|e| {
                                error!(target: "session", error = %e, "failed to handle ApplicationPrivateData");
                                e
                            })?;
                    }
                    ironposh_psrp::MessageType::RunspacepoolState => {
                        debug!(target: "runspace", "handling RunspacepoolState message");
                        self.handle_runspacepool_state(ps_value).map_err(|e| {
                            error!(target: "runspace", error = %e, "failed to handle RunspacepoolState");
                            e
                        })?;
                    }
                    ironposh_psrp::MessageType::ProgressRecord => {
                        debug!(
                            target: "progress",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling ProgressRecord message"
                        );
                        let record =
                            self.handle_progress_record(ps_value, stream.name(), stream.command_id())
                                .map_err(|e| {
                                error!(target: "progress", error = %e, "failed to handle ProgressRecord");
                                e
                                })?;

                        let cmd = *stream.command_id().ok_or_else(|| {
                            crate::PwshCoreError::InvalidResponse(
                                "ProgressRecord message must have a command_id".into(),
                            )
                        })?;
                        let message_type = message.message_type.clone();
                        let message_type_value = message_type.value();
                        result.push(AcceptResponsResult::PipelineRecord {
                            record: crate::psrp_record::PsrpRecord::Progress {
                                meta: crate::psrp_record::PsrpRecordMeta {
                                    message_type,
                                    message_type_value,
                                    stream: stream.name().to_string(),
                                    command_id: Some(cmd),
                                    data_len: message.data.len(),
                                },
                                record,
                            },
                            handle: PipelineHandle { id: cmd },
                        });
                    }
                    ironposh_psrp::MessageType::InformationRecord => {
                        debug!(
                            target: "information",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling InformationRecord message"
                        );
                        let Some(cmd) = stream.command_id().copied() else {
                            warn!(
                                target: "ps_message",
                                message_type = ?message.message_type,
                                message_type_value = message.message_type.value(),
                                stream = %stream.name(),
                                command_id = ?stream.command_id(),
                                "InformationRecord message missing command_id; ignoring"
                            );
                            continue;
                        };

                        let record = self
                            .handle_information_record(ps_value, stream.name(), &cmd)
                            .map_err(|e| {
                                error!(
                                    target: "information",
                                    error = %e,
                                    "failed to handle InformationRecord"
                                );
                                e
                            })?;
                        let message_type = message.message_type.clone();
                        let message_type_value = message_type.value();
                        result.push(AcceptResponsResult::PipelineRecord {
                            record: crate::psrp_record::PsrpRecord::Information {
                                meta: crate::psrp_record::PsrpRecordMeta {
                                    message_type,
                                    message_type_value,
                                    stream: stream.name().to_string(),
                                    command_id: Some(cmd),
                                    data_len: message.data.len(),
                                },
                                record,
                            },
                            handle: PipelineHandle { id: cmd },
                        });
                    }
                    ironposh_psrp::MessageType::DebugRecord
                    | ironposh_psrp::MessageType::VerboseRecord
                    | ironposh_psrp::MessageType::WarningRecord => {
                        let Some(cmd) = stream.command_id().copied() else {
                            warn!(
                                target: "ps_message",
                                message_type = ?message.message_type,
                                message_type_value = message.message_type.value(),
                                stream = %stream.name(),
                                command_id = ?stream.command_id(),
                                "record message missing command_id; ignoring"
                            );
                            continue;
                        };

                        let msg = ps_value.as_string().unwrap_or_else(|| ps_value.to_string());

                        let message_type = message.message_type.clone();
                        let message_type_value = message_type.value();
                        let meta = crate::psrp_record::PsrpRecordMeta {
                            message_type: message_type.clone(),
                            message_type_value,
                            stream: stream.name().to_string(),
                            command_id: Some(cmd),
                            data_len: message.data.len(),
                        };

                        let record = match message_type {
                            ironposh_psrp::MessageType::DebugRecord => {
                                crate::psrp_record::PsrpRecord::Debug { meta, message: msg }
                            }
                            ironposh_psrp::MessageType::VerboseRecord => {
                                crate::psrp_record::PsrpRecord::Verbose { meta, message: msg }
                            }
                            ironposh_psrp::MessageType::WarningRecord => {
                                crate::psrp_record::PsrpRecord::Warning { meta, message: msg }
                            }
                            _ => unreachable!("guarded by match arm"),
                        };

                        result.push(AcceptResponsResult::PipelineRecord {
                            record,
                            handle: PipelineHandle { id: cmd },
                        });
                    }
                    ironposh_psrp::MessageType::PipelineState => {
                        debug!(
                            target: "pipeline",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling PipelineState message"
                        );
                        self.handle_pipeline_state(ps_value, stream.name(), stream.command_id())
                            .map_err(|e| {
                                error!(target: "pipeline", error = %e, "failed to handle PipelineState");
                                e
                            })?;
                    }
                    ironposh_psrp::MessageType::PipelineHostCall => {
                        debug!(
                            target: "host_call",
                            stream_name = ?stream.name(),
                            pipeline_id = ?stream.command_id(),
                            "handling PipelineHostCall message"
                        );

                        let host_call = self
                            .handle_pipeline_host_call(ps_value, stream.name(), stream.command_id())
                            .map_err(|e| {
                                error!(target: "host_call", error = %e, "failed to handle PipelineHostCall");
                                e
                            })?;
                        debug!(target: "host_call", host_call = ?host_call, "successfully created host call");

                        let needs_session_key = super::host_call::needs_session_key(&host_call);

                        let has_session_key = self
                            .key_exchange
                            .as_ref()
                            .and_then(|s| s.session_key.as_ref())
                            .is_some();

                        if needs_session_key && !has_session_key {
                            info!(
                                target: "key_exchange",
                                host_call_method = host_call.method_name(),
                                "deferring host call until PSRP session key is established"
                            );
                            self.pending_host_calls.push_back(host_call);

                            if !self.psrp_key_exchange_pending {
                                self.psrp_key_exchange_pending = true;

                                info!(
                                    target: "key_exchange",
                                    "starting client-initiated PSRP key exchange"
                                );
                                let public_key_b64 = self.build_public_key_blob_base64()?;
                                let public_key_msg = ironposh_psrp::PublicKey {
                                    public_key: public_key_b64,
                                };
                                let send_xml = self.send_runspace_pool_message(&public_key_msg)?;
                                result.push(AcceptResponsResult::SendThenReceive {
                                    send_xml,
                                    desired_streams: DesiredStream::runspace_pool_streams(),
                                });
                            }
                        } else {
                            result.push(AcceptResponsResult::HostCall(host_call));
                        }
                    }
                    ironposh_psrp::MessageType::PipelineOutput => {
                        debug!(
                            target: "pipeline_output",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling PipelineOutput message"
                        );

                        let output = self.handle_pipeline_output(ps_value)?;

                        debug!(target: "pipeline_output", output = ?output, "successfully handled PipelineOutput");
                        result.push(AcceptResponsResult::PipelineOutput {
                            output,
                            handle: PipelineHandle {
                                id: *stream.command_id().ok_or_else(|| {
                                    crate::PwshCoreError::InvalidResponse(
                                        "PipelineOutput message must have a command_id".into(),
                                    )
                                })?,
                            },
                        });
                    }
                    ironposh_psrp::MessageType::ErrorRecord => {
                        debug!(
                            target: "error_record",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling ErrorRecord message"
                        );

                        let PsValue::Object(complex_object) = ps_value else {
                            return Err(crate::PwshCoreError::InvalidResponse(
                                "Expected ErrorRecord as PsValue::Object".into(),
                            ));
                        };

                        let error_record = ErrorRecord::try_from(complex_object).map_err(|e| {
                            error!(target: "error_record", error = %e, "failed to parse ErrorRecord");
                            e
                        })?;

                        debug!(target: "error_record", error_record = ?error_record, "successfully parsed ErrorRecord");
                        result.push(AcceptResponsResult::ErrorRecord {
                            error_record,
                            handle: PipelineHandle {
                                id: *stream.command_id().ok_or_else(|| {
                                    crate::PwshCoreError::InvalidResponse(
                                        "ErrorRecord message must have a command_id".into(),
                                    )
                                })?,
                            },
                        });
                    }
                    _ => {
                        let data_len = message.data.len();
                        let data_preview = String::from_utf8_lossy(
                            &message.data[..std::cmp::min(message.data.len(), 512)],
                        );
                        error!(
                            target: "ps_message",
                            message_type = ?message.message_type,
                            message_type_value = message.message_type.value(),
                            stream = %stream.name(),
                            command_id = ?stream.command_id(),
                            data_len,
                            data_preview = %data_preview,
                            "received message type but no handler implemented"
                        );

                        let Some(cmd) = stream.command_id().copied() else {
                            // No pipeline to attach to; log only (do not crash the session).
                            continue;
                        };
                        let message_type = message.message_type.clone();
                        let message_type_value = message_type.value();

                        result.push(AcceptResponsResult::PipelineRecord {
                            record: crate::psrp_record::PsrpRecord::Unsupported {
                                meta: crate::psrp_record::PsrpRecordMeta {
                                    message_type,
                                    message_type_value,
                                    stream: stream.name().to_string(),
                                    command_id: Some(cmd),
                                    data_len,
                                },
                                data_preview: data_preview.to_string(),
                            },
                            handle: PipelineHandle { id: cmd },
                        });
                    }
                }
            }
        }

        info!(
            target: "pwsh_responses",
            result_count = result.len(),
            "processed PowerShell responses"
        );
        Ok(result)
    }

    #[instrument(skip(self, session_capability), fields(protocol_version = tracing::field::Empty, ps_version = tracing::field::Empty))]
    pub(super) fn handle_session_capability(
        &mut self,
        session_capability: PsValue,
    ) -> Result<(), crate::PwshCoreError> {
        let PsValue::Object(session_capability) = session_capability else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected SessionCapability as PsValue::Object".into(),
            ));
        };

        let session_capability = SessionCapability::try_from(session_capability)?;

        debug!(
            target: "session",
            capability = ?session_capability,
            "received SessionCapability"
        );
        self.session_capability = Some(session_capability);
        Ok(())
    }

    #[instrument(skip(self, app_data))]
    pub(super) fn handle_application_private_data(
        &mut self,
        app_data: PsValue,
    ) -> Result<(), crate::PwshCoreError> {
        let PsValue::Object(app_data) = app_data else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected ApplicationPrivateData as PsValue::Object".into(),
            ));
        };

        let app_data = ApplicationPrivateData::try_from(app_data)?;
        trace!(target: "session", app_data = ?app_data, "received ApplicationPrivateData");
        self.application_private_data = Some(app_data);
        Ok(())
    }

    #[instrument(skip(self, ps_value), fields(runspace_state = tracing::field::Empty))]
    fn handle_runspacepool_state(&mut self, ps_value: PsValue) -> Result<(), crate::PwshCoreError> {
        let PsValue::Object(runspacepool_state) = ps_value else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected RunspacepoolState as PsValue::Object".into(),
            ));
        };

        let runspacepool_state = RunspacePoolStateMessage::try_from(runspacepool_state)?;

        // Record the state in the span
        let span = tracing::Span::current();
        span.record(
            "runspace_state",
            format!("{:?}", runspacepool_state.runspace_state),
        );

        trace!(target: "runspace", state = ?runspacepool_state, "received RunspacePoolState");

        self.state = RunspacePoolState::from(&runspacepool_state.runspace_state);

        Ok(())
    }

    #[instrument(skip(self, ps_value), fields(stream_name, command_id = ?command_id))]
    fn handle_progress_record(
        &mut self,
        ps_value: PsValue,
        stream_name: &str,
        command_id: Option<&Uuid>,
    ) -> Result<ironposh_psrp::ProgressRecord, crate::PwshCoreError> {
        let PsValue::Object(progress_record) = ps_value else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected ProgressRecord as PsValue::Object".into(),
            ));
        };

        let progress_record = ironposh_psrp::ProgressRecord::try_from(progress_record)?;

        // Question: Can we have a Optional command id here?
        let Some(command_id) = command_id else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected command_id to be Some".into(),
            ));
        };

        trace!(
            target: "progress",
            progress_record = ?progress_record,
            stream_name = stream_name,
            command_id = ?command_id,
            "received ProgressRecord"
        );

        // Find the pipeline by command_id
        let pipeline = self.pipelines.get_mut(command_id).ok_or_else(|| {
            PwshCoreError::InvalidResponse("Pipeline not found for command_id".into())
        })?;

        pipeline.add_progress_record(progress_record.clone());

        Ok(progress_record)
    }

    #[instrument(skip(self, ps_value, stream_name, command_id))]
    fn handle_information_record(
        &mut self,
        ps_value: PsValue,
        stream_name: &str,
        command_id: &Uuid,
    ) -> Result<ironposh_psrp::InformationRecord, crate::PwshCoreError> {
        let (info_record_obj, lossy_fallback_str) = match ps_value {
            PsValue::Object(obj) => {
                let fallback = obj
                    .to_string
                    .clone()
                    .or_else(|| {
                        obj.properties
                            .get("MessageData")
                            .or_else(|| obj.properties.get("InformationalRecord_Message"))
                            .map(ToString::to_string)
                    })
                    .unwrap_or_else(|| "<InformationRecord>".to_string());
                (obj, fallback)
            }
            other @ PsValue::Primitive(_) => {
                warn!(
                    target: "information",
                    stream_name = stream_name,
                    command_id = %command_id,
                    "InformationRecord payload was not an object; using lossy string"
                );
                return Ok(ironposh_psrp::InformationRecord::builder()
                    .message_data(ironposh_psrp::InformationMessageData::String(
                        other.to_string(),
                    ))
                    .build());
            }
        };

        let info_record = match ironposh_psrp::InformationRecord::try_from(info_record_obj) {
            Ok(info_record) => info_record,
            Err(e) => {
                // `Write-Information -MessageData` is typed as `object` and does not always serialize
                // as a primitive string. Keep the session alive and fall back to a best-effort
                // string representation.
                warn!(
                    target: "information",
                    error = %e,
                    stream_name = stream_name,
                    command_id = %command_id,
                    "failed to parse InformationRecord; using lossy message_data"
                );
                ironposh_psrp::InformationRecord::builder()
                    .message_data(ironposh_psrp::InformationMessageData::String(
                        lossy_fallback_str,
                    ))
                    .build()
            }
        };
        trace!(
            ?info_record,
            stream_name = stream_name,
            command_id = %command_id,
            "Received InformationRecord"
        );

        // Find the pipeline by command_id
        let pipeline = self.pipelines.get_mut(command_id).ok_or_else(|| {
            PwshCoreError::InvalidResponse("Pipeline not found for command_id".into())
        })?;

        pipeline.add_information_record(info_record.clone());

        Ok(info_record)
    }

    #[instrument(skip(self, ps_value, stream_name, command_id))]
    fn handle_pipeline_state(
        &mut self,
        ps_value: PsValue,
        stream_name: &str,
        command_id: Option<&Uuid>,
    ) -> Result<(), crate::PwshCoreError> {
        let PsValue::Object(pipeline_state) = ps_value else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected PipelineState as PsValue::Object".into(),
            ));
        };

        let pipeline_state = ironposh_psrp::PipelineStateMessage::try_from(pipeline_state)?;
        trace!(
            ?pipeline_state,
            stream_name = stream_name,
            command_id = ?command_id,
            "Received PipelineState"
        );
        // Question: Can we have a Optional command id here?
        let Some(command_id) = command_id else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected command_id to be Some".into(),
            ));
        };

        // Find the pipeline by command_id
        let pipeline = self.pipelines.get_mut(command_id).ok_or_else(|| {
            PwshCoreError::InvalidResponse("Pipeline not found for command_id".into())
        })?;
        // Update the pipeline state
        pipeline.set_state(PsInvocationState::from(pipeline_state.pipeline_state));

        Ok(())
    }

    #[instrument(skip_all)]
    pub fn handle_pipeline_host_call(
        &mut self,
        ps_value: PsValue,
        stream_name: &str,
        command_id: Option<&Uuid>,
    ) -> Result<HostCall, crate::PwshCoreError> {
        super::host_call::pipeline_host_call_from(ps_value, stream_name, command_id)
    }

    pub fn handle_pipeline_output(
        &mut self,
        ps_value: PsValue,
    ) -> Result<PipelineOutput, PwshCoreError> {
        let pipeline_output = PipelineOutput::from(ps_value);

        Ok(pipeline_output)
    }
}
