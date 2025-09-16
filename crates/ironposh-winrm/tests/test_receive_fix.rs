use ironposh_winrm::{
    cores::{DesiredStream, Receive, Tag, Text},
    rsp::receive::ReceiveValue,
    soap::SoapEnvelope,
};
use ironposh_xml::{builder::Element, parser::XmlDeserialize};
use uuid::Uuid;

#[test]
fn test_receive_with_single_desired_stream() {
    // Test case: Command-level receive with CommandId
    let command_id = Uuid::new_v4();

    // Create a ReceiveValue with single DesiredStream containing space-separated streams
    let receive = ReceiveValue::builder()
        .desired_streams(vec![
            Tag::from_name(DesiredStream)
                .with_value(Text::from("stdout stderr"))
                .with_attribute(ironposh_winrm::cores::Attribute::CommandId(command_id)),
        ])
        .build();

    let receive_tag = Tag::from_name(Receive)
        .with_value(receive)
        .with_declaration(ironposh_winrm::cores::Namespace::WsmanShell);

    let element: Element = receive_tag.into_element();
    let xml_string = element.to_xml_string().unwrap();

    println!("Generated XML: {}", xml_string);

    // Verify the XML contains a single DesiredStream element with space-separated streams
    assert!(xml_string.contains("<rsp:DesiredStream"));
    assert!(xml_string.contains("stdout stderr"));
    assert!(xml_string.contains(&format!(
        "CommandId=\"{}\"",
        command_id.to_string().to_uppercase()
    )));

    // Should only have one DesiredStream element (not two separate ones)
    let desired_stream_count = xml_string.matches("<rsp:DesiredStream").count();
    assert_eq!(
        desired_stream_count, 1,
        "Should have exactly one DesiredStream element"
    );
}

#[test]
fn test_receive_shell_level_without_command_id() {
    // Test case: Shell-level receive without CommandId
    let receive = ReceiveValue::builder()
        .desired_streams(vec![
            Tag::from_name(DesiredStream).with_value(Text::from("stdout stderr")),
        ])
        .build();

    let receive_tag = Tag::from_name(Receive)
        .with_value(receive)
        .with_declaration(ironposh_winrm::cores::Namespace::WsmanShell);

    let element: Element = receive_tag.into_element();
    let xml_string = element.to_xml_string().unwrap();

    println!("Generated Shell-level XML: {}", xml_string);

    // Verify the XML contains a single DesiredStream element without CommandId
    assert!(xml_string.contains("<rsp:DesiredStream"));
    assert!(xml_string.contains("stdout stderr"));
    assert!(!xml_string.contains("CommandId="));

    // Should only have one DesiredStream element
    let desired_stream_count = xml_string.matches("<rsp:DesiredStream").count();
    assert_eq!(
        desired_stream_count, 1,
        "Should have exactly one DesiredStream element"
    );
}

#[test]
fn test_soap_fault_with_unknown_namespace() {
    // Test SOAP fault parsing with unknown namespace (like WS-Eventing)
    let soap_fault_xml = r#"
    <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
        <soap:Body>
            <soap:Fault>
                <soap:Code>
                    <soap:Value>soap:Sender</soap:Value>
                    <soap:Subcode>
                        <soap:Value>w:SchemaValidationError</soap:Value>
                    </soap:Subcode>
                </soap:Code>
                <soap:Reason>
                    <soap:Text xml:lang="en-US">Schema validation error</soap:Text>
                </soap:Reason>
                <soap:Detail>
                    <f:WSManFault xmlns:f="http://schemas.microsoft.com/wbem/wsman/1/wsmanfault" Code="2150858817">
                        <f:Message>The WS-Management service received a SOAP packet that contained an invalid or missing HTTP request URI.</f:Message>
                    </f:WSManFault>
                    <e:UnknownEventing xmlns:e="http://schemas.xmlsoap.org/ws/2004/08/eventing">
                        <e:SomeUnknownTag>Unknown content</e:SomeUnknownTag>
                    </e:UnknownEventing>
                </soap:Detail>
            </soap:Fault>
        </soap:Body>
    </soap:Envelope>
    "#;

    // This should now parse successfully instead of failing on unknown namespace
    let parsed = ironposh_xml::parser::parse(soap_fault_xml).expect("Should parse XML");
    let envelope = SoapEnvelope::from_node(parsed.root_element())
        .expect("Should parse SOAP envelope despite unknown namespace");

    // Verify we can access the fault information
    if let Some(fault) = &envelope.body.as_ref().fault {
        println!("Successfully parsed SOAP fault: {:?}", fault);

        // Check that we can access the fault code and reason
        if let Some(code) = &fault.as_ref().code {
            if let Some(value) = &code.as_ref().value {
                assert_eq!(value.as_ref().as_ref(), "soap:Sender");
            }
        }

        if let Some(reason) = &fault.as_ref().reason {
            if let Some(text) = &reason.as_ref().text {
                assert_eq!(text.as_ref().as_ref(), "Schema validation error");
            }
        }
    } else {
        panic!("Should have parsed the SOAP fault successfully");
    }
}
