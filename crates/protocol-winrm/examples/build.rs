use protocol_winrm::{
    cores::{
        Tag, TagList,
        tag_name::*,
        tag_value::{Empty, Text},
    },
    rsp::rsp::ShellValue,
    soap::{SoapEnvelope, body::SoapBody, header::SoapHeaders},
    ws_addressing::AddressValue,
    ws_management::header::OptionSetValue,
};
use tracing::{debug, info};

pub fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("Starting SOAP envelope building process");
    info!("Starting SOAP envelope building process");

    // Create an empty TagList for creation_xml (simplified for now)
    debug!("Creating empty TagList for creation_xml");
    let _creation_xml_content = TagList::new();

    // Build the Shell content for the body
    debug!("Building Shell content");
    let shell = ShellValue::builder()
        .name("Runspace1")
        .input_streams("stdin pr")
        .output_streams("stdout")
        .creation_xml("Mimic-the-base64-encoded XML content here")
        .build();

    let shell_tag = Tag::new(shell)
        .with_name(Shell)
        .with_attribute(protocol_winrm::cores::Attribute::ShellId(
            "2D6534D0-6B12-40E3-B773-CBA26459CFA8".into(),
        ))
        .with_attribute(protocol_winrm::cores::Attribute::Name("Runspace1".into()));

    // Build the OptionSet with protocolversion
    debug!("Building OptionSet");
    let option_set_tag = OptionSetValue::new()
        .add_option("WINRS_CONSOLEMODE_STDIN", "TRUE", None)
        .add_option("protocolversion", "2.3", Some(true));

    // Build ReplyTo with Address
    let reply_to_address = Tag::new(AddressValue {
        url: Tag::new(Text::from(
            "http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous",
        ))
        .with_name(Address)
        .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(true)),
    })
    .with_name(ReplyTo)
    .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(true));

    // Build the complete SOAP envelope
    debug!("Building SOAP envelope");
    let envelope = SoapEnvelope::builder()
        .header(
            SoapHeaders::builder()
                // WS-Addressing headers
                .to("http://10.10.0.3:5985/wsman?PSVersion=7.4.10")
                .action(
                    Tag::new(Text::from(
                        "http://schemas.xmlsoap.org/ws/2004/09/transfer/Create",
                    ))
                    .with_name(Action)
                    .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(true)),
                )
                .reply_to(reply_to_address)
                .message_id("uuid:D1D65143-B634-4725-BBF6-869CC4D3062F")
                // WS-Management headers
                .resource_uri(
                    Tag::new(Text::from(
                        "http://schemas.microsoft.com/powershell/Microsoft.PowerShell",
                    ))
                    .with_name(ResourceURI)
                    .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(true)),
                )
                .max_envelope_size(
                    Tag::new(Text::from("512000"))
                        .with_name(MaxEnvelopeSize)
                        .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(true)),
                )
                .locale(
                    Tag::new(())
                        .with_name(Locale)
                        .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(false)),
                )
                .data_locale(
                    Tag::new(())
                        .with_name(DataLocale)
                        .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(false)),
                )
                .session_id(
                    Tag::new(Text::from("uuid:9EC885D6-F5A4-4771-9D47-4BDF7DAAEA8C"))
                        .with_name(SessionId)
                        .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(false)),
                )
                .operation_id(
                    Tag::new(Text::from("uuid:73C4BCA6-7FF0-4AFE-B8C3-335FB19BA649"))
                        .with_name(OperationID)
                        .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(false)),
                )
                .sequence_id(
                    Tag::new(Text::from("1"))
                        .with_name(SequenceId)
                        .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(false)),
                )
                .option_set(
                    Tag::new(option_set_tag)
                        .with_name(OptionSet)
                        .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(true)),
                )
                .operation_timeout("PT180.000S")
                .compression_type(
                    Tag::new(Text::from("xpress"))
                        .with_name(CompressionType)
                        .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(true)),
                )
                .build(),
        )
        .body(SoapBody::builder().shell(shell_tag).build())
        .build();

    debug!("Converting envelope to Tag and adding namespace declarations");
    let envelope: Tag<'_, _, Envelope> = envelope.into();
    let envelope = envelope
        .with_declaration(protocol_winrm::cores::namespace::Namespace::SoapEnvelope2003)
        .with_declaration(protocol_winrm::cores::namespace::Namespace::WsAddressing2004)
        .with_declaration(protocol_winrm::cores::namespace::Namespace::MsWsmanSchema)
        .with_declaration(protocol_winrm::cores::namespace::Namespace::DmtfWsmanSchema)
        .with_declaration(protocol_winrm::cores::namespace::Namespace::WsTransfer2004)
        .with_declaration(protocol_winrm::cores::namespace::Namespace::WsmanShell);

    debug!("Converting Tag to Element");
    let element = envelope.into_element();
    debug!("Element created successfully with namespace declarations");

    debug!("Creating XML builder");
    let xml_builder = xml::builder::Builder::new(None, element);

    debug!("Converting to string...");
    let xml_string = xml_builder.to_string();
    info!("SOAP Envelope built successfully!");
    println!("XML Output:\n{}", xml_string);
}
