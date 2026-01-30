use crate::MessageType;
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, PsObjectWithType, PsPrimitiveValue, PsProperty, PsValue,
};
use std::collections::BTreeMap;

/// Client → Server public key used for PSRP session key exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey {
    /// Base64-encoded public key blob as defined by MS-PSRP.
    pub public_key: String,
}

impl PsObjectWithType for PublicKey {
    fn message_type(&self) -> MessageType {
        MessageType::PublicKey
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}

impl From<PublicKey> for ComplexObject {
    fn from(value: PublicKey) -> Self {
        let mut extended_properties = BTreeMap::new();
        extended_properties.insert(
            "PublicKey".to_string(),
            PsProperty {
                name: "PublicKey".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Str(value.public_key)),
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

impl TryFrom<ComplexObject> for PublicKey {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let prop = value.extended_properties.get("PublicKey").ok_or_else(|| {
            Self::Error::InvalidMessage("Missing property: PublicKey".to_string())
        })?;

        let public_key = match &prop.value {
            PsValue::Primitive(PsPrimitiveValue::Str(s)) => s.clone(),
            other => {
                return Err(Self::Error::InvalidMessage(format!(
                    "PublicKey must be a string, got {other:?}"
                )));
            }
        };

        Ok(Self { public_key })
    }
}
