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

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! compare_xml {
        ($left:expr, $right:expr) => {{
            let normalize = |s: &str| s.replace('\n', "");
            assert_eq!(normalize($left), normalize($right));
        }};
    }

    #[test]
    fn test_simple_xml() {
        let element = Element::new("root");
        let root_element = RootElement::new(element);
        let builder = Builder::new(None, root_element);
        let xml_string = builder.to_string();
        compare_xml!(&xml_string, "<root/>");
    }

    #[test]
    fn test_xml_with_attributes() {
        let element = Element::new("root").add_attribute(Attribute::new("attr1", "value1"));
        let root_element = RootElement::new(element);
        let builder = Builder::new(None, root_element);
        let xml_string = builder.to_string();
        compare_xml!(&xml_string, r#"<root attr1="value1"/>"#);
    }

    #[test]
    fn test_xml_with_child_elements() {
        let child = Element::new("child");
        let element = Element::new("root").add_child(child);
        let root_element = RootElement::new(element);
        let builder = Builder::new(None, root_element);
        let xml_string = builder.to_string();
        let expected_xml = "<root>
    <child/>
</root>";
        compare_xml!(&xml_string, expected_xml);
    }

    #[test]
    fn test_xml_with_namespaces() {
        let element =
            Element::new("root").set_namespace(Namespace::new("ns1", "http://example.com/ns1"));
        let root_element = RootElement::new(element);
        let builder = Builder::new(None, root_element);
        let xml_string = builder.to_string();
        compare_xml!(
            &xml_string,
            r#"<ns1:root xmlns:ns1="http://example.com/ns1"/>"#
        );
    }

    #[test]
    fn test_full_xml_document() {
        let declaration = Declaration::new("1.0", "UTF-8").with_standalone(true);
        let child = Element::new("child")
            .set_namespace(Namespace::new("ns2", "http://example.com/ns2"))
            .add_attribute(Attribute::new("attr2", "value2"));
        let element = Element::new("root")
            .set_namespace(Namespace::new("ns1", "http://example.com/ns1"))
            .add_attribute(Attribute::new("attr1", "value1"))
            .add_child(child);
        let root_element = RootElement::new(element);
        let builder = Builder::new(Some(declaration), root_element);
        let xml_string = builder.to_string();
        let expected_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ns1:root xmlns:ns1="http://example.com/ns1" xmlns:ns2="http://example.com/ns2" attr1="value1">
    <ns2:child attr2="value2"/>
</ns1:root>"#;
        compare_xml!(&xml_string, expected_xml);
    }

    #[test]
    fn test_element_with_text() {
        let element = Element::new("message").set_text("Hello, world!");
        let root_element = RootElement::new(element);
        let builder = Builder::new(None, root_element);
        let xml_string = builder.to_string();
        assert_eq!(xml_string, "<message>Hello, world!</message>");
    }

    #[test]
    fn test_element_with_text_and_attributes() {
        let element = Element::new("message")
            .add_attribute(Attribute::new("lang", "en"))
            .set_text("Hello, world!");
        let root_element = RootElement::new(element);
        let builder = Builder::new(None, root_element);
        let xml_string = builder.to_string();
        assert_eq!(xml_string, r#"<message lang="en">Hello, world!</message>"#);
    }

    #[test]
    fn test_adding_child_overwrites_text() {
        let child = Element::new("item");
        let element = Element::new("container")
            .set_text("Initial text")
            .add_child(child);
        let root_element = RootElement::new(element);
        let builder = Builder::new(None, root_element);
        let xml_string = builder.to_string();
        let expected_xml = "<container>
    <item/>
</container>";
        assert_eq!(xml_string, expected_xml);
    }

    #[test]
    fn test_setting_text_overwrites_children() {
        let child = Element::new("item");
        let element = Element::new("container")
            .add_child(child)
            .set_text("New text");
        let root_element = RootElement::new(element);
        let builder = Builder::new(None, root_element);
        let xml_string = builder.to_string();
        assert_eq!(xml_string, "<container>New text</container>");
    }
}
