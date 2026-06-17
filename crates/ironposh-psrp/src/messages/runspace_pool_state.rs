use crate::ps_value::PsValue;
use ironposh_macros::{PsDeserialize, PsEnum, PsSerialize};

#[derive(Debug, Clone, PartialEq, Eq, PsEnum)]
#[ps(repr = "i32")]
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
            Self::BeforeOpen => 0,
            Self::Opening => 1,
            Self::Opened => 2,
            Self::Closed => 3,
            Self::Closing => 4,
            Self::Broken => 5,
            Self::NegotiationSent => 6,
            Self::NegotiationSucceeded => 7,
            Self::Connecting => 8,
            Self::Disconnected => 9,
        }
    }
}

impl TryFrom<i32> for RunspacePoolStateValue {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::BeforeOpen),
            1 => Ok(Self::Opening),
            2 => Ok(Self::Opened),
            3 => Ok(Self::Closed),
            4 => Ok(Self::Closing),
            5 => Ok(Self::Broken),
            6 => Ok(Self::NegotiationSent),
            7 => Ok(Self::NegotiationSucceeded),
            8 => Ok(Self::Connecting),
            9 => Ok(Self::Disconnected),
            _ => Err(crate::PowerShellRemotingError::InvalidMessage(format!(
                "Invalid RunspacePoolState value: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder, PsSerialize, PsDeserialize)]
#[ps(message_type = RunspacepoolState)]
pub struct RunspacePoolStateMessage {
    #[ps(name = "RunspaceState")]
    pub runspace_state: RunspacePoolStateValue,
    #[builder(default)]
    #[ps(name = "ExceptionAsErrorRecord")]
    pub exception_as_error_record: Option<PsValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ps_value::{ComplexObject, PsObjectWithType, PsPrimitiveValue};

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
            .exception_as_error_record(Some(exception))
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
