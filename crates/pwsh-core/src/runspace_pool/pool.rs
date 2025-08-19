use std::{collections::HashMap, sync::Arc};

use base64::Engine;
use protocol_powershell_remoting::{
    ApartmentState, ApplicationPrivateData, Commands, CreatePipeline, Defragmenter, HostInfo,
    InitRunspacePool, PSThreadOptions, PowerShellPipeline, PsValue, RunspacePoolStateMessage,
    SessionCapability, fragment,
};
use protocol_winrm::{
    soap::SoapEnvelope,
    ws_management::{OptionSetValue, WsMan},
};
use tracing::{debug, info, instrument, trace};
use uuid::Uuid;
use xml::parser::XmlDeserialize;

use crate::{
    PwshCoreError,
    host::{HostCall, HostCallType},
    pipeline::Pipeline,
    powershell::PowerShell,
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

    pub(crate) fn stdout_for_command(command_id: Uuid) -> Self {
        DesiredStream {
            name: "stdout".to_string(),
            command_id: Some(command_id),
        }
    }
}

#[derive(Debug)]
pub enum AcceptResponsResult {
    ReceiveResponse { desired_streams: Vec<DesiredStream> },
    NewPipeline(PowerShell),
    HostCall(HostCall),
}

#[derive(Debug)]
pub enum PwshMessageResponse {
    HostCall(HostCall),
    // TODO: Add other message responses like PipelineOutput, PipelineError, etc.
    // PipelineOutput
}

impl From<PwshMessageResponse> for AcceptResponsResult {
    fn from(response: PwshMessageResponse) -> Self {
        match response {
            PwshMessageResponse::HostCall(host_call) => AcceptResponsResult::HostCall(host_call),
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
    pub(super) fragmenter: fragment::Fragmenter,
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

        debug!(session_capability = ?session_capability);
        debug!(init_runspace_pool = ?init_runspace_pool);

        let request_groups = self.fragmenter.fragment_multiple(
            &[&session_capability, &init_runspace_pool],
            self.id,
            None,
        )?;

        trace!(request_groups = ?request_groups, "Fragmented negotiation requests");

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
            result.into().to_string(),
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
            .to_string())
    }

    #[instrument(skip(self, soap_envelope))]
    pub(crate) fn accept_response(
        &mut self,
        soap_envelope: String,
    ) -> Result<Vec<AcceptResponsResult>, crate::PwshCoreError> {
        let parsed = xml::parser::parse(soap_envelope.as_str())?;
        let soap_envelope = SoapEnvelope::from_node(parsed.root_element())
            .map_err(crate::PwshCoreError::XmlParsingError)?;

        let mut result = Vec::new();

        if soap_envelope.body.as_ref().receive_response.is_some() {
            let (streams, command_state) = self.shell.accept_receive_response(&soap_envelope)?;
            let streams_ids = streams
                .iter()
                .filter_map(|stream| stream.command_id().cloned())
                .collect::<Vec<_>>();

            let handle_pwsh_response = self.handle_pwsh_responses(streams)?;
            result.extend(handle_pwsh_response.into_iter().map(|resp| resp.into()));

            if let Some(command_state) = command_state
                && command_state.is_done()
            {
                // If command state is done, we can remove the pipeline from the pool
                self.pipelines.remove(&command_state.command_id);
            }

            let desired_streams = if !streams_ids.is_empty() {
                // find the intersetction of streams.id and self.pipelines.keys()
                streams_ids
                    .into_iter()
                    .filter(|stream| {
                        self.pipelines
                            .keys()
                            .any(|pipeline_id| pipeline_id == stream)
                    })
                    .map(|stream| DesiredStream::new("stdout", stream.to_owned().into()))
                    .collect::<Vec<_>>()
            } else {
                DesiredStream::runspace_pool_streams()
            };

            result.push(AcceptResponsResult::ReceiveResponse { desired_streams });
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

        debug_assert!(!result.is_empty(), "We should have at least one result");

        Ok(result)
    }

    pub(crate) fn init_pipeline(&mut self) -> PowerShell {
        let pineline_id = uuid::Uuid::new_v4();
        self.pipelines.insert(pineline_id, Pipeline::new());
        PowerShell { id: pineline_id }
    }

    #[instrument(skip(self))]
    #[deprecated]
    pub(crate) fn fire_create_pipeline(&mut self) -> Result<String, crate::PwshCoreError> {
        if self.state != RunspacePoolState::Opened {
            return Err(crate::PwshCoreError::InvalidState(
                "RunspacePool must be in Opened state to create a pipeline",
            ));
        }

        let pipeline_id = uuid::Uuid::new_v4();

        self.pipelines.insert(pipeline_id, Pipeline::new());

        // Create a command to execute instead of empty command list
        let cmd = protocol_powershell_remoting::Command::builder()
            .cmd(r#"Write-Host "Remote System: $($env:COMPUTERNAME) - $(Get-Date)""#)
            .is_script(true)
            .build();

        let pipeline_message = PowerShellPipeline::builder()
            .is_nested(false)
            .redirect_shell_error_output_pipe(true)
            .cmds(Commands::new(cmd))
            .build();

        let create_pipeline = CreatePipeline::builder()
            .power_shell(pipeline_message)
            .host_info(self.host_info.clone())
            .apartment_state(self.apartment_state)
            .build();

        debug!(?create_pipeline);

        let fragmented =
            self.fragmenter
                .fragment(&create_pipeline, self.id, Some(pipeline_id), None)?;

        let arguments = fragmented
            .into_iter()
            .map(|bytes| base64::engine::general_purpose::STANDARD.encode(&bytes[..]))
            .collect::<Vec<_>>();

        let request = self.shell.create_pipeline_request(
            &self.connection,
            pipeline_id,
            arguments,
            None,
            None,
        )?;

        Ok(request.into().to_string())
    }

    /// Fire create pipeline for a specific pipeline handle (used by service API)
    #[instrument(skip(self, responses))]
    fn handle_pwsh_responses(
        &mut self,
        responses: Vec<crate::runspace::win_rs::Stream>,
    ) -> Result<Vec<PwshMessageResponse>, crate::PwshCoreError> {
        let mut result = Vec::new();
        for stream in responses {
            let messages = match self.defragmenter.defragment(stream.value())? {
                fragment::DefragmentResult::Incomplete => continue,
                fragment::DefragmentResult::Complete(power_shell_remoting_messages) => {
                    power_shell_remoting_messages
                }
            };

            for message in messages {
                let ps_value = message.parse_ps_message()?;
                trace!(?ps_value, "Parsed PS message");
                match message.message_type {
                    protocol_powershell_remoting::MessageType::SessionCapability => {
                        self.handle_session_capability(ps_value)?;
                    }
                    protocol_powershell_remoting::MessageType::ApplicationPrivateData => {
                        self.handle_application_private_data(ps_value)?;
                    }
                    protocol_powershell_remoting::MessageType::RunspacepoolState => {
                        self.handle_runspacepool_state(ps_value)?;
                    }
                    protocol_powershell_remoting::MessageType::ProgressRecord => {
                        self.handle_progress_record(ps_value, stream.name(), stream.command_id())?;
                    }
                    protocol_powershell_remoting::MessageType::InformationRecord => {
                        self.handle_information_record(
                            ps_value,
                            stream.name(),
                            stream.command_id(),
                        )?;
                    }
                    protocol_powershell_remoting::MessageType::PipelineState => {
                        self.handle_pipeline_state(ps_value, stream.name(), stream.command_id())?;
                    }
                    protocol_powershell_remoting::MessageType::PipelineHostCall => {
                        let host_call = self.handle_pipeline_host_call(
                            ps_value,
                            stream.name(),
                            stream.command_id(),
                        )?;
                        result.push(PwshMessageResponse::HostCall(host_call));
                    }
                    _ => {
                        info!(
                            "Received message of type {:?}, but no handler implemented",
                            message.message_type
                        );
                        todo!("Handle other message types as needed");
                    }
                }
            }
        }

        Ok(result)
    }

    #[instrument(skip(self))]
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
        debug!(?session_capability, "Received SessionCapability");
        self.session_capability = Some(session_capability);
        Ok(())
    }

    #[instrument(skip(self))]
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
        trace!(?app_data, "Received ApplicationPrivateData");
        self.application_private_data = Some(app_data);
        Ok(())
    }

