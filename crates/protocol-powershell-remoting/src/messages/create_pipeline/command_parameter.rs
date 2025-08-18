use super::super::{ComplexObject, ComplexObjectContent, PsPrimitiveValue, PsProperty, PsValue};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct CommandParameter {
    #[builder(default, setter(into, strip_option))]
    pub name: Option<String>,
    pub value: PsValue,
}

impl From<CommandParameter> for ComplexObject {
    fn from(param: CommandParameter) -> Self {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "N".to_string(),
            PsProperty {
                name: "N".to_string(),
                value: match param.name {
                    Some(name) => PsValue::Primitive(PsPrimitiveValue::Str(name)),
                    None => PsValue::Primitive(PsPrimitiveValue::Nil),
                },
            },
        );

        extended_properties.insert(
            "V".to_string(),
            PsProperty {
                name: "V".to_string(),
                value: param.value,
            },
        );

        ComplexObject {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl TryFrom<ComplexObject> for CommandParameter {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let get_property = |name: &str| -> Result<&PsProperty, Self::Error> {
            value
                .extended_properties
                .get(name)
                .ok_or_else(|| Self::Error::InvalidMessage(format!("Missing property: {}", name)))
        };

        let name = match &get_property("N")?.value {
            PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
            PsValue::Primitive(PsPrimitiveValue::Nil) => None,
            _ => None,
        };

        let value = get_property("V")?.value.clone();

        Ok(CommandParameter { name, value })
    }
}
