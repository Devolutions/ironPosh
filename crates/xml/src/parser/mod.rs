pub use roxmltree::*;

use crate::XmlError;

impl<'a> TryFrom<roxmltree::Node<'a, 'a>> for crate::builder::Element<'a> {
    type Error = crate::XmlError<'a>;

    fn try_from(value: roxmltree::Node<'a, 'a>) -> Result<Self, Self::Error> {
        if !value.is_element() {
            return Err(crate::XmlError::InvalidNodeType {
                expected: NodeType::Element,
                found: value.node_type(),
            });
        }

        let tag_name = value.tag_name();
        let namespace = tag_name
            .namespace()
            .map(|ns| crate::builder::Namespace::new(ns));

        let name = tag_name.name();

        let mut element = crate::builder::Element::new(name);

        element = element.set_namespace_optional(namespace);

        Ok(element)
    }
}

pub fn parse<'a>(xml: &'a str) -> Result<Document<'a>, roxmltree::Error> {
    roxmltree::Document::parse(xml)
}

/// =========== 1.  The Visitor every type supplies  ===========
pub trait XmlVisitor<'a> {
    /// Rust value produced after the whole subtree was walked.
    type Value;

    /// Called once per node; implementer decides what to do with it
    /// (recurse, grab attributes, read text, etc.).
    fn visit(&mut self, node: roxmltree::Node<'a, 'a>) -> Result<(), crate::XmlError<'a>>;

    /// Return the finished value after traversal.
    fn finish(self) -> Result<Self::Value, XmlError<'a>>;
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
    pub fn deserialize<V>(self, mut visitor: V) -> Result<V::Value, XmlError<'a>>
    where
        V: XmlVisitor<'a>,
    {
        visitor.visit(self.root)?;
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
    fn from_node(node: roxmltree::Node<'a, 'a>) -> Result<Self, XmlError<'a>> {
        NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}
