use ironposh_winrm::{
    cores::{Namespace, Tag, Text, tag_name::*},
    rsp::connect::ConnectValue,
    soap::{SoapEnvelope, body::SoapBody},
    ws_management::{SelectorSetValue, WsAction, WsMan},
};
use ironposh_xml::parser::XmlDeserialize;

const SHELL_ID: &str = "2D6534D0-6B12-40E3-B773-CBA26459CFA8";
const RESOURCE_URI: &str = "http://schemas.microsoft.com/powershell/Microsoft.PowerShell";
const CONNECT_PAYLOAD: &str = "AAAAAAAAAAEAAAAAAAAAAAMAAADConnectPayloadBase64==";

#[test]
fn test_connect_action_uri() {
    assert_eq!(
        WsAction::Connect.as_str(),
        "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Connect"
    );
}

#[test]
fn test_build_connect_envelope() {
    let ws_man = WsMan::builder()
        .to("http://10.10.0.3:5985/wsman".to_string())
        .build();

    let connect_value = ConnectValue {
        connect_xml: Tag::new(Text::from(CONNECT_PAYLOAD))
            .with_declaration(Namespace::PowerShellRemoting),
    };

    let connect_tag = Tag::from_name(Connect)
        .with_declaration(Namespace::WsmanShell)
        .with_value(connect_value);

    let selector_set = SelectorSetValue::new().add_selector("ShellId", SHELL_ID);

    let envelope = ws_man.invoke(
        &WsAction::Connect,
        Some(RESOURCE_URI),
        SoapBody::builder().connect(connect_tag).build(),
        None,
        Some(selector_set),
    );

    let xml_string = envelope
        .into_element()
        .to_xml_string()
        .expect("Failed to build XML");

    assert!(
        xml_string.contains("http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Connect"),
        "Envelope should contain the Connect action URI, got: {xml_string}"
    );
    assert!(
        xml_string.contains(SHELL_ID),
        "Envelope should contain the ShellId selector value, got: {xml_string}"
    );
    assert!(
        xml_string.contains("<rsp:Connect"),
        "Envelope should contain the rsp:Connect body element, got: {xml_string}"
    );
    assert!(
        xml_string.contains(r#"<connectXml xmlns="http://schemas.microsoft.com/powershell">"#),
        "Envelope should declare the powershell namespace on connectXml, got: {xml_string}"
    );
    assert!(
        xml_string.contains(CONNECT_PAYLOAD),
        "Envelope should carry the base64 connect payload, got: {xml_string}"
    );
}

#[test]
fn test_parse_connect_response() {
    let xml_content = r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd"
    xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell"
    xmlns:p="http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd">
    <s:Header>
        <a:Action>http://schemas.microsoft.com/wbem/wsman/1/windows/shell/ConnectResponse</a:Action>
        <a:MessageID>uuid:6C334787-EF2C-40E4-992F-DE4599ED2505</a:MessageID>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
        <p:OperationID s:mustUnderstand="false">uuid:672d68a1-9782-4f78-bebc-8b5db2355fda</p:OperationID>
        <p:SequenceId>1</p:SequenceId>
        <a:RelatesTo>uuid:87d0a667-c08e-4311-8d2d-069367f452d8</a:RelatesTo>
    </s:Header>
    <s:Body>
        <rsp:ConnectResponse>
            <connectResponseXml xmlns="http://schemas.microsoft.com/powershell">QmFzZTY0UmVzcG9uc2VQYXlsb2Fk</connectResponseXml>
        </rsp:ConnectResponse>
    </s:Body>
</s:Envelope>"#;

    let document = ironposh_xml::parser::parse(xml_content).expect("Failed to parse XML content");
    let soap_envelope = SoapEnvelope::from_node(document.root_element())
        .expect("Failed to deserialize XML into SoapEnvelope");

    let connect_response = soap_envelope
        .body
        .as_ref()
        .connect_response
        .as_ref()
        .expect("SoapBody should contain ConnectResponse");

    let connect_response_xml = connect_response
        .as_ref()
        .connect_response_xml
        .as_ref()
        .expect("ConnectResponse should carry connectResponseXml");

    assert_eq!(
        connect_response_xml.value.as_ref(),
        "QmFzZTY0UmVzcG9uc2VQYXlsb2Fk"
    );
}

#[test]
fn test_parse_connect_response_without_payload_is_tolerated() {
    // The struct itself is tolerant; missing payloads are surfaced to callers
    // as `None` so they can produce a descriptive error.
    let xml_content = r#"<s:Envelope
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell">
    <s:Header></s:Header>
    <s:Body>
        <rsp:ConnectResponse/>
    </s:Body>
</s:Envelope>"#;

    let document = ironposh_xml::parser::parse(xml_content).expect("Failed to parse XML content");
    let soap_envelope = SoapEnvelope::from_node(document.root_element())
        .expect("Failed to deserialize XML into SoapEnvelope");

    let connect_response = soap_envelope
        .body
        .as_ref()
        .connect_response
        .as_ref()
        .expect("SoapBody should contain ConnectResponse");

    assert!(
        connect_response.as_ref().connect_response_xml.is_none(),
        "missing connectResponseXml must parse as None"
    );
}
