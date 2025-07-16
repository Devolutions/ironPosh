use std::borrow::Cow;

use xml::{
    builder::Element,
    parser::{XmlDeserialize, XmlVisitor},
};

pub trait TagValue<'a> {
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Text<'a>(&'a str);

impl<'a> From<&'a str> for Text<'a> {
    fn from(value: &'a str) -> Self {
        Text(value)
    }
}

impl<'a> Into<&'a str> for Text<'a> {
    fn into(self) -> &'a str {
        self.0
    }
}

impl<'a> TagValue<'a> for Text<'a> {
    fn into_element(self, name: &'a str, namespace: Option<&'a str>) -> Element<'a> {
        let mut element = Element::new(name).set_text(self.0.as_ref());
        if let Some(ns) = namespace {
            element = element.set_namespace(ns);
        }
        element
    }
}

pub struct TextVisitor<'a> {
    value: Option<Text<'a>>,
}

impl<'a> XmlVisitor<'a> for TextVisitor<'a> {
    type Value = Text<'a>;

    fn visit(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        if let Some(text) = node.text() {
            self.value = Some(Text(text.trim()));
        }
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        self.value.ok_or(xml::XmlError::InvalidXml(
            "No text found in the node".to_string(),
        ))
    }
}

impl<'a> XmlDeserialize<'a> for Text<'a> {
    type Visitor = TextVisitor<'a>;

    fn visitor() -> Self::Visitor {
        TextVisitor { value: None }
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError<'a>> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}

impl<'a> TagValue<'a> for () {
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a> {
        let mut element = Element::new(name);

        if let Some(ns) = namespace {
            element = element.set_namespace(ns);
        }

        element
    }
}
