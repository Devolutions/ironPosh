pub use roxmltree::*;

use crate::XmlError;

impl<'a> TryFrom<crate::parser::Node<'a, 'a>> for crate::builder::Element<'a> {
    type Error = crate::XmlError;

    fn try_from(value: roxmltree::Node<'a, 'a>) -> Result<Self, Self::Error> {
        if !value.is_element() {
            return Err(crate::XmlError::InvalidNodeType {
                expected: NodeType::Element,
                found: value.node_type(),
            });
        }

        let tag_name = value.tag_name();
        let namespace = tag_name.namespace().map(crate::builder::Namespace::new);

        let name = tag_name.name();

        let mut element = crate::builder::Element::new(name);

        element = element.set_namespace_optional(namespace);

        Ok(element)
    }
}

pub fn parse(xml: &str) -> Result<Document<'_>, crate::XmlError> {
    roxmltree::Document::parse(xml).map_err(crate::XmlError::ParserError)
}

/// Legacy visitor-based deserialization.
///
/// Superseded by [`crate::mapping::FromXml`] (direct, namespace-aware) for the
/// WinRM/SOAP layer. Retained because `ironposh-psrp`'s CLIXML primitive layer
/// still rides on it; once that migrates, this can go.
pub trait XmlVisitor<'a> {
    type Value;

    fn visit_children(
        &mut self,
        _node: impl Iterator<Item = crate::parser::Node<'a, 'a>>,
    ) -> Result<(), crate::XmlError> {
        Err(crate::XmlError::NotSupposeToBeCalled {
            extra_info: "Default visit_children called, should be overridden or not called at all"
                .to_string(),
        })
    }

    fn visit_node(&mut self, _node: crate::parser::Node<'a, 'a>) -> Result<(), crate::XmlError> {
        Err(crate::XmlError::NotSupposeToBeCalled {
            extra_info: "Default visit_node called, should be overridden or not called at all"
                .to_string(),
        })
    }

    fn finish(self) -> Result<Self::Value, XmlError>;
}

pub struct NodeDeserializer<'a> {
    root: roxmltree::Node<'a, 'a>,
}

impl<'a> NodeDeserializer<'a> {
    pub fn new(root: roxmltree::Node<'a, 'a>) -> Self {
        Self { root }
    }

    pub fn deserialize<V>(self, mut visitor: V) -> Result<V::Value, XmlError>
    where
        V: XmlVisitor<'a>,
    {
        visitor.visit_node(self.root)?;
        visitor.finish()
    }
}

pub trait XmlDeserialize<'a>: Sized {
    type Visitor: XmlVisitor<'a, Value = Self>;

    fn visitor() -> Self::Visitor;

    fn from_node(node: roxmltree::Node<'a, 'a>) -> Result<Self, XmlError> {
        NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}
