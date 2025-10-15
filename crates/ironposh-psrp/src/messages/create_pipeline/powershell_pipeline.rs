use super::command::Command;
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, Container, PsPrimitiveValue, PsProperty, PsType, PsValue,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct PowerShellPipeline {
    #[builder(default = false)]
    pub is_nested: bool,
    #[builder(setter(into))]
    pub cmds: Vec<Command>,
    #[builder(default)]
    pub history: String,
    #[builder(default = false)]
    pub redirect_shell_error_output_pipe: bool,
}

impl From<PowerShellPipeline> for ComplexObject {
    fn from(pipeline: PowerShellPipeline) -> Self {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "IsNested".to_string(),
            PsProperty {
                name: "IsNested".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Bool(pipeline.is_nested)),
            },
        );

        // Commands as ArrayList
        let cmds: Vec<PsValue> = pipeline
            .cmds
            .into_iter()
            .map(|cmd| PsValue::Object(Self::from(cmd)))
            .collect();

        let cmds_obj = Self {
            type_def: Some(PsType::array_list()),
            to_string: None,
            content: ComplexObjectContent::Container(Container::List(cmds)),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        };

        extended_properties.insert(
            "Cmds".to_string(),
            PsProperty {
                name: "Cmds".to_string(),
                value: PsValue::Object(cmds_obj),
            },
        );

        extended_properties.insert(
            "History".to_string(),
            PsProperty {
                name: "History".to_string(),
                value: if pipeline.history.is_empty() {
                    PsValue::Primitive(PsPrimitiveValue::Nil)
                } else {
                    PsValue::Primitive(PsPrimitiveValue::Str(pipeline.history))
                },
            },
        );

        extended_properties.insert(
            "RedirectShellErrorOutputPipe".to_string(),
            PsProperty {
                name: "RedirectShellErrorOutputPipe".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Bool(
                    pipeline.redirect_shell_error_output_pipe,
                )),
            },
        );

        Self {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl TryFrom<ComplexObject> for PowerShellPipeline {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let get_property = |name: &str| -> Result<&PsProperty, Self::Error> {
            value
                .extended_properties
                .get(name)
                .ok_or_else(|| Self::Error::InvalidMessage(format!("Missing property: {name}")))
        };

        let is_nested = match &get_property("IsNested")?.value {
            PsValue::Primitive(PsPrimitiveValue::Bool(b)) => *b,
            _ => {
                return Err(Self::Error::InvalidMessage(
                    "IsNested must be a bool".to_string(),
                ));
            }
        };

        let cmds = match &get_property("Cmds")?.value {
            PsValue::Object(obj) => match &obj.content {
                ComplexObjectContent::Container(Container::List(list)) => {
                    let mut commands = Vec::new();
                    for item in list {
                        if let PsValue::Object(cmd_obj) = item {
                            commands.push(Command::try_from(cmd_obj.clone())?);
                        }
                    }
                    commands
                }
                _ => {
                    return Err(Self::Error::InvalidMessage(
                        "Cmds must be a list".to_string(),
                    ));
                }
            },
            PsValue::Primitive(_) => {
                return Err(Self::Error::InvalidMessage(
                    "Cmds must be an object".to_string(),
                ));
            }
        };

        let history = value
            .extended_properties
            .get("History")
            .map_or_else(String::new, |prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::Str(s)) => s.clone(),
                _ => String::new(),
            });

        let redirect_shell_error_output_pipe =
            match &get_property("RedirectShellErrorOutputPipe")?.value {
                PsValue::Primitive(PsPrimitiveValue::Bool(b)) => *b,
                _ => false,
            };

        Ok(Self {
            is_nested,
            cmds,
            history,
            redirect_shell_error_output_pipe,
        })
    }
}
