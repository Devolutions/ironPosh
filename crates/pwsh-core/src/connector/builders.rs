use base64::Engine;
use protocol_powershell_remoting::{
    PowerShellFragment, PowerShellRemotingMessage,
    messages::{PsObject, PsProperty, PsValue},
};
use std::collections::HashMap;
use typed_builder::TypedBuilder;
use uuid::Uuid;

/// Creates a SESSION_CAPABILITY object as described in the PSRP specification
fn create_session_capability() -> PsObject {
    PsObject {
        ref_id: Some(0),
        type_names: None,
        tn_ref: None,
        props: vec![],
        ms: vec![
            PsProperty {
                name: Some("protocolversion".to_string()),
                ref_id: None,
                value: PsValue::Version("2.3".to_string()),
            },
            PsProperty {
                name: Some("PSVersion".to_string()),
                ref_id: None,
                value: PsValue::Version("2.0".to_string()),
            },
            PsProperty {
                name: Some("SerializationVersion".to_string()),
                ref_id: None,
                value: PsValue::Version("1.1.0.1".to_string()),
            },
        ],
        lst: vec![],
        dct: HashMap::new(),
    }
}

/// Creates the creation XML content by building PowerShell remoting messages
pub fn create_creation_xml(
    runspace_id: Uuid,
    process_id: Uuid,
    object_id: u64,
) -> Result<String, Box<dyn std::error::Error>> {
    // Create SESSION_CAPABILITY message
    let session_capability = create_session_capability();
    let message = PowerShellRemotingMessage::new(
        protocol_powershell_remoting::Destination::Server,
        protocol_powershell_remoting::MessageType::InitRunspacepool,
        runspace_id,
        process_id,
        &session_capability,
    );

    let fragment = PowerShellFragment::new(object_id, 0, true, false, message);

    // Serialize the fragment to a string (this is a placeholder, actual serialization logic needed)
    let session_xml = fragment.into_vec();

    let base64_encoded = base64::engine::general_purpose::STANDARD.encode(session_xml);

    Ok(base64_encoded)
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct WinRmInitializationConfig<'a> {
    // Connection details
    #[builder(setter(into))]
    pub endpoint_url: &'a str,
    #[builder(default = "7.4.10", setter(into))]
    pub ps_version: &'a str,

    // Shell configuration
    #[builder(setter(into))]
    pub shell_id: &'a str,
    #[builder(default = "Runspace1", setter(into))]
    pub shell_name: &'a str,
    #[builder(default = "stdin pr", setter(into))]
    pub input_streams: &'a str,
    #[builder(default = "stdout", setter(into))]
    pub output_streams: &'a str,
    #[builder(setter(into))]
    pub creation_xml: &'a str,

    // Protocol configuration
    #[builder(default = "2.3", setter(into))]
    pub protocol_version: &'a str,
    #[builder(default = "512000", setter(into))]
    pub max_envelope_size: &'a str,
    #[builder(default = "PT180.000S", setter(into))]
    pub operation_timeout: &'a str,
    #[builder(default = "xpress", setter(into))]
    pub compression_type: &'a str,

    // Localization
    #[builder(default = "en-US", setter(into))]
    pub locale: &'a str,
    #[builder(default = "en-CA", setter(into))]
    pub data_locale: &'a str,

    // Generated IDs (should be UUIDs)
    #[builder(setter(into))]
    pub message_id: &'a str,
    #[builder(setter(into))]
    pub session_id: &'a str,
    #[builder(setter(into))]
    pub operation_id: &'a str,
    #[builder(default = "1", setter(into))]
    pub sequence_id: &'a str,

    // WS-Addressing
    #[builder(
        default = "http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous",
        setter(into)
    )]
    pub reply_to_address: &'a str,
    #[builder(
        default = "http://schemas.microsoft.com/powershell/Microsoft.PowerShell",
        setter(into)
    )]
    pub resource_uri: &'a str,
    #[builder(
        default = "http://schemas.xmlsoap.org/ws/2004/09/transfer/Create",
        setter(into)
    )]
    pub action: &'a str,
}

