use protocol_macros::{SimpleTagValue, SimpleXmlDeserialize};
use tracing::warn;

use crate::cores::{DesiredStream, Stream, Tag, TagName, TagValue, Text};
use xml::{
    XmlError,
    builder::Element,
    parser::{XmlDeserialize, XmlVisitor},
};

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, SimpleXmlDeserialize)]
pub struct ReceiveValue<'a> {
    pub desired_stream: Tag<'a, Text<'a>, DesiredStream>,
}

// ReceiveResponse main structure
#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct ReceiveResponseValue<'a> {
    pub streams: Vec<Tag<'a, Text<'a>, Stream>>,
}

impl<'a> TagValue<'a> for ReceiveResponseValue<'a> {
    fn append_to_element(self, mut element: Element<'a>) -> Element<'a> {
        for stream in self.streams {
            element = element.add_child(stream.into_element());
        }

        element
    }
}

pub struct ReceiveResponseVisitor<'a> {
    pub streams: Vec<Tag<'a, Text<'a>, Stream>>,
}

impl<'a> XmlVisitor<'a> for ReceiveResponseVisitor<'a> {
    type Value = ReceiveResponseValue<'a>;

    fn visit_children(
        &mut self,
        nodes: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        for node in nodes {
            match (node.tag_name().name(), node.tag_name().namespace()) {
                (Stream::TAG_NAME, Stream::NAMESPACE) => {
                    let stream = Tag::from_node(node)?;
                    self.streams.push(stream);
                }
                _ => {
                    warn!(
                        "Unexpected tag in ReceiveResponse: {}",
                        node.tag_name().name()
                    );
                }
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, XmlError> {
        Ok(ReceiveResponseValue {
            streams: self.streams,
        })
    }
}

impl<'a> XmlDeserialize<'a> for ReceiveResponseValue<'a> {
    type Visitor = ReceiveResponseVisitor<'a>;

    fn visitor() -> Self::Visitor {
        ReceiveResponseVisitor {
            streams: Vec::new(),
        }
    }
}
