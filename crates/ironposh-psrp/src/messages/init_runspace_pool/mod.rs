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
    ComplexObject, ComplexObjectContent, Properties, PsObjectWithType, PsPrimitiveValue, PsValue,
};

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
        let mut properties = Properties::new();

        properties.insert_extended(
            "MinRunspaces",
            PsValue::Primitive(PsPrimitiveValue::I32(init.min_runspaces)),
        );

        properties.insert_extended(
            "MaxRunspaces",
            PsValue::Primitive(PsPrimitiveValue::I32(init.max_runspaces)),
        );

        properties.insert_extended(
            "PSThreadOptions",
            PsValue::Object(init.thread_options.into()),
        );

        properties.insert_extended(
            "ApartmentState",
            PsValue::Object(init.apartment_state.into()),
        );

        properties.insert_extended("HostInfo", PsValue::Object(init.host_info.clone().into()));

        if init.application_arguments.is_empty() {
            properties.insert_extended(
                "ApplicationArguments",
                PsValue::Primitive(PsPrimitiveValue::Nil),
            );
        } else {
            properties.insert_extended(
                "ApplicationArguments",
                PsValue::Object(init.application_arguments.into()),
            );
        }

        Self {
            content: ComplexObjectContent::Standard,
            properties,
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
