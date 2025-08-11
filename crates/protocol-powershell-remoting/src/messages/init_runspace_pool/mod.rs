pub mod apartment_state;
pub mod host_default_data;
pub mod host_info;
pub mod ps_thread_options;

pub use apartment_state::ApartmentState;
pub use host_default_data::{Coordinates, HostDefaultData, Size};
pub use host_info::HostInfo;
pub use ps_thread_options::PSThreadOptions;

use super::{
    ComplexObject, ComplexObjectContent, Container, PsObjectWithType, PsPrimitiveValue, PsProperty,
    PsType, PsValue,
};
use crate::MessageType;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitRunspacePool {
    pub ref_id: u32,
    pub min_runspaces: i32,
    pub max_runspaces: i32,
    pub thread_options: PSThreadOptions,
    pub apartment_state: ApartmentState,
    pub host_info: HostInfo,
    pub application_arguments: BTreeMap<PsValue, PsValue>,
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

        extended_properties.insert(
            "HostInfo".to_string(),
            PsProperty {
                name: "HostInfo".to_string(),
                value: PsValue::Object(init.host_info.clone().into()),
            },
        );

        if init.application_arguments.is_empty() {
            extended_properties.insert(
                "ApplicationArguments".to_string(),
                PsProperty {
                    name: "ApplicationArguments".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Nil),
                },
            );
        } else {
            let app_args_obj = ComplexObject {
                type_def: Some(PsType::ps_primitive_dictionary()),
                content: ComplexObjectContent::Container(Container::Dictionary(
                    init.application_arguments,
                )),
                ..Default::default()
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
            content: ComplexObjectContent::Standard,
            extended_properties,
            ..Default::default()
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
