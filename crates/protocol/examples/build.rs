use protocol::{
    cores::{
        Tag, TagList,
        tag_name::*,
        tag_value::{Empty, Text},
    },
    rsp::rsp::Shell,
    soap::{SoapEnvelope, body::SoapBody, header::SoapHeaders},
    ws_addressing::AddressValue,
    ws_management::header::OptionSetValue,
};
use std::collections::HashSet;

pub fn main() {
    // Create an empty TagList for creation_xml (simplified for now)
    let creation_xml_content = TagList::new();

    // Build the Shell content for the body
    let shell = Shell::builder()
        .shell_id("2D6534D0-6B12-40E3-B773-CBA26459CFA8")
        .name("Runspace1")
        .input_streams("stdin pr")
        .output_streams("stdout")
        .creation_xml("Mimic-the-base64-encoded XML content here")
        .build();

    // Build the OptionSet with protocolversion
    let mut option_set_values = HashSet::new();
    option_set_values.insert(Text::from("2.3")); // protocolversion value
    let option_set_tag = OptionSetValue::new(option_set_values);

    // Build the complete SOAP envelope
    let envelope = SoapEnvelope::builder()
        .header(
            SoapHeaders::builder()
                // WS-Addressing headers
                .to("http://10.10.0.3:5985/wsman?PSVersion=7.4.10")
                .action("http://schemas.xmlsoap.org/ws/2004/09/transfer/Create")
                .reply_to(
                    Tag::new(AddressValue {
                        url: "http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous"
                            .into(),
                    })
                    .with_attribute(protocol::cores::Attribute::MustUnderstand(true)),
                )
                .message_id("uuid:D1D65143-B634-4725-BBF6-869CC4D3062F")
                // WS-Management headers
                .resource_uri("http://schemas.microsoft.com/powershell/Microsoft.PowerShell")
                .max_envelope_size("512000")
                .locale("en-US")
                .data_locale("en-CA")
                .session_id("uuid:9EC885D6-F5A4-4771-9D47-4BDF7DAAEA8C")
                .operation_id("uuid:73C4BCA6-7FF0-4AFE-B8C3-335FB19BA649")
                .sequence_id("1")
                // .option_set(option_set_tag)
                .operation_timeout("PT180.000S")
                // .compression_type(compression_type_content)
                .build(),
        )
        .body(SoapBody::builder().build())
        .build();

    let envelope: Tag<'_, _, Envelope> = envelope.into();

    let element = envelope.into_element();

    let xml_builder = xml::builder::Builder::new(None, element);

    println!(
        "SOAP Envelope built successfully: {:#?}",
        xml_builder.to_string()
    );
}
