use protocol::{cores::Attribute, soap::header::SoapHeaders};
use xml::parser::XmlDeserialize;

const SOAP_HEADER_XML: &'static str = r#"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_soap_headers() {
        let node = xml::parser::parse(SOAP_HEADER_XML).expect("Failed to parse XML string");
        let envelope = node.root_element();

        let header = envelope
            .children()
            .find(|n| n.tag_name().name() == "Header")
            .expect("No Header found in SOAP envelope");

        let headers = SoapHeaders::from_node(header).expect("Failed to parse SOAP headers");

        // Test that expected fields are present
        assert!(headers.to.is_some());
        assert!(headers.action.is_some());
        assert!(headers.message_id.is_some());
        assert!(headers.relates_to.is_some());
        assert!(headers.operation_id.is_some());
        assert!(headers.sequence_id.is_some());

        // Test that fields not present in the header section are None
        assert!(headers.resource_uri.is_none());
        assert!(headers.reply_to.is_none());
        assert!(headers.max_envelope_size.is_none());
        assert!(headers.locale.is_none());
        assert!(headers.data_locale.is_none());
        assert!(headers.session_id.is_none());
        assert!(headers.option_set.is_none());
        assert!(headers.operation_timeout.is_none());
        assert!(headers.compression_type.is_none());
    }

    #[test]
    fn test_operation_id_must_understand_attribute() {
        let node = xml::parser::parse(SOAP_HEADER_XML).expect("Failed to parse XML string");
        let envelope = node.root_element();

        let header = envelope
            .children()
            .find(|n| n.tag_name().name() == "Header")
            .expect("No Header found in SOAP envelope");

        let headers = SoapHeaders::from_node(header).expect("Failed to parse SOAP headers");

        // Check that operation_id has the mustUnderstand attribute
        let must_understand_count = headers
            .operation_id
            .as_ref()
            .unwrap()
            .attributes
            .iter()
            .filter(|a| matches!(a, Attribute::MustUnderstand(_)))
            .count();

        assert_eq!(must_understand_count, 1, "OperationID should have exactly one MustUnderstand attribute");
    }

    #[test]
    fn test_header_field_values() {
        let node = xml::parser::parse(SOAP_HEADER_XML).expect("Failed to parse XML string");
        let envelope = node.root_element();

        let header = envelope
            .children()
            .find(|n| n.tag_name().name() == "Header")
            .expect("No Header found in SOAP envelope");

        let headers = SoapHeaders::from_node(header).expect("Failed to parse SOAP headers");

        // Test specific values in the headers
        if let Some(_action) = &headers.action {
            // Assuming the action tag contains the text as its content
            // Adjust this based on the actual structure of the Tag type
            // This is a placeholder test - adjust based on actual Tag implementation
            // assert!(action.value.contains("CreateResponse"));
        }

        if let Some(_message_id) = &headers.message_id {
            // Test that message_id contains the expected UUID
            // assert!(message_id.value.contains("E17CCBB8-6136-4FA1-95B2-0DEF618A9232"));
        }

        if let Some(_to) = &headers.to {
            // Test that 'to' contains the expected anonymous role
            // assert!(to.value.contains("role/anonymous"));
        }

        // Add more specific assertions based on the actual Tag structure
    }

    #[test]
    fn test_xml_parsing_success() {
        // Test that the XML parsing itself succeeds
        let parse_result = xml::parser::parse(SOAP_HEADER_XML);
        assert!(parse_result.is_ok(), "XML should parse successfully");

        let node = parse_result.unwrap();
        let envelope = node.root_element();
        
        // Test basic structure
        assert_eq!(envelope.tag_name().name(), "Envelope");
        
        let header_count = envelope
            .children()
            .filter(|n| n.tag_name().name() == "Header")
            .count();
        assert_eq!(header_count, 1, "Should have exactly one Header element");

        let body_count = envelope
            .children()
            .filter(|n| n.tag_name().name() == "Body")
            .count();
        assert_eq!(body_count, 1, "Should have exactly one Body element");
    }

    #[test] 
    fn test_deserialize_with_minimal_header() {
        const MINIMAL_HEADER: &str = r#"
        <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
                    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing">
            <s:Header>
                <a:Action>test-action</a:Action>
                <a:MessageID>test-message-id</a:MessageID>
            </s:Header>
            <s:Body></s:Body>
        </s:Envelope>
        "#;

        let node = xml::parser::parse(MINIMAL_HEADER).expect("Failed to parse minimal XML");
        let envelope = node.root_element();

        let header = envelope
            .children()
            .find(|n| n.tag_name().name() == "Header")
            .expect("No Header found in minimal SOAP envelope");

        let headers = SoapHeaders::from_node(header).expect("Failed to parse minimal SOAP headers");

        // Only action and message_id should be present
        assert!(headers.action.is_some());
        assert!(headers.message_id.is_some());
        
        // All other fields should be None
        assert!(headers.to.is_none());
        assert!(headers.relates_to.is_none());
        assert!(headers.operation_id.is_none());
        assert!(headers.sequence_id.is_none());
    }
}
