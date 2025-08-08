pub mod apartment_state;
pub mod host_default_data;
pub mod host_info;
pub mod ps_thread_options;

pub use apartment_state::ApartmentState;
pub use host_default_data::{Coordinates, HostDefaultData, Size};
pub use host_info::HostInfo;
pub use ps_thread_options::PSThreadOptions;

use crate::{MessageType, PsObject, PsObjectWithType, PsProperty, PsValue};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitRunspacePool {
    pub ref_id: u32,
    pub min_runspaces: i32,
    pub max_runspaces: i32,
    pub thread_options: PSThreadOptions,
    pub apartment_state: ApartmentState,
    pub host_info: Option<HostInfo>,
    pub application_arguments: HashMap<PsValue, PsValue>,
}

impl Default for InitRunspacePool {
    fn default() -> Self {
        Self {
            ref_id: 0,
            min_runspaces: 1,
            max_runspaces: 1,
            thread_options: PSThreadOptions::Default,
            apartment_state: ApartmentState::MTA,
            host_info: None,
            application_arguments: HashMap::new(),
        }
    }
}

impl From<InitRunspacePool> for PsObject {
    fn from(init: InitRunspacePool) -> Self {
        let mut ms = Vec::new();

        ms.push(PsProperty {
            name: Some("MinRunspaces".to_string()),
            ref_id: None,
            value: PsValue::I32(init.min_runspaces),
        });

        ms.push(PsProperty {
            name: Some("MaxRunspaces".to_string()),
            ref_id: None,
            value: PsValue::I32(init.max_runspaces),
        });

        ms.push(PsProperty {
            name: Some("PSThreadOptions".to_string()),
            ref_id: None,
            value: PsValue::Object(init.thread_options.into()),
        });

        ms.push(PsProperty {
            name: Some("ApartmentState".to_string()),
            ref_id: None,
            value: PsValue::Object(init.apartment_state.into()),
        });

        if let Some(host_info) = init.host_info.as_ref() {
            ms.push(PsProperty {
                name: Some("HostInfo".to_string()),
                ref_id: None,
                value: PsValue::Object(host_info.clone().into()),
            });
        }

        if init.application_arguments.is_empty() {
            ms.push(PsProperty {
                name: Some("ApplicationArguments".to_string()),
                ref_id: None,
                value: PsValue::Nil,
            });
        } else {
            let app_args_obj = PsObject {
                type_names: Some(vec![
                    "System.Management.Automation.PSPrimitiveDictionary".to_string(),
                    "System.Collections.Hashtable".to_string(),
                    "System.Object".to_string(),
                ]),
                dct: init.application_arguments,
                ..Default::default()
            };
            ms.push(PsProperty {
                name: Some("ApplicationArguments".to_string()),
                ref_id: None,
                value: PsValue::Object(app_args_obj),
            });
        }

        PsObject {
            ms,
            ref_id: init.ref_id,
            ..Default::default()
        }
    }
}

impl PsObjectWithType for InitRunspacePool {
    fn message_type(&self) -> MessageType {
        MessageType::InitRunspacepool
    }

    fn to_ps_object(&self) -> PsObject {
        PsObject::from(self.clone())
    }
}
