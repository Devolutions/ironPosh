use std::{collections::BTreeMap, fmt::Display};

use serde::{Deserialize, Serialize};

use super::PsValue;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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

impl Display for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Container::Stack(_) => write!(f, "System.Collections.Stack"),
            Container::Queue(_) => write!(f, "System.Collections.Queue"),
            Container::List(items) => {
                let s: Vec<String> = items.iter().map(|v| v.to_string()).collect();
                write!(f, "{}", s.join(" "))
            }
            Container::Dictionary(_) => write!(f, "System.Collections.Hashtable"),
        }
    }
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PsEnums {
    pub value: i32,
}

impl Display for PsEnums {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The to_string of the ComplexObject holding this enum should be used.
        // This is a fallback.
        write!(f, "{}", self.value)
    }
}
