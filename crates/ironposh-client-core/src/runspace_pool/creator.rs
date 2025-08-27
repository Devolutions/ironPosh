use std::{collections::HashMap, sync::Arc};

use ironposh_psrp::{
    ApartmentState, ApplicationPrivateData, Defragmenter, Fragmenter, HostInfo, PSThreadOptions,
    PsValue, SessionCapability,
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

    #[builder(default = std::collections::BTreeMap::new())]
    application_arguments: std::collections::BTreeMap<PsValue, PsValue>,

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
        let shell = WinRunspace::builder().id(self.id).build();

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
            runspace_pool_desired_stream_is_pooling: false,
        }
    }
}
