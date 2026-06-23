use std::borrow::Cow;

use ironposh_xml::{XmlError, builder::Element, mapping::FromXml, parser::Node};

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

impl std::convert::From<String> for Text<'_> {
    fn from(value: String) -> Self {
        Text(value.into())
    }
}

impl AsRef<str> for Text<'_> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<'a> From<Text<'a>> for Cow<'a, str> {
    fn from(val: Text<'a>) -> Self {
        val.0
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

impl<'a> FromXml<'a> for Text<'a> {
    fn from_xml(node: Node<'a, 'a>) -> Result<Self, XmlError> {
        Ok(Text(node.text().unwrap_or("").trim().into()))
    }
}

impl<'a> TagValue<'a> for () {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        element
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Empty;

impl<'a> TagValue<'a> for Empty {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        element
    }
}

impl<'a> FromXml<'a> for Empty {
    fn from_xml(_node: Node<'a, 'a>) -> Result<Self, XmlError> {
        Ok(Empty)
    }
}

impl From<()> for Empty {
    fn from(_value: ()) -> Self {
        Self
    }
}

xml_num_value!(U8, u8);
xml_num_value!(U32, u32);
xml_num_value!(U64, u64);
xml_num_value!(I32, i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WsUuid(pub uuid::Uuid);

impl<'a> TagValue<'a> for WsUuid {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        element.set_text(format!("uuid:{}", self.0))
    }
}

impl<'a> FromXml<'a> for WsUuid {
    fn from_xml(node: Node<'a, 'a>) -> Result<Self, XmlError> {
        let text = node.text().unwrap_or("").trim();
        // WS-Management prefixes UUIDs with "uuid:" — strip it if present.
        let raw = text.strip_prefix("uuid:").unwrap_or(text);
        uuid::Uuid::parse_str(raw)
            .map(WsUuid)
            .map_err(|_| XmlError::InvalidXml(format!("Invalid UUID format: {text}")))
    }
}

impl From<uuid::Uuid> for WsUuid {
    fn from(value: uuid::Uuid) -> Self {
        Self(value)
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

impl<'a> FromXml<'a> for Time {
    fn from_xml(node: Node<'a, 'a>) -> Result<Self, XmlError> {
        // WS-Management timeout format: "PT180.000S".
        let text = node.text().unwrap_or("").trim();
        let seconds = text
            .strip_prefix("PT")
            .and_then(|s| s.strip_suffix('S'))
            .ok_or_else(|| XmlError::InvalidXml(format!("Invalid time format: {text}")))?
            .parse::<f64>()
            .map_err(|_| XmlError::InvalidXml(format!("Invalid time value: {text}")))?;
        Ok(Time(seconds))
    }
}

impl From<f64> for Time {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<u32> for Time {
    fn from(value: u32) -> Self {
        Self(f64::from(value))
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

#[derive(Debug, Clone)]
pub enum ReadOnlyUnParsed<'a> {
    Node(Node<'a, 'a>),
    Children(Vec<Node<'a, 'a>>),
}

impl<'a> TagValue<'a> for ReadOnlyUnParsed<'a> {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        element
    }
}

impl<'a> FromXml<'a> for ReadOnlyUnParsed<'a> {
    fn from_xml(node: Node<'a, 'a>) -> Result<Self, XmlError> {
        Ok(ReadOnlyUnParsed::Node(node))
    }
}
