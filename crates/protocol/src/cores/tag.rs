use xml::builder::Element;
use xml::parser::{XmlDeserialize, XmlVisitor};
use tracing::debug;

use crate::cores::Namespace;
use crate::cores::namespace::NamespaceDeclaration;
use crate::cores::tag_value::Text;

use super::attribute::Attribute;
use super::tag_name::TagName;
use super::tag_value::TagValue;

#[derive(Debug, Clone)]
pub struct Tag<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    pub value: V,
    pub attributes: Vec<Attribute>,
    pub namespaces: NamespaceDeclaration,
    pub namespace: Option<Namespace>,

    __phantom: std::marker::PhantomData<&'a V>,
    __phantom_name: std::marker::PhantomData<N>,
}

pub struct TagVisitor<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    pub tag: Option<V>,
    pub attributes: Vec<Attribute>,
    pub namespaces: NamespaceDeclaration,
    pub namespace: Option<Namespace>,
    __phantom: std::marker::PhantomData<&'a N>,
}

pub struct NodeDeserializer<'a> {
    root: xml::parser::Node<'a, 'a>,
}

impl<'a> NodeDeserializer<'a> {
    pub fn new(root: xml::parser::Node<'a, 'a>) -> Self {
        Self { root }
    }

    /// Drive any visitor over the subtree rooted at `self.root`
    pub fn deserialize<V>(self, mut visitor: V) -> Result<V::Value, xml::XmlError<'a>>
    where
        V: XmlVisitor<'a>,
    {
        visitor.visit_node(self.root)?;
        visitor.finish()
    }
}

impl<'a, V, N> XmlVisitor<'a> for TagVisitor<'a, V, N>
where
    V: TagValue<'a> + 'a + XmlDeserialize<'a>,
    N: TagName,
{
    type Value = Tag<'a, V, N>;

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        debug!("TagVisitor visiting node: tag_name='{}', expected='{}', namespace={:?}", 
               node.tag_name().name(), N::TAG_NAME, node.tag_name().namespace());
        
        if node.is_element() && node.tag_name().name() == N::TAG_NAME {
            debug!("Tag name matches! Processing children...");
            let value =
                V::from_children(node.children().filter(|c| c.is_element() || c.is_text()))?;
            self.tag = Some(value);
            debug!("Successfully created tag value");
        } else {
            debug!("Tag name doesn't match or node is not an element");
        }

        for attr in node.attributes() {
            debug!("Processing attribute: name='{}', value='{}'", attr.name(), attr.value());
            if let Ok(attribute) = Attribute::from_node(node) {
                debug!("Successfully parsed attribute: {:?}", attribute);
                self.attributes.push(attribute);
            } else {
                debug!("Failed to parse attribute: {}", attr.name());
            }
        }

        self.namespaces = NamespaceDeclaration::from_node(node)?;

        self.namespace = Namespace::from_node(node).ok();

        Ok(())
    }    fn visit_children(
        &mut self,
        _children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError<'a>> {
        Err(xml::XmlError::InvalidXml(
            "Expected a single tag, found multiple children".to_string(),
        ))
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        self.tag
            .map(|value| Tag {
                value,
                attributes: self.attributes,
                namespaces: self.namespaces,
                namespace: self.namespace,
                __phantom: std::marker::PhantomData,
                __phantom_name: std::marker::PhantomData,
            })
            .ok_or(xml::XmlError::InvalidXml(
                "TagVisitor did not find a valid tag".to_string(),
            ))
    }
}

impl<'a, V, N> XmlDeserialize<'a> for Tag<'a, V, N>
where
    V: TagValue<'a> + XmlDeserialize<'a>,
    N: TagName + 'a,
{
    type Visitor = TagVisitor<'a, V, N>;

    fn visitor() -> Self::Visitor {
        TagVisitor {
            tag: None,
            attributes: Vec::new(),
            namespaces: NamespaceDeclaration::new(),
            namespace: None,
            __phantom: std::marker::PhantomData,
        }
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError<'a>> {
        NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}

impl<'a, V, N> AsRef<V> for Tag<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    fn as_ref(&self) -> &V {
        &self.value
    }
}

impl<'a, V, N> Tag<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    pub fn new(value: V) -> Self {
        Self {
            value,
            attributes: Vec::new(),
            namespaces: NamespaceDeclaration::new(),
            namespace: None,
            __phantom: std::marker::PhantomData,
            __phantom_name: std::marker::PhantomData,
        }
    }

    pub fn into_element(self) -> Element<'a> {
        self.value.into_element(N::TAG_NAME, N::NAMESPACE)
    }
}

impl<'a, V, N> From<Tag<'a, V, N>> for Element<'a>
where
    V: TagValue<'a>,
    N: TagName,
{
    fn from(val: Tag<'a, V, N>) -> Self {
        val.into_element()
    }
}

impl<'a, N> From<&'a str> for Tag<'a, Text<'a>, N>
where
    N: TagName,
{
    fn from(value: &'a str) -> Self {
        Tag::new(value.into())
    }
}
