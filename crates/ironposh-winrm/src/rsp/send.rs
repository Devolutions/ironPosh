use crate::cores::{
    Tag, TagValue, Text,
    tag_name::{Stream, TagName},
};
use ironposh_xml::{
    XmlError,
    builder::Element,
    parser::{XmlDeserialize, XmlVisitor},
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

// Minimal visitor for deserialization (SendValue is primarily for serialization)
pub struct SendValueVisitor<'a> {
    pub streams: Vec<Tag<'a, Text<'a>, Stream>>,
}

impl<'a> XmlVisitor<'a> for SendValueVisitor<'a> {
    type Value = SendValue<'a>;

    fn visit_children(
        &mut self,
        nodes: impl Iterator<Item = ironposh_xml::parser::Node<'a, 'a>>,
    ) -> Result<(), ironposh_xml::XmlError> {
        for node in nodes {
            if matches!(
                (node.tag_name().name(), node.tag_name().namespace()),
                (Stream::TAG_NAME, Stream::NAMESPACE)
            ) {
                let stream = Tag::from_node(node)?;
                self.streams.push(stream);
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, XmlError> {
        Ok(SendValue {
            streams: self.streams,
        })
    }
}

impl<'a> XmlDeserialize<'a> for SendValue<'a> {
    type Visitor = SendValueVisitor<'a>;

    fn visitor() -> Self::Visitor {
        SendValueVisitor {
            streams: Vec::new(),
        }
    }
}
