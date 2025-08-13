use super::super::{
    ComplexObject, ComplexObjectContent, Container, PsObjectWithType, PsPrimitiveValue, PsProperty,
    PsType, PsValue,
};
use crate::MessageType;
use std::collections::BTreeMap;

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
        let mut extended_properties = BTreeMap::new();

        // Call ID (ci)
        extended_properties.insert(
            "ci".to_string(),
            PsProperty {
                name: "ci".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I64(host_call.call_id)),
            },
        );

        // Host method identifier (mi)
        let method_id_obj = ComplexObject {
            type_def: Some(PsType::remote_host_method_id()),
            to_string: Some(host_call.method_name),
            content: ComplexObjectContent::ExtendedPrimitive(PsPrimitiveValue::I32(host_call.method_id)),
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

        // Method parameters (mp) as ArrayList
        let parameters_obj = ComplexObject {
            type_def: Some(PsType::array_list()),
            to_string: None,
            content: ComplexObjectContent::Container(Container::List(host_call.parameters)),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        };

        extended_properties.insert(
            "mp".to_string(),
            PsProperty {
                name: "mp".to_string(),
                value: PsValue::Object(parameters_obj),
            },
        );

        ComplexObject {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl TryFrom<ComplexObject> for RunspacePoolHostCall {
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

        // Extract method parameters (mp)
        let mp_property = value.extended_properties.get("mp").ok_or_else(|| {
            Self::Error::InvalidMessage("Missing method parameters (mp) property".to_string())
        })?;

        let PsValue::Object(mp_obj) = &mp_property.value else {
            return Err(Self::Error::InvalidMessage(
                "Method parameters (mp) is not an object".to_string(),
            ));
        };

        let parameters = if let ComplexObjectContent::Container(Container::List(params)) = &mp_obj.content {
            params.clone()
        } else {
            // Empty list case
            Vec::new()
        };

        Ok(RunspacePoolHostCall {
            call_id: *call_id,
            method_id: *method_id,
            method_name,
            parameters,
        })
    }
}