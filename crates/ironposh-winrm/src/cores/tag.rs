use ironposh_xml::builder::Element;
use ironposh_xml::parser::{XmlDeserialize, XmlVisitor};
use tracing::{debug, trace, warn};

use crate::cores::namespace::NamespaceDeclaration;
use crate::cores::tag_value::{Text, U32};
use crate::cores::{Namespace, WsUuid};
use crate::impl_tag_from;

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
    pub attributes: Vec<Attribute<'a>>,
    /// The namespaces are the declaration of namespaces used in this tag.
    /// For example
    /// <s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
    /// would have a namespace declaration for "s" with the URI "http://schemas.xmlsoap.org/soap/envelope/".
    pub namespaces_declaration: NamespaceDeclaration,

    __phantom: std::marker::PhantomData<&'a V>,
    __phantom_name: std::marker::PhantomData<N>,
}

pub struct TagNameHolder<'a, N, V>
where
    N: TagName,
    V: TagValue<'a>,
{
    name: N,
    attributes: Option<Vec<Attribute<'a>>>,
    namespaces_declaration: NamespaceDeclaration,
    __phantom: std::marker::PhantomData<&'a V>,
}

impl<'a, N, V> TagNameHolder<'a, N, V>
where
    N: TagName,
    V: TagValue<'a>,
{
    pub fn with_value(self, value: V) -> Tag<'a, V, N> {
        let mut tag = Tag::new(value).with_name(self.name);

        if let Some(attrs) = self.attributes {
            for attr in attrs {
                tag = tag.with_attribute(attr);
            }
        }

        for declaration in self.namespaces_declaration {
            tag = tag.with_declaration(declaration);
        }

        tag
    }

    pub fn with_attribute(mut self, attribute: Attribute<'a>) -> Self {
        if let Some(ref mut attrs) = self.attributes {
            attrs.push(attribute);
        } else {
            self.attributes = Some(vec![attribute]);
        }
        self
    }

    pub fn with_declaration(mut self, declaration: Namespace) -> Self {
        self.namespaces_declaration.push(declaration);
        self
    }
}

impl<'a, V, N> Tag<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    pub fn new(value: impl Into<V>) -> Self {
        Self {
            value: value.into(),
            attributes: Vec::new(),
            namespaces_declaration: NamespaceDeclaration::new(),
            __phantom: std::marker::PhantomData,
            __phantom_name: std::marker::PhantomData,
        }
    }

    pub fn from_name(name: N) -> TagNameHolder<'a, N, V>
    where
        N: TagName,
    {
        TagNameHolder {
            name,
            attributes: None,
            namespaces_declaration: NamespaceDeclaration::new(),
            __phantom: std::marker::PhantomData,
        }
    }

    /// Does not do anything, just returns self.
    /// This is useful for compiler to infer the type of `N` when using `Tag::new`.
    pub fn with_name(self, _name: N) -> Self {
        self
    }

    pub fn with_attribute(mut self, attribute: Attribute<'a>) -> Self {
        self.attributes.push(attribute);
        self
    }

    pub fn with_declaration(mut self, declaration: Namespace) -> Self {
        self.namespaces_declaration.push(declaration);
        self
    }

    pub fn into_element(self) -> Element<'a> {
        let mut element = Element::new(N::TAG_NAME);
        if let Some(ns) = N::NAMESPACE {
            element = element.set_namespace(ns);
        }

        // Add namespace declarations to the element
        for namespace in self.namespaces_declaration.namespaces() {
            let (url, alias) = namespace.as_tuple();
            element = element.add_namespace_declaration(url, alias);
        }

        for attribute in self.attributes {
            element = element.add_attribute(attribute.into());
        }

        self.value.append_to_element(element)
    }

    pub fn name(&self) -> &'static str {
        N::TAG_NAME
    }

    pub fn clone_value(&self) -> V
    where
        V: Clone,
    {
        self.value.clone()
    }
}

impl<'a, V, N> From<V> for Tag<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName + 'a,
{
    fn from(value: V) -> Self {
        Tag::new(value)
    }
}

