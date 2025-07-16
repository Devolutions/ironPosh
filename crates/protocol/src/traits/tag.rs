use std::fmt::Debug;
use xml::builder::Element;
use xml::parser::{XmlDeserialize, XmlVisitor};

use crate::traits::Tag1;
use crate::traits::tag_value::Text;

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

    __phantom: std::marker::PhantomData<&'a V>,
    __phantom_name: std::marker::PhantomData<N>,
}

pub struct TagVisitor<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    pub tag: Option<V>,
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
        visitor.visit(self.root)?;
        visitor.finish()
    }
}

impl<'a, V, N> XmlVisitor<'a> for TagVisitor<'a, V, N>
where
    V: TagValue<'a> + 'a + XmlDeserialize<'a>,
    N: TagName,
{
    type Value = Tag<'a, V, N>;

    fn visit(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        if node.is_element() && node.tag_name().name() == N::TAG_NAME {
            let value = V::from_node(node)?;
            self.tag = Some(value);
        }
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        self.tag
            .map(|value| Tag {
                value,
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
            __phantom: std::marker::PhantomData,
        }
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError<'a>> {
        NodeDeserializer::new(node)
            .deserialize(Self::visitor())
            .map_err(|e| xml::XmlError::from(e))
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
            __phantom: std::marker::PhantomData,
            __phantom_name: std::marker::PhantomData,
        }
    }

    pub fn into_element(self) -> Element<'a> {
        self.value.into_element(N::TAG_NAME, N::NAMESPACE)
    }
}

impl<'a, V, N> Into<Element<'a>> for Tag<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    fn into(self) -> Element<'a> {
        self.into_element()
    }
}

impl<'a, N, V> TagValue<'a> for Tag<'a, V, N>
where
    N: TagName,
    V: TagValue<'a>,
{
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a> {
        let parent = Element::new(name).set_namespace_optional(namespace);

        let child = self.into_element();

        parent.add_child(child)
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

impl<'a, N, A, V> From<(V, A)> for Tag1<'a, V, N, A>
where
    N: TagName,
    A: Attribute<'a>,
    V: TagValue<'a>,
{
    fn from((value, attr): (V, A)) -> Self {
        Tag1::new(value, attr)
    }
}

impl<'a, N, A> From<(&'a str, A)> for Tag1<'a, Text<'a>, N, A>
where
    N: TagName,
    A: Attribute<'a>,
{
    fn from(value: (&'a str, A)) -> Self {
        Tag1::new(value.0.into(), value.1)
    }
}

impl<'a, V, N> std::convert::From<V> for Tag<'a, V, N>
where
    V: crate::traits::TagValue<'a>,
    N: crate::traits::TagName,
{
    fn from(value: V) -> Self {
        Tag::new(value)
    }
}


pub struct Tag1Visitor<'a, V, N, A>
where
    V: TagValue<'a>,
    N: TagName,
    A: Attribute<'a>,
{
    tag: Option<Tag1<'a, V, N, A>>,
    __phantom: std::marker::PhantomData<&'a N>,
}