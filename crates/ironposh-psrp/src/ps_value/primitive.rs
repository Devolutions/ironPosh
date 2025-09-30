use std::fmt::Display;

use serde::{Deserialize, Serialize};

///  https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/c8c85974-ffd7-4455-84a8-e49016c20683
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PsPrimitiveValue {
    Str(String),
    Bool(bool),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    Guid(String),
    Char(char),
    Nil,
    Bytes(Vec<u8>),
    Version(String),
    DateTime(String), // Store as string for now
                      // Add more primitive types as needed
}

impl Display for PsPrimitiveValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsPrimitiveValue::Str(s) => write!(f, "{s}"),
            PsPrimitiveValue::Bool(b) => write!(f, "{b}"),
            PsPrimitiveValue::I32(i) => write!(f, "{i}"),
            PsPrimitiveValue::U32(u) => write!(f, "{u}"),
            PsPrimitiveValue::I64(i) => write!(f, "{i}"),
            PsPrimitiveValue::U64(u) => write!(f, "{u}"),
            PsPrimitiveValue::Guid(g) => write!(f, "{g}"),
            PsPrimitiveValue::Char(c) => write!(f, "{c}"),
            PsPrimitiveValue::Nil => write!(f, ""), // PowerShell $null stringifies to empty string
            PsPrimitiveValue::Bytes(_bytes) => write!(f, "System.Byte[]"),
            PsPrimitiveValue::Version(v) => write!(f, "{v}"),
            PsPrimitiveValue::DateTime(d) => write!(f, "{d}"),
        }
    }
}

impl From<()> for PsPrimitiveValue {
    fn from(_: ()) -> Self {
        PsPrimitiveValue::Nil
    }
}

impl From<uuid::Uuid> for PsPrimitiveValue {
    fn from(guid: uuid::Uuid) -> Self {
        PsPrimitiveValue::Guid(guid.to_string().to_uppercase())
    }
}

impl From<&str> for PsPrimitiveValue {
    fn from(s: &str) -> Self {
        PsPrimitiveValue::Str(s.to_string())
    }
}

impl From<String> for PsPrimitiveValue {
    fn from(s: String) -> Self {
        PsPrimitiveValue::Str(s)
    }
}

impl From<bool> for PsPrimitiveValue {
    fn from(b: bool) -> Self {
        PsPrimitiveValue::Bool(b)
    }
}

impl From<i32> for PsPrimitiveValue {
    fn from(i: i32) -> Self {
        PsPrimitiveValue::I32(i)
    }
}
impl From<u32> for PsPrimitiveValue {
    fn from(u: u32) -> Self {
        PsPrimitiveValue::U32(u)
    }
}

impl From<i64> for PsPrimitiveValue {
    fn from(i: i64) -> Self {
        PsPrimitiveValue::I64(i)
    }
}

impl From<char> for PsPrimitiveValue {
    fn from(c: char) -> Self {
        PsPrimitiveValue::Char(c)
    }
}

impl From<u64> for PsPrimitiveValue {
    fn from(u: u64) -> Self {
        PsPrimitiveValue::U64(u)
    }
}

impl From<Vec<u8>> for PsPrimitiveValue {
    fn from(bytes: Vec<u8>) -> Self {
        PsPrimitiveValue::Bytes(bytes)
    }
}

impl From<&[u8]> for PsPrimitiveValue {
    fn from(bytes: &[u8]) -> Self {
        PsPrimitiveValue::Bytes(bytes.to_vec())
    }
}