pub struct TagVisitor<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    pub tag: Option<V>,
    pub attributes: Vec<Attribute<'a>>,
    pub namespaces: NamespaceDeclaration,
    pub namespace: Option<Namespace>,
    __phantom: std::marker::PhantomData<&'a N>,
}

pub struct NodeDeserializer<'a> {
    root: ironposh_xml::parser::Node<'a, 'a>,
}

impl<'a> NodeDeserializer<'a> {
    pub fn new(root: ironposh_xml::parser::Node<'a, 'a>) -> Self {
        Self { root }
    }

    /// Drive any visitor over the subtree rooted at `self.root`
    pub fn deserialize<V>(self, mut visitor: V) -> Result<V::Value, ironposh_xml::XmlError>
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

    fn visit_node(
        &mut self,
        node: ironposh_xml::parser::Node<'a, 'a>,
    ) -> Result<(), ironposh_xml::XmlError> {
        trace!(
            expected_tag_name = N::TAG_NAME,
            actual_tag_name = node.tag_name().name(),
            namespace = ?node.tag_name().namespace(),
            "TagVisitor visiting node",
        );

        if node.is_element() && node.tag_name().name() == N::TAG_NAME {
            let value =
                V::from_children(node.children().filter(|c| c.is_element() || c.is_text()))?;
            self.tag = Some(value);
            trace!(tag_name = N::TAG_NAME, "Successfully created tag value");
        } else {
            warn!(
                actual_tag_name = node.tag_name().name(),
                expected_tag_name = N::TAG_NAME,
                "Tag name doesn't match or node is not an element"
            );
        }

        for attr in node.attributes() {
            trace!(
                name = attr.name(),
                value = attr.value(),
                "Processing attribute"
            );
            if let Ok(attribute) = Attribute::from_attribute(attr) {
                trace!("Successfully parsed attribute: {:?}", attribute);
                self.attributes.push(attribute);
            } else {
                debug!("Failed to parse attribute: {}", attr.name());
            }
        }

        self.namespaces = NamespaceDeclaration::from_node(node)?;

        self.namespace = Namespace::from_node(node).ok();

        Ok(())
    }

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = ironposh_xml::parser::Node<'a, 'a>>,
    ) -> Result<(), ironposh_xml::XmlError> {
        for child in children {
            if child.is_element()
                && child.tag_name().name() == N::TAG_NAME
                && child.tag_name().namespace() == N::NAMESPACE
            {
                debug!("Visiting child node: {}", child.tag_name().name());
                self.visit_node(child)?;
            } else {
                warn!(
                    "Skipping child node: {} (namespace: {:?})",
                    child.tag_name().name(),
                    child.tag_name().namespace()
                );

                return Err(ironposh_xml::XmlError::InvalidXml(format!(
                    "Unexpected child node: {} (namespace: {:?})",
                    child.tag_name().name(),
                    child.tag_name().namespace()
                )));
            }
        }

        Ok(())
    }

    fn finish(self) -> Result<Self::Value, ironposh_xml::XmlError> {
        self.tag
            .map(|value| Tag {
                value,
                attributes: self.attributes,
                namespaces_declaration: self.namespaces,
                __phantom: std::marker::PhantomData,
                __phantom_name: std::marker::PhantomData,
            })
            .ok_or(ironposh_xml::XmlError::InvalidXml(format!(
                "Tag visitor cannot built for tag: {}",
                N::TAG_NAME
            )))
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

    fn from_node(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, ironposh_xml::XmlError> {
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

impl<'a, V, N> From<Tag<'a, V, N>> for Element<'a>
where
    V: TagValue<'a>,
    N: TagName,
{
    fn from(val: Tag<'a, V, N>) -> Self {
        val.into_element()
    }
}

impl<'a, V, N> TagValue<'a> for Tag<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        let inner_element = self.into_element();
        element.add_child(inner_element)
    }
}

impl_tag_from!(&'a str => Tag<'a, Text<'a>, N>);
impl_tag_from!(String => Tag<'a, Text<'a>, N>);
impl_tag_from!(u32 => Tag<'a, U32, N>);
impl_tag_from!(uuid::Uuid => Tag<'a, WsUuid, N>);
