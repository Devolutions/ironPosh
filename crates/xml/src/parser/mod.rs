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

pub fn parse<'a>(xml: &'a str) -> Result<Document<'a>, crate::XmlError> {
    roxmltree::Document::parse(xml).map_err(crate::XmlError::ParserError)
}

/// =========== 1.  The Visitor every type supplies  ===========
pub trait XmlVisitor<'a> {
    /// Rust value produced after the whole subtree was walked.
    type Value;

    /// Visit a specific node - used by Tag types that need to match by name
    /// Default implementation calls visit_children for backward compatibility
    fn visit_children(
        &mut self,
        _node: impl Iterator<Item = crate::parser::Node<'a, 'a>>,
    ) -> Result<(), crate::XmlError> {
        Err(crate::XmlError::NotSupposeToBeCalled {
            extra_info: "Default visit_children called, should be overridden or not called at all".to_string(),
        })
    }

    /// Visit the children of a node - used by TagValue types that process content
    /// Default implementation does nothing
    fn visit_node(&mut self, _node: crate::parser::Node<'a, 'a>) -> Result<(), crate::XmlError> {
        Err(crate::XmlError::NotSupposeToBeCalled {
            extra_info: "Default visit_node called, should be overridden or not called at all".to_string(),
        })
    }

    /// Return the finished value after traversal.
    fn finish(self) -> Result<Self::Value, XmlError>;
}

/// =========== 2.  Blanket “Deserializer” driver  =============
pub struct NodeDeserializer<'a> {
    root: roxmltree::Node<'a, 'a>,
}

impl<'a> NodeDeserializer<'a> {
    pub fn new(root: roxmltree::Node<'a, 'a>) -> Self {
        Self { root }
    }

    /// Drive any visitor over the subtree rooted at `self.root`
    pub fn deserialize<V>(self, mut visitor: V) -> Result<V::Value, XmlError>
    where
        V: XmlVisitor<'a>,
    {
        visitor.visit_node(self.root)?;
        visitor.finish()
    }
}

/// =========== 3.  Per-type convenience trait  ================
pub trait XmlDeserialize<'a>: Sized {
    /// “Associated visitor” type that knows how to build Self
    type Visitor: XmlVisitor<'a, Value = Self>;

    /// Create the visitor that will build Self.
    fn visitor() -> Self::Visitor;

    /// One-liner users will call.
    fn from_node(node: roxmltree::Node<'a, 'a>) -> Result<Self, XmlError> {
        NodeDeserializer::new(node).deserialize(Self::visitor())
    }

    fn from_children(
        children: impl Iterator<Item = crate::parser::Node<'a, 'a>>,
    ) -> Result<Self, XmlError> {
        let mut visitor = Self::visitor();
        visitor.visit_children(children)?;
        visitor.finish()
    }
}
