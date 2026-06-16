use crate::MessageType;
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, Properties, PsObjectWithType, PsPrimitiveValue, PsType,
    PsValue,
};

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
    #[builder(default, setter(strip_option(fallback_suffix = "_opt")))]
    pub method_result: Option<PsValue>,
    /// Optional exception thrown by the method invocation
    #[builder(default, setter(strip_option(fallback_suffix = "_opt")))]
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
        let mut properties = Properties::new();

        // Call ID (ci)
        properties.insert_extended(
            "ci",
            PsValue::Primitive(PsPrimitiveValue::I64(host_response.call_id)),
        );

        // Host method identifier (mi)
        let method_id_obj = Self {
            type_def: Some(PsType::remote_host_method_id()),
            to_string: Some(host_response.method_name),
            content: ComplexObjectContent::ExtendedPrimitive(PsPrimitiveValue::I32(
                host_response.method_id,
            )),
            properties: Properties::new(),
        };

        properties.insert_extended("mi", PsValue::Object(method_id_obj));

        // Method result (mr) - optional
        if let Some(result) = host_response.method_result {
            properties.insert_extended("mr", result);
        }

        // Method exception (me) - optional
        if let Some(exception) = host_response.method_exception {
            properties.insert_extended("me", exception);
        }

        Self {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            properties,
        }
    }
}

impl TryFrom<ComplexObject> for RunspacePoolHostResponse {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        // Extract call_id (ci)
        let ci_value = value.properties.get("ci").ok_or_else(|| {
            Self::Error::InvalidMessage("Missing call ID (ci) property".to_string())
        })?;

        let PsValue::Primitive(PsPrimitiveValue::I64(call_id)) = ci_value else {
            return Err(Self::Error::InvalidMessage(
                "Call ID (ci) is not a signed long integer".to_string(),
            ));
        };

        // Extract method identifier (mi)
        let mi_value = value.properties.get("mi").ok_or_else(|| {
            Self::Error::InvalidMessage("Missing method identifier (mi) property".to_string())
        })?;

        let PsValue::Object(mi_obj) = mi_value else {
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
        let method_result = value.properties.get("mr").cloned();

        // Extract optional method exception (me)
        let method_exception = value.properties.get("me").cloned();

        Ok(Self {
            call_id: *call_id,
            method_id: *method_id,
            method_name,
            method_result,
            method_exception,
        })
    }
}
