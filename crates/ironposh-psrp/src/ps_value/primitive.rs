use std::fmt::Display;

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
            PsPrimitiveValue::Nil => write!(f, ""), // PowerShell $null stringifies to empty string
            PsPrimitiveValue::Bytes(_bytes) => write!(f, "System.Byte[]"),
            PsPrimitiveValue::Version(v) => write!(f, "{v}"),
            PsPrimitiveValue::DateTime(d) => write!(f, "{d}"),
        }
    }
}
