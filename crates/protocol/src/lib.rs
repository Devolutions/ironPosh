/*

I want something like this:

Protocol::soap_builder()
    .soap_version(SoapVersion::V1_2)
    .add_header_nodes(
        ws_addressing::headers_builder()
        .add_node(WsAddressing::to("http://example.com/endpoint"))
        .add_node(WsAddressing::action("http://example.com/action"))
        .build()
    )
    .add_header_nodes(
        ws_man::headers_builder()
            .add_node(WsMan::resource_uri("http://schemas.dmtf.org/wbem/wscim/1/cim-schema/2/CIM_ComputerSystem"))
            .build()
    )
    .add_body_node(
        ws_man::body_builder()
            .add_node(WsMan::get())
            .build()
    ).build()

*/

use xml_builder::Element;

use crate::soap::{Header, NodeValue};

pub mod error;
pub(crate) mod macros;
pub mod soap;
pub mod ws_addressing;
pub mod ws_management;

pub(crate) type Result<T> = std::result::Result<T, crate::error::ProtocolError>;

// We define into_element ourself to avoid Orphan rules
// For example, if we want implement `Node` for `&str`, we cannot do it directly
pub trait MustUnderstand<'a, T>
where
    T: NodeValue<'a>,
{
    fn must_understand(self) -> Header<'a, T>;
}

impl<'a, TNodeValue, THeader> MustUnderstand<'a, TNodeValue> for THeader
where
    TNodeValue: NodeValue<'a>,
    THeader: Into<Header<'a, TNodeValue>>,
{
    fn must_understand(self) -> Header<'a, TNodeValue> {
        let mut header = self.into();
        header.must_understand = true;
        header
    }
}

impl<'a> NodeValue<'a> for &'a str {
    fn into_element(self, name: &'static str) -> Element<'a> {
        let mut element = Element::new(name);
        element.with_text(self);
        element
    }
}
