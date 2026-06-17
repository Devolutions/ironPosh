use super::{CommandParameter, PipelineResultTypes};
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, Container, Properties, PsPrimitiveValue, PsType, PsValue,
};

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
        let mut properties = Properties::new();

        let cmd_str = command.cmd.clone();

        properties.insert_extended(
            "Cmd",
            PsValue::Primitive(PsPrimitiveValue::Str(command.cmd)),
        );

        properties.insert_extended(
            "IsScript",
            PsValue::Primitive(PsPrimitiveValue::Bool(command.is_script)),
        );

        // Args as ArrayList of CommandParameter objects
        let args_values: Vec<PsValue> = command
            .args
            .into_iter()
            .map(|param| PsValue::Object(param.into()))
            .collect();

        let args_obj = Self {
            type_def: Some(PsType::array_list()),
            to_string: cmd_str.clone().into(),
            content: ComplexObjectContent::Container(Container::List(args_values)),
            properties: Properties::new(),
        };

        properties.insert_extended("Args", PsValue::Object(args_obj));

        properties.insert_extended(
            "UseLocalScope",
            command.use_local_scope.map_or(
                PsValue::Primitive(PsPrimitiveValue::Nil),
                |use_local_scope| PsValue::Primitive(PsPrimitiveValue::Bool(use_local_scope)),
            ),
        );

        properties.insert_extended(
            "MergeMyResult",
            PsValue::Object(command.merge_my_result.into()),
        );

        properties.insert_extended(
            "MergeToResult",
            PsValue::Object(command.merge_to_result.into()),
        );

        properties.insert_extended(
            "MergePreviousResults",
            PsValue::Object(command.merge_previous_results.into()),
        );

        properties.insert_extended("MergeDebug", PsValue::Object(command.merge_debug.into()));

        properties.insert_extended("MergeError", PsValue::Object(command.merge_error.into()));

        properties.insert_extended(
            "MergeInformation",
            PsValue::Object(command.merge_information.into()),
        );

        properties.insert_extended(
            "MergeVerbose",
            PsValue::Object(command.merge_verbose.into()),
        );

        properties.insert_extended(
            "MergeWarning",
            PsValue::Object(command.merge_warning.into()),
        );

        Self {
            type_def: None,
            to_string: Some(cmd_str),
            content: ComplexObjectContent::Standard,
            properties,
        }
    }
}

impl TryFrom<ComplexObject> for Command {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let get_property = |name: &str| -> Result<&PsValue, Self::Error> {
            value
                .properties
                .get(name)
                .ok_or_else(|| Self::Error::InvalidMessage(format!("Missing property: {name}")))
        };

        let cmd = match get_property("Cmd")? {
            PsValue::Primitive(PsPrimitiveValue::Str(s)) => s.clone(),
            _ => {
                return Err(Self::Error::InvalidMessage(
                    "Cmd must be a string".to_string(),
                ));
            }
        };

        let is_script = match get_property("IsScript")? {
            PsValue::Primitive(PsPrimitiveValue::Bool(b)) => *b,
            _ => {
                return Err(Self::Error::InvalidMessage(
                    "IsScript must be a bool".to_string(),
                ));
            }
        };

        let args = match get_property("Args")? {
            PsValue::Object(obj) => match &obj.content {
                ComplexObjectContent::Container(Container::List(list)) => {
                    let mut command_params = Vec::new();
                    for item in list {
                        if let PsValue::Object(param_obj) = item
                            && let Ok(param) = CommandParameter::try_from(param_obj.clone())
                        {
                            command_params.push(param);
                        }
                    }
                    command_params
                }
                _ => vec![],
            },
            PsValue::Primitive(_) => vec![],
        };

        let use_local_scope = if let Some(value) = value.properties.get("UseLocalScope")
            && let PsValue::Primitive(PsPrimitiveValue::Bool(b)) = value
        {
            Some(*b)
        } else {
            None
        };

        let get_merge_property = |name: &str| -> PipelineResultTypes {
            value.properties.get(name).map_or_else(
                PipelineResultTypes::default,
                |value| match value {
                    PsValue::Object(obj) => {
                        PipelineResultTypes::from_ps_object(obj.clone()).unwrap_or_default()
                    }
                    PsValue::Primitive(_) => PipelineResultTypes::default(),
                },
            )
        };

        let merge_my_result = get_merge_property("MergeMyResult");
        let merge_to_result = get_merge_property("MergeToResult");
        let merge_previous_results = get_merge_property("MergePreviousResults");
        let merge_debug = get_merge_property("MergeDebug");
        let merge_error = get_merge_property("MergeError");
        let merge_information = get_merge_property("MergeInformation");
        let merge_verbose = get_merge_property("MergeVerbose");
        let merge_warning = get_merge_property("MergeWarning");

        Ok(Self {
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
