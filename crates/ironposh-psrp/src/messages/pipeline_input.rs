use crate::{
    MessageType,
    ps_value::{PsObjectWithType, PsPrimitiveValue, PsValue},
};

/// PIPELINE_INPUT (MS-PSRP §2.2.2.17): client → server. The message data *is* the
/// serialized input object fed to a running pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineInput {
    pub data: PsValue,
}

impl PipelineInput {
    pub fn new(data: PsValue) -> Self {
        Self { data }
    }
}

impl PsObjectWithType for PipelineInput {
    fn message_type(&self) -> MessageType {
        MessageType::PipelineInput
    }

    fn to_ps_object(&self) -> PsValue {
        self.data.clone()
    }
}

/// END_OF_PIPELINE_INPUT (MS-PSRP §2.2.2.18): client → server. Closes the input
/// collection for a pipeline. The reference implementation sends a null data
/// object, so the body is `<Nil/>`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EndOfPipelineInput;

impl PsObjectWithType for EndOfPipelineInput {
    fn message_type(&self) -> MessageType {
        MessageType::EndOfPipelineInput
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Primitive(PsPrimitiveValue::Nil)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PowerShellRemotingMessage;
    use uuid::Uuid;

    #[test]
    fn pipeline_input_carries_the_input_object() {
        let input = PipelineInput::new(PsValue::Primitive(PsPrimitiveValue::Str("hello".into())));
        let msg = PowerShellRemotingMessage::from_ps_message(
            &input,
            Uuid::nil(),
            Some(Uuid::from_u128(7)),
        )
        .expect("build message");
        assert_eq!(msg.message_type, MessageType::PipelineInput);
        let xml = String::from_utf8(msg.data).unwrap();
        assert!(
            xml.contains("<S>hello</S>"),
            "the input object must be serialized into the message: {xml}"
        );
    }

    #[test]
    fn end_of_pipeline_input_is_a_null_typed_message() {
        let msg = PowerShellRemotingMessage::from_ps_message(
            &EndOfPipelineInput,
            Uuid::nil(),
            Some(Uuid::from_u128(7)),
        )
        .expect("build message");
        assert_eq!(msg.message_type, MessageType::EndOfPipelineInput);
        let xml = String::from_utf8(msg.data).unwrap();
        assert!(
            xml.contains("<Nil"),
            "end-of-input body should be a null object: {xml}"
        );
    }
}
