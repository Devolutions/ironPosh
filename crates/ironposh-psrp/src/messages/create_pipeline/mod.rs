mod command;
mod command_parameter;
mod pipeline_result_types;
mod powershell_pipeline;
mod remote_stream_options;
#[cfg(test)]
mod test;

pub use command::Command;
pub use command_parameter::CommandParameter;
pub use pipeline_result_types::PipelineResultTypes;
pub use powershell_pipeline::PowerShellPipeline;
pub use remote_stream_options::RemoteStreamOptions;

use super::init_runspace_pool::{ApartmentState, HostInfo};
use crate::MessageType;
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, Properties, PsEnums, PsObjectWithType, PsPrimitiveValue,
    PsType, PsValue,
};
use std::borrow::Cow;
use std::vec;

#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct CreatePipeline {
    #[builder(default = true)]
    pub no_input: bool,
    #[builder(default = ApartmentState::Unknown)]
    pub apartment_state: ApartmentState,
    #[builder(default = RemoteStreamOptions::None)]
    pub remote_stream_options: RemoteStreamOptions,
    #[builder(default = false)]
    pub add_to_history: bool,
    pub host_info: HostInfo,
    pub pipeline: PowerShellPipeline,
    #[builder(default = false)]
    pub is_nested: bool,
}

impl PsObjectWithType for CreatePipeline {
    fn message_type(&self) -> MessageType {
        MessageType::CreatePipeline
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}

impl From<CreatePipeline> for ComplexObject {
    fn from(create_pipeline: CreatePipeline) -> Self {
        let mut properties = Properties::new();

        properties.insert_extended(
            "NoInput",
            PsValue::Primitive(PsPrimitiveValue::Bool(create_pipeline.no_input)),
        );

        properties.insert_extended(
            "ApartmentState",
            PsValue::Object(Self::from(create_pipeline.apartment_state)),
        );

        properties.insert_extended(
            "RemoteStreamOptions",
            PsValue::Object(Self::from(create_pipeline.remote_stream_options)),
        );

        properties.insert_extended(
            "AddToHistory",
            PsValue::Primitive(PsPrimitiveValue::Bool(create_pipeline.add_to_history)),
        );

        properties.insert_extended(
            "HostInfo",
            PsValue::Object(Self::from(create_pipeline.host_info)),
        );

        properties.insert_extended(
            "PowerShell",
            PsValue::Object(Self::from(create_pipeline.pipeline)),
        );

        properties.insert_extended(
            "IsNested",
            PsValue::Primitive(PsPrimitiveValue::Bool(create_pipeline.is_nested)),
        );

        Self {
            type_def: Some(PsType {
                type_names: vec![Cow::Borrowed("System.Object")],
            }),
            to_string: None,
            content: ComplexObjectContent::Standard,
            properties,
        }
    }
}

impl TryFrom<ComplexObject> for CreatePipeline {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let get_property = |name: &str| -> Result<&PsValue, Self::Error> {
            value
                .properties
                .get(name)
                .ok_or_else(|| Self::Error::InvalidMessage(format!("Missing property: {name}")))
        };

        let no_input = match get_property("NoInput")? {
            PsValue::Primitive(PsPrimitiveValue::Bool(b)) => *b,
            _ => true,
        };

        let apartment_state = match get_property("ApartmentState")? {
            PsValue::Object(obj) => match &obj.content {
                ComplexObjectContent::PsEnums(PsEnums { value }) => match *value {
                    0 => ApartmentState::STA,
                    1 => ApartmentState::MTA,
                    _ => ApartmentState::Unknown, // 2 is also Unknown
                },
                _ => ApartmentState::Unknown,
            },
            PsValue::Primitive(_) => ApartmentState::Unknown,
        };

        let remote_stream_options = match get_property("RemoteStreamOptions")? {
            PsValue::Object(obj) => RemoteStreamOptions::from_ps_object(obj.clone())?,
            PsValue::Primitive(_) => RemoteStreamOptions::None,
        };

        let add_to_history = match get_property("AddToHistory")? {
            PsValue::Primitive(PsPrimitiveValue::Bool(b)) => *b,
            _ => false,
        };

        let host_info = match get_property("HostInfo")? {
            PsValue::Object(obj) => HostInfo::try_from(obj.clone())
                .map_err(|_| Self::Error::InvalidMessage("Failed to parse HostInfo".to_string()))?,
            PsValue::Primitive(_) => {
                return Err(Self::Error::InvalidMessage(
                    "HostInfo must be an object".to_string(),
                ));
            }
        };

        let power_shell = match get_property("PowerShell")? {
            PsValue::Object(obj) => PowerShellPipeline::try_from(obj.clone())?,
            PsValue::Primitive(_) => {
                return Err(Self::Error::InvalidMessage(
                    "PowerShell must be an object".to_string(),
                ));
            }
        };

        let is_nested = match get_property("IsNested")? {
            PsValue::Primitive(PsPrimitiveValue::Bool(b)) => *b,
            _ => false,
        };

        Ok(Self {
            no_input,
            apartment_state,
            remote_stream_options,
            add_to_history,
            host_info,
            pipeline: power_shell,
            is_nested,
        })
    }
}
