use crate::ps_value::{ComplexObject, ComplexObjectContent, PsEnums, PsType};
use std::{borrow::Cow, collections::BTreeMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteStreamOptions {
    None = 0,
    AddInvocationInfo = 1,
}

impl From<RemoteStreamOptions> for ComplexObject {
    fn from(options: RemoteStreamOptions) -> Self {
        let type_def = PsType {
            type_names: vec![
                Cow::Borrowed("System.Management.Automation.RemoteStreamOptions"),
                Cow::Borrowed("System.Enum"),
                Cow::Borrowed("System.ValueType"),
                Cow::Borrowed("System.Object"),
            ],
        };

        let to_string = match options {
            RemoteStreamOptions::None => "None".to_string(),
            RemoteStreamOptions::AddInvocationInfo => "AddInvocationInfo".to_string(),
        };

        ComplexObject {
            type_def: Some(type_def),
            to_string: Some(to_string),
            content: ComplexObjectContent::PsEnums(PsEnums {
                value: options as i32,
            }),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        }
    }
}

impl TryFrom<ComplexObject> for RemoteStreamOptions {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        match value.content {
            ComplexObjectContent::PsEnums(PsEnums { value }) => match value {
                0 => Ok(RemoteStreamOptions::None),
                1 => Ok(RemoteStreamOptions::AddInvocationInfo),
                _ => Err(Self::Error::InvalidMessage(format!(
                    "Invalid RemoteStreamOptions value: {value}"
                ))),
            },
            _ => Err(Self::Error::InvalidMessage(
                "RemoteStreamOptions must be an enum".to_string(),
            )),
        }
    }
}
