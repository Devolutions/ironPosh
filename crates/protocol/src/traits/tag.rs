use std::fmt::Debug;
use xml::builder::Element;

use super::tag_name::TagName;
use super::tag_value::TagValue;

#[derive(Debug, Clone)]
pub struct Tag<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    pub name: N,
    pub value: V,

    __phantom: std::marker::PhantomData<&'a V>,
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
    pub fn new(name: N, value: V) -> Self {
        Self {
            name,
            value,
            __phantom: std::marker::PhantomData,
        }
    }

    pub fn into_element(self) -> Element<'a> {
        self.value
            .into_element(self.name.tag_name(), self.name.namespace())
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
