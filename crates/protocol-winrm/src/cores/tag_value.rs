use std::borrow::Cow;

use xml::{
    builder::Element,
    parser::{XmlDeserialize, XmlVisitor},
};

use crate::xml_num_value;

pub trait TagValue<'a> {
    fn append_to_element(self, element: Element<'a>) -> Element<'a>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Text<'a>(Cow<'a, str>);

impl<'a> std::convert::From<&'a str> for Text<'a> {
    fn from(value: &'a str) -> Self {
        Text(value.into())
    }
}

impl<'a> std::convert::From<String> for Text<'a> {
    fn from(value: String) -> Self {
        Text(value.into())
    }
}

impl<'a> AsRef<str> for Text<'a> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<'a> Into<Cow<'a, str>> for Text<'a> {
    fn into(self) -> Cow<'a, str> {
        self.0
    }
}

impl<'a> From<&'a Text<'a>> for &'a str {
    fn from(val: &'a Text<'a>) -> Self {
        val.0.as_ref()
    }
}

impl<'a> TagValue<'a> for Text<'a> {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        element.set_text(self.0)
    }
}

impl<'a> TagValue<'a> for () {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        element
    }
}

pub struct TextVisitor<'a> {
    value: Option<Text<'a>>,
}

impl<'a> XmlVisitor<'a> for TextVisitor<'a> {
    type Value = Text<'a>;

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        Ok(())
    }

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        let child_nodes: Vec<_> = children.collect();

        // Validate there's only one child node
        if child_nodes.len() != 1 {
            return Err(xml::XmlError::InvalidXml(format!(
                "Expected exactly one text node, found {} children",
                child_nodes.len()
            )));
        }

        let child = child_nodes.first().ok_or_else(|| {
            xml::XmlError::InvalidXml("Expected at least one child node".to_string())
        })?;

        // Validate that child node is a text node
        if !child.is_text() {
            return Err(xml::XmlError::InvalidXml(
                "Expected text node, found non-text child".to_string(),
            ));
        }

        if let Some(text) = child.text() {
            self.value = Some(Text(text.trim().into()));
        }

        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
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

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}

pub struct EmptyVisitor;

impl<'a> XmlVisitor<'a> for EmptyVisitor {
    type Value = Empty;

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        Ok(())
    }

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        let child_count = children.count();

        if child_count != 0 {
            return Err(xml::XmlError::InvalidXml(format!(
                "Expected empty tag with no children, found {child_count} children"
            )));
        }

        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        Ok(Empty)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Empty;

impl<'a> XmlDeserialize<'a> for Empty {
    type Visitor = EmptyVisitor;

    fn visitor() -> Self::Visitor {
        EmptyVisitor
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}

impl<'a> TagValue<'a> for Empty {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        element
    }
}

impl Into<Empty> for () {
    fn into(self) -> Empty {
        Empty
    }
}

xml_num_value!(U8, u8);
xml_num_value!(U32, u32);
xml_num_value!(U64, u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WsUuid(pub uuid::Uuid);

impl<'a> TagValue<'a> for WsUuid {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        element.set_text(format!("uuid:{}", self.0))
    }
}

pub struct WsUuidVisitor {
    value: Option<WsUuid>,
}

