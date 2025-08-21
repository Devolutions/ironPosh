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

pub type AliasMap<'a> = HashMap<Namespace<'a>, Option<&'a str>>;

#[derive(Debug, thiserror::Error)]
pub enum XmlBuilderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("Missing alias map for element '{tag}' in namespace '{ns}'")]
    MissingAliasMapForElement { tag: String, ns: String },
    #[error("Missing alias map for attribute '{attr}' in namespace '{ns}'")]
    MissingAliasMapForAttribute { attr: String, ns: String },
    #[error("Namespace '{ns}' not declared for tag '{tag}'")]
    NamespaceNotDeclared { tag: String, ns: String },
    #[error("Namespace '{ns}' has no alias for tag '{tag}'")]
    NamespaceHasNoAlias { tag: String, ns: String },
}

pub trait NamespaceWrite<'a> {
    fn ns_write<W: std::io::Write>(
        &self,
        w: &mut W,
        aliases: Option<&AliasMap<'a>>,
    ) -> Result<(), XmlBuilderError>;
}

// Keep the old trait for backward compatibility during transition
pub trait NamespaceFmt {
    fn ns_fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        namespaces_alias_map: Option<&HashMap<Namespace<'_>, Option<&str>>>,
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

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        compare_xml!(&xml_string, "<root/>");
    }

    #[test]
    fn test_xml_with_attributes() {
        let element = Element::new("root").add_attribute(Attribute::new("attr1", "value1"));

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        compare_xml!(&xml_string, r#"<root attr1="value1"/>"#);
    }

    #[test]
    fn test_xml_with_child_elements() {
        let child = Element::new("child");
        let element = Element::new("root").add_child(child);

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        let expected_xml = "<root><child/></root>";
        compare_xml!(&xml_string, expected_xml);
    }

    #[test]
    fn test_xml_with_namespaces() {
        let element = Element::new("root")
            .set_namespace(Namespace::new("http://example.com/ns1"))
            .add_namespace_declaration("http://example.com/ns1", Some("ns1"));

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        compare_xml!(
            &xml_string,
            r#"<ns1:root xmlns:ns1="http://example.com/ns1"/>"#
        );
    }

    #[test]
    fn test_full_xml_document() {
        let declaration = Declaration::new("1.0", "UTF-8").with_standalone(true);
        let child = Element::new("child")
            .set_namespace(Namespace::new("http://example.com/ns2"))
            .add_attribute(Attribute::new("attr2", "value2"));
        let element = Element::new("root")
            .set_namespace(Namespace::new("http://example.com/ns1"))
            .add_namespace_declaration("http://example.com/ns1", Some("ns1"))
            .add_namespace_declaration("http://example.com/ns2", Some("ns2"))
            .add_attribute(Attribute::new("attr1", "value1"))
            .add_child(child);

        let builder = Builder::new(Some(declaration), element);
        let xml_string = builder.to_xml_string().unwrap();
        // The declaration includes a space after "?>" and before the root element
        assert!(
            xml_string.starts_with(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?> "#)
        );
        assert!(xml_string.contains(r#"<ns1:root"#));
        assert!(xml_string.contains(r#"xmlns:ns1="http://example.com/ns1""#));
        assert!(xml_string.contains(r#"xmlns:ns2="http://example.com/ns2""#));
        assert!(xml_string.contains(r#"attr1="value1""#));
        assert!(xml_string.contains(r#"<ns2:child attr2="value2"/>"#));
        assert!(xml_string.ends_with(r#"</ns1:root>"#));
    }

    #[test]
    fn test_element_with_text() {
        let element = Element::new("message").set_text("Hello, world!");

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        assert_eq!(xml_string, "<message>Hello, world!</message>");
    }

    #[test]
    fn test_element_with_text_and_attributes() {
        let element = Element::new("message")
            .add_attribute(Attribute::new("lang", "en"))
            .set_text("Hello, world!");

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        assert_eq!(xml_string, r#"<message lang="en">Hello, world!</message>"#);
    }

    #[test]
    fn test_adding_child_overwrites_text() {
        let child = Element::new("item");
        let element = Element::new("container")
            .set_text("Initial text")
            .add_child(child);

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        let expected_xml = "<container><item/></container>";
        compare_xml!(&xml_string, expected_xml);
    }

    #[test]
    fn test_setting_text_overwrites_children() {
        let child = Element::new("item");
        let element = Element::new("container")
            .add_child(child)
            .set_text("New text");

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        assert_eq!(xml_string, "<container>New text</container>");
    }

    // Comprehensive Element tests
    #[test]
    fn test_element_with_multiple_attributes() {
        let element = Element::new("root")
            .add_attribute(Attribute::new("attr1", "value1"))
            .add_attribute(Attribute::new("attr2", "value2"))
            .add_attribute(Attribute::new("attr3", "value3"));

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        compare_xml!(
            &xml_string,
            r#"<root attr1="value1" attr2="value2" attr3="value3"/>"#
        );
    }

    #[test]
    fn test_element_with_nested_children() {
        let grandchild = Element::new("grandchild").set_text("Deep content");
        let child1 = Element::new("child1").add_child(grandchild);
        let child2 = Element::new("child2").set_text("Child 2 content");
        let element = Element::new("root").add_child(child1).add_child(child2);

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        let expected_xml = "<root><child1><grandchild>Deep content</grandchild></child1><child2>Child 2 content</child2></root>";
        compare_xml!(&xml_string, expected_xml);
    }

    #[test]
    fn test_element_add_children_bulk() {
        let children = vec![
            Element::new("child1"),
            Element::new("child2"),
            Element::new("child3"),
        ];
        let element = Element::new("root").add_children(children);

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        let expected_xml = "<root><child1/><child2/><child3/></root>";
        compare_xml!(&xml_string, expected_xml);
    }

    #[test]
    fn test_element_text_methods() {
        // Test set_text_owned
        let owned_text = String::from("Owned text");
        let element1 = Element::new("test1").set_text_owned(owned_text);

        let builder1 = Builder::new(None, element1);
        let xml_string1 = builder1.to_xml_string().unwrap();
        assert_eq!(xml_string1, "<test1>Owned text</test1>");

        // Test with_text mutable reference method
        let mut element2 = Element::new("test2");
        element2.with_text("Borrowed text");

        let builder2 = Builder::new(None, element2);
        let xml_string2 = builder2.to_xml_string().unwrap();
        assert_eq!(xml_string2, "<test2>Borrowed text</test2>");

        // Test with_text_owned mutable reference method
        let mut element3 = Element::new("test3");
        element3.with_text_owned(String::from("Owned mutable text"));

        let builder3 = Builder::new(None, element3);
        let xml_string3 = builder3.to_xml_string().unwrap();
        assert_eq!(xml_string3, "<test3>Owned mutable text</test3>");
    }

    #[test]
    fn test_element_namespace_optional() {
        let ns = Namespace::new("http://example.com");
        let element_with_ns = Element::new("test")
            .set_namespace_optional(Some(ns))
            .add_namespace_declaration("http://example.com", Some("ex"));

        let builder = Builder::new(None, element_with_ns);
        let xml_string = builder.to_xml_string().unwrap();
        compare_xml!(&xml_string, r#"<ex:test xmlns:ex="http://example.com"/>"#);

        // Test with None namespace
        let element_without_ns = Element::new("test").set_namespace_optional(None::<Namespace>);

        let builder2 = Builder::new(None, element_without_ns);
        let xml_string2 = builder2.to_xml_string().unwrap();
        compare_xml!(&xml_string2, "<test/>");
    }

    #[test]
    fn test_element_multiple_namespace_declarations() {
        let element = Element::new("root")
            .add_namespace_declaration("http://example.com/ns1", Some("ns1"))
            .add_namespace_declaration("http://example.com/ns2", Some("ns2"))
            .add_namespace_declaration("http://example.com/default", None);

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        // Check that all namespace declarations are present (order may vary due to HashMap)
        assert!(xml_string.contains(r#"xmlns:ns1="http://example.com/ns1""#));
        assert!(xml_string.contains(r#"xmlns:ns2="http://example.com/ns2""#));
        assert!(xml_string.contains(r#"xmlns="http://example.com/default""#));
        assert!(xml_string.starts_with("<root "));
        assert!(xml_string.ends_with("/>"));
    }

    // Attribute tests
    #[test]
    fn test_attribute_with_cow_borrowed() {
        let attr = Attribute::new("name", "borrowed_value");
        let element = Element::new("test").add_attribute(attr);

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        assert_eq!(xml_string, r#"<test name="borrowed_value"/>"#);
    }

    #[test]
    fn test_attribute_with_cow_owned() {
        let owned_value = String::from("owned_value");
        let attr = Attribute::new("name", owned_value);
        let element = Element::new("test").add_attribute(attr);

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        assert_eq!(xml_string, r#"<test name="owned_value"/>"#);
    }

    #[test]
    fn test_attribute_with_namespace() {
        let ns = Namespace::new("http://example.com");
        let attr = Attribute::new_with_namespace("attr", "value", Some(ns));
        let element = Element::new("test")
            .add_attribute(attr)
            .add_namespace_declaration("http://example.com", Some("ex"));

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        compare_xml!(
            &xml_string,
            r#"<test xmlns:ex="http://example.com" ex:attr="value"/>"#
        );
    }

    #[test]
    fn test_attribute_set_namespace_builder_pattern() {
        let ns = Namespace::new("http://example.com");
        let attr = Attribute::new("attr", "value").set_namespace(ns);
        let element = Element::new("test")
            .add_attribute(attr)
            .add_namespace_declaration("http://example.com", Some("ex"));

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        compare_xml!(
            &xml_string,
            r#"<test xmlns:ex="http://example.com" ex:attr="value"/>"#
        );
    }

    // Namespace tests
    #[test]
    fn test_namespace_from_str() {
        let ns: Namespace = "http://example.com".into();
        assert_eq!(ns.url, "http://example.com");
    }

    #[test]
    fn test_namespace_equality() {
        let ns1 = Namespace::new("http://example.com");
        let ns2 = Namespace::new("http://example.com");
        let ns3 = Namespace::new("http://different.com");

        assert_eq!(ns1, ns2);
        assert_ne!(ns1, ns3);
    }

    #[test]
    fn test_namespace_hash() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        let ns1 = Namespace::new("http://example.com");
        let ns2 = Namespace::new("http://example.com");

        map.insert(ns1, "value1");
        assert_eq!(map.get(&ns2), Some(&"value1"));
    }

    #[test]
    fn test_namespace_display() {
        let ns = Namespace::new("http://example.com");
        assert_eq!(format!("{}", ns), "http://example.com");
    }

    // Declaration tests
    #[test]
    fn test_declaration_basic() {
        let declaration = Declaration::new("1.0", "UTF-8");
        assert_eq!(
            format!("{}", declaration),
            r#"<?xml version="1.0" encoding="UTF-8"?>"#
        );
    }

    #[test]
    fn test_declaration_with_standalone_true() {
        let declaration = Declaration::new("1.0", "UTF-8").with_standalone(true);
        assert_eq!(
            format!("{}", declaration),
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#
        );
    }

    #[test]
    fn test_declaration_with_standalone_false() {
        let declaration = Declaration::new("1.0", "UTF-8").with_standalone(false);
        assert_eq!(
            format!("{}", declaration),
            r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>"#
        );
    }

    #[test]
    fn test_declaration_different_versions() {
        let declaration1 = Declaration::new("1.1", "ISO-8859-1");
        assert_eq!(
            format!("{}", declaration1),
            r#"<?xml version="1.1" encoding="ISO-8859-1"?>"#
        );
    }

    // Builder tests
    #[test]
    fn test_builder_without_declaration() {
        let element = Element::new("root").set_text("content");
        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        assert_eq!(xml_string, "<root>content</root>");
    }

    #[test]
    fn test_builder_with_declaration() {
        let declaration = Declaration::new("1.0", "UTF-8");
        let element = Element::new("root").set_text("content");
        let builder = Builder::new(Some(declaration), element);
        let xml_string = builder.to_xml_string().unwrap();
        let expected = r#"<?xml version="1.0" encoding="UTF-8"?> 
<root>content</root>"#;
        compare_xml!(&xml_string, expected);
    }

    // Complex namespace scenarios
    #[test]
    fn test_inherited_namespace_declarations() {
        let grandchild =
            Element::new("grandchild").set_namespace(Namespace::new("http://example.com/ns1"));

        let child = Element::new("child")
            .set_namespace(Namespace::new("http://example.com/ns2"))
            .add_child(grandchild);

        let root = Element::new("root")
            .add_namespace_declaration("http://example.com/ns1", Some("ns1"))
            .add_namespace_declaration("http://example.com/ns2", Some("ns2"))
            .add_child(child);

        let builder = Builder::new(None, root);
        let xml_string = builder.to_xml_string().unwrap();
        // Check components due to HashMap ordering
        assert!(xml_string.starts_with("<root"));
        assert!(xml_string.contains(r#"xmlns:ns1="http://example.com/ns1""#));
        assert!(xml_string.contains(r#"xmlns:ns2="http://example.com/ns2""#));
        assert!(xml_string.contains("<ns2:child>"));
        assert!(xml_string.contains("<ns1:grandchild/>"));
        assert!(xml_string.ends_with("</ns2:child></root>"));
    }

    #[test]
    fn test_namespace_override_in_child() {
        let child = Element::new("child")
            .set_namespace(Namespace::new("http://example.com/ns2"))
            .add_namespace_declaration("http://example.com/ns1", Some("override"))
            .add_namespace_declaration("http://example.com/ns2", Some("ns2"));

        let root = Element::new("root")
            .add_namespace_declaration("http://example.com/ns1", Some("ns1"))
            .add_child(child);

        let builder = Builder::new(None, root);
        let xml_string = builder.to_xml_string().unwrap();
        // Check key components due to HashMap ordering
        assert!(xml_string.contains(r#"<root xmlns:ns1="http://example.com/ns1""#));
        assert!(xml_string.contains(r#"<ns2:child"#));
        assert!(xml_string.contains(r#"xmlns:override="http://example.com/ns1""#));
        assert!(xml_string.contains(r#"xmlns:ns2="http://example.com/ns2""#));
        assert!(xml_string.ends_with("/></root>"));
    }

    #[test]
    fn test_default_namespace() {
        let element =
            Element::new("root").add_namespace_declaration("http://example.com/default", None);

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        // Test that default namespaces work when no element namespace is set
        compare_xml!(&xml_string, r#"<root xmlns="http://example.com/default"/>"#);
    }

    // Edge cases and error scenarios
    #[test]
    fn test_empty_element_name() {
        let element = Element::new("");
        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        compare_xml!(&xml_string, "</>");
    }

    #[test]
    fn test_empty_attribute_name() {
        let attr = Attribute::new("", "value");
        let element = Element::new("test").add_attribute(attr);
        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        compare_xml!(&xml_string, r#"<test ="value"/>"#);
    }

    #[test]
    fn test_empty_attribute_value() {
        let attr = Attribute::new("name", "");
        let element = Element::new("test").add_attribute(attr);
        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        compare_xml!(&xml_string, r#"<test name=""/>"#);
    }

    #[test]
    fn test_special_characters_in_text() {
        let element = Element::new("test").set_text("Text with <>&\"' characters");
        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        // Note: This test shows current behavior - proper XML should escape these
        assert_eq!(xml_string, "<test>Text with <>&\"' characters</test>");
    }

    #[test]
    fn test_special_characters_in_attributes() {
        let attr = Attribute::new("name", "value with <>&\"' characters");
        let element = Element::new("test").add_attribute(attr);
        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        // Note: This test shows current behavior - proper XML should escape these
        assert_eq!(xml_string, r#"<test name="value with <>&"' characters"/>"#);
    }

    #[test]
    fn test_unicode_content() {
        let element = Element::new("test").set_text("Hello 世界 🌍");
        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        assert_eq!(xml_string, "<test>Hello 世界 🌍</test>");
    }

    #[test]
    fn test_very_long_content() {
        let long_text = "a".repeat(10000);
        let element = Element::new("test").set_text(long_text.clone());
        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        assert_eq!(xml_string, format!("<test>{}</test>", long_text));
    }

    // Content enum specific tests
    #[test]
    fn test_content_none_display() {
        let element = Element::new("empty");
        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        compare_xml!(&xml_string, "<empty/>");
    }

    #[test]
    fn test_content_transitions() {
        // Start with None, add text
        let mut element = Element::new("test");
        element.with_text("initial text");

        // Then add child (should overwrite text)
        let element = element.add_child(Element::new("child"));

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        let expected = "<test><child/></test>";
        compare_xml!(&xml_string, expected);
    }

    #[test]
    fn test_multiple_children_with_mixed_content() {
        let child1 = Element::new("child1").set_text("Text 1");
        let child2 = Element::new("child2");
        let child3 = Element::new("child3").set_text("Text 3");

        let element = Element::new("root")
            .add_child(child1)
            .add_child(child2)
            .add_child(child3);

        let builder = Builder::new(None, element);
        let xml_string = builder.to_xml_string().unwrap();
        let expected = "<root><child1>Text 1</child1><child2/><child3>Text 3</child3></root>";
        compare_xml!(&xml_string, expected);
    }
}
