pub mod builder;
pub mod complex;
pub mod container;
pub mod convert;
pub mod deserialize;
pub mod known_types;
pub mod primitive;
pub mod property;
pub mod serialize;
pub mod types;
pub mod value;

pub use builder::*;
pub use complex::*;
pub use container::*;
pub use convert::*;
pub use deserialize::*;
pub use known_types::*;
pub use primitive::*;
pub use property::*;
pub use serialize::*;
pub use types::*;
pub use value::*;

use crate::MessageType;

pub trait PsObjectWithType {
    fn message_type(&self) -> MessageType;
    fn to_ps_object(&self) -> PsValue;
}
