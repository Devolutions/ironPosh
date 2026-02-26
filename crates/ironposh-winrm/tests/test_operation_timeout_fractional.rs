use ironposh_winrm::{
    soap::body::SoapBody,
    ws_management::{WsAction, WsMan},
};
use ironposh_xml::builder::Builder;

#[test]
fn wsman_operation_timeout_supports_fractional_seconds() {
    let wsman = WsMan::builder()
        .to("http://example.local/wsman".to_string())
        .operation_timeout(0.5)
        .build();

    let envelope = wsman.invoke(
        &WsAction::Get,
        None,
        SoapBody::builder().build(),
        None,
        None,
    );
    let element = envelope.into_element();
    let xml_string = Builder::new(None, element)
        .to_xml_string()
        .expect("failed to build XML");

    // WS-Management OperationTimeout format is `PT{seconds:.3}S`.
    assert!(
        xml_string.contains("OperationTimeout>PT0.500S</"),
        "expected fractional operation timeout in SOAP header. xml={xml_string}"
    );
}
