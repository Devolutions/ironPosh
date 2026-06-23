//! Generic, namespace-aware mapping traits — the use-case-agnostic core.
//!
//! `ironposh-xml` owns the XML *mechanism*: parse (roxmltree), build
//! ([`Element`](crate::builder::Element)), and namespace matching/emission **by
//! URI**. It owns no *vocabulary*: concrete namespace URIs, tag names, and
//! schema rules all live in consumer crates. The two traits below are the
//! entire mapping contract — one entry point each, no visitor split.
//!
//! The invariant that makes namespaces correct: an element's identity is the
//! pair `(namespace-URI, local-name)`. The prefix (`s:`, `soap:`, …) is an
//! arbitrary, document-local alias and is never compared. roxmltree resolves
//! prefixes to URIs for us; [`NodeExt`] is the single matching primitive built
//! on top of that.

use crate::XmlError;
use crate::builder::Element;
use crate::parser::Node;

/// Namespace-aware identity helpers for a parsed node.
///
/// Matching is always against the namespace **URI** (caller-supplied), never
/// the prefix. `None` denotes "no namespace" (an unprefixed element under no
/// default namespace, or an unqualified attribute).
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
        self.is_element()
            && self.tag_name().name() == local
            && self.tag_name().namespace() == ns
    }
}

/// Build `Self` from a parsed XML element node.
///
/// One entry point. A consumer type (or, later, a `#[derive(FromXml)]`) walks
/// the node's attributes/children using [`NodeExt`] to match by `(URI, name)`.
pub trait FromXml<'a>: Sized {
    fn from_xml(node: Node<'a, 'a>) -> Result<Self, XmlError>;
}

/// Render `Self` into an XML [`Element`], borrowing from `self`.
///
/// The symmetric counterpart to [`FromXml`]: the same field shape that
/// `from_xml` reads, `to_xml` writes — declaring each namespace by URI with a
/// chosen alias.
pub trait ToXml {
    fn to_xml(&self) -> Element<'_>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    // ── Vocabulary lives in the *consumer*, never in ironposh-xml. ──
    // These are plain `&str` URIs a downstream crate would own (e.g. the WinRM
    // `Namespace` registry). The traits above never hard-code them.
    const SOAP_ENVELOPE: &str = "http://www.w3.org/2003/05/soap-envelope";
    const WS_ADDRESSING: &str = "http://schemas.xmlsoap.org/ws/2004/08/addressing";
    const OTHER_NS: &str = "http://example.com/other";

    /// A trivial consumer-side message type. This hand-written impl is exactly
    /// the shape a future `#[derive(FromXml, ToXml)]` would generate — it is the
    /// macro's expansion target, written out so we can prove the foundation.
    #[derive(Debug, PartialEq, Eq)]
    struct SoapBody {
        action: Option<String>,
    }

    impl<'a> FromXml<'a> for SoapBody {
        fn from_xml(node: Node<'a, 'a>) -> Result<Self, XmlError> {
            // Identity check by (URI, local) — prefix-blind.
            if !node.is_element_named(Some(SOAP_ENVELOPE), "Body") {
                let (ns, local) = node.expanded_name();
                return Err(XmlError::XmlInvalidNamespace {
                    expected: format!("({SOAP_ENVELOPE}, Body)"),
                    found: Some(format!("({ns:?}, {local})")),
                });
            }

            let mut action = None;
            for child in node.children() {
                // Same-local-name in a different namespace must NOT match.
                if child.is_element_named(Some(WS_ADDRESSING), "Action") {
                    action = child.text().map(str::to_owned);
                }
            }
            Ok(SoapBody { action })
        }
    }

    impl ToXml for SoapBody {
        fn to_xml(&self) -> Element<'_> {
            let mut body = Element::new("Body")
                .set_namespace(SOAP_ENVELOPE)
                .add_namespace_declaration(SOAP_ENVELOPE, Some("s"))
                .add_namespace_declaration(WS_ADDRESSING, Some("a"));

            if let Some(action) = &self.action {
                body = body.add_child(
                    Element::new("Action")
                        .set_namespace(WS_ADDRESSING)
                        .set_text(action.as_str()),
                );
            }
            body
        }
    }

    fn root_of<'a>(doc: &'a roxmltree::Document<'a>) -> Node<'a, 'a> {
        doc.root_element()
    }

    #[test]
    fn matches_regardless_of_prefix() {
        // Same document, three encodings of the *same* element. A correct
        // namespace-aware reader treats them identically.
        let with_s = r#"<s:Body xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"><a:Action>cmd</a:Action></s:Body>"#;
        let with_soap = r#"<soap:Body xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"><a:Action>cmd</a:Action></soap:Body>"#;
        let default_ns = r#"<Body xmlns="http://www.w3.org/2003/05/soap-envelope" xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"><a:Action>cmd</a:Action></Body>"#;

        let expected = SoapBody {
            action: Some("cmd".to_owned()),
        };

        for xml in [with_s, with_soap, default_ns] {
            let doc = parse(xml).unwrap();
            let parsed = SoapBody::from_xml(root_of(&doc)).unwrap();
            assert_eq!(parsed, expected, "prefix should not affect identity: {xml}");
        }
    }

    #[test]
    fn rejects_same_local_name_in_wrong_namespace() {
        // Local name "Body" but the wrong namespace URI: must not be accepted.
        let xml = format!(r#"<Body xmlns="{OTHER_NS}"/>"#);
        let doc = parse(&xml).unwrap();
        let err = SoapBody::from_xml(root_of(&doc)).unwrap_err();
        assert!(matches!(err, XmlError::XmlInvalidNamespace { .. }));
    }

    #[test]
    fn child_in_wrong_namespace_is_ignored() {
        // "Action" exists but under SOAP, not WS-Addressing — it must not bind.
        let xml = r#"<s:Body xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Action>cmd</s:Action></s:Body>"#;
        let doc = parse(xml).unwrap();
        let parsed = SoapBody::from_xml(root_of(&doc)).unwrap();
        assert_eq!(parsed, SoapBody { action: None });
    }

    #[test]
    fn round_trips_through_to_xml() {
        let original = SoapBody {
            action: Some("cmd".to_owned()),
        };

        let xml = original.to_xml().to_xml_string().unwrap();
        let doc = parse(&xml).unwrap();
        let reparsed = SoapBody::from_xml(root_of(&doc)).unwrap();

        assert_eq!(reparsed, original);
    }
}
