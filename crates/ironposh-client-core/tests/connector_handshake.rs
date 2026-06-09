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
    let auth_value = request
        .headers
        .iter()
        .find(|(k, _)| k == "Authorization")
        .map(|(_, v)| v.as_str())
        .expect("Basic auth header must be present");
    // base64("user:pass") — pinned to the harness config credentials.
    assert_eq!(
        auth_value, "Basic dXNlcjpwYXNz",
        "Authorization header must carry the exact Basic credentials"
    );
}

/// A JEA `configuration_name` must replace the default shell resource URI.
#[test]
fn configuration_name_sets_shell_resource_uri() {
    let mut config = support::test_config();
    config.configuration_name = Some("MyJEAEndpoint".into());
    let mut connector = Connector::new(config);

    let result = connector.step(None).expect("idle step");
    let ConnectorStepResult::SendBack { try_send } = result else {
        panic!("expected SendBack");
    };

    let (request, _conn) = support::expect_just_send(try_send);
    let body = request.body.expect("create has a body");
    let xml = body.as_str().expect("plaintext body in HttpInsecure mode");

    assert!(
        xml.contains("http://schemas.microsoft.com/powershell/MyJEAEndpoint"),
        "shell Create must target the JEA endpoint resource URI"
    );
    assert!(
        !xml.contains("powershell/Microsoft.PowerShell"),
        "default resource URI must not appear when configuration_name is set"
    );
}

/// With a JEA `configuration_name`, post-create operations must keep targeting the
/// JEA resource URI even when the server's CreateResponse omits the ResourceUri echo.
#[test]
fn configuration_name_survives_create_response_without_resource_uri_echo() {
    let mut config = support::test_config();
    config.configuration_name = Some("MyJEAEndpoint".into());
    let mut connector = Connector::new(config);

    // 1. Idle step emits the shell Create request.
    let result = connector.step(None).expect("idle step");
    let ConnectorStepResult::SendBack { try_send } = result else {
        panic!("expected SendBack for Create");
    };
    let (_request, conn_id) = support::expect_just_send(try_send);

    // 2. Reply with a CreateResponse whose Shell does NOT echo a ResourceUri element.
    let create_response = include_str!("resources/resource_created.xml");
    let strip_resource_uri =
        regex::Regex::new(r"(?s)<rsp:ResourceUri>.*?</rsp:ResourceUri>").expect("valid regex");
    let create_response = strip_resource_uri.replace(create_response, "").into_owned();
    assert!(
        !create_response.contains("rsp:ResourceUri"),
        "fixture must not echo a shell ResourceUri for this test"
    );

    let result = connector
        .step(Some(support::xml_response(conn_id, create_response)))
        .expect("accept CreateResponse");
    let ConnectorStepResult::SendBack { try_send } = result else {
        panic!("expected SendBack for Receive");
    };

    // 3. The post-create Receive must still target the JEA endpoint resource URI.
    let (request, _conn_id) = support::expect_just_send(try_send);
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
    assert!(
        receive_xml.contains("powershell/MyJEAEndpoint"),
        "post-create Receive must target the JEA endpoint resource URI"
    );
    assert!(
        !receive_xml.contains("powershell/Microsoft.PowerShell"),
        "post-create Receive must not fall back to the default resource URI"
    );
}

/// Drive the connector to `Connected` against the fake server and return the
/// ActiveSession (handshake mechanics are asserted by `handshake_reaches_connected`).
fn establish_active_session() -> ironposh_client_core::connector::active_session::ActiveSession {
    let mut connector = Connector::new(support::test_config());

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
    let (_request, conn_id) = support::expect_just_send(try_send);

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
        ConnectorStepResult::Connected { active_session, .. } => *active_session,
        other @ ConnectorStepResult::SendBack { .. } => {
            panic!("expected Connected, got {}", other.name())
        }
    }
}

/// Build a minimal response envelope with the given body element (e.g.
/// `<rsp:DisconnectResponse/>`), mirroring real WinRM response headers.
fn shell_op_response_xml(action: &str, body_element: &str) -> String {
    format!(
        r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd"
    xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell"
    xmlns:p="http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd">
    <s:Header>
        <a:Action>http://schemas.microsoft.com/wbem/wsman/1/windows/shell/{action}</a:Action>
        <a:MessageID>uuid:6C334787-EF2C-40E4-992F-DE4599ED2505</a:MessageID>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
        <a:RelatesTo>uuid:87d0a667-c08e-4311-8d2d-069367f452d8</a:RelatesTo>
    </s:Header>
    <s:Body>
        {body_element}
    </s:Body>
</s:Envelope>"#
    )
}

