use crate::RemoteHostMethodId;
use crate::ps_value::PsValue;
use ironposh_macros::{PsDeserialize, PsSerialize};

/// PIPELINE_HOST_CALL (MS-PSRP §2.2.2.27): server → client request to run a host
/// method against a pipeline's host. Same shape as RUNSPACEPOOL_HOST_CALL.
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder, PsSerialize, PsDeserialize)]
#[ps(message_type = PipelineHostCall)]
pub struct PipelineHostCall {
    #[ps(name = "ci")]
    pub call_id: i64,
    #[ps(name = "mi")]
    pub method: RemoteHostMethodId,
    #[builder(default)]
    #[ps(name = "mp")]
    pub parameters: Vec<PsValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MessageType;
    use crate::ps_value::{ComplexObject, PsObjectWithType, PsPrimitiveValue};

    #[test]
    fn test_pipeline_host_call_roundtrip() {
        let original = PipelineHostCall::builder()
            .call_id(42)
            .method(RemoteHostMethodId::ReadLine)
            .parameters(vec![PsValue::Primitive(PsPrimitiveValue::Str(
                "Please enter your username".to_string(),
            ))])
            .build();

        let complex_obj = ComplexObject::from(original.clone());
        let restored = PipelineHostCall::try_from(complex_obj).unwrap();

        assert_eq!(original, restored);
    }

    #[test]
    fn test_pipeline_host_call_empty_parameters() {
        let original = PipelineHostCall::builder()
            .call_id(1)
            .method(RemoteHostMethodId::WriteProgress)
            .build();

        let complex_obj = ComplexObject::from(original.clone());
        let restored = PipelineHostCall::try_from(complex_obj).unwrap();

        assert_eq!(original, restored);
        assert!(restored.parameters.is_empty());
    }

    #[test]
    fn test_pipeline_host_call_message_type() {
        let host_call = PipelineHostCall::builder()
            .call_id(1)
            .method(RemoteHostMethodId::ReadLine)
            .build();

        assert_eq!(host_call.message_type(), MessageType::PipelineHostCall);
    }
}
