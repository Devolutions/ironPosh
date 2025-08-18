use super::{
    ComplexObject, ComplexObjectContent, PsObjectWithType, PsPrimitiveValue, PsProperty, PsValue,
};
use crate::MessageType;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunspacePoolStateValue {
    BeforeOpen = 0,
    Opening = 1,
    Opened = 2,
    Closed = 3,
    Closing = 4,
    Broken = 5,
    NegotiationSent = 6,
    NegotiationSucceeded = 7,
    Connecting = 8,
    Disconnected = 9,
}

impl RunspacePoolStateValue {
    pub fn as_i32(&self) -> i32 {
        match self {
            RunspacePoolStateValue::BeforeOpen => 0,
            RunspacePoolStateValue::Opening => 1,
            RunspacePoolStateValue::Opened => 2,
            RunspacePoolStateValue::Closed => 3,
            RunspacePoolStateValue::Closing => 4,
            RunspacePoolStateValue::Broken => 5,
            RunspacePoolStateValue::NegotiationSent => 6,
            RunspacePoolStateValue::NegotiationSucceeded => 7,
            RunspacePoolStateValue::Connecting => 8,
            RunspacePoolStateValue::Disconnected => 9,
        }
    }
}

impl TryFrom<i32> for RunspacePoolStateValue {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(RunspacePoolStateValue::BeforeOpen),
            1 => Ok(RunspacePoolStateValue::Opening),
            2 => Ok(RunspacePoolStateValue::Opened),
            3 => Ok(RunspacePoolStateValue::Closed),
            4 => Ok(RunspacePoolStateValue::Closing),
            5 => Ok(RunspacePoolStateValue::Broken),
            6 => Ok(RunspacePoolStateValue::NegotiationSent),
            7 => Ok(RunspacePoolStateValue::NegotiationSucceeded),
            8 => Ok(RunspacePoolStateValue::Connecting),
            9 => Ok(RunspacePoolStateValue::Disconnected),
            _ => Err(crate::PowerShellRemotingError::InvalidMessage(format!(
                "Invalid RunspacePoolState value: {}",
                value
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct RunspacePoolStateMessage {
    pub runspace_state: RunspacePoolStateValue,
    #[builder(default)]
    pub exception_as_error_record: Option<PsValue>,
}

impl PsObjectWithType for RunspacePoolStateMessage {
    fn message_type(&self) -> MessageType {
        MessageType::RunspacepoolState
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}

impl From<RunspacePoolStateMessage> for ComplexObject {
    fn from(state: RunspacePoolStateMessage) -> Self {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "RunspaceState".to_string(),
            PsProperty {
                name: "RunspaceState".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I32(state.runspace_state.as_i32())),
            },
        );

        if let Some(exception) = state.exception_as_error_record {
            extended_properties.insert(
                "ExceptionAsErrorRecord".to_string(),
                PsProperty {
                    name: "ExceptionAsErrorRecord".to_string(),
                    value: exception,
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

impl TryFrom<ComplexObject> for RunspacePoolStateMessage {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let runspace_state_prop =
            value
                .extended_properties
                .get("RunspaceState")
                .ok_or_else(|| {
                    Self::Error::InvalidMessage("Missing RunspaceState property".to_string())
                })?;

        let runspace_state = match &runspace_state_prop.value {
            PsValue::Primitive(PsPrimitiveValue::I32(state)) => {
                RunspacePoolStateValue::try_from(*state)?
            }
            _ => {
                return Err(Self::Error::InvalidMessage(
                    "RunspaceState property is not an I32".to_string(),
                ));
            }
        };

        let exception_as_error_record = value
            .extended_properties
            .get("ExceptionAsErrorRecord")
            .map(|prop| prop.value.clone());

        Ok(RunspacePoolStateMessage {
            runspace_state,
            exception_as_error_record,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runspace_pool_state_opened() {
        let state = RunspacePoolStateMessage::builder()
            .runspace_state(RunspacePoolStateValue::Opened)
            .build();

        let complex_obj = ComplexObject::from(state.clone());
        let roundtrip = RunspacePoolStateMessage::try_from(complex_obj).unwrap();

        assert_eq!(state, roundtrip);
    }

    #[test]
    fn test_runspace_pool_state_broken_with_exception() {
        let exception = PsValue::Primitive(PsPrimitiveValue::Str("Test error".to_string()));
        let state = RunspacePoolStateMessage::builder()
            .runspace_state(RunspacePoolStateValue::Broken)
            .exception_as_error_record(Some(exception.clone()))
            .build();

        let complex_obj = ComplexObject::from(state.clone());
        let roundtrip = RunspacePoolStateMessage::try_from(complex_obj).unwrap();

        assert_eq!(state, roundtrip);
    }

    #[test]
    fn test_message_type() {
        let state = RunspacePoolStateMessage::builder()
            .runspace_state(RunspacePoolStateValue::Opened)
            .build();

        assert_eq!(state.message_type().value(), 0x00021005);
    }
}
