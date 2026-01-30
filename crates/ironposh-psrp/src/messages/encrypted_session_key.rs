use crate::MessageType;
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, PsObjectWithType, PsPrimitiveValue, PsProperty, PsValue,
};
use std::collections::BTreeMap;

/// Server → Client encrypted session key for PSRP session key exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedSessionKey {
    /// Base64-encoded encrypted session key blob as defined by MS-PSRP.
    pub encrypted_session_key: String,
}

impl PsObjectWithType for EncryptedSessionKey {
    fn message_type(&self) -> MessageType {
        MessageType::EncryptedSessionKey
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}

impl From<EncryptedSessionKey> for ComplexObject {
    fn from(value: EncryptedSessionKey) -> Self {
        let mut extended_properties = BTreeMap::new();
        extended_properties.insert(
            "EncryptedSessionKey".to_string(),
            PsProperty {
                name: "EncryptedSessionKey".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Str(value.encrypted_session_key)),
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

impl TryFrom<ComplexObject> for EncryptedSessionKey {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let prop = value
            .extended_properties
            .get("EncryptedSessionKey")
            .ok_or_else(|| {
                Self::Error::InvalidMessage("Missing property: EncryptedSessionKey".to_string())
            })?;

        let encrypted_session_key = match &prop.value {
            PsValue::Primitive(PsPrimitiveValue::Str(s)) => s.clone(),
            other => {
                return Err(Self::Error::InvalidMessage(format!(
                    "EncryptedSessionKey must be a string, got {other:?}"
                )));
            }
        };

        Ok(Self {
            encrypted_session_key,
        })
    }
}
