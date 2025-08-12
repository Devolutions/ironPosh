use super::super::{
    ComplexObject, ComplexObjectContent, Container, PsObjectWithType, PsPrimitiveValue, PsProperty,
    PsType, PsValue,
};
use crate::MessageType;
use std::collections::BTreeMap;

/// ApplicationPrivateData is a specific message type within the PowerShell Remoting Protocol (PSRP)
/// that facilitates the exchange of private application-level data between a server and a client.
///
/// MessageType value: 0x00021009
/// Direction: Server to Client
/// Target: RunspacePool
///
/// The data contains an extended property named "ApplicationPrivateData" with a value that is
/// either a Primitive Dictionary or a Null Value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationPrivateData {
    /// The application private data as a dictionary of string keys to primitive values
    pub data: Option<BTreeMap<String, PsPrimitiveValue>>,
}

impl ApplicationPrivateData {
    /// Create a new ApplicationPrivateData with no data (null value)
    pub fn new() -> Self {
        Self { data: None }
    }

    /// Create a new ApplicationPrivateData with the provided dictionary
    pub fn with_data(data: BTreeMap<String, PsPrimitiveValue>) -> Self {
        Self { data: Some(data) }
    }
}

impl Default for ApplicationPrivateData {
    fn default() -> Self {
        Self::new()
    }
}

impl PsObjectWithType for ApplicationPrivateData {
    fn message_type(&self) -> MessageType {
        MessageType::ApplicationPrivateData
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}

impl From<ApplicationPrivateData> for ComplexObject {
    fn from(app_data: ApplicationPrivateData) -> Self {
        let mut extended_properties = BTreeMap::new();

        let application_private_data_value = match app_data.data {
            Some(data) => {
                // Convert BTreeMap<String, PsPrimitiveValue> to BTreeMap<PsValue, PsValue>
                let ps_dict: BTreeMap<PsValue, PsValue> = data
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            PsValue::Primitive(PsPrimitiveValue::Str(k)),
                            PsValue::Primitive(v),
                        )
                    })
                    .collect();

                PsValue::Object(ComplexObject {
                    type_def: Some(PsType::ps_primitive_dictionary()),
                    to_string: None,
                    content: ComplexObjectContent::Container(Container::Dictionary(ps_dict)),
                    adapted_properties: BTreeMap::new(),
                    extended_properties: BTreeMap::new(),
                })
            }
            None => PsValue::Primitive(PsPrimitiveValue::Nil),
        };

        extended_properties.insert(
            "ApplicationPrivateData".to_string(),
            PsProperty {
                name: "ApplicationPrivateData".to_string(),
                value: application_private_data_value,
            },
        );

        ComplexObject {
            type_def: Some(PsType::ps_primitive_dictionary()),
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl TryFrom<ComplexObject> for ApplicationPrivateData {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let app_data_property = value
            .extended_properties
            .get("ApplicationPrivateData")
            .ok_or_else(|| {
                Self::Error::InvalidMessage("Missing ApplicationPrivateData property".to_string())
            })?;

        let data = if let PsValue::Primitive(PsPrimitiveValue::Nil) = &app_data_property.value {
            None
        } else {
            let PsValue::Object(obj) = &app_data_property.value else {
                return Err(Self::Error::InvalidMessage(
                    "ApplicationPrivateData property has invalid type".to_string(),
                ));
            };

            let ComplexObjectContent::Container(Container::Dictionary(dict)) = &obj.content else {
                return Err(Self::Error::InvalidMessage(
                    "ApplicationPrivateData is not a dictionary".to_string(),
                ));
            };

            let mut result = BTreeMap::new();
            for (key, value) in dict {
                let PsValue::Primitive(PsPrimitiveValue::Str(key_str)) = key else {
                    return Err(Self::Error::InvalidMessage(
                        "Dictionary key is not a string".to_string(),
                    ));
                };

                let PsValue::Primitive(value_primitive) = value else {
                    return Err(Self::Error::InvalidMessage(
                        "Dictionary value is not a primitive".to_string(),
                    ));
                };

                result.insert(key_str.clone(), value_primitive.clone());
            }

            Some(result)
        };

        Ok(ApplicationPrivateData { data })
    }
}
