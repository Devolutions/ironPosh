use super::{CommandParameter, PipelineResultTypes};
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, Container, PsPrimitiveValue, PsProperty, PsType, PsValue,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct Command {
    #[builder(setter(into))]
    pub cmd: String,
    #[builder(default = false)]
    pub is_script: bool,
    #[builder(default)]
    pub args: Vec<CommandParameter>,
    #[builder(default)]
    pub use_local_scope: Option<bool>,
    #[builder(default)]
    pub merge_my_result: PipelineResultTypes,
    #[builder(default)]
    pub merge_to_result: PipelineResultTypes,
    #[builder(default)]
    pub merge_previous_results: PipelineResultTypes,
    #[builder(default)]
    pub merge_debug: PipelineResultTypes,
    #[builder(default)]
    pub merge_error: PipelineResultTypes,
    #[builder(default)]
    pub merge_information: PipelineResultTypes,
    #[builder(default)]
    pub merge_verbose: PipelineResultTypes,
    #[builder(default)]
    pub merge_warning: PipelineResultTypes,
}

impl From<Command> for ComplexObject {
    fn from(command: Command) -> Self {
        let mut extended_properties = BTreeMap::new();

        let cmd_str = command.cmd.clone();

        extended_properties.insert(
            "Cmd".to_string(),
            PsProperty {
                name: "Cmd".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Str(command.cmd)),
            },
        );

        extended_properties.insert(
            "IsScript".to_string(),
            PsProperty {
                name: "IsScript".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Bool(command.is_script)),
            },
        );

        // Args as ArrayList of CommandParameter objects
        let args_values: Vec<PsValue> = command
            .args
            .into_iter()
            .map(|param| PsValue::Object(param.into()))
            .collect();

        let args_obj = ComplexObject {
            type_def: Some(PsType::array_list()),
            to_string: cmd_str.clone().into(),
            content: ComplexObjectContent::Container(Container::List(args_values)),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        };

        extended_properties.insert(
            "Args".to_string(),
            PsProperty {
                name: "Args".to_string(),
                value: PsValue::Object(args_obj),
            },
        );

        extended_properties.insert(
            "UseLocalScope".to_string(),
            PsProperty {
                name: "UseLocalScope".to_string(),
                value: match command.use_local_scope {
                    Some(use_local_scope) => {
                        PsValue::Primitive(PsPrimitiveValue::Bool(use_local_scope))
                    }
                    None => PsValue::Primitive(PsPrimitiveValue::Nil),
                },
            },
        );

        extended_properties.insert(
            "MergeMyResult".to_string(),
            PsProperty {
                name: "MergeMyResult".to_string(),
                value: PsValue::Object(command.merge_my_result.into()),
            },
        );

        extended_properties.insert(
            "MergeToResult".to_string(),
            PsProperty {
                name: "MergeToResult".to_string(),
                value: PsValue::Object(command.merge_to_result.into()),
            },
        );

        extended_properties.insert(
            "MergePreviousResults".to_string(),
            PsProperty {
                name: "MergePreviousResults".to_string(),
                value: PsValue::Object(command.merge_previous_results.into()),
            },
        );

        extended_properties.insert(
            "MergeDebug".to_string(),
            PsProperty {
                name: "MergeDebug".to_string(),
                value: PsValue::Object(command.merge_debug.into()),
            },
        );

        extended_properties.insert(
            "MergeError".to_string(),
            PsProperty {
                name: "MergeError".to_string(),
                value: PsValue::Object(command.merge_error.into()),
            },
        );

        extended_properties.insert(
            "MergeInformation".to_string(),
            PsProperty {
                name: "MergeInformation".to_string(),
                value: PsValue::Object(command.merge_information.into()),
            },
        );

        extended_properties.insert(
            "MergeVerbose".to_string(),
            PsProperty {
                name: "MergeVerbose".to_string(),
                value: PsValue::Object(command.merge_verbose.into()),
            },
        );

        extended_properties.insert(
            "MergeWarning".to_string(),
            PsProperty {
                name: "MergeWarning".to_string(),
                value: PsValue::Object(command.merge_warning.into()),
            },
        );

        ComplexObject {
            type_def: None,
            to_string: Some(cmd_str),
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl TryFrom<ComplexObject> for Command {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let get_property = |name: &str| -> Result<&PsProperty, Self::Error> {
            value
                .extended_properties
                .get(name)
                .ok_or_else(|| Self::Error::InvalidMessage(format!("Missing property: {name}")))
        };

        let cmd = match &get_property("Cmd")?.value {
            PsValue::Primitive(PsPrimitiveValue::Str(s)) => s.clone(),
            _ => {
                return Err(Self::Error::InvalidMessage(
                    "Cmd must be a string".to_string(),
                ));
            }
        };

        let is_script = match &get_property("IsScript")?.value {
            PsValue::Primitive(PsPrimitiveValue::Bool(b)) => *b,
            _ => {
                return Err(Self::Error::InvalidMessage(
                    "IsScript must be a bool".to_string(),
                ));
            }
        };

        let args = match &get_property("Args")?.value {
            PsValue::Object(obj) => match &obj.content {
                ComplexObjectContent::Container(Container::List(list)) => {
                    let mut command_params = Vec::new();
                    for item in list {
                        if let PsValue::Object(param_obj) = item {
                            match CommandParameter::try_from(param_obj.clone()) {
                                Ok(param) => command_params.push(param),
                                Err(_) => continue,
                            }
                        }
                    }
                    command_params
                }
                _ => vec![],
            },
            _ => vec![],
        };

        let use_local_scope = match value.extended_properties.get("UseLocalScope") {
            Some(prop) => match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::Bool(b)) => Some(*b),
                PsValue::Primitive(PsPrimitiveValue::Nil) => None,
                _ => None,
            },
            None => None,
        };

        let get_merge_property = |name: &str| -> PipelineResultTypes {
            match value.extended_properties.get(name) {
                Some(prop) => match &prop.value {
                    PsValue::Object(obj) => {
                        PipelineResultTypes::try_from(obj.clone()).unwrap_or_default()
                    }
                    _ => PipelineResultTypes::default(),
                },
                None => PipelineResultTypes::default(),
            }
        };

        let merge_my_result = get_merge_property("MergeMyResult");
        let merge_to_result = get_merge_property("MergeToResult");
        let merge_previous_results = get_merge_property("MergePreviousResults");
        let merge_debug = get_merge_property("MergeDebug");
        let merge_error = get_merge_property("MergeError");
        let merge_information = get_merge_property("MergeInformation");
        let merge_verbose = get_merge_property("MergeVerbose");
        let merge_warning = get_merge_property("MergeWarning");

        Ok(Command {
            cmd,
            is_script,
            args,
            use_local_scope,
            merge_my_result,
            merge_to_result,
            merge_previous_results,
            merge_debug,
            merge_error,
            merge_information,
            merge_verbose,
            merge_warning,
        })
    }
}
