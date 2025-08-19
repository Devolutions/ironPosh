pub mod create_pipeline;
pub mod deserialize;
pub mod information_record;
pub mod init_runspace_pool;
pub mod pipeline_host_call;
pub mod pipeline_host_response;
pub mod pipeline_input;
pub mod pipeline_state;
pub mod progress_record;
pub mod runspace_pool_host_call;
pub mod runspace_pool_host_response;
pub mod runspace_pool_state;
pub mod serialize;
pub mod session_capability;

pub use create_pipeline::*;
pub use information_record::*;
pub use init_runspace_pool::*;
pub use pipeline_host_call::*;
pub use pipeline_host_response::*;
pub use pipeline_state::*;
pub use progress_record::*;
pub use runspace_pool_host_call::*;
pub use runspace_pool_host_response::*;
pub use runspace_pool_state::*;
pub use session_capability::*;

use std::{borrow::Cow, collections::BTreeMap, hash::Hash};

use crate::MessageType;

pub trait PsObjectWithType {
    fn message_type(&self) -> MessageType;
    fn to_ps_object(&self) -> PsValue;
}

///  https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/c8c85974-ffd7-4455-84a8-e49016c20683
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PsPrimitiveValue {
    Str(String),
    Bool(bool),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    Guid(String),
    Nil,
    Bytes(Vec<u8>),
    Version(String),
    DateTime(String), // Store as string for now
                      // Add more primitive types as needed
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PsValue {
    Primitive(PsPrimitiveValue),
    Object(ComplexObject),
}

impl PsValue {
    pub fn as_object(&self) -> Option<&ComplexObject> {
        if let PsValue::Object(obj) = self {
            Some(obj)
        } else {
            None
        }
    }

    /// Extract i32 value from PsValue
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            PsValue::Primitive(PsPrimitiveValue::I32(val)) => Some(*val),
            _ => None,
        }
    }

    /// Extract i64 value from PsValue
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            PsValue::Primitive(PsPrimitiveValue::I64(val)) => Some(*val),
            _ => None,
        }
    }

    /// Extract string value from PsValue
    pub fn as_string(&self) -> Option<String> {
        match self {
            PsValue::Primitive(PsPrimitiveValue::Str(val)) => Some(val.clone()),
            _ => None,
        }
    }

    /// Extract string array from PsValue (simplified implementation)
    pub fn as_string_array(&self) -> Option<Vec<String>> {
        // For now, simplified - in reality this would need to parse complex objects
        // that represent string arrays
        match self {
            PsValue::Object(_obj) => {
                // TODO: Parse array objects properly
                // For now return empty vec as placeholder
                Some(vec![])
            }
            _ => None,
        }
    }

    /// Create an array from a Vec of PsValues
    pub fn from_array(values: Vec<PsValue>) -> Self {
        PsValue::Object(ComplexObject {
            type_def: Some(PsType::array_list()),
            to_string: None,
            content: ComplexObjectContent::Container(Container::List(values)),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        })
    }

    /// Create a string array from a Vec of strings
    pub fn from_string_array(strings: Vec<String>) -> Self {
        let values: Vec<PsValue> = strings
            .into_iter()
            .map(|s| PsValue::Primitive(PsPrimitiveValue::Str(s)))
            .collect();
        Self::from_array(values)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PsProperty {
    pub name: String,
    pub value: PsValue,
}

/// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/2784bd9c-267d-4297-b603-722c727f85f1
#[derive(Debug, Clone, Eq, Default, Hash, PartialOrd, Ord)]
pub struct PsType {
    /// The <TN> element contains <T> elements, each of which contains the name of a type associated with the object being serialized.
    /// <T> elements MUST be ordered from the most specific (that is, point) to least specific (that is, object).
    /// Type names MUST be encoded as described in section 2.2.5.3.2.
    ///  Mapping type names to concrete types is outside the scope of the protocol and is an implementation detail.
    pub type_names: Vec<Cow<'static, str>>,
}

impl PsType {
    pub fn ps_primitive_dictionary() -> Self {
        PsType {
            type_names: vec![
                Cow::Borrowed("System.Management.Automation.PSPrimitiveDictionary"),
                Cow::Borrowed("System.Collections.Hashtable"),
                Cow::Borrowed("System.Object"),
            ],
        }
    }

    pub fn remote_host_method_id() -> Self {
        PsType {
            type_names: vec![
                Cow::Borrowed("System.Management.Automation.Remoting.RemoteHostMethodId"),
                Cow::Borrowed("System.Enum"),
                Cow::Borrowed("System.ValueType"),
                Cow::Borrowed("System.Object"),
            ],
        }
    }

    pub fn array_list() -> Self {
        PsType {
            type_names: vec![
                Cow::Borrowed("System.Collections.ArrayList"),
                Cow::Borrowed("System.Object"),
            ],
        }
    }

    pub fn pipeline_result_types() -> Self {
        PsType {
            type_names: vec![
                Cow::Borrowed("System.Management.Automation.Runspaces.PipelineResultTypes"),
                Cow::Borrowed("System.Enum"),
                Cow::Borrowed("System.ValueType"),
                Cow::Borrowed("System.Object"),
            ],
        }
    }
}

impl PartialEq for PsType {
    fn eq(&self, other: &Self) -> bool {
        for (ty1, ty2) in self.type_names.iter().zip(other.type_names.iter()) {
            if ty1.as_ref() != ty2.as_ref() {
                return false;
            }
        }
        true
    }
}

/*
https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/3e107e78-3f28-4f85-9e25-493fd9b09726
The <Obj> element can include the following subelements in any order.
    Type names (section 2.2.5.2.3).
    ToString (section 2.2.5.2.4).
    Element generated by one of the following:
        Value of a primitive type (when the Complex Object is an Extended Primitive Object) (section 2.2.5.2.5).
        Contents of known containers (section 2.2.5.2.6).
        Contents of enums (section 2.2.5.2.7).
    Adapted Properties (section 2.2.5.2.8).
    Extended properties (section 2.2.5.2.9).

*/
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ComplexObject {
    pub type_def: Option<PsType>,
    pub to_string: Option<String>,
    pub content: ComplexObjectContent,
    pub adapted_properties: BTreeMap<String, PsProperty>,
    pub extended_properties: BTreeMap<String, PsProperty>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum ComplexObjectContent {
    /// If the Complex Object being serialized is an Extended Primitive Object, then the value of the primitive type is serialized as described in section 2.2.5.1.
    ///Example (compare with the serialization of a string without notes in section 2.2.5.1.1):
    ///
    ///     <Obj RefId="RefId-0">
    ///       <S>This is a string</S>
    ///       <MS>
    ///         <S N="Note1">My note</S>
    ///       </MS>
    ///     </Obj>
    ExtendedPrimitive(PsPrimitiveValue),
    Container(Container),
    PsEnums(PsEnums),
    #[default]
    Standard,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Container {
    ///https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/e9cf648e-38fe-42ba-9ca3-d89a9e0a856a
    Stack(Vec<PsValue>),
    ///https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/ade9f023-ac30-4b7e-be17-900c02a6f837
    Queue(Vec<PsValue>),
    ///https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/f4bdb166-cefc-4d49-848c-7d08680ae0a7
    List(Vec<PsValue>),
    /// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/c4e000a2-21d8-46c0-a71b-0051365d8273
    Dictionary(BTreeMap<PsValue, PsValue>),
}

/// Enums specify a value of an enumeration. An enumeration is a distinct type consisting of a set of named constants. Every enumeration type has an underlying type, which can be any integral type. The default underlying type of the enumeration elements is a 32-bit integer (see section 2.2.5.1.11). Enums never have adapted properties (see section 2.2.5.3.4.1).
/// XML Element: element corresponding to the primitive integer type (see section 2.2.5.1) that is underlying the enumeration type.
/// XML Contents: value of the enumeration converted to the underlying type.
///
/// Example:
///
///      <Obj RefId="0">
///        <TN RefId="0">
///          <T>System.ConsoleColor</T>
///          <T>System.Enum</T>
///          <T>System.ValueType</T>
///          <T>System.Object</T>
///        </TN>
///        <ToString>Blue</ToString>
///        <I32>9</I32>
///      </Obj>
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PsEnums {
    pub value: i32,
}
