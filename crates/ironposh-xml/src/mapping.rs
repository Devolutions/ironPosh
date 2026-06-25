//! Namespace-aware XML deserialization — the use-case-agnostic core.
//!
//! `ironposh-xml` owns the *mechanism*: parse, build, and namespace matching by
//! URI. It owns no *vocabulary* — concrete namespace URIs and tag names live in
//! consumer crates. Deserialization is a single direct trait, [`FromXml`]; there
//! is no visitor indirection.
//!
//! The invariant that makes namespaces correct: an element's identity is the
//! pair `(namespace-URI, local-name)`. The prefix (`s:`, `soap:`, …) is an
//! arbitrary, document-local alias and is never compared. roxmltree resolves
//! prefixes to URIs for us; [`NodeExt`] is the single matching primitive.

use crate::parser::Node;
use crate::XmlError;

/// Namespace-aware identity helpers for a parsed node.
///
/// Matching is always against the namespace **URI** (caller-supplied), never
/// the prefix. `None` denotes "no namespace".
pub trait NodeExt<'a> {
    /// The element's expanded name: `(namespace_uri, local_name)`.
    fn expanded_name(&self) -> (Option<&'a str>, &'a str);

    /// `true` iff this node is an element whose expanded name equals
    /// `(ns, local)`. The prefix is irrelevant.
    fn is_element_named(&self, ns: Option<&str>, local: &str) -> bool;
}

impl<'a> NodeExt<'a> for Node<'a, 'a> {
    fn expanded_name(&self) -> (Option<&'a str>, &'a str) {
        let name = self.tag_name();
        (name.namespace(), name.name())
    }

    fn is_element_named(&self, ns: Option<&str>, local: &str) -> bool {
        self.is_element() && self.tag_name().name() == local && self.tag_name().namespace() == ns
    }
}

/// Build `Self` from a parsed XML element node — one entry point, no visitor.
pub trait FromXml<'a>: Sized {
    fn from_xml(node: Node<'a, 'a>) -> Result<Self, XmlError>;
}

/// Reject mixed content: non-whitespace text directly under a container element.
///
/// Whitespace from pretty-printing and any comments/PIs are tolerated. Containers
/// carry child elements, not text, so stray text means the input is malformed.
pub fn reject_mixed_content(node: Node<'_, '_>) -> Result<(), XmlError> {
    for child in node.children() {
        if child.is_text() && child.text().is_some_and(|t| !t.trim().is_empty()) {
            return Err(XmlError::InvalidXml(format!(
                "<{}> contains unexpected text content",
                node.tag_name().name()
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    const ENVELOPE: &str = "http://www.w3.org/2003/05/soap-envelope";

    #[test]
    fn identity_is_uri_and_local_name_not_prefix() {
        // Same element, three encodings — all share one (URI, local) identity.
        for xml in [
            r#"<s:Body xmlns:s="http://www.w3.org/2003/05/soap-envelope"/>"#,
            r#"<soap:Body xmlns:soap="http://www.w3.org/2003/05/soap-envelope"/>"#,
            r#"<Body xmlns="http://www.w3.org/2003/05/soap-envelope"/>"#,
        ] {
            let doc = parse(xml).unwrap();
            assert!(doc.root_element().is_element_named(Some(ENVELOPE), "Body"));
        }
    }

    #[test]
    fn wrong_namespace_does_not_match() {
        let doc = parse(r#"<Body xmlns="http://example.com/other"/>"#).unwrap();
        assert!(!doc.root_element().is_element_named(Some(ENVELOPE), "Body"));
    }
}
