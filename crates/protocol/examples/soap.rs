use protocol::MustUnderstand;

pub fn main() {
    let result = protocol::soap::SoapBuilder::new()
        .add_header_nodes(
            protocol::ws_addressing::headers_builder()
                .action("http://example.com/action".must_understand())
                .to("http://example.com/endpoint")
                .message_id("urn:uuid:12345678-1234-5678-1234-567812345678")
                .build(),
        )
        .build()
        .expect("Failed to build SOAP message");

    println!("{}", result);
}
