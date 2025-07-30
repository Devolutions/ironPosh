use protocol_winrm::{
    cores::{Attribute, Tag, TagList, tag_name::*, tag_value::Text},
    rsp::rsp::ShellValue,
    soap::{SoapEnvelope, body::SoapBody, header::SoapHeaders},
    ws_addressing::AddressValue,
    ws_management::header::OptionSetValue,
};

#[cfg(test)]
mod tests {
    use protocol_winrm::cores::{Empty, EmptyVisitor};

    use super::*;

    #[test]
    fn test_build_soap_envelope_for_shell_creation() {
        // Create an empty TagList for creation_xml (simplified for now)
        let _creation_xml_content = TagList::new();

        // Build the Shell content for the body
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
                        Tag::new(()).with_name(Locale).with_attribute(
                            protocol_winrm::cores::Attribute::MustUnderstand(false),
                        ),
                    )
                    .data_locale(
                        Tag::new(()).with_name(DataLocale).with_attribute(
                            protocol_winrm::cores::Attribute::MustUnderstand(false),
                        ),
                    )
                    .session_id(
                        Tag::new(Text::from("uuid:9EC885D6-F5A4-4771-9D47-4BDF7DAAEA8C"))
                            .with_name(SessionId)
                            .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(
                                false,
                            )),
                    )
                    .operation_id(
                        Tag::new(Text::from("uuid:73C4BCA6-7FF0-4AFE-B8C3-335FB19BA649"))
                            .with_name(OperationID)
                            .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(
                                false,
                            )),
                    )
                    .sequence_id(
                        Tag::new(Text::from("1"))
                            .with_name(SequenceId)
                            .with_attribute(protocol_winrm::cores::Attribute::MustUnderstand(
                                false,
                            )),
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

        // Convert envelope to Tag and add namespace declarations
        let envelope: Tag<'_, _, Envelope> = envelope.into();
        let envelope = envelope
            .with_declaration(protocol_winrm::cores::namespace::Namespace::SoapEnvelope2003)
            .with_declaration(protocol_winrm::cores::namespace::Namespace::WsAddressing2004)
            .with_declaration(protocol_winrm::cores::namespace::Namespace::MsWsmanSchema)
            .with_declaration(protocol_winrm::cores::namespace::Namespace::WsTransfer2004)
            .with_declaration(protocol_winrm::cores::namespace::Namespace::PowerShellRemoting);

        // Convert Tag to Element
        let element = envelope.into_element();

        // Create XML builder and convert to string
        let xml_builder = xml::builder::Builder::new(None, element);
        let xml_string = xml_builder.to_string();

        // Assertions to verify the generated XML structure
        assert!(xml_string.contains("s:Envelope"));
        assert!(xml_string.contains("s:Header"));
        assert!(xml_string.contains("s:Body"));
        assert!(xml_string.contains("a:Action"));
        assert!(xml_string.contains("a:MessageID"));
        assert!(xml_string.contains("rsp:Shell"));
        assert!(xml_string.contains("Runspace1"));
        assert!(xml_string.contains("2D6534D0-6B12-40E3-B773-CBA26459CFA8"));
        assert!(xml_string.contains("http://schemas.xmlsoap.org/ws/2004/09/transfer/Create"));

        // Print the XML for debugging purposes (can be removed or made conditional)
        println!("Generated SOAP Envelope:\n{}", xml_string);
    }

    #[test]
    fn test_shell_value_builder() {
        let shell = ShellValue::builder()
            .name("TestShell")
            .input_streams("stdin")
            .output_streams("stdout stderr")
            .creation_xml("test xml content")
            .build();

        // Verify the shell was built correctly
        // The fields are Tag<Text, TagName> types, so we check if they are present
        assert!(shell.name.is_some());
        assert!(shell.input_streams.is_some());
        assert!(shell.output_streams.is_some());
        assert!(shell.creation_xml.is_some());

        // We can verify the text content by checking the value field
        if let Some(name_tag) = &shell.name {
            assert_eq!(name_tag.value, Text::from("TestShell"));
        }
        if let Some(input_tag) = &shell.input_streams {
            assert_eq!(input_tag.value, Text::from("stdin"));
        }
        if let Some(output_tag) = &shell.output_streams {
            assert_eq!(output_tag.value, Text::from("stdout stderr"));
        }
    }

    #[test]
    fn test_option_set_creation() {
        let option_set = OptionSetValue::new()
            .add_option("WINRS_CONSOLEMODE_STDIN", "TRUE", None)
            .add_option("protocolversion", "2.3", Some(true));

        // Verify options were added correctly
        assert_eq!(option_set.options.len(), 2);

        // Find and verify the console mode option
        let console_option = option_set
            .options
            .iter()
            .find(|opt| {
                opt.attributes.iter().any(|attr| {
                    if let Attribute::Name(name) = attr {
                        name == "WINRS_CONSOLEMODE_STDIN"
                    } else {
                        false
                    }
                })
            })
            .expect("Console mode option should exist");
        assert_eq!(console_option.value, Text::from("TRUE"));

        // Verify it doesn't have MustComply attribute
        let has_must_comply = console_option
            .attributes
            .iter()
            .any(|attr| matches!(attr, Attribute::MustComply(_)));
        assert!(!has_must_comply);

        // Find and verify the protocol version option
        let protocol_option = option_set
            .options
            .iter()
            .find(|opt| {
                opt.attributes.iter().any(|attr| {
                    if let Attribute::Name(name) = attr {
                        name == "protocolversion"
                    } else {
                        false
                    }
                })
            })
            .expect("Protocol version option should exist");
        assert_eq!(protocol_option.value, Text::from("2.3"));

        // Verify it has MustComply attribute set to true
        let must_comply_value = protocol_option.attributes.iter().find_map(|attr| {
            if let Attribute::MustComply(value) = attr {
                Some(*value)
            } else {
                None
            }
        });
        assert_eq!(must_comply_value, Some(true));
    }
}
