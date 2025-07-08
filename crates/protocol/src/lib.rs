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

use crate::soap::Header;

pub mod error;
pub(crate) mod macros;
pub mod soap;
pub mod ws_addressing;
pub mod ws_management;

pub(crate) type Result<T> = std::result::Result<T, crate::error::ProtocolError>;

// We define into_element ourself to avoid Orphan rules
// For example, if we want implement `Node` for `&str`, we cannot do it directly
pub trait Node<'element> {
    fn into_element(self) -> Element<'element>;
}

pub trait MustUnderstand<'a, T>
where
    T: Node<'a>,
{
    fn must_understand(self) -> Header<'a, T>;
}

impl<'a, TNode, THeader> MustUnderstand<'a, TNode> for THeader
where
    TNode: Node<'a>,
    THeader: Into<Header<'a, TNode>>,
{
    fn must_understand(self) -> Header<'a, TNode> {
        let mut header = self.into();
        header.must_understand = true;
        header
    }
}

impl<'a> Node<'a> for &'a str {
    fn into_element(self) -> Element<'a> {
        Element::new(self)
    }
}

const fn stringify_boolean(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
