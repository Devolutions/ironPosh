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

use xml::builder::Element;

use crate::soap::Value;

pub mod error;
pub(crate) mod macros;
pub mod shell;
pub mod soap;
pub mod ws_addressing;
pub mod ws_management;

pub(crate) type Result<T> = std::result::Result<T, crate::error::ProtocolError>;

pub trait TagName {
    fn tag_name(&self) -> &'static str;
}

pub struct Tag<'a, V, N>
where
    V: Value<'a>,
    N: TagName,
{
    pub name: N,
    pub value: V,

    __phantom: std::marker::PhantomData<&'a V>,
}

impl<'a> Value<'a> for &'a str {
    fn into_element(self, name: &'static str) -> Element<'a> {
        Element::new(name).set_text(self)
    }
}
