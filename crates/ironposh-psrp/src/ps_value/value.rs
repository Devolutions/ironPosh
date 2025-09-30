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
        PsValue::Primitive(p.into())
    }
}

impl Display for PsValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsValue::Primitive(p) => p.fmt(f),
            PsValue::Object(o) => o.fmt(f),
        }
    }
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
            content: super::ComplexObjectContent::Container(super::Container::List(values)),
            adapted_properties: std::collections::BTreeMap::new(),
            extended_properties: std::collections::BTreeMap::new(),
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
