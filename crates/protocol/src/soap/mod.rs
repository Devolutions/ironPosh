pub mod header;
pub mod parsing;
use xml::builder::Element;

use crate::{
    ws_addressing::{WSA_NAMESPACE, WSA_NAMESPACE_ALIAS},
    ws_management::{WSMAN_NAMESPACE, WSMAN_NAMESPACE_ALIAS},
};

pub const SOAP_NAMESPACE: &str = "http://www.w3.org/2003/05/soap-envelope";
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

pub trait SoapHeaders<'a>: IntoIterator<Item = Element<'a>> {
    const NAMESPACE: &'static str;
}
pub trait SoapBodys<'a>: IntoIterator<Item = Element<'a>> {
    const NAMESPACE: &'static str;
}

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
            root_element
                .add_namespace_alias(SOAP_NAMESPACE, SOAP_ALIAS)
                .add_namespace_alias(WSA_NAMESPACE, WSA_NAMESPACE_ALIAS)
                .add_namespace_alias(WSMAN_NAMESPACE, WSMAN_NAMESPACE_ALIAS),
        );

        Ok(builder.to_string())
    }
}
