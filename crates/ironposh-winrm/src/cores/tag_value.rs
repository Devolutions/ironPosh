use std::borrow::Cow;

use ironposh_xml::{XmlError, builder::Element, mapping::FromXml, parser::Node};

use crate::xml_num_value;

pub trait TagValue<'a> {
    fn append_to_element(self, element: Element<'a>) -> Element<'a>;
}

/// The text content of a leaf element, rejecting mixed content. A text-valued
/// element (`Text`, `WsUuid`, `Time`, numerics) must not contain child elements;
/// silently truncating such malformed input would let it slip through.
pub(crate) fn leaf_text<'a>(node: Node<'a, 'a>) -> Result<Cow<'a, str>, XmlError> {
    // A text leaf carries no child elements; an element child is mixed content
    // that would corrupt the value. Comments/PIs are tolerated, and text runs are
    // concatenated — a comment between two runs splits one logical value into two
    // text nodes, so reading only the first would silently truncate it.
    if node.children().any(|child| child.is_element()) {
        return Err(XmlError::InvalidXml(format!(
            "<{}> is a text leaf but contains a child element",
            node.tag_name().name()
        )));
    }
    // Only text nodes contribute — a comment node's own text would otherwise leak in.
    let mut runs = node
        .children()
        .filter(Node::is_text)
        .filter_map(|c| c.text());
    let first = runs.next().unwrap_or("");
    runs.next().map_or_else(
        || Ok(Cow::Borrowed(first.trim())),
        |second| {
            let mut combined = String::from(first);
            combined.push_str(second);
            for run in runs {
                combined.push_str(run);
            }
            Ok(Cow::Owned(combined.trim().to_string()))
        },
    )
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
        Ok(Self(leaf_text(node)?))
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
    fn from_xml(node: Node<'a, 'a>) -> Result<Self, XmlError> {
        // `leaf_text` rejects child elements; an empty tag additionally must have
        // no non-whitespace text.
        if !leaf_text(node)?.is_empty() {
            return Err(XmlError::InvalidXml(format!(
                "<{}> must be empty but has text content",
                node.tag_name().name()
            )));
        }
        Ok(Self)
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
        let text = leaf_text(node)?;
        let text = text.as_ref();
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
        let text = leaf_text(node)?;
        let seconds = text
            .strip_prefix("PT")
            .and_then(|s| s.strip_suffix('S'))
            .ok_or_else(|| XmlError::InvalidXml(format!("Invalid time format: {text}")))?
            .parse::<f64>()
            .map_err(|_| XmlError::InvalidXml(format!("Invalid time value: {text}")))?;
        Ok(Self(seconds))
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

#[cfg(test)]
mod tests {
    use super::*;
    use ironposh_xml::parser::parse;

    #[test]
    fn text_leaf_rejects_mixed_content() {
        let doc = parse("<x>text<child/></x>").unwrap();
        assert!(Text::from_xml(doc.root_element()).is_err());
    }

    #[test]
    fn text_leaf_trims_surrounding_whitespace() {
        let doc = parse("<x>  hello  </x>").unwrap();
        assert_eq!(
            Text::from_xml(doc.root_element()).unwrap().as_ref(),
            "hello"
        );
    }

    #[test]
    fn text_leaf_tolerates_comment_and_concatenates_runs() {
        let doc = parse("<x>foo<!--c-->bar</x>").unwrap();
        assert_eq!(
            Text::from_xml(doc.root_element()).unwrap().as_ref(),
            "foobar"
        );
    }

    #[test]
    fn numeric_leaf_rejects_mixed_content() {
        let doc = parse("<x>1<child/></x>").unwrap();
        assert!(U32::from_xml(doc.root_element()).is_err());
    }

    #[test]
    fn wsuuid_leaf_rejects_mixed_content() {
        let doc = parse("<x>uuid:2d6534d0-6b12-40e3-b773-cba26459cfa8<child/></x>").unwrap();
        assert!(WsUuid::from_xml(doc.root_element()).is_err());
    }

    #[test]
    fn empty_rejects_text_content() {
        let doc = parse("<x>nope</x>").unwrap();
        assert!(Empty::from_xml(doc.root_element()).is_err());
    }

    #[test]
    fn empty_accepts_whitespace_only() {
        let doc = parse("<x>   </x>").unwrap();
        assert!(Empty::from_xml(doc.root_element()).is_ok());
    }
}
