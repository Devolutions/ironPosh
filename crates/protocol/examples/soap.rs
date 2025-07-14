use protocol::{
    traits::MustUnderstand,
    ws_addressing::{Action, MessageID, To},
    ws_management::{MaxEnvelopeSize, ResourceURI},
};

pub fn main() {
    let result = protocol::soap::SoapBuilder::new()
        .add_header_nodes(
            protocol::ws_addressing::headers_builder()
                .action(Action::new_tag1(
                    "http://example.com/action",
                    MustUnderstand::yes(),
                ))
                .to(To::new_tag("http://example.com/endpoint"))
                .message_id(MessageID::new_tag(
                    "urn:uuid:12345678-1234-5678-1234-567812345678",
                ))
                .build(),
        )
        .add_header_nodes(
            protocol::ws_management::headers_builder()
                .resource_uri(ResourceURI::new_tag1(
                    "http://schemas.dmtf.org/wbem/wscim/1/cim-schema/2/CIM_ComputerSystem",
                    MustUnderstand::yes(),
                ))
                .max_envelope_size(MaxEnvelopeSize::new_tag("153600"))
                .build(),
        )
        .build()
        .expect("Failed to build SOAP message");

    println!("{}", result);
}
