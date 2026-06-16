use crate::MessageType;
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, Container, Properties, PsObjectWithType, PsPrimitiveValue,
    PsType, PsValue,
};

/// RunspacePoolHostCall is a message sent from the server to the client to perform
/// a method call on the host associated with the RunspacePool on the server.
///
/// MessageType value: 0x00021100
/// Direction: Server to Client
/// Target: RunspacePool
///
/// The message contains:
/// - Call ID (ci): A signed long integer to associate with the response
/// - Host method identifier (mi): Identifies the specific host method to execute
/// - Parameters for the method (mp): Arguments required for the host method call
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct RunspacePoolHostCall {
    /// Unique identifier for this host call
    pub call_id: i64,
    /// The host method identifier (enum value)
    pub method_id: i32,
    /// String representation of the method name
    pub method_name: String,
    /// Parameters for the method call as a list of values
    #[builder(default)]
    pub parameters: Vec<PsValue>,
}

impl PsObjectWithType for RunspacePoolHostCall {
    fn message_type(&self) -> MessageType {
        MessageType::RunspacepoolHostCall
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}

impl From<RunspacePoolHostCall> for ComplexObject {
    fn from(host_call: RunspacePoolHostCall) -> Self {
        let mut properties = Properties::new();

        // Call ID (ci)
        properties.insert_extended(
            "ci",
            PsValue::Primitive(PsPrimitiveValue::I64(host_call.call_id)),
        );

        // Host method identifier (mi)
        let method_id_obj = Self {
            type_def: Some(PsType::remote_host_method_id()),
            to_string: Some(host_call.method_name),
            content: ComplexObjectContent::ExtendedPrimitive(PsPrimitiveValue::I32(
                host_call.method_id,
            )),
            properties: Properties::new(),
        };

        properties.insert_extended("mi", PsValue::Object(method_id_obj));

        // Method parameters (mp) as ArrayList
        let parameters_obj = Self {
            type_def: Some(PsType::array_list()),
            to_string: None,
            content: ComplexObjectContent::Container(Container::List(host_call.parameters)),
            properties: Properties::new(),
        };

        properties.insert_extended("mp", PsValue::Object(parameters_obj));

        Self {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            properties,
        }
    }
}

impl TryFrom<ComplexObject> for RunspacePoolHostCall {
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

        // Extract method parameters (mp)
        let mp = value.properties.get("mp").ok_or_else(|| {
            Self::Error::InvalidMessage("Missing method parameters (mp) property".to_string())
        })?;

        let PsValue::Object(obj) = mp else {
            return Err(Self::Error::InvalidMessage(
                "Method parameters (mp) is not an object".to_string(),
            ));
        };

        let parameters =
            if let ComplexObjectContent::Container(Container::List(params)) = &obj.content {
                params.clone()
            } else {
                // Empty list case
                Vec::new()
            };

        Ok(Self {
            call_id: *call_id,
            method_id: *method_id,
            method_name,
            parameters,
        })
    }
}
