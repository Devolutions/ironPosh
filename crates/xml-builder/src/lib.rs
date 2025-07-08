//! # xml-builder-rs
//!  A lightweight and intuitive library for generating XML documents in Rust. With an easy-to-use API, it allows you to create well-formed XML structures programmatically. Add elements, attributes, namespaces, and CDATA sections effortlessly.
//! ```
mod attribute;
mod builder;
mod declaration;
mod element;
mod namespace;

use std::collections::HashMap;

pub use self::attribute::*;
pub use self::builder::*;
pub use self::declaration::*;
pub use self::element::*;
pub use self::namespace::*;

pub trait ElementFmt {
    fn serialize(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        namespace_alias_map: HashMap<String, String>,
    ) -> std::fmt::Result;
}
