pub mod deserialize;
mod init_runspace_pool;
mod pipeline_input;
pub mod serialize;
mod session_capability;

use std::{collections::HashMap, hash::Hash};

pub use init_runspace_pool::*;
pub use session_capability::*;

use crate::MessageType;

pub trait PsObjectWithType {
    fn message_type(&self) -> MessageType;
    fn to_ps_object(&self) -> PsObject;
}

/// One PS “primitive” or nested object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PsValue {
    Str(String),     // <S>
    Bool(bool),      // <B>
    I32(i32),        // <I32>
    U32(u32),        // <U32>
    I64(i64),        // <I64>
    Guid(String),    // <G>
    Nil,             // <Nil/>
    Bytes(Vec<u8>),  // <BA>
    Version(String), // <Version>
    Object(PsObject), // <Obj> … </Obj>
                     // Extend as needed...
}

impl Hash for PsValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            PsValue::Str(s) => s.hash(state),
            PsValue::Bool(b) => b.hash(state),
            PsValue::I32(i) => i.hash(state),
            PsValue::U32(u) => u.hash(state),
            PsValue::I64(i) => i.hash(state),
            PsValue::Guid(g) => g.hash(state),
            PsValue::Nil => 0_u8.hash(state),
            PsValue::Bytes(b) => b.hash(state),
            PsValue::Version(v) => v.hash(state),
            PsValue::Object(o) => o.to_element().to_string().hash(state), // recursive
        }
    }
}

/// A property wrapper that carries the `N=` and `RefId=` attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsProperty {
    pub name: Option<String>, //  N="..."
    pub ref_id: Option<u32>,  //  RefId="..."
    pub value: PsValue,       //  actual payload
}

/// A full <Obj>.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PsObject {
    pub ref_id: Option<u32>,             // <Obj RefId="...">
    pub type_names: Option<Vec<String>>, // <TN><T>...</T></TN>
    pub tn_ref: Option<u32>,             // <TNRef RefId="..."/>
    pub props: Vec<PsProperty>,          // <Props>  🔸 optional helper bag
    pub ms: Vec<PsProperty>,             // <MS>     🔸 “member set”
    pub lst: Vec<PsProperty>,            // <LST>    🔸 list / array
    pub dct: HashMap<PsValue, PsValue>,  // <DCT>    🔸 dictionary
}
