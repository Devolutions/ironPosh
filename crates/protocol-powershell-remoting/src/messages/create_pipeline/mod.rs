mod command;
mod command_parameter;
mod pipeline_result_types;
mod powershell_pipeline;
mod remote_stream_options;
#[cfg(test)]
mod test;

pub use command::{Command, Commands};
pub use command_parameter::CommandParameter;
pub use pipeline_result_types::PipelineResultTypes;
pub use powershell_pipeline::PowerShellPipeline;
pub use remote_stream_options::RemoteStreamOptions;

use super::init_runspace_pool::{ApartmentState, HostInfo};
use crate::MessageType;
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, PsEnums, PsObjectWithType, PsPrimitiveValue, PsProperty,
    PsType, PsValue,
};
use std::{borrow::Cow, collections::BTreeMap};

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
    pub power_shell: PowerShellPipeline,
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
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "NoInput".to_string(),
            PsProperty {
                name: "NoInput".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Bool(create_pipeline.no_input)),
            },
        );

        extended_properties.insert(
            "ApartmentState".to_string(),
            PsProperty {
                name: "ApartmentState".to_string(),
                value: PsValue::Object(ComplexObject::from(create_pipeline.apartment_state)),
            },
        );

        extended_properties.insert(
            "RemoteStreamOptions".to_string(),
            PsProperty {
                name: "RemoteStreamOptions".to_string(),
                value: PsValue::Object(ComplexObject::from(create_pipeline.remote_stream_options)),
            },
        );

        extended_properties.insert(
            "AddToHistory".to_string(),
            PsProperty {
                name: "AddToHistory".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Bool(create_pipeline.add_to_history)),
            },
        );

        extended_properties.insert(
            "HostInfo".to_string(),
            PsProperty {
                name: "HostInfo".to_string(),
                value: PsValue::Object(ComplexObject::from(create_pipeline.host_info)),
            },
        );

        extended_properties.insert(
            "PowerShell".to_string(),
            PsProperty {
                name: "PowerShell".to_string(),
                value: PsValue::Object(ComplexObject::from(create_pipeline.power_shell)),
            },
        );

        extended_properties.insert(
            "IsNested".to_string(),
            PsProperty {
                name: "IsNested".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Bool(create_pipeline.is_nested)),
            },
        );

        ComplexObject {
            type_def: Some(PsType {
                type_names: vec![Cow::Borrowed("System.Object")],
            }),
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl TryFrom<ComplexObject> for CreatePipeline {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let get_property = |name: &str| -> Result<&PsProperty, Self::Error> {
            value
                .extended_properties
                .get(name)
                .ok_or_else(|| Self::Error::InvalidMessage(format!("Missing property: {name}")))
        };

        let no_input = match &get_property("NoInput")?.value {
            PsValue::Primitive(PsPrimitiveValue::Bool(b)) => *b,
            _ => true,
        };

        let apartment_state = match &get_property("ApartmentState")?.value {
            PsValue::Object(obj) => match &obj.content {
                ComplexObjectContent::PsEnums(PsEnums { value }) => match *value {
                    0 => ApartmentState::STA,
                    1 => ApartmentState::MTA,
                    2 => ApartmentState::Unknown,
                    _ => ApartmentState::Unknown,
                },
                _ => ApartmentState::Unknown,
            },
            _ => ApartmentState::Unknown,
        };

        let remote_stream_options = match &get_property("RemoteStreamOptions")?.value {
            PsValue::Object(obj) => RemoteStreamOptions::try_from(obj.clone())?,
            _ => RemoteStreamOptions::None,
        };

        let add_to_history = match &get_property("AddToHistory")?.value {
            PsValue::Primitive(PsPrimitiveValue::Bool(b)) => *b,
            _ => false,
        };

        let host_info = match &get_property("HostInfo")?.value {
            PsValue::Object(obj) => HostInfo::try_from(obj.clone())
                .map_err(|_| Self::Error::InvalidMessage("Failed to parse HostInfo".to_string()))?,
            _ => {
                return Err(Self::Error::InvalidMessage(
                    "HostInfo must be an object".to_string(),
                ));
            }
        };

        let power_shell = match &get_property("PowerShell")?.value {
            PsValue::Object(obj) => PowerShellPipeline::try_from(obj.clone())?,
            _ => {
                return Err(Self::Error::InvalidMessage(
                    "PowerShell must be an object".to_string(),
                ));
            }
        };

        let is_nested = match &get_property("IsNested")?.value {
            PsValue::Primitive(PsPrimitiveValue::Bool(b)) => *b,
            _ => false,
        };

        Ok(CreatePipeline {
            no_input,
            apartment_state,
            remote_stream_options,
            add_to_history,
            host_info,
            power_shell,
            is_nested,
        })
    }
}

impl CreatePipeline {
    pub fn simple_command(command: &str) -> Self {
        let cmd = Command::builder().cmd(command.to_string()).build();

        let pipeline = PowerShellPipeline::builder()
            .cmds(Commands::new(cmd))
            .build();

        let host_info = HostInfo::builder().build();

        CreatePipeline::builder()
            .host_info(host_info)
            .power_shell(pipeline)
            .build()
    }

    pub fn script_command(script: &str) -> Self {
        let cmd = Command::builder()
            .cmd(script.to_string())
            .is_script(true)
            .build();

        let pipeline = PowerShellPipeline::builder()
            .cmds(Commands::new(cmd))
            .build();

        let host_info = HostInfo::builder().build();

        CreatePipeline::builder()
            .host_info(host_info)
            .power_shell(pipeline)
            .build()
    }
}
