use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use base64::Engine;
use protocol_powershell_remoting::{
    ApartmentState, ApplicationPrivateData, CreatePipeline, Defragmenter, HostInfo,
    InitRunspacePool, PSThreadOptions, PipelineOutput, PsValue, RunspacePoolStateMessage,
    SessionCapability, fragmentation,
};
use protocol_winrm::{
    soap::SoapEnvelope,
    ws_management::{OptionSetValue, WsMan},
};
use tracing::{debug, error, info, instrument, trace, warn};
use uuid::Uuid;
use xml::parser::XmlDeserialize;

use crate::{
    PwshCoreError,
    host::{HostCallRequest, HostCallScope},
    pipeline::{Pipeline, PipelineCommand},
    powershell::{PipelineHandle, PipelineOutputType},
    runspace::win_rs::WinRunspace,
    runspace_pool::PsInvocationState,
};

use super::enums::RunspacePoolState;

const PROTOCOL_VERSION: &str = "2.3";
const PS_VERSION: &str = "2.0";
const SERIALIZATION_VERSION: &str = "1.1.0.1";

#[derive(Debug, Clone)]
pub struct DesiredStream {
    name: String,
    command_id: Option<Uuid>,
}
impl DesiredStream {
    pub(crate) fn new(name: impl Into<String>, command_id: Option<Uuid>) -> Self {
        Self {
            name: name.into(),
            command_id,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn command_id(&self) -> Option<&Uuid> {
        self.command_id.as_ref()
    }

    pub(crate) fn runspace_pool_streams() -> Vec<Self> {
        vec![DesiredStream {
            name: "stdout".to_string(),
            command_id: None,
        }]
    }

    pub(crate) fn pipeline_streams(command_id: Uuid) -> Vec<Self> {
        vec![
            DesiredStream {
                name: "stdout".to_string(),
                command_id: Some(command_id),
            },
            DesiredStream {
                name: "stderr".to_string(),
                command_id: Some(command_id),
            },
        ]
    }

    pub(crate) fn stdout_for_command(command_id: Uuid) -> Self {
        DesiredStream {
            name: "stdout".to_string(),
            command_id: Some(command_id),
        }
    }
}

#[derive(Debug)]
pub enum AcceptResponsResult {
    ReceiveResponse {
        desired_streams: Vec<DesiredStream>,
    },
    PipelineCreated(PipelineHandle),
    PipelineFinished(PipelineHandle),
    HostCall(HostCallRequest),
    PipelineOutput {
        output: PipelineOutput,
        handle: PipelineHandle,
    },
}

#[derive(Debug)]
pub enum PwshMessageResponse {
    HostCall(HostCallRequest),
    PipelineOutput {
        output: PipelineOutput,
        handle: PipelineHandle,
    },
}

impl PwshMessageResponse {
    pub fn name(&self) -> &str {
        match self {
            PwshMessageResponse::HostCall(_) => "HostCall",
            PwshMessageResponse::PipelineOutput { .. } => "PipelineOutput",
        }
    }
}

impl From<PwshMessageResponse> for AcceptResponsResult {
    fn from(response: PwshMessageResponse) -> Self {
        match response {
            PwshMessageResponse::HostCall(host_call) => AcceptResponsResult::HostCall(host_call),
            PwshMessageResponse::PipelineOutput { output, handle } => {
                AcceptResponsResult::PipelineOutput { output, handle }
            }
        }
    }
}

#[derive(Debug)]
pub struct RunspacePool {
    pub(super) id: uuid::Uuid,
    pub(crate) state: RunspacePoolState,
    pub(super) min_runspaces: usize,
    pub(super) max_runspaces: usize,
    pub(super) thread_options: PSThreadOptions,
    pub(super) apartment_state: ApartmentState,
    pub(super) host_info: HostInfo,
    pub(super) application_arguments: std::collections::BTreeMap<PsValue, PsValue>,
    pub(super) shell: WinRunspace,
    pub(super) connection: Arc<WsMan>,
    pub(super) defragmenter: Defragmenter,
    pub(super) application_private_data: Option<ApplicationPrivateData>,
    pub(super) session_capability: Option<SessionCapability>,
    pub(super) pipelines: HashMap<uuid::Uuid, Pipeline>,
    pub(super) fragmenter: fragmentation::Fragmenter,
    pub(super) runspace_pool_desired_stream_is_pooling: bool,
}

impl RunspacePool {
    #[instrument(skip(self), name = "RunspacePool::open")]
    pub fn open(
        mut self,
    ) -> Result<(String, super::expect_shell_created::ExpectShellCreated), crate::PwshCoreError>
    {
        if self.state != RunspacePoolState::BeforeOpen {
            return Err(crate::PwshCoreError::InvalidState(
                "RunspacePool must be in BeforeOpen state to open",
            ));
        }

        let session_capability = SessionCapability {
            protocol_version: PROTOCOL_VERSION.to_string(),
            ps_version: PS_VERSION.to_string(),
            serialization_version: SERIALIZATION_VERSION.to_string(),
            time_zone: None,
        };

        let init_runspace_pool = InitRunspacePool {
            min_runspaces: self.min_runspaces as i32,
            max_runspaces: self.max_runspaces as i32,
            thread_options: self.thread_options,
            apartment_state: self.apartment_state,
            host_info: self.host_info.clone(),
            application_arguments: self.application_arguments.clone(),
        };

        debug!(
            session_capability = ?session_capability,
            min_runspaces = self.min_runspaces,
            max_runspaces = self.max_runspaces,
            "starting runspace pool open"
        );
        debug!(init_runspace_pool = ?init_runspace_pool);

        let request_groups = self.fragmenter.fragment_multiple(
            &[&session_capability, &init_runspace_pool],
            self.id,
            None,
        )?;

        trace!(
            target: "fragmentation",
            request_groups = ?request_groups,
            group_count = request_groups.len(),
            "fragmented negotiation requests"
        );

        self.state = RunspacePoolState::NegotiationSent;

        debug_assert!(
            request_groups.len() == 1,
            "We should have only one request group for the opening negotiation"
        );

        let request = request_groups
            .into_iter()
            .next()
            .ok_or(crate::PwshCoreError::UnlikelyToHappen(
                "No request group generated for negotiation",
            ))
            .map(|bytes| base64::engine::general_purpose::STANDARD.encode(&bytes[..]))?;

        let option_set = OptionSetValue::new().add_option("protocolversion", PROTOCOL_VERSION);

        let result = self
            .shell
            .open(&self.connection, Some(option_set), &request);

        Ok((
            result.into().to_xml_string()?,
            super::expect_shell_created::ExpectShellCreated {
                runspace_pool: self,
            },
        ))
    }

    // We should accept the pipeline id here, but for now let's ignore it
    pub(crate) fn fire_receive<'a>(
        &mut self,
        desired_streams: Vec<DesiredStream>,
    ) -> Result<String, crate::PwshCoreError> {
        debug_assert!(!desired_streams.is_empty(), "At least one desired stream");
        Ok(self
            .shell
            .fire_receive(&self.connection, desired_streams)
            .into()
            .to_xml_string()?)
    }

    #[instrument(skip(self, soap_envelope), fields(envelope_length = soap_envelope.len()))]
    pub(crate) fn accept_response(
        &mut self,
        soap_envelope: String,
    ) -> Result<Vec<AcceptResponsResult>, crate::PwshCoreError> {
        debug!(target: "soap", "parsing SOAP envelope");

        let parsed = xml::parser::parse(soap_envelope.as_str()).map_err(|e| {
            error!(target: "xml", error = %e, "failed to parse XML");
            e
        })?;

        let soap_envelope = SoapEnvelope::from_node(parsed.root_element()).map_err(|e| {
            error!(target: "soap", error = %e, "failed to parse SOAP envelope");
            crate::PwshCoreError::XmlParsingError(e)
        })?;

        let mut result = Vec::new();

        if soap_envelope.body.as_ref().receive_response.is_some() {
            debug!(target: "receive", "processing receive response");

            let (streams, command_state) = self
                .shell
                .accept_receive_response(&soap_envelope)
                .map_err(|e| {
                    error!(target: "receive", error = %e, "failed to accept receive response");
                    e
                })?;

            let streams_ids = streams
                .iter()
                .filter_map(|stream| stream.command_id().cloned())
                .collect::<Vec<_>>();

            let is_there_a_stream_has_no_command_id =
                streams.iter().any(|stream| stream.command_id().is_none());
            if is_there_a_stream_has_no_command_id {
                debug!(
                    target: "receive",
                    "stream without command_id found, should be runspace pool stream"
                );
                self.runspace_pool_desired_stream_is_pooling = false
            }

            debug!(
                target: "receive",
                stream_count = streams.len(),
                stream_command_ids = ?streams_ids,
                "processing streams"
            );

            let handle_pwsh_response = self.handle_pwsh_responses(streams).map_err(|e| {
                error!(target: "pwsh", error = %e, "failed to handle PowerShell responses");
                e
            })?;

            debug!(
                target: "pwsh",
                response_names = ?handle_pwsh_response.iter().map(|r| r.name()).collect::<Vec<_>>(),
                response_count = handle_pwsh_response.len(),
                "handled PowerShell responses"
            );

            result.extend(handle_pwsh_response.into_iter().map(|resp| resp.into()));

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
                    .map(|stream| DesiredStream::new("stdout", stream.to_owned().into()))
                    .collect::<Vec<_>>()
            } else if !self.runspace_pool_desired_stream_is_pooling {
                self.runspace_pool_desired_stream_is_pooling = true;
                DesiredStream::runspace_pool_streams()
            } else {
                vec![]
            };

            if !desired_streams.is_empty() {
                result.push(AcceptResponsResult::ReceiveResponse { desired_streams });
            }
        }

        if soap_envelope.body.as_ref().command_response.is_some() {
            let pipeline_id = self.shell.accept_commannd_response(&soap_envelope)?;

            // We have received the pipeline creation response
            // 1. update the state of the pipeline
            // 2. fire receive request for the new pipeline
            self.pipelines
                .get_mut(&pipeline_id)
                .ok_or(crate::PwshCoreError::InvalidResponse(
                    "Pipeline not found for command response".into(),
                ))?
                .state = PsInvocationState::Running;

            result.push(AcceptResponsResult::ReceiveResponse {
                desired_streams: vec![DesiredStream::stdout_for_command(pipeline_id)],
            });
        }

        debug!(
            target: "accept_response",
            result_count = result.len(),
            result_types = ?result.iter().map(std::mem::discriminant).collect::<Vec<_>>(),
            "accept response results"
        );

        Ok(result)
    }