pub(crate) fn initialization_winrm_xml(config: WinRmInitializationConfig) -> String {
    use protocol_winrm::{
        cores::{
            Attribute, Namespace, Tag,
            tag_name::*,
            tag_value::{Empty, Text},
        },
        rsp::rsp::ShellValue,
        soap::{SoapEnvelope, body::SoapBody, header::SoapHeaders},
        ws_addressing::AddressValue,
        ws_management::header::OptionSetValue,
    };

    // Build the Shell content for the body
    let creation_xml_tag = Tag::new(Text::from(config.creation_xml))
        .with_name(CreationXml)
        .with_declaration(Namespace::PowerShellRemoting);

    let shell = ShellValue::builder()
        .input_streams(Tag::new(Text::from(config.input_streams)).with_name(InputStreams))
        .output_streams(Tag::new(Text::from(config.output_streams)).with_name(OutputStreams))
        .creation_xml(creation_xml_tag)
        .build();

    let shell_tag = Tag::new(shell)
        .with_name(Shell)
        .with_attribute(Attribute::Name(config.shell_name.into()))
        .with_attribute(Attribute::ShellId(config.shell_id.into()))
        .with_declaration(Namespace::WsmanShell);

    // Build the OptionSet with protocolversion and add xsi namespace declaration
    let option_set_tag =
        OptionSetValue::new().add_option("protocolversion", config.protocol_version, Some(true));

    // Build ReplyTo with Address
    let reply_to_address = AddressValue {
        url: Tag::new(Text::from(config.reply_to_address))
            .with_name(Address)
            .with_attribute(Attribute::MustUnderstand(true)),
    };

    // Build the SOAP headers
    let headers = SoapHeaders::builder()
        .to(Tag::new(Text::from(config.endpoint_url)).with_name(To))
        .resource_uri(
            Tag::new(Text::from(config.resource_uri))
                .with_name(ResourceURI)
                .with_attribute(Attribute::MustUnderstand(true)),
        )
        .reply_to(Tag::new(reply_to_address).with_name(ReplyTo))
        .action(
            Tag::new(Text::from(config.action))
                .with_name(Action)
                .with_attribute(Attribute::MustUnderstand(true)),
        )
        .max_envelope_size(
            Tag::new(Text::from(config.max_envelope_size))
                .with_name(MaxEnvelopeSize)
                .with_attribute(Attribute::MustUnderstand(true)),
        )
        .message_id(Tag::new(Text::from(config.message_id)).with_name(MessageID))
        .locale(
            Tag::new(Empty)
                .with_name(Locale)
                .with_attribute(Attribute::XmlLang(config.locale.into()))
                .with_attribute(Attribute::MustUnderstand(false)),
        )
        .data_locale(
            Tag::new(Empty)
                .with_name(DataLocale)
                .with_attribute(Attribute::XmlLang(config.data_locale.into()))
                .with_attribute(Attribute::MustUnderstand(false)),
        )
        .session_id(
            Tag::new(Text::from(config.session_id))
                .with_name(SessionId)
                .with_attribute(Attribute::MustUnderstand(false)),
        )
        .operation_id(
            Tag::new(Text::from(config.operation_id))
                .with_name(OperationID)
                .with_attribute(Attribute::MustUnderstand(false)),
        )
        .sequence_id(
            Tag::new(Text::from(config.sequence_id))
                .with_name(SequenceId)
                .with_attribute(Attribute::MustUnderstand(false)),
        )
        .option_set(
            Tag::new(option_set_tag)
                .with_name(OptionSet)
                .with_attribute(Attribute::MustUnderstand(true))
                .with_declaration(Namespace::XmlSchemaInstance),
        )
        .operation_timeout(
            Tag::new(Text::from(config.operation_timeout)).with_name(OperationTimeout),
        )
        .compression_type(
            Tag::new(Text::from(config.compression_type))
                .with_name(CompressionType)
                .with_attribute(Attribute::MustUnderstand(true))
                .with_declaration(Namespace::WsmanShell),
        )
        .build();

    // Build the SOAP body
    let body = SoapBody::builder().shell(shell_tag).build();

    // Build the complete SOAP envelope
    let envelope = SoapEnvelope::builder()
        .header(Tag::new(headers).with_name(Header))
        .body(Tag::new(body).with_name(Body))
        .build();

    // Convert envelope to Tag and add namespace declarations
    let envelope: Tag<'_, _, Envelope> = envelope.into();
    let envelope = envelope
        .with_declaration(Namespace::SoapEnvelope2003)
        .with_declaration(Namespace::WsAddressing2004)
        .with_declaration(Namespace::DmtfWsmanSchema)
        .with_declaration(Namespace::MsWsmanSchema);

    // Convert Tag to Element
    let element = envelope.into_element();

    // Create XML builder and convert to string
    let xml_builder = xml::builder::Builder::new(None, element);
    xml_builder.to_string()
}

#[cfg(test)]
pub mod test {

    use super::*;

    #[test]
    fn test_initialization_winrm_xml() {
        let config = WinRmInitializationConfig::builder()
            .endpoint_url("http://example.com")
            .ps_version("7.4.10")
            .shell_id("ShellId123")
            .shell_name("TestShell")
            .input_streams("stdin pr")
            .output_streams("stdout")
            .creation_xml("<CreationXml>Content</CreationXml>")
            .protocol_version("2.3")
            .max_envelope_size("512000")
            .operation_timeout("PT180.000S")
            .compression_type("xpress")
            .locale("en-US")
            .data_locale("en-CA")
            .message_id("MessageID123")
            .session_id("SessionID123")
            .operation_id("OperationID123")
            .sequence_id("1")
            .reply_to_address("http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous")
            .resource_uri("http://schemas.microsoft.com/powershell/Microsoft.PowerShell")
            .action("http://schemas.xmlsoap.org/ws/2004/09/transfer/Create")
            .build();

        let xml = initialization_winrm_xml(config);
        assert!(!xml.is_empty());
    }
}