/// Same-client disconnect → reconnect routed through the ActiveSession layer.
#[test]
fn disconnect_reconnect_through_active_session() {
    use ironposh_client_core::connector::{ActiveSessionOutput, UserOperation};
    use ironposh_client_core::runspace_pool::RunspacePoolState;

    // The fixture's server-assigned shell id (resources/resource_created.xml).
    const FIXTURE_SHELL_ID: &str = "07936B27-7752-4325-8B0D-E7A1E9448320";

    let mut session = establish_active_session();
    assert_eq!(session.runspace_pool_state(), RunspacePoolState::Opened);
    assert_eq!(session.shell_id().as_deref(), Some(FIXTURE_SHELL_ID));

    // 1. Disconnect operation → outgoing request carries the Disconnect action
    //    and targets the shell.
    let out = session
        .accept_client_operation(UserOperation::Disconnect)
        .expect("accept Disconnect operation");
    let ActiveSessionOutput::SendBack(reqs) = out else {
        panic!("expected SendBack for Disconnect, got {out:?}");
    };
    assert_eq!(reqs.len(), 1, "Disconnect must produce exactly one request");
    let (request, conn_id) = support::expect_just_send(reqs.into_iter().next().unwrap());
    let disconnect_xml = request
        .body
        .expect("disconnect has a body")
        .as_str()
        .expect("plaintext body")
        .to_owned();
    assert!(
        disconnect_xml
            .contains("http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Disconnect"),
        "outgoing XML must carry the Disconnect action, got: {disconnect_xml}"
    );
    assert!(
        disconnect_xml.contains(FIXTURE_SHELL_ID),
        "outgoing XML must carry the ShellId selector, got: {disconnect_xml}"
    );
    assert_eq!(
        session.runspace_pool_state(),
        RunspacePoolState::Disconnecting
    );

    // 2. DisconnectResponse → pool is Disconnected.
    let outputs = session
        .accept_server_response(support::xml_response(
            conn_id,
            shell_op_response_xml("DisconnectResponse", "<rsp:DisconnectResponse/>"),
        ))
        .expect("accept DisconnectResponse");
    assert_eq!(
        session.runspace_pool_state(),
        RunspacePoolState::Disconnected
    );
    assert!(
        outputs
            .iter()
            .all(|o| matches!(o, ActiveSessionOutput::OperationSuccess)),
        "DisconnectResponse must yield OperationSuccess only, got: {outputs:?}"
    );

    // 3. Reconnect operation → outgoing request carries the Reconnect action.
    let out = session
        .accept_client_operation(UserOperation::Reconnect)
        .expect("accept Reconnect operation");
    let ActiveSessionOutput::SendBack(reqs) = out else {
        panic!("expected SendBack for Reconnect, got {out:?}");
    };
    let (request, conn_id) = support::expect_just_send(reqs.into_iter().next().unwrap());
    let reconnect_xml = request
        .body
        .expect("reconnect has a body")
        .as_str()
        .expect("plaintext body")
        .to_owned();
    assert!(
        reconnect_xml.contains("http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Reconnect"),
        "outgoing XML must carry the Reconnect action, got: {reconnect_xml}"
    );
    assert!(
        reconnect_xml.contains(FIXTURE_SHELL_ID),
        "outgoing XML must carry the ShellId selector, got: {reconnect_xml}"
    );
    assert_eq!(session.runspace_pool_state(), RunspacePoolState::Connecting);

    // 4. ReconnectResponse → pool is Opened again and the receive loop resumes.
    let outputs = session
        .accept_server_response(support::xml_response(
            conn_id,
            shell_op_response_xml("ReconnectResponse", "<rsp:ReconnectResponse/>"),
        ))
        .expect("accept ReconnectResponse");
    assert_eq!(session.runspace_pool_state(), RunspacePoolState::Opened);
    assert!(
        outputs
            .iter()
            .any(|o| matches!(o, ActiveSessionOutput::PendingReceive { .. })),
        "ReconnectResponse must schedule a Receive to resume the loop, got: {outputs:?}"
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
    // Note: the fixture's ShellId differs from the client-generated RPID. The connector
    // currently accepts whatever id the server returns without cross-checking, so the
    // mismatch is harmless here; if id validation is ever added, regenerate the fixture.
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
