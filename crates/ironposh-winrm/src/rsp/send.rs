use crate::cores::{Stream, StreamTag, TagName, TagValue};
use crate::tag;
use ironposh_xml::{
    XmlError,
    builder::Element,
    mapping::{FromXml, NodeExt},
};

// `Send` here is this type alias, not `std::marker::Send` — don't write a bare
// `Send` trait bound in modules that import it.
tag!(Send = SendValue<'a> => WsmanShell);

/// Value for Send element containing multiple Stream elements
/// Each Stream contains a base64-encoded PSRP fragment
#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct SendValue<'a> {
    pub streams: Vec<Stream<'a>>,
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
        ironposh_xml::mapping::reject_mixed_content(node)?;
        let mut streams = Vec::new();
        for child in node.children() {
            if child.is_element_named(StreamTag::NAMESPACE, StreamTag::TAG_NAME) {
                streams.push(Stream::from_xml(child)?);
            }
        }
        Ok(SendValue { streams })
    }
}
