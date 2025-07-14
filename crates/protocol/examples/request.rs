use hyper::{
    Request,
    header::{AUTHORIZATION, CONTENT_TYPE, HOST},
};
use protocol::{
    traits::{DeclareNamespaces, MustUnderstand, Tag, Tag1},
    ws_addressing::{Action, Address, MessageID, ReplyTo, To},
    ws_management::{
        CompressionType, DataLocale, Locale, MaxEnvelopeSize, OperationID, OperationTimeout,
        OptionSet, OptionSetValue, ResourceURI, SequenceId, SessionId,
    },
};

pub fn main() {
    // Create the option set for protocol version
    let mut options = std::collections::HashSet::new();
    options.insert("protocolversion");
    let option_set_value = OptionSetValue::new(options);

    let soap = protocol::soap::SoapBuilder::default()
        .add_header_nodes(
            protocol::ws_addressing::headers_builder()
                .to("http://10.10.0.3:5985/wsman?PSVersion=7.4.10")
                .action((
                    "http://schemas.xmlsoap.org/ws/2004/09/transfer/Create",
                    MustUnderstand::yes(),
                ))
                .reply_to(Tag1::from((
                    "http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous",
                    MustUnderstand::yes(),
                )))
                .message_id("uuid:D1D65143-B634-4725-BBF6-869CC4D3062F")
                .build(),
        )
        .add_header_nodes(
            protocol::ws_management::headers_builder()
                .resource_uri((
                    "http://schemas.microsoft.com/powershell/Microsoft.PowerShell",
                    MustUnderstand::yes(),
                ))
                .max_envelope_size("512000")
                .locale("en-US")
                .data_locale("en-CA")
                .sequence_id("1")
                .operation_id("uuid:73C4BCA6-7FF0-4AFE-B8C3-335FB19BA649")
                .operation_timeout("PT180.000S")
                .session_id((
                    "http://schemas.microsoft.com/powershell/Microsoft.PowerShell",
                    MustUnderstand::no(),
                ))
                .compression_type(DeclareNamespaces::new(Tag1::from((
                    "xpress",
                    MustUnderstand::yes(),
                ))))
                .option_set(option_set_value)
                .build(),
        )
        .build()
        .expect("Failed to build SOAP message");

    let request = Request::builder()
        .method("POST")
        .uri("/wsman?PSVersion=7.4.10")
        .header(hyper::header::CONNECTION, "keep-alive")
        .header(CONTENT_TYPE, "application/soap+xml; charset=utf-8")
        .header("User-Agent", "IronWinRM/0.1")
        .header(HOST, "10.10.0.3:5985")
        .header(AUTHORIZATION, "YWRtaW5pc3RyYXRvcjpEZXZvTGFiczEyMyE=")
        .body(soap);

    match request {
        Ok(req) => println!("SOAP message: {}", req.body()),
        Err(e) => println!("Error building request: {}", e),
    }
}
