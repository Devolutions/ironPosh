use xml::builder::Element;

use crate::define_tag;

pub trait Attribute<'a> {
    fn name(&self) -> &'static str;
    fn value(&self) -> &'a str;
    fn namespace(&self) -> Option<&'static str>;
}

#[derive(Debug, Clone)]
pub struct MustUnderstand {
    pub value: bool,
}

impl MustUnderstand {
    pub fn yes() -> Self {
        MustUnderstand { value: true }
    }

    pub fn no() -> Self {
        MustUnderstand { value: false }
    }
}

impl<'a> Attribute<'a> for MustUnderstand {
    fn name(&self) -> &'static str {
        "mustUnderstand"
    }

    fn value(&self) -> &'a str {
        if self.value { "true" } else { "false" }
    }

    fn namespace(&self) -> Option<&'static str> {
        Some(crate::soap::SOAP_NAMESPACE)
    }
}

pub trait TagName {
    fn tag_name(&self) -> &'static str;
    fn namespace(&self) -> Option<&'static str>;
}

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

define_tag!(Tag1, (Attribute1, attribute1));
define_tag!(Tag2, (Attribute1, attribute1), (Attribute2, attribute2));
define_tag!(
    Tag3,
    (Attribute1, attribute1),
    (Attribute2, attribute2),
    (Attribute3, attribute3)
);

pub trait TagValue<'a> {
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a>;
}

impl<'a> TagValue<'a> for &'a str {
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a> {
        let mut element = Element::new(name).set_text(self);
        if let Some(ns) = namespace {
            element = element.set_namespace(ns);
        }
        element
    }
}

impl<'a> TagValue<'a> for () {
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a> {
        let mut element = Element::new(name);

        if let Some(ns) = namespace {
            element = element.set_namespace(ns);
        }

        element
    }
}
