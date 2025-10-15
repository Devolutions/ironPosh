use crate::MessageType;
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, PsObjectWithType, PsPrimitiveValue, PsProperty, PsValue,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PSInvocationState {
    NotStarted = 0,
    Running = 1,
    Stopping = 2,
    Stopped = 3,
    Completed = 4,
    Failed = 5,
    Disconnected = 6,
}

impl PSInvocationState {
    pub fn as_i32(&self) -> i32 {
        match self {
            Self::NotStarted => 0,
            Self::Running => 1,
            Self::Stopping => 2,
            Self::Stopped => 3,
            Self::Completed => 4,
            Self::Failed => 5,
            Self::Disconnected => 6,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Stopped
        )
    }
}

impl TryFrom<i32> for PSInvocationState {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::NotStarted),
            1 => Ok(Self::Running),
            2 => Ok(Self::Stopping),
            3 => Ok(Self::Stopped),
            4 => Ok(Self::Completed),
            5 => Ok(Self::Failed),
            6 => Ok(Self::Disconnected),
            _ => Err(crate::PowerShellRemotingError::InvalidMessage(format!(
                "Invalid PSInvocationState value: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct PipelineStateMessage {
    pub pipeline_state: PSInvocationState,
    #[builder(default)]
    pub exception_as_error_record: Option<PsValue>,
}

impl PsObjectWithType for PipelineStateMessage {
    fn message_type(&self) -> MessageType {
        MessageType::PipelineState
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}

impl From<PipelineStateMessage> for ComplexObject {
    fn from(state: PipelineStateMessage) -> Self {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "PipelineState".to_string(),
            PsProperty {
                name: "PipelineState".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I32(state.pipeline_state.as_i32())),
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

        Self {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl TryFrom<ComplexObject> for PipelineStateMessage {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let pipeline_state_prop =
            value
                .extended_properties
                .get("PipelineState")
                .ok_or_else(|| {
                    Self::Error::InvalidMessage("Missing PipelineState property".to_string())
                })?;

        let pipeline_state = match &pipeline_state_prop.value {
            PsValue::Primitive(PsPrimitiveValue::I32(state)) => {
                PSInvocationState::try_from(*state)?
            }
            _ => {
                return Err(Self::Error::InvalidMessage(
                    "PipelineState property is not an I32".to_string(),
                ));
            }
        };

        let exception_as_error_record = value
            .extended_properties
            .get("ExceptionAsErrorRecord")
            .map(|prop| prop.value.clone());

        Ok(Self {
            pipeline_state,
            exception_as_error_record,
        })
    }
}

impl PipelineStateMessage {
    pub fn completed() -> Self {
        Self::builder()
            .pipeline_state(PSInvocationState::Completed)
            .build()
    }

    pub fn failed_with_error(error_record: PsValue) -> Self {
        Self::builder()
            .pipeline_state(PSInvocationState::Failed)
            .exception_as_error_record(Some(error_record))
            .build()
    }

    pub fn stopped_with_error(error_record: PsValue) -> Self {
        Self::builder()
            .pipeline_state(PSInvocationState::Stopped)
            .exception_as_error_record(Some(error_record))
            .build()
    }

    pub fn running() -> Self {
        Self::builder()
            .pipeline_state(PSInvocationState::Running)
            .build()
    }

    pub fn is_terminal(&self) -> bool {
        self.pipeline_state.is_terminal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_state_completed() {
        let state = PipelineStateMessage::completed();

        let complex_obj = ComplexObject::from(state.clone());
        let roundtrip = PipelineStateMessage::try_from(complex_obj).unwrap();

        assert_eq!(state, roundtrip);
        assert!(state.is_terminal());
    }

    #[test]
    fn test_pipeline_state_failed_with_error() {
        let error_record = PsValue::Primitive(PsPrimitiveValue::Str("Test error".to_string()));
        let state = PipelineStateMessage::failed_with_error(error_record.clone());

        let complex_obj = ComplexObject::from(state.clone());
        let roundtrip = PipelineStateMessage::try_from(complex_obj).unwrap();

        assert_eq!(state, roundtrip);
        assert!(state.is_terminal());
        assert_eq!(state.pipeline_state, PSInvocationState::Failed);
        assert_eq!(state.exception_as_error_record, Some(error_record));
    }

    #[test]
    fn test_pipeline_state_running() {
        let state = PipelineStateMessage::running();

        let complex_obj = ComplexObject::from(state.clone());
        let roundtrip = PipelineStateMessage::try_from(complex_obj).unwrap();

        assert_eq!(state, roundtrip);
        assert!(!state.is_terminal());
        assert_eq!(state.pipeline_state, PSInvocationState::Running);
    }

    #[test]
    fn test_pipeline_state_stopped_with_error() {
        let error_record = PsValue::Primitive(PsPrimitiveValue::Str("Stopped by user".to_string()));
        let state = PipelineStateMessage::stopped_with_error(error_record.clone());

        assert_eq!(state.pipeline_state, PSInvocationState::Stopped);
        assert!(state.is_terminal());
        assert_eq!(state.exception_as_error_record, Some(error_record));
    }

    #[test]
    fn test_message_type() {
        let state = PipelineStateMessage::completed();
        assert_eq!(state.message_type().value(), 0x00041006);
    }

    #[test]
    fn test_ps_invocation_state_values() {
        assert_eq!(PSInvocationState::NotStarted.as_i32(), 0);
        assert_eq!(PSInvocationState::Running.as_i32(), 1);
        assert_eq!(PSInvocationState::Stopping.as_i32(), 2);
        assert_eq!(PSInvocationState::Stopped.as_i32(), 3);
        assert_eq!(PSInvocationState::Completed.as_i32(), 4);
        assert_eq!(PSInvocationState::Failed.as_i32(), 5);
        assert_eq!(PSInvocationState::Disconnected.as_i32(), 6);
    }

    #[test]
    fn test_ps_invocation_state_try_from() {
        assert_eq!(
            PSInvocationState::try_from(0).unwrap(),
            PSInvocationState::NotStarted
        );
        assert_eq!(
            PSInvocationState::try_from(1).unwrap(),
            PSInvocationState::Running
        );
        assert_eq!(
            PSInvocationState::try_from(2).unwrap(),
            PSInvocationState::Stopping
        );
        assert_eq!(
            PSInvocationState::try_from(3).unwrap(),
            PSInvocationState::Stopped
        );
        assert_eq!(
            PSInvocationState::try_from(4).unwrap(),
            PSInvocationState::Completed
        );
        assert_eq!(
            PSInvocationState::try_from(5).unwrap(),
            PSInvocationState::Failed
        );
        assert_eq!(
            PSInvocationState::try_from(6).unwrap(),
            PSInvocationState::Disconnected
        );

        assert!(PSInvocationState::try_from(7).is_err());
        assert!(PSInvocationState::try_from(-1).is_err());
    }

    #[test]
    fn test_terminal_states() {
        assert!(!PSInvocationState::NotStarted.is_terminal());
        assert!(!PSInvocationState::Running.is_terminal());
        assert!(!PSInvocationState::Stopping.is_terminal());
        assert!(PSInvocationState::Stopped.is_terminal());
        assert!(PSInvocationState::Completed.is_terminal());
        assert!(PSInvocationState::Failed.is_terminal());
        assert!(!PSInvocationState::Disconnected.is_terminal());
    }
}
