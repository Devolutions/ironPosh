use super::super::{
    ComplexObject, ComplexObjectContent, PsObjectWithType, PsPrimitiveValue, PsProperty,
    PsType, PsValue,
};
use crate::MessageType;
use std::collections::BTreeMap;

/// RunspacePoolHostResponse is a message sent from the client to the server as a response
/// from a host call executed on the client RunspacePool's host.
/// 
/// MessageType value: 0x00021101
/// Direction: Client to Server
/// Target: RunspacePool
///
/// The message contains:
/// - Call ID (ci): Must match the corresponding RUNSPACEPOOL_HOST_CALL message
/// - Host method identifier (mi): Identifies the host method from which the response originates
/// - Return value of the method (mr): Optional return value from the host method
/// - Exception thrown by a host method invocation (me): Optional error information
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct RunspacePoolHostResponse {
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

impl PsObjectWithType for RunspacePoolHostResponse {
    fn message_type(&self) -> MessageType {
        MessageType::RunspacepoolHostResponse
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}

impl From<RunspacePoolHostResponse> for ComplexObject {
    fn from(host_response: RunspacePoolHostResponse) -> Self {
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
            content: ComplexObjectContent::ExtendedPrimitive(PsPrimitiveValue::I32(host_response.method_id)),
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

impl TryFrom<ComplexObject> for RunspacePoolHostResponse {
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

        let ComplexObjectContent::ExtendedPrimitive(PsPrimitiveValue::I32(method_id)) = &mi_obj.content else {
            return Err(Self::Error::InvalidMessage(
                "Method identifier content is not an I32".to_string(),
            ));
        };

        let method_name = mi_obj.to_string.clone().unwrap_or_default();

        // Extract optional method result (mr)
        let method_result = value.extended_properties
            .get("mr")
            .map(|prop| prop.value.clone());

        // Extract optional method exception (me)
        let method_exception = value.extended_properties
            .get("me")
            .map(|prop| prop.value.clone());

        Ok(RunspacePoolHostResponse {
            call_id: *call_id,
            method_id: *method_id,
            method_name,
            method_result,
            method_exception,
        })
    }
}