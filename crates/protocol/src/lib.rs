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
pub(crate) mod macros;
pub mod shell;
pub mod soap;
pub mod ws_addressing;
pub mod ws_management;
pub mod traits;

pub(crate) type Result<T> = std::result::Result<T, crate::error::ProtocolError>;

