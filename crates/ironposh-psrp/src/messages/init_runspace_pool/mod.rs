pub mod apartment_state;
pub mod application_arguments;
pub mod application_private_data;
pub mod host_default_data;
pub mod host_info;
pub mod ps_thread_options;

pub use apartment_state::ApartmentState;
pub use application_arguments::{ApplicationArguments, PSVersionTable};
pub use application_private_data::ApplicationPrivateData;
pub use host_default_data::{Coordinates, HostDefaultData, Size};
pub use host_info::HostInfo;
pub use ps_thread_options::PSThreadOptions;

use crate::MessageType;
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, PsObjectWithType, PsPrimitiveValue, PsProperty, PsValue,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitRunspacePool {
    pub min_runspaces: i32,
    pub max_runspaces: i32,
    pub thread_options: PSThreadOptions,
    pub apartment_state: ApartmentState,
    pub host_info: HostInfo,
    pub application_arguments: ApplicationArguments,
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
            extended_properties.insert(
                "ApplicationArguments".to_string(),
                PsProperty {
                    name: "ApplicationArguments".to_string(),
                    value: PsValue::Object(init.application_arguments.into()),
                },
            );
        }

        Self {
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
