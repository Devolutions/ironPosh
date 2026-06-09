//! Fixture-driven Connector handshake tests: a fake server drives `Connector::step`
//! all the way to `Connected` with zero network.

mod support;

use ironposh_client_core::connector::{Connector, ConnectorStepResult};
use ironposh_psrp::{
    ApplicationPrivateData, RunspacePoolStateMessage, RunspacePoolStateValue, SessionCapability,
};

/// Idle step must emit the shell Create envelope with Basic auth preformatted.
#[test]
fn idle_step_emits_shell_create() {
    let mut connector = Connector::new(support::test_config());

    let result = connector.step(None).expect("idle step");
    let ConnectorStepResult::SendBack { try_send } = result else {
        panic!("expected SendBack");
    };

    let (request, _conn) = support::expect_just_send(try_send);
    let body = request.body.expect("create has a body");
    let xml = body.as_str().expect("plaintext body in HttpInsecure mode");

    assert!(xml.contains("http://schemas.xmlsoap.org/ws/2004/09/transfer/Create"));
    assert!(xml.contains("http://schemas.microsoft.com/powershell/Microsoft.PowerShell"));
    assert!(
        request
            .headers
            .iter()
            .any(|(k, _)| k == "Authorization"),
        "Basic auth header must be present"
    );
}

/// Drive the connector through the full handshake against a fake server:
/// Create -> CreateResponse -> Receive -> ReceiveResponse(PSRP negotiation) -> Connected.
#[test]
fn handshake_reaches_connected() {
    let mut connector = Connector::new(support::test_config());

    // 1. Idle step emits the shell Create request; learn the client's RPID from it.
    let result = connector.step(None).expect("idle step");
    let ConnectorStepResult::SendBack { try_send } = result else {
        panic!("expected SendBack for Create");
    };
    let (request, conn_id) = support::expect_just_send(try_send);
    let create_xml = request
        .body
        .expect("create has a body")
        .as_str()
        .expect("plaintext body")
        .to_owned();
    let rpid = support::extract_shell_id(&create_xml);

    // 2. Reply with a CreateResponse fixture; the connector must fire a Receive.
    let create_response = include_str!("resources/resource_created.xml");
    let result = connector
        .step(Some(support::xml_response(
            conn_id,
            create_response.to_owned(),
        )))
        .expect("accept CreateResponse");
    let ConnectorStepResult::SendBack { try_send } = result else {
        panic!("expected SendBack for Receive");
    };
    let (request, conn_id) = support::expect_just_send(try_send);
    let receive_xml = request
        .body
        .expect("receive has a body")
        .as_str()
        .expect("plaintext body")
        .to_owned();
    assert!(
        receive_xml.contains("http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Receive"),
        "connector must fire a Receive after shell creation"
    );

    // 3. Reply with the PSRP negotiation messages the server would send.
    let session_capability = SessionCapability {
        protocol_version: "2.3".to_owned(),
        ps_version: "2.0".to_owned(),
        serialization_version: "1.1.0.1".to_owned(),
        time_zone: None,
    };
    let application_private_data = ApplicationPrivateData::new();
    let pool_opened = RunspacePoolStateMessage::builder()
        .runspace_state(RunspacePoolStateValue::Opened)
        .build();

    let receive_response = support::receive_response_xml(
        rpid,
        &[&session_capability, &application_private_data, &pool_opened],
    );

    let result = connector
        .step(Some(support::xml_response(conn_id, receive_response)))
        .expect("accept ReceiveResponse");

    match result {
        ConnectorStepResult::Connected { .. } => {}
        other @ ConnectorStepResult::SendBack { .. } => {
            panic!("expected Connected, got {}", other.name())
        }
    }
}
