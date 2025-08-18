use super::super::{
    ComplexObject, ComplexObjectContent, PsObjectWithType, PsPrimitiveValue, PsProperty, PsType,
    PsValue,
};
use crate::MessageType;
use std::collections::BTreeMap;

/// PipelineHostResponse is a message sent from the client to the server as a response
/// from a host call executed on the client Pipeline's host.
///
/// MessageType value: 0x00041101
/// Direction: Client to Server
/// Target: Pipeline
///
/// The message format is identical to RUNSPACEPOOL_HOST_RESPONSE but applies to
/// a specific pipeline rather than the runspace pool.
///
/// The message contains:
/// - Call ID (ci): Must match the corresponding PIPELINE_HOST_CALL message
/// - Host method identifier (mi): Identifies the host method from which the response originates
/// - Return value of the method (mr): Optional return value from the host method
/// - Exception thrown by a host method invocation (me): Optional error information
///
/// Example scenarios:
/// - Response to Read-Host with user input ("Alice")
/// - Response to Write-Progress (typically no return value)
/// - Response to other host interaction methods with their respective return values or exceptions
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct PipelineHostResponse {
    /// Call ID that matches the corresponding host call
    pub call_id: i64,
    /// The host method identifier (enum value)
    pub method_id: i32,
    /// String representation of the method name
    pub method_name: String,
    /// Optional return value from the method
    #[builder(default, setter(strip_option))]
    pub method_result: Option<PsValue>,
    /// Optional exception thrown by the method invocation
    #[builder(default, setter(strip_option))]
    pub method_exception: Option<PsValue>,
}

impl PsObjectWithType for PipelineHostResponse {
    fn message_type(&self) -> MessageType {
        MessageType::PipelineHostResponse
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}

impl From<PipelineHostResponse> for ComplexObject {
    fn from(host_response: PipelineHostResponse) -> Self {
        let mut extended_properties = BTreeMap::new();

        // Call ID (ci)
        extended_properties.insert(
            "ci".to_string(),
            PsProperty {
                name: "ci".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I64(host_response.call_id)),
            },
        );

        // Host method identifier (mi)
        let method_id_obj = ComplexObject {
            type_def: Some(PsType::remote_host_method_id()),
            to_string: Some(host_response.method_name),
            content: ComplexObjectContent::ExtendedPrimitive(PsPrimitiveValue::I32(
                host_response.method_id,
            )),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        };

        extended_properties.insert(
            "mi".to_string(),
            PsProperty {
                name: "mi".to_string(),
                value: PsValue::Object(method_id_obj),
            },
        );

        // Method result (mr) - optional
        if let Some(result) = host_response.method_result {
            extended_properties.insert(
                "mr".to_string(),
                PsProperty {
                    name: "mr".to_string(),
                    value: result,
                },
            );
        }

        // Method exception (me) - optional
        if let Some(exception) = host_response.method_exception {
            extended_properties.insert(
                "me".to_string(),
                PsProperty {
                    name: "me".to_string(),
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

impl TryFrom<ComplexObject> for PipelineHostResponse {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        // Extract call_id (ci)
        let ci_property = value.extended_properties.get("ci").ok_or_else(|| {
            Self::Error::InvalidMessage("Missing call ID (ci) property".to_string())
        })?;

        let PsValue::Primitive(PsPrimitiveValue::I64(call_id)) = &ci_property.value else {
            return Err(Self::Error::InvalidMessage(
                "Call ID (ci) is not a signed long integer".to_string(),
            ));
        };

        // Extract method identifier (mi)
        let mi_property = value.extended_properties.get("mi").ok_or_else(|| {
            Self::Error::InvalidMessage("Missing method identifier (mi) property".to_string())
        })?;

        let PsValue::Object(mi_obj) = &mi_property.value else {
            return Err(Self::Error::InvalidMessage(
                "Method identifier (mi) is not an object".to_string(),
            ));
        };

        let ComplexObjectContent::ExtendedPrimitive(PsPrimitiveValue::I32(method_id)) =
            &mi_obj.content
        else {
            return Err(Self::Error::InvalidMessage(
                "Method identifier content is not an I32".to_string(),
            ));
        };

        let method_name = mi_obj.to_string.clone().unwrap_or_default();

        // Extract optional method result (mr)
        let method_result = value
            .extended_properties
            .get("mr")
            .map(|prop| prop.value.clone());

        // Extract optional method exception (me)
        let method_exception = value
            .extended_properties
            .get("me")
            .map(|prop| prop.value.clone());

        Ok(PipelineHostResponse {
            call_id: *call_id,
            method_id: *method_id,
            method_name,
            method_result,
            method_exception,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::PsPrimitiveValue;

    #[test]
    fn test_pipeline_host_response_roundtrip() {
        let original = PipelineHostResponse::builder()
            .call_id(42)
            .method_id(11) // ReadLine method
            .method_name("ReadLine".to_string())
            .method_result(PsValue::Primitive(PsPrimitiveValue::Str("Alice".to_string())))
            .build();

        let complex_obj = ComplexObject::from(original.clone());
        let restored = PipelineHostResponse::try_from(complex_obj).unwrap();

        assert_eq!(original, restored);
    }

    #[test]
    fn test_pipeline_host_response_with_exception() {
        let original = PipelineHostResponse::builder()
            .call_id(1)
            .method_id(20) // WriteProgress method
            .method_name("WriteProgress".to_string())
            .method_exception(PsValue::Primitive(PsPrimitiveValue::Str("Test exception".to_string())))
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
            .method_id(20) // WriteProgress method
            .method_name("WriteProgress".to_string())
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
            .method_id(11)
            .method_name("ReadLine".to_string())
            .build();

        assert_eq!(host_response.message_type(), MessageType::PipelineHostResponse);
    }
}