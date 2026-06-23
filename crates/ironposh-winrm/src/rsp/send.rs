use crate::cores::{
    Tag, TagValue, Text,
    tag_name::{Stream, TagName},
};
use ironposh_xml::{
    XmlError,
    builder::Element,
    mapping::{FromXml, NodeExt},
};

/// Value for Send element containing multiple Stream elements
/// Each Stream contains a base64-encoded PSRP fragment
#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct SendValue<'a> {
    pub streams: Vec<Tag<'a, Text<'a>, Stream>>,
}

impl<'a> TagValue<'a> for SendValue<'a> {
    fn append_to_element(self, mut element: Element<'a>) -> Element<'a> {
        // Add each Stream tag as a child element
        for stream in self.streams {
            element = element.add_child(stream.into_element());
        }
        element
    }
}

impl<'a> FromXml<'a> for SendValue<'a> {
    fn from_xml(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, XmlError> {
        let mut streams = Vec::new();
        for child in node.children() {
            if child.is_element_named(Stream::NAMESPACE, Stream::TAG_NAME) {
                streams.push(Tag::from_xml(child)?);
            }
        }
        Ok(SendValue { streams })
    }
}
