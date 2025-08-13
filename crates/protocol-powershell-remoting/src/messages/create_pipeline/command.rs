use super::super::{
    ComplexObject, ComplexObjectContent, Container, PsPrimitiveValue, PsProperty, PsType, PsValue,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct Command {
    pub cmd: String,
    #[builder(default = false)]
    pub is_script: bool,
    #[builder(default)]
    pub args: Vec<PsValue>,
    #[builder(default)]
    pub use_local_scope: Option<bool>,
}

impl From<Command> for ComplexObject {
    fn from(command: Command) -> Self {
        let mut extended_properties = BTreeMap::new();

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

        // Args as ArrayList
        let args_obj = ComplexObject {
            type_def: Some(PsType::array_list()),
            to_string: None,
            content: ComplexObjectContent::Container(Container::List(command.args)),
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

        if let Some(use_local_scope) = command.use_local_scope {
            extended_properties.insert(
                "UseLocalScope".to_string(),
                PsProperty {
                    name: "UseLocalScope".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Bool(use_local_scope)),
                },
            );
        } else {
            extended_properties.insert(
                "UseLocalScope".to_string(),
                PsProperty {
                    name: "UseLocalScope".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Nil),
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

impl TryFrom<ComplexObject> for Command {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let get_property = |name: &str| -> Result<&PsProperty, Self::Error> {
            value.extended_properties.get(name).ok_or_else(|| {
                Self::Error::InvalidMessage(format!("Missing property: {}", name))
            })
        };

        let cmd = match &get_property("Cmd")?.value {
            PsValue::Primitive(PsPrimitiveValue::Str(s)) => s.clone(),
            _ => return Err(Self::Error::InvalidMessage("Cmd must be a string".to_string())),
        };

        let is_script = match &get_property("IsScript")?.value {
            PsValue::Primitive(PsPrimitiveValue::Bool(b)) => *b,
            _ => return Err(Self::Error::InvalidMessage("IsScript must be a bool".to_string())),
        };

        let args = match &get_property("Args")?.value {
            PsValue::Object(obj) => match &obj.content {
                ComplexObjectContent::Container(Container::List(list)) => list.clone(),
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

        Ok(Command {
            cmd,
            is_script,
            args,
            use_local_scope,
        })
    }
}