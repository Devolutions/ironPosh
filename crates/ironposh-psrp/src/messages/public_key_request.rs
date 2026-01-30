use crate::MessageType;
use crate::ps_value::{PsObjectWithType, PsPrimitiveValue, PsValue};

/// Server → Client request asking for the client's PSRP session key exchange public key.
///
/// Per MS-PSRP, the `Data` field is equivalent to serializing an empty String: `<S></S>`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PublicKeyRequest;

impl PsObjectWithType for PublicKeyRequest {
    fn message_type(&self) -> MessageType {
        MessageType::PublicKeyRequest
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Primitive(PsPrimitiveValue::Str(String::new()))
    }
}

impl TryFrom<PsValue> for PublicKeyRequest {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: PsValue) -> Result<Self, Self::Error> {
        match value {
            PsValue::Primitive(PsPrimitiveValue::Str(s)) if s.is_empty() => Ok(Self),
            PsValue::Primitive(PsPrimitiveValue::Nil) => Ok(Self),
            other => Err(crate::PowerShellRemotingError::InvalidMessage(format!(
                "Invalid PublicKeyRequest payload: expected empty string, got {other:?}"
            ))),
        }
    }
}
