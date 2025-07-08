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

pub mod error;
pub mod soap;
pub mod ws_addressing;

pub(crate) type Result<T> = std::result::Result<T, crate::error::ProtocolError>;

pub trait Node {
    fn node_name(&self) -> &str;
    fn namespace(&self) -> Option<&str>;
    fn serialize(&self, namespace_alias: Option<&str>) -> String;
}

pub trait SoapHeader: Node {}
pub trait SoapBody: Node {}
