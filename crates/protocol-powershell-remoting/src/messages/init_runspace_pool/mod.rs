pub mod apartment_state;
pub mod host_default_data;
pub mod host_info;
pub mod ps_thread_options;

pub use apartment_state::ApartmentState;
pub use host_default_data::{Coordinates, HostDefaultData, Size};
pub use host_info::HostInfo;
pub use ps_thread_options::PSThreadOptions;

use crate::MessageType;
use super::{PsObjectWithType, PsValue, PsProperty, ComplexObject, ComplexObjectContent, PsType, PsPrimitiveValue, Container};
use std::{borrow::Cow, collections::{HashMap, BTreeMap}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitRunspacePool {
    pub ref_id: u32,
    pub min_runspaces: i32,
    pub max_runspaces: i32,
    pub thread_options: PSThreadOptions,
    pub apartment_state: ApartmentState,
    pub host_info: Option<HostInfo>,
    pub application_arguments: BTreeMap<PsValue, PsValue>,
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
            application_arguments: BTreeMap::new(),
        }
    }
}

impl From<InitRunspacePool> for ComplexObject {
    fn from(init: InitRunspacePool) -> Self {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "MinRunspaces".to_string(),
            PsProperty {
                name: "MinRunspaces".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I32(init.min_runspaces)),
            },
        );

        extended_properties.insert(
            "MaxRunspaces".to_string(),
            PsProperty {
                name: "MaxRunspaces".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I32(init.max_runspaces)),
            },
        );

        extended_properties.insert(
            "PSThreadOptions".to_string(),
            PsProperty {
                name: "PSThreadOptions".to_string(),
                value: PsValue::Object(init.thread_options.into()),
            },
        );

        extended_properties.insert(
            "ApartmentState".to_string(),
            PsProperty {
                name: "ApartmentState".to_string(),
                value: PsValue::Object(init.apartment_state.into()),
            },
        );

        if let Some(host_info) = init.host_info.as_ref() {
            extended_properties.insert(
                "HostInfo".to_string(),
                PsProperty {
                    name: "HostInfo".to_string(),
                    value: PsValue::Object(host_info.clone().into()),
                },
            );
        }

        if init.application_arguments.is_empty() {
            extended_properties.insert(
                "ApplicationArguments".to_string(),
                PsProperty {
                    name: "ApplicationArguments".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Nil),
                },
            );
        } else {
            let app_args_type = PsType {
                type_names: vec![
                    Cow::Borrowed("System.Management.Automation.PSPrimitiveDictionary"),
                    Cow::Borrowed("System.Collections.Hashtable"),
                    Cow::Borrowed("System.Object"),
                ],
            };
            
            let app_args_obj = ComplexObject {
                type_def: Some(app_args_type),
                to_string: None,
                content: ComplexObjectContent::Container(Container::Dictionary(init.application_arguments)),
                adapted_properties: BTreeMap::new(),
                extended_properties: BTreeMap::new(),
            };
            
            extended_properties.insert(
                "ApplicationArguments".to_string(),
                PsProperty {
                    name: "ApplicationArguments".to_string(),
                    value: PsValue::Object(app_args_obj),
                },
            );
        }

        ComplexObject {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl PsObjectWithType for InitRunspacePool {
    fn message_type(&self) -> MessageType {
        MessageType::InitRunspacepool
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}
