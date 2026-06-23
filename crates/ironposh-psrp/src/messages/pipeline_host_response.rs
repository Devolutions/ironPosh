use crate::RemoteHostMethodId;
use crate::ps_value::PsValue;
use ironposh_macros::{PsDeserialize, PsSerialize};

/// PIPELINE_HOST_RESPONSE (MS-PSRP §2.2.2.28): client → server response to a
/// pipeline host call.
///
/// `ci` = call id, `mi` = the host method, `mr` = optional return value,
/// `me` = optional exception (both the dynamic value boundary).
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder, PsSerialize, PsDeserialize)]
#[ps(message_type = PipelineHostResponse)]
pub struct PipelineHostResponse {
    #[ps(name = "ci")]
    pub call_id: i64,
    #[ps(name = "mi")]
    pub method: RemoteHostMethodId,
    #[builder(default, setter(strip_option(fallback_suffix = "_opt")))]
    #[ps(name = "mr")]
    pub method_result: Option<PsValue>,
    #[builder(default, setter(strip_option(fallback_suffix = "_opt")))]
    #[ps(name = "me")]
    pub method_exception: Option<PsValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MessageType;
    use crate::ps_value::{ComplexObject, PsObjectWithType, PsPrimitiveValue};

    #[test]
    fn test_pipeline_host_response_roundtrip() {
        let original = PipelineHostResponse::builder()
            .call_id(42)
            .method(RemoteHostMethodId::ReadLine)
            .method_result(PsValue::Primitive(PsPrimitiveValue::Str(
                "Alice".to_string(),
            )))
            .build();

        let complex_obj = ComplexObject::from(original.clone());
        let restored = PipelineHostResponse::try_from(complex_obj).unwrap();

        assert_eq!(original, restored);
    }

    #[test]
    fn test_pipeline_host_response_with_exception() {
        let original = PipelineHostResponse::builder()
            .call_id(1)
            .method(RemoteHostMethodId::WriteProgress)
            .method_exception(PsValue::Primitive(PsPrimitiveValue::Str(
                "Test exception".to_string(),
            )))
            .build();

        let complex_obj = ComplexObject::from(original.clone());
        let restored = PipelineHostResponse::try_from(complex_obj).unwrap();

        assert_eq!(original, restored);
        assert!(restored.method_result.is_none());
        assert!(restored.method_exception.is_some());
    }

    #[test]
    fn test_pipeline_host_response_empty() {
        let original = PipelineHostResponse::builder()
            .call_id(1)
            .method(RemoteHostMethodId::WriteProgress)
            .build();

        let complex_obj = ComplexObject::from(original.clone());
        let restored = PipelineHostResponse::try_from(complex_obj).unwrap();

        assert_eq!(original, restored);
        assert!(restored.method_result.is_none());
        assert!(restored.method_exception.is_none());
    }

    #[test]
    fn test_pipeline_host_response_message_type() {
        let host_response = PipelineHostResponse::builder()
            .call_id(1)
            .method(RemoteHostMethodId::ReadLine)
            .build();

        assert_eq!(
            host_response.message_type(),
            MessageType::PipelineHostResponse
        );
    }
}
