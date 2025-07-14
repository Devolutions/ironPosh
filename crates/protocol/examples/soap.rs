use protocol::traits::{MustUnderstand, Tag1};

pub fn main() {
    let result = protocol::soap::SoapBuilder::new()
        .add_header_nodes(
            protocol::ws_addressing::headers_builder()
                .action(("http://example.com/action", MustUnderstand::yes()))
                .to("http://example.com/endpoint")
                .message_id("urn:uuid:12345678-1234-5678-1234-567812345678")
                .reply_to(Tag1::from((
                    "http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous",
                    MustUnderstand::yes(),
                )))
                .build(),
        )
        .add_header_nodes(
            protocol::ws_management::headers_builder()
                .resource_uri((
                    "http://schemas.dmtf.org/wbem/wscim/1/cim-schema/2/CIM_ComputerSystem",
                    MustUnderstand::yes(),
                ))
                .max_envelope_size("153600")
                .build(),
        )
        .build()
        .expect("Failed to build SOAP message");

    println!("{}", result);
}