    #[instrument(skip(self, ps_value))]
    fn handle_runspacepool_state(&mut self, ps_value: PsValue) -> Result<(), crate::PwshCoreError> {
        let PsValue::Object(runspacepool_state) = ps_value else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected RunspacepoolState as PsValue::Object".into(),
            ));
        };

        let runspacepool_state = RunspacePoolStateMessage::try_from(runspacepool_state)?;
        trace!(?runspacepool_state, "Received RunspacePoolState");

        self.state = RunspacePoolState::from(&runspacepool_state.runspace_state);

        Ok(())
    }

    #[instrument(skip(self, ps_value, stream_name, command_id))]
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
            ?progress_record,
            stream_name = stream_name,
            command_id = ?command_id,
            "Received ProgressRecord"
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

    /// Adds a script to the specified pipeline.
    ///
    /// # Arguments
    /// * `handle`: The handle to the pipeline to modify.
    /// * `script`: The script string to add.
    pub fn add_script(
        &mut self,
        handle: PowerShell,
        script: impl Into<String>,
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

        pipeline.add_script(script.into());
        Ok(())
    }

    /// Adds a command (cmdlet) to the specified pipeline.
    pub fn add_command(
        &mut self,
        handle: PowerShell,
        command: impl Into<String>,
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

        pipeline.add_command(command.into());
        Ok(())
    }

    /// Adds a parameter to the last command in the specified pipeline.
    pub fn add_parameter(
        &mut self,
        handle: PowerShell,
        name: String,
        value: crate::pipeline::ParameterValue,
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

        pipeline.add_parameter(name, value);
        Ok(())
    }

    /// Adds a switch parameter (no value) to the last command in the specified pipeline.
    pub fn add_switch_parameter(
        &mut self,
        handle: PowerShell,
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
    pub fn invoke_pipeline_request(&mut self, handle: PowerShell) -> Result<String, PwshCoreError> {
        let pipeline = self
            .pipelines
            .get_mut(&handle.id())
            .ok_or(PwshCoreError::InvalidState("Pipeline handle not found"))?;

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

        Ok(request.into().to_string())
    }

    pub fn handle_pipeline_host_call(
        &mut self,
        ps_value: PsValue,
        stream_name: &str,
        command_id: Option<&Uuid>,
    ) -> Result<HostCall, crate::PwshCoreError> {
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
            "Received PipelineHostCall"
        );

        // Question: Can we have a Optional command id here?
        let Some(command_id) = command_id else {
            return Err(PwshCoreError::InvalidResponse(
                "Expected command_id to be Some".into(),
            ));
        };

        Ok(HostCall::from((
            &pipeline_host_call,
            HostCallType::Pipeline {
                id: command_id.to_owned(),
            },
        )))
    }
}