    pub(crate) fn init_pipeline(
        &mut self,
        uuid: Uuid,
    ) -> Result<PipelineHandle, crate::PwshCoreError> {
        if let Some(_) = self.pipelines.get(&uuid) {
            return Err(crate::PwshCoreError::InvalidState(
                "Pipeline with this UUID already exists",
            ));
        }

        self.pipelines.insert(uuid, Pipeline::new());
        Ok(PipelineHandle { id: uuid })
    }

    /// Fire create pipeline for a specific pipeline handle (used by service API)
    #[instrument(
        skip(self, responses),
        fields(
            response_count = responses.len(),
            processed_messages = 0u32
        )
    )]
    fn handle_pwsh_responses(
        &mut self,
        responses: Vec<crate::runspace::win_rs::Stream>,
    ) -> Result<Vec<PwshMessageResponse>, crate::PwshCoreError> {
        let mut result = Vec::new();
        let span = tracing::Span::current();
        let mut message_count = 0u32;

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
                message_count += 1;
                span.record("processed_messages", message_count);

                let ps_value = message.parse_ps_message().map_err(|e| {
                    error!(
                        target: "ps_message",
                        stream_index,
                        msg_index,
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
                    protocol_powershell_remoting::MessageType::SessionCapability => {
                        debug!(target: "session", "handling SessionCapability message");
                        self.handle_session_capability(ps_value).map_err(|e| {
                            error!(target: "session", error = %e, "failed to handle SessionCapability");
                            e
                        })?;
                    }
                    protocol_powershell_remoting::MessageType::ApplicationPrivateData => {
                        debug!(target: "session", "handling ApplicationPrivateData message");
                        self.handle_application_private_data(ps_value)
                            .map_err(|e| {
                                error!(target: "session", error = %e, "failed to handle ApplicationPrivateData");
                                e
                            })?;
                    }
                    protocol_powershell_remoting::MessageType::RunspacepoolState => {
                        debug!(target: "runspace", "handling RunspacepoolState message");
                        self.handle_runspacepool_state(ps_value).map_err(|e| {
                            error!(target: "runspace", error = %e, "failed to handle RunspacepoolState");
                            e
                        })?;
                    }
                    protocol_powershell_remoting::MessageType::ProgressRecord => {
                        debug!(
                            target: "progress",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling ProgressRecord message"
                        );
                        self.handle_progress_record(ps_value, stream.name(), stream.command_id())
                            .map_err(|e| {
                                error!(target: "progress", error = %e, "failed to handle ProgressRecord");
                                e
                            })?;
                    }
                    protocol_powershell_remoting::MessageType::InformationRecord => {
                        debug!(
                            target: "information",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling InformationRecord message"
                        );
                        self.handle_information_record(
                            ps_value,
                            stream.name(),
                            stream.command_id(),
                        )
                        .map_err(|e| {
                            error!(target: "information", error = %e, "failed to handle InformationRecord");
                            e
                        })?;
                    }
                    protocol_powershell_remoting::MessageType::PipelineState => {
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
                    protocol_powershell_remoting::MessageType::PipelineHostCall => {
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
                        result.push(PwshMessageResponse::HostCall(host_call));
                        debug!(target: "host_call", result_len = result.len(), "pushed HostCall response");
                    }
                    protocol_powershell_remoting::MessageType::PipelineOutput => {
                        debug!(
                            target: "pipeline_output",
                            stream_name = ?stream.name(),
                            command_id = ?stream.command_id(),
                            "handling PipelineOutput message"
                        );

                        let output = self.handle_pipeline_output(ps_value)?;

                        debug!(target: "pipeline_output", output = ?output, "successfully handled PipelineOutput");
                        result.push(PwshMessageResponse::PipelineOutput {
                            output,
                            handle: PipelineHandle {
                                id: *stream.command_id().ok_or(
                                    crate::PwshCoreError::InvalidResponse(
                                        "PipelineOutput message must have a command_id".into(),
                                    ),
                                )?,
                            },
                        });
                    }
                    _ => {
                        error!(
                            target: "ps_message",
                            message_type = ?message.message_type,
                            "received message type but no handler implemented"
                        );
                        todo!("Handle other message types as needed");
                    }
                }
            }
        }

        info!(
            target: "pwsh_responses",
            result_count = result.len(),
            total_messages_processed = message_count,
            "processed PowerShell responses"
        );
        Ok(result)
    }

    #[instrument(skip(self, session_capability), fields(protocol_version = tracing::field::Empty, ps_version = tracing::field::Empty))]
    fn handle_session_capability(
        &mut self,
        session_capability: PsValue,
    ) -> Result<(), crate::PwshCoreError> {
        let PsValue::Object(session_capability) = session_capability else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected SessionCapability as PsValue::Object".into(),
            ));
        };

        let session_capability = SessionCapability::try_from(session_capability)?;

        // Record the protocol and PS versions in the span
        let span = tracing::Span::current();
        span.record("protocol_version", &session_capability.protocol_version);
        span.record("ps_version", &session_capability.ps_version);

        debug!(
            target: "session",
            capability = ?session_capability,
            "received SessionCapability"
        );
        self.session_capability = Some(session_capability);
        Ok(())
    }

    #[instrument(skip(self, app_data))]
    fn handle_application_private_data(
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
    ) -> Result<(), crate::PwshCoreError> {
        let PsValue::Object(progress_record) = ps_value else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected ProgressRecord as PsValue::Object".into(),
            ));
        };

        let progress_record =
            protocol_powershell_remoting::ProgressRecord::try_from(progress_record)?;

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
        let pipeline = self
            .pipelines
            .get_mut(command_id)
            .ok_or(PwshCoreError::InvalidResponse(
                "Pipeline not found for command_id".into(),
            ))?;

        pipeline.add_progress_record(progress_record);

        Ok(())
    }

    #[instrument(skip(self, ps_value, stream_name, command_id))]
    fn handle_information_record(
        &mut self,
        ps_value: PsValue,
        stream_name: &str,
        command_id: Option<&Uuid>,
    ) -> Result<(), crate::PwshCoreError> {
        let PsValue::Object(info_record) = ps_value else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected InformationRecord as PsValue::Object".into(),
            ));
        };

        let info_record = protocol_powershell_remoting::InformationRecord::try_from(info_record)?;
        trace!(
            ?info_record,
            stream_name = stream_name,
            command_id = ?command_id,
            "Received InformationRecord"
        );

        // Question: Can we have a Optional command id here?
        let Some(command_id) = command_id else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected command_id to be Some".into(),
            ));
        };

        // Find the pipeline by command_id
        let pipeline = self
            .pipelines
            .get_mut(command_id)
            .ok_or(PwshCoreError::InvalidResponse(
                "Pipeline not found for command_id".into(),
            ))?;

        pipeline.add_information_record(info_record);

        Ok(())
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

        let pipeline_state =
            protocol_powershell_remoting::PipelineStateMessage::try_from(pipeline_state)?;
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
        let pipeline = self
            .pipelines
            .get_mut(command_id)
            .ok_or(PwshCoreError::InvalidResponse(
                "Pipeline not found for command_id".into(),
            ))?;
        // Update the pipeline state
        pipeline.state = PsInvocationState::from(pipeline_state.pipeline_state);

        Ok(())
    }

    // --- PowerShell Pipeline Management API ---
    // Note: PowerShell handles are created by the server via fire_create_pipeline/accept_response flow
    // Users should get handles from the ActiveSession after calling CreatePipeline operation

    /// Adds a switch parameter (no value) to the last command in the specified pipeline.
    pub fn add_switch_parameter(
        &mut self,
        handle: PipelineHandle,
        name: String,
    ) -> Result<(), PwshCoreError> {
        let pipeline = self
            .pipelines
            .get_mut(&handle.id())
            .ok_or(PwshCoreError::InvalidState("Pipeline handle not found"))?;

        if pipeline.state != PsInvocationState::NotStarted {
            return Err(PwshCoreError::InvalidState(
                "Cannot add to a pipeline that has already been started",
            ));
        }

        pipeline.add_switch_parameter(name);
        Ok(())
    }

    /// Invokes the specified pipeline and waits for its completion.
    ///
    /// This method will handle the entire PSRP message exchange:
    /// 1. Send the `CreatePipeline` message.
    /// 2. Send `Command`, `Send`, and `EndOfInput` messages.
    /// 3. Enter a loop to `Receive` and process responses.
    /// 4. Defragment and deserialize messages, updating the pipeline's state, output, and error streams.
    /// 5. Return the final output upon completion.
    pub fn invoke_pipeline_request(
        &mut self,
        handle: PipelineHandle,
        output_type: PipelineOutputType,
    ) -> Result<String, PwshCoreError> {
        let pipeline = self
            .pipelines
            .get_mut(&handle.id())
            .ok_or(PwshCoreError::InvalidState("Pipeline handle not found"))?;

        if let PipelineOutputType::Streamed = output_type {
            pipeline.add_command(PipelineCommand::new_output_stream());
        }

        // Set pipeline state to Running
        pipeline.state = PsInvocationState::Running;
        info!(pipeline_id = %handle.id(), "Invoking pipeline");

        // Convert business pipeline to protocol pipeline and build CreatePipeline message
        let protocol_pipeline = pipeline.to_protocol_pipeline()?;
        let create_pipeline = CreatePipeline::builder()
            .power_shell(protocol_pipeline)
            .host_info(self.host_info.clone())
            .apartment_state(self.apartment_state)
            .build();

        debug!(?create_pipeline);

        let fragmented =
            self.fragmenter
                .fragment(&create_pipeline, self.id, Some(handle.id()), None)?;

        let arguments = fragmented
            .into_iter()
            .map(|bytes| base64::engine::general_purpose::STANDARD.encode(&bytes[..]))
            .collect::<Vec<_>>();

        let request = self.shell.create_pipeline_request(
            &self.connection,
            handle.id(),
            arguments,
            None,
            None,
        )?;

        Ok(request.into().to_xml_string()?)
    }

    #[instrument(skip_all)]
    pub fn handle_pipeline_host_call(
        &mut self,
        ps_value: PsValue,
        stream_name: &str,
        command_id: Option<&Uuid>,
    ) -> Result<HostCallRequest, crate::PwshCoreError> {
        let PsValue::Object(pipeline_host_call) = ps_value else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected PipelineHostCall as PsValue::Object".into(),
            ));
        };

        let pipeline_host_call =
            protocol_powershell_remoting::PipelineHostCall::try_from(pipeline_host_call)?;

        debug!(
            ?pipeline_host_call,
            stream_name = stream_name,
            command_id = ?command_id,
            method_id = pipeline_host_call.method_id,
            method_name = pipeline_host_call.method_name,
            parameter_count = pipeline_host_call.parameters.len(),
            "Received PipelineHostCall"
        );

        // Question: Can we have a Optional command id here?
        let Some(command_id) = command_id else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected command_id to be Some".into(),
            ));
        };

        Ok(HostCallRequest::from((
            &pipeline_host_call,
            HostCallScope::Pipeline {
                command_id: command_id.to_owned(),
            },
        )))
    }

    /// Send a pipeline host response to the server
    pub fn send_pipeline_host_response(
        &mut self,
        command_id: uuid::Uuid,
        host_response: protocol_powershell_remoting::PipelineHostResponse,
    ) -> Result<String, PwshCoreError> {
        // Fragment the host response message
        let fragmented =
            self.fragmenter
                .fragment(&host_response, self.id, Some(command_id), None)?;

        // Encode fragments as base64
        let arguments = fragmented
            .into_iter()
            .map(|bytes| base64::engine::general_purpose::STANDARD.encode(&bytes[..]))
            .collect::<Vec<_>>();

        // Create WS-Man Send request (send data to stdin)
        let request =
            self.shell
                .send_data_request(&self.connection, Some(command_id), arguments)?;

        Ok(request.into().to_xml_string()?)
    }

    /// Send a runspace pool host response to the server
    pub fn send_runspace_pool_host_response(
        &mut self,
        host_response: protocol_powershell_remoting::RunspacePoolHostResponse,
    ) -> Result<String, PwshCoreError> {
        // Fragment the host response message
        let fragmented = self.fragmenter.fragment(
            &host_response,
            self.id,
            None, // No command ID for runspace pool messages
            None,
        )?;

        // Encode fragments as base64
        let arguments = fragmented
            .into_iter()
            .map(|bytes| base64::engine::general_purpose::STANDARD.encode(&bytes[..]))
            .collect::<Vec<_>>();

        // Create WS-Man Send request (send data to stdin)
        let request = self.shell.send_data_request(
            &self.connection,
            None, // No command ID for runspace pool
            arguments,
        )?;

        Ok(request.into().to_xml_string()?)
    }

    pub fn handle_pipeline_output(
        &mut self,
        ps_value: PsValue,
    ) -> Result<PipelineOutput, PwshCoreError> {
        let pipeline_output = PipelineOutput::from(ps_value);

        Ok(pipeline_output)
    }

    pub(crate) fn add_command(
        &mut self,
        powershell: PipelineHandle,
        command: PipelineCommand,
    ) -> Result<(), PwshCoreError> {
        let pipeline = self
            .pipelines
            .get_mut(&powershell.id())
            .ok_or(PwshCoreError::InvalidState("Pipeline handle not found"))?;

        if pipeline.state != PsInvocationState::NotStarted {
            return Err(PwshCoreError::InvalidState(
                "Cannot add to a pipeline that has already been started",
            ));
        }

        pipeline.add_command(command);
        Ok(())
    }
}
