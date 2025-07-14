use std::fmt::Debug;
use xml::builder::Element;

use crate::traits::Tag1;

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

impl<'a, N, A> From<(&'a str, A)> for Tag1<'a, &'a str, N, A>
where
    N: TagName,
    A: Attribute<'a>,
{
    fn from((value, attr): (&'a str, A)) -> Self {
        Tag1::new(value, attr)
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
