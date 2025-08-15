use core::error;
use tracing::error;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use base64::Engine;
use protocol_powershell_remoting::{
    ApartmentState, ApplicationPrivateData, CreatePipeline, Defragmenter, HostInfo,
    InitRunspacePool, PSThreadOptions, PowerShellPipeline, PsValue, RunspacePoolStateMessage,
    SessionCapability, fragment,
};
use protocol_winrm::{
    soap::SoapEnvelope,
    ws_management::{OptionSetValue, WsMan},
};
use tracing::{debug, info, instrument, trace};
use xml::parser::XmlDeserialize;

use crate::{PwshCoreError, runspace::win_rs::WinRunspace, runspace_pool::PsInvocationState};

use super::{
    enums::RunspacePoolState,
    types::{PipelineRepresentation, PowerShell},
};

const PROTOCOL_VERSION: &str = "2.3";
const PS_VERSION: &str = "2.0";
const SERIALIZATION_VERSION: &str = "1.1.0.1";

pub enum AcceptResponsResult {
    ReceiveResponse,
    NewPipeline(PowerShell),
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
    pub(super) pipelines: HashSet<PipelineRepresentation>,
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
    pub(crate) fn fire_receive(&mut self) -> Result<String, crate::PwshCoreError> {
        Ok(self
            .shell
            .fire_receive(
                &self.connection,
                None, // No specific stream
                None, // No command ID
            )
            .into()
            .to_string())
    }

    #[instrument(skip(self, soap_envelope))]
    pub(crate) fn accept_response<'a>(
        &mut self,
        soap_envelope: String,
    ) -> Result<AcceptResponsResult, crate::PwshCoreError> {
        let parsed = xml::parser::parse(soap_envelope.as_str())?;
        let soap_envelope = SoapEnvelope::from_node(parsed.root_element())
            .map_err(crate::PwshCoreError::XmlParsingError)?;

        if soap_envelope.body.as_ref().receive_response.is_some() {
            let streams = self.shell.accept_receive_response(&soap_envelope)?;
            self.parse_responses(streams)?;
            return Ok(AcceptResponsResult::ReceiveResponse);
        }

        if soap_envelope.body.as_ref().command_response.is_some() {
            let pipeline_id = self.shell.accept_commannd_response(soap_envelope)?;
            self.pipelines
                .remove(&PipelineRepresentation::new(pipeline_id));

            self.pipelines.insert(PipelineRepresentation {
                id: pipeline_id,
                state: PsInvocationState::Running,
            });

            return Ok(AcceptResponsResult::NewPipeline(PowerShell {
                id: pipeline_id,
            }));
        }


        error!(
            "Unimplemented handler for soap envelope body: {:?}",
            soap_envelope.body
        );

        Err(crate::PwshCoreError::InvalidResponse(
            format!(
                "Unimplemented handler for soap envelope body: {:?}",
                soap_envelope.body
            )
            .into(),
        ))
    }

    #[instrument(skip(self))]
    pub(crate) fn fire_create_pipeline(&mut self) -> Result<String, crate::PwshCoreError> {
        if self.state != RunspacePoolState::Opened {
            return Err(crate::PwshCoreError::InvalidState(
                "RunspacePool must be in Opened state to create a pipeline",
            ));
        }

        let pipeline_id = uuid::Uuid::new_v4();
        self.pipelines.insert(PipelineRepresentation {
            id: pipeline_id,
            state: PsInvocationState::NotStarted,
        });

        let pipeline_message = PowerShellPipeline::builder()
            .is_nested(false)
            .redirect_shell_error_output_pipe(true)
            .cmds(vec![])
            .build();
        debug!(?pipeline_message);

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
            &pipeline_id,
            arguments,
            None,
            None,
        )?;

        Ok(request.into().to_string())
    }

    fn parse_responses(&mut self, responses: Vec<Vec<u8>>) -> Result<(), crate::PwshCoreError> {
        for response in responses {
            let messages = match self.defragmenter.defragment(&response)? {
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

        Ok(())
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
}
