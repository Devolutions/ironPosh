use std::{collections::HashMap, sync::Arc};

use base64::Engine;
use protocol_powershell_remoting::{
    ApartmentState, ApplicationPrivateData, Defragmenter, HostInfo, InitRunspacePool,
    PSThreadOptions, PsValue, RunspacePoolStateMessage, RunspacePoolStateValue, SessionCapability,
    fragment,
};
use protocol_winrm::{
    soap::SoapEnvelope,
    ws_management::{OptionSetValue, WsMan},
};
use tracing::{debug, info, instrument, trace};
use xml::parser::XmlDeserialize;

use crate::{PwshCoreError, runspace::win_rs::WinRunspace};

const PROTOCOL_VERSION: &str = "2.3";
const PS_VERSION: &str = "2.0";
const SERIALIZATION_VERSION: &str = "1.1.0.1";
const DEFAULT_CONFIGURATION_NAME: &str = "Microsoft.PowerShell";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PsInvocationState {
    NotStarted = 0,
    Running = 1,
    Stopping = 2,
    Stopped = 3,
    Completed = 4,
    Failed = 5,
    Disconnected = 6,
}

#[derive(Debug, Clone)]
pub struct PowerShell {
    // Ok, think about it
}

/// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/b05495bc-a9b2-4794-9f43-4bf1f3633900
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RunspacePoolState {
    BeforeOpen = 0,
    Opening = 1,
    Opened = 2,
    Closed = 3,
    Closing = 4,
    Broken = 5,
    NegotiationSent = 6,
    NegotiationSucceeded = 7,
    Connecting = 8,
    Disconnected = 9,
}

impl From<&RunspacePoolStateValue> for RunspacePoolState {
    fn from(value: &RunspacePoolStateValue) -> Self {
        match value {
            RunspacePoolStateValue::BeforeOpen => RunspacePoolState::BeforeOpen,
            RunspacePoolStateValue::Opening => RunspacePoolState::Opening,
            RunspacePoolStateValue::Opened => RunspacePoolState::Opened,
            RunspacePoolStateValue::Closed => RunspacePoolState::Closed,
            RunspacePoolStateValue::Closing => RunspacePoolState::Closing,
            RunspacePoolStateValue::Broken => RunspacePoolState::Broken,
            RunspacePoolStateValue::NegotiationSent => RunspacePoolState::NegotiationSent,
            RunspacePoolStateValue::NegotiationSucceeded => RunspacePoolState::NegotiationSucceeded,
            RunspacePoolStateValue::Connecting => RunspacePoolState::Connecting,
            RunspacePoolStateValue::Disconnected => RunspacePoolState::Disconnected,
        }
    }
}

// pub struct Pipeline
pub struct Runspace {
    pub id: uuid::Uuid,
    pub state: RunspacePoolState,
}

#[derive(Debug, typed_builder::TypedBuilder)]
pub struct RunspacePool {
    #[builder(default = uuid::Uuid::new_v4())]
    id: uuid::Uuid,
    #[builder(default = RunspacePoolState::BeforeOpen)]
    pub(crate) state: RunspacePoolState,

    #[builder(default = 1)]
    min_runspaces: usize,
    #[builder(default = 1)]
    max_runspaces: usize,

    #[builder(default = PSThreadOptions::Default)]
    thread_options: PSThreadOptions,

    #[builder(default = ApartmentState::Unknown)]
    apartment_state: ApartmentState,

    host_info: HostInfo,

    #[builder(default = std::collections::BTreeMap::new())]
    application_arguments: std::collections::BTreeMap<PsValue, PsValue>,

    connection: Arc<WsMan>,

    #[builder(default, setter(strip_option))]
    shell: Option<WinRunspace>,

    #[builder(default)]
    pipeline: HashMap<String, PowerShell>,

    #[builder(default = Defragmenter::new())]
    defragmenter: Defragmenter,

    #[builder(default)]
    application_private_data: Option<ApplicationPrivateData>,

    #[builder(default)]
    session_capability: Option<SessionCapability>,
}

impl RunspacePool {
    #[instrument(skip(self), name = "RunspacePool::open")]
    pub fn open(mut self) -> Result<(String, ExpectShellCreated), crate::PwshCoreError> {
        if self.state != RunspacePoolState::BeforeOpen {
            return Err(crate::PwshCoreError::InvalidState(
                "RunspacePool must be in BeforeOpen state to open",
            ));
        }

        self.shell = Some(
            WinRunspace::builder()
                .ws_man(Arc::clone(&self.connection))
                .id(self.id)
                .build(),
        );

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

        let mut fragmenter =
            fragment::Fragmenter::new(self.connection.max_envelope_size() as usize);

        let request_groups = fragmenter.fragment_multiple(
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
            .as_ref()
            .expect("Shell must be initialized")
            .open(Some(option_set), &request);

        Ok((
            result.into().to_string(),
            ExpectShellCreated {
                runspace_pool: self,
            },
        ))
    }

    fn safe_shell(&mut self) -> Result<&mut WinRunspace, crate::PwshCoreError> {
        self.shell
            .as_mut()
            .ok_or(crate::PwshCoreError::InvalidState(
                "Shell must be initialized before using",
            ))
    }

    // We should accept the pipeline id here, but for now let's ignore it
    pub(crate) fn fire_receive(&mut self) -> Result<String, crate::PwshCoreError> {
        Ok(self
            .safe_shell()?
            .fire_receive(
                None, // No specific stream
                None, // No command ID
            )
            .into()
            .to_string())
    }

    #[instrument(skip(self, soap_envelope))]
    pub(crate) fn accept_receive_response<'a>(
        &mut self,
        soap_envelope: String,
    ) -> Result<(), crate::PwshCoreError> {
        let parsed = xml::parser::parse(soap_envelope.as_str())?;
        let soap_envelope = SoapEnvelope::from_node(parsed.root_element())
            .map_err(crate::PwshCoreError::XmlParsingError)?;

        let streams = self.safe_shell()?.accept_receive_response(&soap_envelope)?;

        self.parse_responses(streams)?;

        Ok(())
    }

    pub(crate) fn parse_responses(
        &mut self,
        responses: Vec<Vec<u8>>,
    ) -> Result<(), crate::PwshCoreError> {
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
                "Expected SessionCapability as PsValue::Object",
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
                "Expected ApplicationPrivateData as PsValue::Object",
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
                "Expected RunspacepoolState as PsValue::Object",
            ));
        };

        let runspacepool_state = RunspacePoolStateMessage::try_from(runspacepool_state)?;
        trace!(?runspacepool_state, "Received RunspacePoolState");

        self.state = RunspacePoolState::from(&runspacepool_state.runspace_state);

        Ok(())
    }
}

#[derive(Debug)]
pub struct ExpectShellCreated {
    runspace_pool: RunspacePool,
}

impl ExpectShellCreated {
    pub fn accept(self, response: String) -> Result<RunspacePool, crate::PwshCoreError> {
        let ExpectShellCreated { mut runspace_pool } = self;

        let parsed = xml::parser::parse(response.as_str())?;

        let soap_response = SoapEnvelope::from_node(parsed.root_element())
            .map_err(crate::PwshCoreError::XmlParsingError)?;

        runspace_pool
            .shell
            .as_mut()
            .ok_or({
                crate::PwshCoreError::InvalidState("Shell must be initialized to parse response")
            })?
            .accept_create_response(soap_response)?;

        Ok(runspace_pool)
    }
}
