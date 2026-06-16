use std::collections::BTreeMap;
use std::collections::btree_map;

use serde::{Deserialize, Serialize};

use super::PsValue;

/// Which member set a property belongs to (RFC #12, L1).
///
/// .NET's adapted (`<Props>`) vs extended (`<MS>`) distinction matters to its
/// member-resolution chain, not to clients — the reference itself coalesces
/// both into one bag on deserialize — but the wire format keeps them separate,
/// so we retain the tag to round-trip faithfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PropertyKind {
    /// Adapted property, serialized inside `<Props>`.
    Adapted,
    /// Extended property, serialized inside `<MS>`.
    Extended,
}

/// A single property value plus the member set it belongs to.
///
/// The property *name* is the key it is stored under in [`Properties`]; it is
/// deliberately not duplicated here (RFC #12 removed the old
/// `PsProperty { name, value }` that duplicated its map key).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Property {
    pub kind: PropertyKind,
    pub value: PsValue,
}

impl Property {
    pub fn adapted(value: PsValue) -> Self {
        Self {
            kind: PropertyKind::Adapted,
            value,
        }
    }

    pub fn extended(value: PsValue) -> Self {
        Self {
            kind: PropertyKind::Extended,
            value,
        }
    }
}

/// One ordered, name-keyed map of both adapted and extended properties (RFC #12, L1).
///
/// Each entry is tagged with its [`PropertyKind`]; this single map replaced the
/// old pair of `BTreeMap<String, PsProperty>` (the RFC's one breaking change).
/// Ordering is by name (the underlying [`BTreeMap`]), and serialization emits
/// adapted properties (`<Props>`) before extended (`<MS>`), each sorted by
/// name — byte-identical to the previous two-map representation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct Properties {
    entries: BTreeMap<String, Property>,
}

impl Properties {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert (or replace) an extended (`<MS>`) property.
    pub fn insert_extended(&mut self, name: impl Into<String>, value: impl Into<PsValue>) {
        self.entries
            .insert(name.into(), Property::extended(value.into()));
    }

    /// Insert (or replace) an adapted (`<Props>`) property.
    pub fn insert_adapted(&mut self, name: impl Into<String>, value: impl Into<PsValue>) {
        self.entries
            .insert(name.into(), Property::adapted(value.into()));
    }

    /// Insert a pre-tagged property.
    pub fn insert(&mut self, name: impl Into<String>, property: Property) {
        self.entries.insert(name.into(), property);
    }

    /// Borrow a property value by name, regardless of member set.
    pub fn get(&self, name: &str) -> Option<&PsValue> {
        self.entries.get(name).map(|p| &p.value)
    }

    /// Borrow the full tagged property by name.
    pub fn get_property(&self, name: &str) -> Option<&Property> {
        self.entries.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Iterate name → value over adapted properties, in name order.
    pub fn adapted(&self) -> impl Iterator<Item = (&str, &PsValue)> {
        self.entries
            .iter()
            .filter(|(_, p)| p.kind == PropertyKind::Adapted)
            .map(|(n, p)| (n.as_str(), &p.value))
    }

    /// Iterate name → value over extended properties, in name order.
    pub fn extended(&self) -> impl Iterator<Item = (&str, &PsValue)> {
        self.entries
            .iter()
            .filter(|(_, p)| p.kind == PropertyKind::Extended)
            .map(|(n, p)| (n.as_str(), &p.value))
    }

    /// True if there is at least one adapted property.
    pub fn has_adapted(&self) -> bool {
        self.entries
            .values()
            .any(|p| p.kind == PropertyKind::Adapted)
    }

    /// True if there is at least one extended property.
    pub fn has_extended(&self) -> bool {
        self.entries
            .values()
            .any(|p| p.kind == PropertyKind::Extended)
    }

    /// Iterate name → tagged property, in name order.
    pub fn iter(&self) -> btree_map::Iter<'_, String, Property> {
        self.entries.iter()
    }

    /// Mutably iterate every property value (both member sets), in name order.
    /// The names and kinds are fixed; only values can be edited in place.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut PsValue> {
        self.entries.values_mut().map(|p| &mut p.value)
    }
}

impl<'a> IntoIterator for &'a Properties {
    type Item = (&'a String, &'a Property);
    type IntoIter = btree_map::Iter<'a, String, Property>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}
