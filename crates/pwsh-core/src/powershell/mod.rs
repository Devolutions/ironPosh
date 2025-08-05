use std::collections::HashMap;

use base64::Engine;
use protocol_powershell_remoting::{
    ApartmentState, Fragment, HostInfo, InitRunspacePool, PSThreadOptions,
    PowerShellRemotingMessage, PsValue, SessionCapability, fragment,
};
use protocol_winrm::ws_management::{OptionSetValue, WsMan};

use crate::runspace::win_rs::WinRunspace;

const PROTOCOL_VERSION: &'static str = "2.3";
const PS_VERSION: &'static str = "2.0";
const SERIALIZATION_VERSION: &'static str = "1.1.0.1";
const DEFAULT_CONFIGURATION_NAME: &'static str = "Microsoft.PowerShell";

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

// pub struct Pipeline
pub struct Runspace {
    pub id: uuid::Uuid,
    pub state: RunspacePoolState,
}

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct RunspacePool<'a> {
    #[builder(default = uuid::Uuid::new_v4())]
    id: uuid::Uuid,
    #[builder(default = RunspacePoolState::BeforeOpen)]
    state: RunspacePoolState,

    #[builder(default = 1)]
    min_runspaces: usize,
    #[builder(default = 1)]
    max_runspaces: usize,

    #[builder(default = PSThreadOptions::Default)]
    thread_options: PSThreadOptions,

    #[builder(default = ApartmentState::Unknown)]
    apartment_state: ApartmentState,

    #[builder(default)]
    host_info: Option<HostInfo>,

    #[builder(default = HashMap::new())]
    application_arguments: HashMap<PsValue, PsValue>,

    connection: WsMan,

    shell: Option<WinRunspace<'a>>,
}

impl<'a> RunspacePool<'a> {
    pub fn open(&'a mut self) -> Result<String, crate::PwshCoreError> {
        if self.state != RunspacePoolState::BeforeOpen {
            return Err(crate::PwshCoreError::InvalidState(
                "RunspacePool must be in BeforeOpen state to open",
            ));
        }

        self.shell = Some(
            WinRunspace::builder()
                .ws_man(&self.connection)
                .id(self.id)
                .build(),
        );

        let session_capability = SessionCapability {
            protocol_version: PROTOCOL_VERSION.to_string(),
            ps_version: PS_VERSION.to_string(),
            serialization_version: SERIALIZATION_VERSION.to_string(),
            time_zone: "UTC".to_string(), // Default to UTC, can be customized
        };

        let init_runspace_pool = InitRunspacePool {
            min_runspaces: self.min_runspaces as i32,
            max_runspaces: self.max_runspaces as i32,
            thread_options: self.thread_options,
            apartment_state: self.apartment_state,
            host_info: self.host_info.clone(),
            application_arguments: self.application_arguments.clone(),
        };

        let mut fragmenter =
            fragment::Fragmenter::new(self.connection.max_envelope_size() as usize);

        let request_groups = fragmenter.fragment_multiple(&[
            PowerShellRemotingMessage::from_ps_message(
                session_capability,
                self.id,
                None, // No PID for this message
            ),
            PowerShellRemotingMessage::from_ps_message(
                init_runspace_pool,
                self.id,
                None, // No PID for this message
            ),
        ]);

        self.state = RunspacePoolState::NegotiationSent;

        if request_groups.len() > 1 {
            // we should definately handle this case better! use panic here for simplicity
            panic!("Multiple request groups generated, expected single group for negotiation");
        }

        let request =
            request_groups
                .into_iter()
                .next()
                .ok_or(crate::PwshCoreError::InvalidState(
                    "No request group generated for negotiation",
                ))?;

        let open_content = Fragment::encode_multiple(&request)?;

        let option_set = OptionSetValue::new().add_option("protocol_version", PROTOCOL_VERSION);

        let result = self
            .shell
            .as_ref()
            .expect("Shell must be initialized")
            .open(Some(option_set), &open_content);

        Ok(result.into().to_string())
    }
}
