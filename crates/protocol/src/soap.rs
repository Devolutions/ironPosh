use xml_builder::Element;

use crate::{Node, stringify_boolean};

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

#[derive(Debug, Clone)]
pub struct Header<'a, T>
where
    T: Node<'a>,
{
    pub node: T,
    pub must_understand: bool,

    __phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, T> Into<Element<'a>> for Header<'a, T>
where
    T: Node<'a>,
{
    fn into(self) -> Element<'a> {
        self.into_element()
    }
}

impl<'a, T> Node<'a> for Header<'a, T>
where
    T: Node<'a>,
{
    fn into_element(self) -> Element<'a> {
        self.node.into_element().add_attribute(
            xml_builder::Attribute::new("MustUnderstand", stringify_boolean(self.must_understand))
                .set_namespace(soap_ns!()),
        )
    }
}

impl<'a, T> Header<'a, T>
where
    T: Node<'a>,
{
    pub fn new(node: T) -> Self {
        Self {
            node,
            must_understand: false,
            __phantom: std::marker::PhantomData,
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
            node: value,
            must_understand: false,
            __phantom: std::marker::PhantomData,
        }
    }
}
