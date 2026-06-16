use crate::ps_value::{ComplexObject, ComplexObjectContent, Properties, PsPrimitiveValue, PsValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandParameter {
    name: Option<String>,
    value: PsValue,
}

impl CommandParameter {
    pub fn named(name: String, value: impl Into<PsValue>) -> Self {
        Self {
            name: Some(name),
            value: value.into(),
        }
    }

    pub fn positional(value: impl Into<PsValue>) -> Self {
        Self {
            name: None,
            value: value.into(),
        }
    }
}

impl From<CommandParameter> for ComplexObject {
    fn from(param: CommandParameter) -> Self {
        let mut properties = Properties::new();

        properties.insert_extended(
            "N",
            param
                .name
                .map_or(PsValue::Primitive(PsPrimitiveValue::Nil), |name| {
                    PsValue::Primitive(PsPrimitiveValue::Str(name))
                }),
        );

        properties.insert_extended("V", param.value);

        Self {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            properties,
        }
    }
}

impl TryFrom<ComplexObject> for CommandParameter {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let get_property = |name: &str| -> Result<&PsValue, Self::Error> {
            value
                .properties
                .get(name)
                .ok_or_else(|| Self::Error::InvalidMessage(format!("Missing property: {name}")))
        };

        let name = if let PsValue::Primitive(PsPrimitiveValue::Str(s)) = get_property("N")? {
            Some(s.clone())
        } else {
            None
        };

        let value = get_property("V")?.clone();

        Ok(Self { name, value })
    }
}
