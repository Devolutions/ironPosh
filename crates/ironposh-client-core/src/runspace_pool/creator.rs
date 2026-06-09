use std::{collections::HashMap, sync::Arc};

use ironposh_psrp::{
    ApartmentState, ApplicationArguments, ApplicationPrivateData, Defragmenter, Fragmenter,
    HostInfo, PSThreadOptions, SessionCapability,
};
use ironposh_winrm::ws_management::WsMan;

use crate::{pipeline::Pipeline, runspace::win_rs::WinRunspace};

use super::{enums::RunspacePoolState, pool::RunspacePool};

#[derive(Debug, typed_builder::TypedBuilder)]
pub struct RunspacePoolCreator {
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

    #[builder(default)]
    application_arguments: ApplicationArguments,

    #[builder(default = Defragmenter::new())]
    defragmenter: Defragmenter,

    #[builder(default)]
    application_private_data: Option<ApplicationPrivateData>,

    #[builder(default)]
    session_capability: Option<SessionCapability>,

    #[builder(default)]
    pipelines: HashMap<uuid::Uuid, Pipeline>,
}

impl RunspacePoolCreator {
    pub fn into_runspace_pool(self, connection: Arc<WsMan>) -> RunspacePool {
        let shell = WinRunspace::builder()
            .id(self.id)
            .resource_uri(connection.resource_uri().to_owned())
            .build();

        self.into_runspace_pool_with_shell(connection, shell)
    }

    /// Build a pool around an EXISTING disconnected shell for WSMan Connect:
    /// the shell id and ShellId selector are seeded upfront (creator `id` ==
    /// shell id == pool RPID) instead of being learned from a CreateResponse.
    pub fn into_connect_runspace_pool(self, connection: Arc<WsMan>) -> RunspacePool {
        let shell_id = self.id.to_string().to_uppercase();
        let shell = WinRunspace::builder()
            .id(self.id)
            .resource_uri(connection.resource_uri().to_owned())
            .shell_id(Some(shell_id.clone()))
            .selector_set(
                ironposh_winrm::ws_management::SelectorSetValue::new()
                    .add_selector("ShellId", shell_id),
            )
            .build();

        self.into_runspace_pool_with_shell(connection, shell)
    }

    fn into_runspace_pool_with_shell(
        self,
        connection: Arc<WsMan>,
        shell: WinRunspace,
    ) -> RunspacePool {
        RunspacePool {
            id: self.id,
            state: self.state,
            min_runspaces: self.min_runspaces,
            max_runspaces: self.max_runspaces,
            thread_options: self.thread_options,
            apartment_state: self.apartment_state,
            host_info: self.host_info,
            application_arguments: self.application_arguments,
            fragmenter: Fragmenter::new(connection.max_envelope_size() as usize),
            connection,
            shell,
            defragmenter: self.defragmenter,
            application_private_data: self.application_private_data,
            session_capability: self.session_capability,
            pipelines: self.pipelines,
            desired_stream_is_pooling: false,
            key_exchange: None,
            psrp_key_exchange_pending: false,
            pending_host_calls: std::collections::VecDeque::new(),
        }
    }
}
