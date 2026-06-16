//! Typed conversions between Rust values and the dynamic [`PsValue`] tree (RFC #12, layer L1).
//!
//! These traits are the ergonomic access layer that replaces the hand-rolled
//! `extended_properties.get(..)` + two-layer enum match + bespoke error per
//! message. [`FromPsValue`] powers [`ComplexObject::req`]/[`ComplexObject::opt`];
//! [`ToPsValue`] powers the [`ComplexObjectBuilder`].
//!
//! [`ComplexObject::req`]: super::ComplexObject::req
//! [`ComplexObject::opt`]: super::ComplexObject::opt
//! [`ComplexObjectBuilder`]: super::ComplexObjectBuilder

use super::{ComplexObjectContent, Container, PsPrimitiveValue, PsValue};
use crate::PowerShellRemotingError;

type Result<T> = std::result::Result<T, PowerShellRemotingError>;

/// A type that can be extracted from a [`PsValue`] read off the wire.
///
/// Implementors describe the primitive/container shape they expect; the
/// blanket error machinery in [`ComplexObject::req`](super::ComplexObject::req)
/// adds the property-name context.
pub trait FromPsValue: Sized {
    /// Human-readable label of the expected shape, used in type-mismatch errors.
    const TYPE_LABEL: &'static str;

    /// Extract `Self` from a borrowed value, or describe why it does not fit.
    fn from_ps_value(value: &PsValue) -> Result<Self>;
}

fn type_mismatch<T: FromPsValue>(value: &PsValue) -> PowerShellRemotingError {
    PowerShellRemotingError::InvalidMessage(format!("expected {}, got {value:?}", T::TYPE_LABEL))
}

macro_rules! impl_from_primitive {
    ($ty:ty, $label:literal, $variant:ident) => {
        impl FromPsValue for $ty {
            const TYPE_LABEL: &'static str = $label;

            fn from_ps_value(value: &PsValue) -> Result<Self> {
                match value {
                    PsValue::Primitive(PsPrimitiveValue::$variant(v)) => Ok(v.clone()),
                    other => Err(type_mismatch::<Self>(other)),
                }
            }
        }
    };
}

impl_from_primitive!(i32, "I32", I32);
impl_from_primitive!(u32, "U32", U32);
impl_from_primitive!(i64, "I64", I64);
impl_from_primitive!(u64, "U64", U64);
impl_from_primitive!(bool, "Boolean", Bool);
impl_from_primitive!(char, "Char", Char);
impl_from_primitive!(String, "String", Str);

impl FromPsValue for Vec<u8> {
    const TYPE_LABEL: &'static str = "ByteArray";

    fn from_ps_value(value: &PsValue) -> Result<Self> {
        match value {
            PsValue::Primitive(PsPrimitiveValue::Bytes(b)) => Ok(b.clone()),
            other => Err(type_mismatch::<Self>(other)),
        }
    }
}

impl FromPsValue for uuid::Uuid {
    const TYPE_LABEL: &'static str = "Guid";

    fn from_ps_value(value: &PsValue) -> Result<Self> {
        match value {
            PsValue::Primitive(PsPrimitiveValue::Guid(g)) => g.parse().map_err(|e| {
                PowerShellRemotingError::InvalidMessage(format!("invalid Guid '{g}': {e}"))
            }),
            other => Err(type_mismatch::<Self>(other)),
        }
    }
}

/// A homogeneous list (`<LST>`/`<STK>`/`<QUE>`) of `T`.
impl<T: FromPsValue> FromPsValue for Vec<T> {
    const TYPE_LABEL: &'static str = "List";

    fn from_ps_value(value: &PsValue) -> Result<Self> {
        match value {
            PsValue::Object(obj) => match &obj.content {
                ComplexObjectContent::Container(
                    Container::List(items) | Container::Stack(items) | Container::Queue(items),
                ) => items.iter().map(T::from_ps_value).collect(),
                _ => Err(type_mismatch::<Self>(value)),
            },
            PsValue::Primitive(_) => Err(type_mismatch::<Self>(value)),
        }
    }
}

/// A type that can be rendered into the dynamic [`PsValue`] tree for serialization.
///
/// Implemented for the primitive Rust types, [`uuid::Uuid`], byte vectors,
/// `Vec<T>` (emitted as an `ArrayList`), and `Option<T>` (`None` → `Nil`).
pub trait ToPsValue {
    /// Borrow-and-build; never consumes `self`, so messages serialize without a
    /// top-level `clone()`.
    fn to_ps_value(&self) -> PsValue;
}

macro_rules! impl_to_primitive {
    ($ty:ty) => {
        impl ToPsValue for $ty {
            fn to_ps_value(&self) -> PsValue {
                PsValue::Primitive(PsPrimitiveValue::from(self.clone()))
            }
        }
    };
}

impl_to_primitive!(i32);
impl_to_primitive!(u32);
impl_to_primitive!(i64);
impl_to_primitive!(u64);
impl_to_primitive!(bool);
impl_to_primitive!(char);
impl_to_primitive!(String);
impl_to_primitive!(uuid::Uuid);
impl_to_primitive!(Vec<u8>);

impl ToPsValue for str {
    fn to_ps_value(&self) -> PsValue {
        PsValue::Primitive(PsPrimitiveValue::Str(self.to_string()))
    }
}

/// Blanket borrow impl so the derive macro can serialize fields by reference
/// (`&self.field`) without cloning the whole message. Also covers `&str` via
/// the `str` impl above.
impl<T: ToPsValue + ?Sized> ToPsValue for &T {
    fn to_ps_value(&self) -> PsValue {
        (**self).to_ps_value()
    }
}

impl<T: ToPsValue> ToPsValue for Vec<T> {
    fn to_ps_value(&self) -> PsValue {
        PsValue::from_array(self.iter().map(ToPsValue::to_ps_value).collect())
    }
}

impl<T: ToPsValue> ToPsValue for Option<T> {
    fn to_ps_value(&self) -> PsValue {
        self.as_ref().map_or_else(
            || PsValue::Primitive(PsPrimitiveValue::Nil),
            ToPsValue::to_ps_value,
        )
    }
}

impl ToPsValue for PsValue {
    fn to_ps_value(&self) -> PsValue {
        self.clone()
    }
}
