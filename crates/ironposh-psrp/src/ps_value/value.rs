use std::fmt::Display;

use serde::{Deserialize, Serialize};

use super::{ComplexObject, PsPrimitiveValue, PsType};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PsValue {
    Primitive(PsPrimitiveValue),
    Object(ComplexObject),
}

impl<IntoPrimitive> From<IntoPrimitive> for PsValue
where
    IntoPrimitive: Into<PsPrimitiveValue>,
{
    fn from(p: IntoPrimitive) -> Self {
        Self::Primitive(p.into())
    }
}

impl Display for PsValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primitive(p) => p.fmt(f),
            Self::Object(o) => o.fmt(f),
        }
    }
}

impl PsValue {
    pub fn as_object(&self) -> Option<&ComplexObject> {
        if let Self::Object(obj) = self {
            Some(obj)
        } else {
            None
        }
    }

    /// Extract i32 value from PsValue
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Self::Primitive(PsPrimitiveValue::I32(val)) => Some(*val),
            _ => None,
        }
    }

    /// Extract i64 value from PsValue
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Primitive(PsPrimitiveValue::I64(val)) => Some(*val),
            _ => None,
        }
    }

    /// Extract string value from PsValue
    pub fn as_string(&self) -> Option<String> {
        match self {
            Self::Primitive(PsPrimitiveValue::Str(val)) => Some(val.clone()),
            _ => None,
        }
    }

    /// Extract a string array from a container value (`<LST>`/`<STK>`/`<QUE>`).
    ///
    /// Returns `None` when the value is not a container or when any element is
    /// not a string, so callers cannot mistake "not an array" for "empty array".
    pub fn as_string_array(&self) -> Option<Vec<String>> {
        use super::{ComplexObjectContent, Container};
        match self {
            Self::Object(obj) => match &obj.content {
                ComplexObjectContent::Container(
                    Container::List(items) | Container::Stack(items) | Container::Queue(items),
                ) => items.iter().map(Self::as_string).collect(),
                _ => None,
            },
            Self::Primitive(_) => None,
        }
    }

    /// Create an array from a Vec of PsValues
    pub fn from_array(values: Vec<Self>) -> Self {
        Self::Object(ComplexObject {
            type_def: Some(PsType::array_list()),
            to_string: None,
            content: super::ComplexObjectContent::Container(super::Container::List(values)),
            adapted_properties: std::collections::BTreeMap::new(),
            extended_properties: std::collections::BTreeMap::new(),
        })
    }

    /// Create a string array from a Vec of strings
    pub fn from_string_array(strings: Vec<String>) -> Self {
        let values: Vec<Self> = strings
            .into_iter()
            .map(|s| Self::Primitive(PsPrimitiveValue::Str(s)))
            .collect();
        Self::from_array(values)
    }
}
