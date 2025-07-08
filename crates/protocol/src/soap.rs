use crate::{SoapBody, SoapHeader};

pub enum SoapVersion {
    V1_2,
}
pub struct SoapBuilder<Header, Body>
where
    Header: SoapHeader,
    Body: SoapBody,
{
    version: SoapVersion,
    header_nodes: Vec<Header>,
    body_nodes: Vec<Body>,
}

impl<Header, Body> SoapBuilder<Header, Body>
where
    Header: SoapHeader,
    Body: SoapBody,
{
    pub fn new(version: SoapVersion) -> Self {
        Self {
            version,
            header_nodes: Vec::new(),
            body_nodes: Vec::new(),
        }
    }

    pub fn add_header_node(&mut self, node: Header) {
        self.header_nodes.push(node);
    }

    pub fn add_body_node(&mut self, node: Body) {
        self.body_nodes.push(node);
    }

    pub fn build(self) -> crate::Result<String> {
        // Implementation for building the SOAP message goes here
        unimplemented!()
    }
}
