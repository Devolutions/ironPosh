use xml_builder::{Attribute, Element};

#[macro_export]
macro_rules! soap_ns {
    () => {
        xml_builder::Namespace::new("s", "http://schemas.xmlsoap.org/soap/envelope/")
    };
}

pub enum SoapVersion {
    V1_2,
}

pub trait SoapHeaders<'a>: IntoIterator<Item = Element<'a>> {}
pub trait SoapBodys<'a>: IntoIterator<Item = Element<'a>> {}

pub struct SoapBuilder<'a> {
    header_nodes: Vec<Element<'a>>,
    body_nodes: Vec<Element<'a>>,
}

impl<'a> Default for SoapBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> SoapBuilder<'a> {
    pub fn new() -> Self {
        Self {
            header_nodes: Vec::new(),
            body_nodes: Vec::new(),
        }
    }

    pub fn add_header_nodes(mut self, nodes: impl IntoIterator<Item = Element<'a>>) -> Self {
        self.header_nodes.extend(nodes);

        self
    }

    pub fn add_body_nodes(mut self, nodes: impl IntoIterator<Item = Element<'a>>) -> Self {
        self.body_nodes.extend(nodes);
        self
    }

    pub fn build(self) -> crate::Result<String> {
        let root_element = Element::new("Envelope")
            .set_namespace(soap_ns!())
            .add_child(
                Element::new("Header")
                    .set_namespace(soap_ns!())
                    .add_children(self.header_nodes),
            )
            .add_child(
                Element::new("Body")
                    .set_namespace(soap_ns!())
                    .add_children(self.body_nodes),
            );

        let builder = xml_builder::Builder::new(None, xml_builder::RootElement::new(root_element));

        Ok(builder.to_string())
    }
}

pub trait NodeValue<'a> {
    fn into_element(self, name: &'static str) -> Element<'a>;
}

#[derive(Debug, Clone)]
pub struct Header<'a, T>
where
    T: NodeValue<'a>,
{
    pub value: T,
    pub must_understand: bool,

    pub _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, T> NodeValue<'a> for Header<'a, T>
where
    T: NodeValue<'a>,
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
    T: NodeValue<'a>,
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

impl<'a> From<&'a str> for Header<'a, &'a str> {
    fn from(value: &'a str) -> Self {
        Header {
            value,
            must_understand: false,
            _phantom: std::marker::PhantomData,
        }
    }
}
