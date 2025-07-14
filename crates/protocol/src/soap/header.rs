use xml::{
    builder::{Attribute, Element},
};

use crate::{soap::Value, soap_ns};

#[derive(Debug, Clone)]
pub struct Header<'a, T>
where
    T: Value<'a>,
{
    pub value: T,
    pub must_understand: bool,

    pub _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, T> AsRef<T> for Header<'a, T>
where
    T: Value<'a>,
{
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<'a, T> From<T> for Header<'a, T>
where
    T: Value<'a>,
{
    fn from(value: T) -> Self {
        Header {
            value,
            must_understand: false,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a, T> Value<'a> for Header<'a, T>
where
    T: Value<'a>,
{
    fn into_element(self, name: &'static str) -> Element<'a> {
        let mut element = self.value.into_element(name);
        if self.must_understand {
            element = element
                .add_attribute(Attribute::new("mustUnderstand", "true").set_namespace(soap_ns!()));
        }
        element
    }
}

impl<'a, T> Header<'a, T>
where
    T: Value<'a>,
{
    pub fn new(value: T) -> Self {
        Self {
            value,
            must_understand: false,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn must_understand(mut self) -> Self {
        self.must_understand = true;
        self
    }
}