impl<'a> XmlVisitor<'a> for WsUuidVisitor {
    type Value = WsUuid;

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        Ok(())
    }

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        let child_nodes: Vec<_> = children.collect();

        // Validate there's only one child node
        if child_nodes.len() != 1 {
            return Err(xml::XmlError::InvalidXml(format!(
                "Expected exactly one text node, found {} children",
                child_nodes.len()
            )));
        }

        let child = child_nodes.first().ok_or_else(|| {
            xml::XmlError::InvalidXml("Expected at least one child node".to_string())
        })?;

        // Validate that child node is a text node
        if !child.is_text() {
            return Err(xml::XmlError::InvalidXml(
                "Expected text node, found non-text child".to_string(),
            ));
        }

        if let Some(text) = child.text() {
            let uuid_str = text.trim();
            // Handle WS-Management format: "uuid:9EC885D6-F5A4-4771-9D47-4BDF7DAAEA8C"
            let uuid_part = if let Some(stripped) = uuid_str.strip_prefix("uuid:") {
                stripped
            } else {
                uuid_str
            };
            
            match uuid::Uuid::parse_str(uuid_part) {
                Ok(uuid) => self.value = Some(WsUuid(uuid)),
                Err(_) => {
                    return Err(xml::XmlError::InvalidXml(format!(
                        "Invalid UUID format: {}",
                        uuid_str
                    )));
                }
            }
        }

        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        self.value.ok_or(xml::XmlError::InvalidXml(
            "No UUID found in the node".to_string(),
        ))
    }
}

impl<'a> XmlDeserialize<'a> for WsUuid {
    type Visitor = WsUuidVisitor;

    fn visitor() -> Self::Visitor {
        WsUuidVisitor { value: None }
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}

impl From<uuid::Uuid> for WsUuid {
    fn from(value: uuid::Uuid) -> Self {
        WsUuid(value)
    }
}

impl From<WsUuid> for uuid::Uuid {
    fn from(value: WsUuid) -> Self {
        value.0
    }
}

impl AsRef<uuid::Uuid> for WsUuid {
    fn as_ref(&self) -> &uuid::Uuid {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Time(pub f64);

impl<'a> TagValue<'a> for Time {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        element.set_text(format!("PT{:.3}S", self.0))
    }
}

pub struct TimeVisitor {
    value: Option<Time>,
}

impl<'a> XmlVisitor<'a> for TimeVisitor {
    type Value = Time;

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        Ok(())
    }

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        let child_nodes: Vec<_> = children.collect();

        // Validate there's only one child node
        if child_nodes.len() != 1 {
            return Err(xml::XmlError::InvalidXml(format!(
                "Expected exactly one text node, found {} children",
                child_nodes.len()
            )));
        }

        let child = child_nodes.first().ok_or_else(|| {
            xml::XmlError::InvalidXml("Expected at least one child node".to_string())
        })?;

        // Validate that child node is a text node
        if !child.is_text() {
            return Err(xml::XmlError::InvalidXml(
                "Expected text node, found non-text child".to_string(),
            ));
        }

        if let Some(text) = child.text() {
            let time_str = text.trim();
            // Handle WS-Management timeout format: "PT180.000S"
            if let Some(stripped) = time_str.strip_prefix("PT") {
                if let Some(time_part) = stripped.strip_suffix("S") {
                    match time_part.parse::<f64>() {
                        Ok(seconds) => self.value = Some(Time(seconds)),
                        Err(_) => {
                            return Err(xml::XmlError::InvalidXml(format!(
                                "Invalid time format: {}",
                                time_str
                            )));
                        }
                    }
                } else {
                    return Err(xml::XmlError::InvalidXml(format!(
                        "Invalid time format, missing 'S' suffix: {}",
                        time_str
                    )));
                }
            } else {
                return Err(xml::XmlError::InvalidXml(format!(
                    "Invalid time format, missing 'PT' prefix: {}",
                    time_str
                )));
            }
        }

        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        self.value.ok_or(xml::XmlError::InvalidXml(
            "No time found in the node".to_string(),
        ))
    }
}

impl<'a> XmlDeserialize<'a> for Time {
    type Visitor = TimeVisitor;

    fn visitor() -> Self::Visitor {
        TimeVisitor { value: None }
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}

impl From<f64> for Time {
    fn from(value: f64) -> Self {
        Time(value)
    }
}

impl From<u32> for Time {
    fn from(value: u32) -> Self {
        Time(value as f64)
    }
}

impl From<Time> for f64 {
    fn from(value: Time) -> Self {
        value.0
    }
}

impl AsRef<f64> for Time {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}
