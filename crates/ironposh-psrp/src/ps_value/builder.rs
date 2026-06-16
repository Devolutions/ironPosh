//! Typed accessors and a fluent builder for [`ComplexObject`] (RFC #12, layer L1).
//!
//! The goal is that message code never touches `extended_properties`/
//! `adapted_properties`/`PsProperty` directly. Reading a property becomes
//! `obj.req::<i32>("MinRunspaces")?`; building one becomes
//! `ComplexObject::standard().extended("MinRunspaces", self.min).build()`.
//! Hiding the representation here is what later lets the two property bags
//! collapse into one ordered map (RFC step 4) without touching call sites.

use std::borrow::Cow;

use super::{
    ComplexObject, ComplexObjectContent, FromPsValue, Properties, PsType, PsValue, ToPsValue,
};
use crate::PowerShellRemotingError;

type Result<T> = std::result::Result<T, PowerShellRemotingError>;

impl ComplexObject {
    /// Borrow a property value by name, searching extended properties first and
    /// then adapted properties.
    ///
    /// Clients do not care about the adapted/extended distinction — the .NET
    /// reference itself coalesces both into one bag on deserialize (RFC finding
    /// 4) — so a single lookup over both is correct.
    #[must_use]
    pub fn get_property(&self, name: &str) -> Option<&PsValue> {
        self.properties.get(name)
    }

    /// Extract a required, typed property. Produces a precise missing-property
    /// or type-mismatch error carrying the property name.
    pub fn req<T: FromPsValue>(&self, name: &str) -> Result<T> {
        let value = self.get_property(name).ok_or_else(|| {
            PowerShellRemotingError::InvalidMessage(format!("Missing property: {name}"))
        })?;
        T::from_ps_value(value).map_err(|err| {
            PowerShellRemotingError::InvalidMessage(format!("Property '{name}': {err}"))
        })
    }

    /// Extract an optional, typed property. A missing property or an explicit
    /// `Nil` both yield `Ok(None)`.
    pub fn opt<T: FromPsValue>(&self, name: &str) -> Result<Option<T>> {
        match self.get_property(name) {
            None | Some(PsValue::Primitive(super::PsPrimitiveValue::Nil)) => Ok(None),
            Some(value) => T::from_ps_value(value).map(Some).map_err(|err| {
                PowerShellRemotingError::InvalidMessage(format!("Property '{name}': {err}"))
            }),
        }
    }

    /// Start building a standard (property-bag) object.
    #[must_use]
    pub fn standard() -> ComplexObjectBuilder {
        ComplexObjectBuilder::new(ComplexObjectContent::Standard)
    }

    /// Start building an object with the given content (container, enum, …).
    #[must_use]
    pub fn builder(content: ComplexObjectContent) -> ComplexObjectBuilder {
        ComplexObjectBuilder::new(content)
    }
}

/// Fluent builder that writes each property name exactly once and hides the
/// `PsProperty { name, value }` duplication.
#[derive(Debug, Clone)]
pub struct ComplexObjectBuilder {
    obj: ComplexObject,
}

impl ComplexObjectBuilder {
    fn new(content: ComplexObjectContent) -> Self {
        Self {
            obj: ComplexObject {
                type_def: None,
                to_string: None,
                content,
                properties: Properties::new(),
            },
        }
    }

    /// Set the type-name chain (most specific first).
    #[must_use]
    pub fn type_names<I>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = Cow<'static, str>>,
    {
        self.obj.type_def = Some(PsType {
            type_names: names.into_iter().collect(),
        });
        self
    }

    /// Set the type definition directly.
    #[must_use]
    pub fn type_def(mut self, type_def: PsType) -> Self {
        self.obj.type_def = Some(type_def);
        self
    }

    /// Set the `<ToString>` display value.
    #[must_use]
    pub fn to_string_repr(mut self, value: impl Into<String>) -> Self {
        self.obj.to_string = Some(value.into());
        self
    }

    /// Add an extended (`<MS>`) property.
    // Taken by value so callers can pass owned values and literals without `&`.
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn extended(mut self, name: impl Into<String>, value: impl ToPsValue) -> Self {
        self.insert_extended(name.into(), value.to_ps_value());
        self
    }

    /// Add an extended property only when present; `None` is skipped entirely.
    #[must_use]
    pub fn extended_opt(mut self, name: impl Into<String>, value: Option<impl ToPsValue>) -> Self {
        if let Some(value) = value {
            self.insert_extended(name.into(), value.to_ps_value());
        }
        self
    }

    /// Add an adapted (`<Props>`) property.
    // Taken by value so callers can pass owned values and literals without `&`.
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn adapted(mut self, name: impl Into<String>, value: impl ToPsValue) -> Self {
        self.obj
            .properties
            .insert_adapted(name.into(), value.to_ps_value());
        self
    }

    fn insert_extended(&mut self, name: String, value: PsValue) {
        self.obj.properties.insert_extended(name, value);
    }

    /// Finish building.
    #[must_use]
    pub fn build(self) -> ComplexObject {
        self.obj
    }

    /// Finish building and wrap in [`PsValue::Object`].
    #[must_use]
    pub fn build_value(self) -> PsValue {
        PsValue::Object(self.obj)
    }
}
