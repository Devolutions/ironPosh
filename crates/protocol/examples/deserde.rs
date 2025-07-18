use protocol::{cores::Attribute, soap::header::SoapHeaders};
use tracing::{debug, info};
use xml::parser::XmlDeserialize;

const SOAP_HEADER: &'static str = r#"
    <s:Envelope
    xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:x="http://schemas.xmlsoap.org/ws/2004/09/transfer"
    xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd"
    xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell"
    xmlns:p="http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd">
    <s:Header>
        <a:Action>
            http://schemas.xmlsoap.org/ws/2004/09/transfer/CreateResponse
            </a:Action>
        <a:MessageID>
            uuid:E17CCBB8-6136-4FA1-95B2-0DEF618A9232
            </a:MessageID>
        <p:OperationID
            s:mustUnderstand="false">
            uuid:73C4BCA6-7FF0-4AFE-B8C3-335FB19BA649
            </p:OperationID>
        <p:SequenceId>
            1
            </p:SequenceId>
        <a:To>
            http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous
            </a:To>
        <a:RelatesTo>
            uuid:D1D65143-B634-4725-BBF6-869CC4D3062F
            </a:RelatesTo>
        </s:Header>
    <s:Body>
        <x:ResourceCreated>
            <a:Address>
                http://10.10.0.3:5985/wsman?PSVersion=7.4.10
                </a:Address>
            <a:ReferenceParameters>
                <w:ResourceURI>
                    http://schemas.microsoft.com/powershell/Microsoft.PowerShell
                    </w:ResourceURI>
                <w:SelectorSet>
                    <w:Selector
                        Name="ShellId">
                        2D6534D0-6B12-40E3-B773-CBA26459CFA8
                        </w:Selector>
                    </w:SelectorSet>
                </a:ReferenceParameters>
            </x:ResourceCreated>
        <rsp:Shell
            xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell">
            <rsp:ShellId>
                2D6534D0-6B12-40E3-B773-CBA26459CFA8
                </rsp:ShellId>
            <rsp:Name>
                Runspace1
                </rsp:Name>
            <rsp:ResourceUri>
                http://schemas.microsoft.com/powershell/Microsoft.PowerShell
                </rsp:ResourceUri>
            <rsp:Owner>
                administrator
                </rsp:Owner>
            <rsp:ClientIP>
                10.10.0.1
                </rsp:ClientIP>
            <rsp:ProcessId>
                5812
                </rsp:ProcessId>
            <rsp:IdleTimeOut>
                PT7200.000S
                </rsp:IdleTimeOut>
            <rsp:InputStreams>
                stdin pr
                </rsp:InputStreams>
            <rsp:OutputStreams>
                stdout
                </rsp:OutputStreams>
            <rsp:MaxIdleTimeOut>
                PT2147483.647S
                </rsp:MaxIdleTimeOut>
            <rsp:Locale>
                en-US
                </rsp:Locale>
            <rsp:DataLocale>
                en-CA
                </rsp:DataLocale>
            <rsp:CompressionMode>
                XpressCompression
                </rsp:CompressionMode>
            <rsp:ProfileLoaded>
                Yes
                </rsp:ProfileLoaded>
            <rsp:Encoding>
                UTF8
                </rsp:Encoding>
            <rsp:BufferMode>
                Block
                </rsp:BufferMode>
            <rsp:State>
                Connected
                </rsp:State>
            <rsp:ShellRunTime>
                P0DT0H0M0S
                </rsp:ShellRunTime>
            <rsp:ShellInactivity>
                P0DT0H0M0S
                </rsp:ShellInactivity>
            </rsp:Shell>
        </s:Body>
    </s:Envelope>
    "#;

pub fn main() {
    tracing_subscriber::fmt::init();

    info!("Starting SOAP header deserialization test");

    let node = xml::parser::parse(SOAP_HEADER).expect("Failed to parse XML string");
    let envelope = node.root_element();

    info!("Parsed XML envelope, looking for Header element");

    let header = envelope
        .children()
        .find(|n| n.tag_name().name() == "Header")
        .expect("No Header found in SOAP envelope");

    info!("Found Header element, deserializing to SoapHeaders");

    let headers = SoapHeaders::from_node(header).expect("Failed to parse SOAP headers");

    info!("Successfully deserialized SoapHeaders");

    // Debug the parsed headers
    debug!("Headers parsed: {:#?}", headers);

    // Check which fields are present
    info!("to: {:?}", headers.to.is_some());
    info!("action: {:?}", headers.action.is_some());
    info!("message_id: {:?}", headers.message_id.is_some());
    info!("relates_to: {:?}", headers.relates_to.is_some());
    info!("resource_uri: {:?}", headers.resource_uri.is_some());
    info!("operation_id: {:?}", headers.operation_id.is_some());
    info!("sequence_id: {:?}", headers.sequence_id.is_some());
    info!(
        "operation_id.must_understand: {:?}",
        headers
            .operation_id
            .as_ref()
            .map(|o| o
                .attributes
                .iter()
                .filter(|a| matches!(a, Attribute::MustUnderstand(_)))
                .count())
            .unwrap_or(0)
    );

    // Only assert for fields that are actually present in the header
    assert!(headers.to.is_some());
    assert!(headers.action.is_some());
    assert!(headers.message_id.is_some());
    assert!(headers.relates_to.is_some());
    assert!(headers.operation_id.is_some());
    assert!(headers.sequence_id.is_some());

    // These fields are not present in the header section of the XML
    assert!(headers.resource_uri.is_none());
    assert!(headers.reply_to.is_none());
    assert!(headers.max_envelope_size.is_none());
    assert!(headers.locale.is_none());
    assert!(headers.data_locale.is_none());
    assert!(headers.session_id.is_none());
    assert!(headers.option_set.is_none());
    assert!(headers.operation_timeout.is_none());
    assert!(headers.compression_type.is_none());

    // Additional assertions can be added to check the values of the fields

    assert_eq!(
        headers
            .operation_id
            .as_ref()
            .unwrap()
            .attributes
            .iter()
            .filter(|a| matches!(a, Attribute::MustUnderstand(_)))
            .count(),
        1,
    )
}
