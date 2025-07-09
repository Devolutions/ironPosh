pub mod header;
pub use header::*;
use xml::builder::Element;

use crate::{
    ws_addressing::{WSA_NAMESPACE, WSA_NAMESPACE_ALIAS},
    ws_management::WSMAN_NAMESPACE,
};

pub const SOAP_NAMESPACE: &str = "http://schemas.xmlsoap.org/soap/envelope/";
pub const SOAP_ALIAS: &str = "s";
#[macro_export]
macro_rules! soap_ns {
    () => {
        xml::builder::Namespace::new(crate::soap::SOAP_NAMESPACE)
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

        let builder = xml::builder::Builder::new(
            None,
            xml::builder::RootElement::new(root_element)
                .set_alias(SOAP_NAMESPACE, SOAP_ALIAS)
                .set_alias(WSA_NAMESPACE, WSA_NAMESPACE_ALIAS)
                .set_alias(WSMAN_NAMESPACE, WSA_NAMESPACE_ALIAS),
        );

        Ok(builder.to_string())
    }
}

pub trait Value<'a> {
    fn into_element(self, name: &'static str) -> Element<'a>;
}
