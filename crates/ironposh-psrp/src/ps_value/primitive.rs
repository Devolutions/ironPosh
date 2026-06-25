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
    SecureString(Vec<u8>),
    Version(String),
    DateTime(String), // Store as string for now
    TimeSpan(String), // Store as string for now
    // Double/Single/Decimal keep their lexical text: the enum must stay
    // Eq+Hash+Ord (it backs the serializer's RefId map, which floats can't), and
    // the wire form is already text, so this also round-trips exactly.
    Double(String),
    Single(String),
    Decimal(String),
    Int16(i16),
    UInt16(u16),
    Byte(u8),
    SByte(i8),
    Uri(String),
    ScriptBlock(String),
    Xml(String),
}

impl Display for PsPrimitiveValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // All string-backed variants render as their text.
            Self::Str(s)
            | Self::Guid(s)
            | Self::Version(s)
            | Self::DateTime(s)
            | Self::TimeSpan(s)
            | Self::Double(s)
            | Self::Single(s)
            | Self::Decimal(s)
            | Self::Uri(s)
            | Self::ScriptBlock(s)
            | Self::Xml(s) => write!(f, "{s}"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::I32(i) => write!(f, "{i}"),
            Self::U32(u) => write!(f, "{u}"),
            Self::I64(i) => write!(f, "{i}"),
            Self::U64(u) => write!(f, "{u}"),
            Self::Char(c) => write!(f, "{c}"),
            Self::Nil => write!(f, ""), // PowerShell $null stringifies to empty string
            Self::Bytes(_bytes) => write!(f, "System.Byte[]"),
            Self::SecureString(_bytes) => write!(f, "System.Security.SecureString"),
            Self::Int16(i) => write!(f, "{i}"),
            Self::UInt16(u) => write!(f, "{u}"),
            Self::Byte(b) => write!(f, "{b}"),
            Self::SByte(i) => write!(f, "{i}"),
        }
    }
}

impl From<()> for PsPrimitiveValue {
    fn from(_value: ()) -> Self {
        Self::Nil
    }
}

impl From<uuid::Uuid> for PsPrimitiveValue {
    fn from(guid: uuid::Uuid) -> Self {
        Self::Guid(super::known_types::format_guid(guid))
    }
}

impl From<&str> for PsPrimitiveValue {
    fn from(s: &str) -> Self {
        Self::Str(s.to_string())
    }
}

impl From<String> for PsPrimitiveValue {
    fn from(s: String) -> Self {
        Self::Str(s)
    }
}

impl From<bool> for PsPrimitiveValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<i32> for PsPrimitiveValue {
    fn from(i: i32) -> Self {
        Self::I32(i)
    }
}
impl From<u32> for PsPrimitiveValue {
    fn from(u: u32) -> Self {
        Self::U32(u)
    }
}

impl From<i64> for PsPrimitiveValue {
    fn from(i: i64) -> Self {
        Self::I64(i)
    }
}

impl From<char> for PsPrimitiveValue {
    fn from(c: char) -> Self {
        Self::Char(c)
    }
}

impl From<u64> for PsPrimitiveValue {
    fn from(u: u64) -> Self {
        Self::U64(u)
    }
}

impl From<Vec<u8>> for PsPrimitiveValue {
    fn from(bytes: Vec<u8>) -> Self {
        Self::Bytes(bytes)
    }
}

impl From<&[u8]> for PsPrimitiveValue {
    fn from(bytes: &[u8]) -> Self {
        Self::Bytes(bytes.to_vec())
    }
}
