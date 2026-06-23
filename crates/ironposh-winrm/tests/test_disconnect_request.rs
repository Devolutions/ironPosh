use ironposh_winrm::{
    cores::{Empty, Namespace, Tag, Time, tag_name::*},
    rsp::disconnect::DisconnectValue,
    soap::{SoapEnvelope, body::SoapBody},
    ws_management::{SelectorSetValue, WsAction, WsMan},
};
use ironposh_xml::mapping::FromXml;

const SHELL_ID: &str = "2D6534D0-6B12-40E3-B773-CBA26459CFA8";
const RESOURCE_URI: &str = "http://schemas.microsoft.com/powershell/Microsoft.PowerShell";

#[test]
fn test_disconnect_action_uri() {
    assert_eq!(
        WsAction::Disconnect.as_str(),
        "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Disconnect"
    );
}

#[test]
fn test_reconnect_action_uri() {
    assert_eq!(
        WsAction::Reconnect.as_str(),
        "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Reconnect"
    );
}

#[test]
fn test_build_disconnect_envelope() {
    let ws_man = WsMan::builder()
        .to("http://10.10.0.3:5985/wsman".to_string())
        .build();

    let disconnect_value = DisconnectValue::builder()
        .idle_time_out(Tag::new(Time(180.0)))
        .build();

    let disconnect_tag = Tag::from_name(Disconnect)
        .with_declaration(Namespace::WsmanShell)
        .with_value(disconnect_value);

    let selector_set = SelectorSetValue::new().add_selector("ShellId", SHELL_ID);

    let envelope = ws_man.invoke(
        &WsAction::Disconnect,
        Some(RESOURCE_URI),
        SoapBody::builder().disconnect(disconnect_tag).build(),
        None,
        Some(selector_set),
    );

    let xml_string = envelope
        .into_element()
        .to_xml_string()
        .expect("Failed to build XML");

    assert!(
        xml_string.contains("http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Disconnect"),
        "Envelope should contain the Disconnect action URI, got: {xml_string}"
    );
    assert!(
        xml_string.contains(SHELL_ID),
        "Envelope should contain the ShellId selector value, got: {xml_string}"
    );
    assert!(
        xml_string.contains("<rsp:Disconnect"),
        "Envelope should contain the rsp:Disconnect body element, got: {xml_string}"
    );
    assert!(
        xml_string.contains("PT180.000S"),
        "Envelope should contain the idle timeout, got: {xml_string}"
    );
}

#[test]
fn test_build_disconnect_envelope_without_idle_timeout() {
    let ws_man = WsMan::builder()
        .to("http://10.10.0.3:5985/wsman".to_string())
        .build();

    let disconnect_tag = Tag::from_name(Disconnect)
        .with_declaration(Namespace::WsmanShell)
        .with_value(DisconnectValue::builder().build());

    let selector_set = SelectorSetValue::new().add_selector("ShellId", SHELL_ID);

    let envelope = ws_man.invoke(
        &WsAction::Disconnect,
        Some(RESOURCE_URI),
        SoapBody::builder().disconnect(disconnect_tag).build(),
        None,
        Some(selector_set),
    );

    let xml_string = envelope
        .into_element()
        .to_xml_string()
        .expect("Failed to build XML");

    assert!(xml_string.contains("<rsp:Disconnect"));
    assert!(!xml_string.contains("IdleTimeOut"));
}

#[test]
fn test_build_reconnect_envelope() {
    let ws_man = WsMan::builder()
        .to("http://10.10.0.3:5985/wsman".to_string())
        .build();

    let reconnect_tag = Tag::from_name(Reconnect)
        .with_declaration(Namespace::WsmanShell)
        .with_value(Empty);

    let selector_set = SelectorSetValue::new().add_selector("ShellId", SHELL_ID);

    let envelope = ws_man.invoke(
        &WsAction::Reconnect,
        Some(RESOURCE_URI),
        SoapBody::builder().reconnect(reconnect_tag).build(),
        None,
        Some(selector_set),
    );

    let xml_string = envelope
        .into_element()
        .to_xml_string()
        .expect("Failed to build XML");

    assert!(
        xml_string.contains("http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Reconnect"),
        "Envelope should contain the Reconnect action URI, got: {xml_string}"
    );
    assert!(
        xml_string.contains(SHELL_ID),
        "Envelope should contain the ShellId selector value, got: {xml_string}"
    );
    assert!(
        xml_string.contains("<rsp:Reconnect"),
        "Envelope should contain the rsp:Reconnect body element, got: {xml_string}"
    );
}

#[test]
fn test_parse_disconnect_response() {
    let xml_content = r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd"
    xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell"
    xmlns:p="http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd">
    <s:Header>
        <a:Action>http://schemas.microsoft.com/wbem/wsman/1/windows/shell/DisconnectResponse</a:Action>
        <a:MessageID>uuid:6C334787-EF2C-40E4-992F-DE4599ED2505</a:MessageID>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
        <p:OperationID s:mustUnderstand="false">uuid:672d68a1-9782-4f78-bebc-8b5db2355fda</p:OperationID>
        <p:SequenceId>1</p:SequenceId>
        <a:RelatesTo>uuid:87d0a667-c08e-4311-8d2d-069367f452d8</a:RelatesTo>
    </s:Header>
    <s:Body>
        <rsp:DisconnectResponse/>
    </s:Body>
</s:Envelope>"#;

    let document = ironposh_xml::parser::parse(xml_content).expect("Failed to parse XML content");
    let soap_envelope = SoapEnvelope::from_xml(document.root_element())
        .expect("Failed to deserialize XML into SoapEnvelope");

    assert!(
        soap_envelope.body.as_ref().disconnect_response.is_some(),
        "SoapBody should contain DisconnectResponse"
    );
}

#[test]
fn test_parse_reconnect_response() {
    let xml_content = r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd"
    xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell"
    xmlns:p="http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd">
    <s:Header>
        <a:Action>http://schemas.microsoft.com/wbem/wsman/1/windows/shell/ReconnectResponse</a:Action>
        <a:MessageID>uuid:1C334787-EF2C-40E4-992F-DE4599ED2505</a:MessageID>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
        <p:OperationID s:mustUnderstand="false">uuid:172d68a1-9782-4f78-bebc-8b5db2355fda</p:OperationID>
        <p:SequenceId>1</p:SequenceId>
        <a:RelatesTo>uuid:17d0a667-c08e-4311-8d2d-069367f452d8</a:RelatesTo>
    </s:Header>
    <s:Body>
        <rsp:ReconnectResponse/>
    </s:Body>
</s:Envelope>"#;

    let document = ironposh_xml::parser::parse(xml_content).expect("Failed to parse XML content");
    let soap_envelope = SoapEnvelope::from_xml(document.root_element())
        .expect("Failed to deserialize XML into SoapEnvelope");

    assert!(
        soap_envelope.body.as_ref().reconnect_response.is_some(),
        "SoapBody should contain ReconnectResponse"
    );
}
