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

/// Minimal WSMan fault envelope (adapted from ironposh-winrm's error_response fixture).
const FAULT_ENVELOPE: &str = r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd"
    xmlns:p="http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd">
    <s:Header>
        <a:Action>http://schemas.dmtf.org/wbem/wsman/1/wsman/fault</a:Action>
        <a:MessageID>uuid:BB7AF8AE-D64A-422D-B36E-15A04FA17C5C</a:MessageID>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
        <a:RelatesTo>uuid:bead0162-a67d-424d-9e22-4a18b6aefea8</a:RelatesTo>
    </s:Header>
    <s:Body>
        <s:Fault>
            <s:Code>
                <s:Value>s:Sender</s:Value>
                <s:Subcode>
                    <s:Value>w:SchemaValidationError</s:Value>
                </s:Subcode>
            </s:Code>
            <s:Reason>
                <s:Text xml:lang="en-US">The SOAP XML in the message does not match the corresponding XML schema definition.</s:Text>
            </s:Reason>
        </s:Fault>
    </s:Body>
</s:Envelope>"#;

/// Mistimed Disconnect/Reconnect operations must be ignored, not kill the session.
#[test]
fn mistimed_disconnect_reconnect_is_nonfatal() {
    use ironposh_client_core::connector::{ActiveSessionOutput, UserOperation};
    use ironposh_client_core::runspace_pool::RunspacePoolState;

    let mut session = establish_active_session();

    // :reconnect while connected must be a no-op, not a session-fatal error.
    let out = session
        .accept_client_operation(UserOperation::Reconnect)
        .expect("mistimed Reconnect must be non-fatal");
    assert!(
        matches!(out, ActiveSessionOutput::Ignore),
        "mistimed Reconnect must be ignored, got: {out:?}"
    );
    assert_eq!(session.runspace_pool_state(), RunspacePoolState::Opened);

    // A second :disconnect while already Disconnecting must also be a no-op.
    let _ = session
        .accept_client_operation(UserOperation::Disconnect)
        .expect("accept Disconnect operation");
    let out = session
        .accept_client_operation(UserOperation::Disconnect)
        .expect("mistimed Disconnect must be non-fatal");
    assert!(
        matches!(out, ActiveSessionOutput::Ignore),
        "mistimed Disconnect must be ignored, got: {out:?}"
    );
    assert_eq!(
        session.runspace_pool_state(),
        RunspacePoolState::Disconnecting
    );
}

/// A SOAP fault answering the Disconnect request itself must abort the disconnect:
/// the pool reverts to Opened instead of staying stuck in Disconnecting forever.
#[test]
fn faulted_disconnect_reverts_pool_to_opened() {
    use ironposh_client_core::connector::{ActiveSessionOutput, UserOperation};
    use ironposh_client_core::runspace_pool::RunspacePoolState;

    let mut session = establish_active_session();

    let out = session
        .accept_client_operation(UserOperation::Disconnect)
        .expect("accept Disconnect operation");
    let ActiveSessionOutput::SendBack(reqs) = out else {
        panic!("expected SendBack for Disconnect, got {out:?}");
    };
    let (_request, conn_id) = support::expect_just_send(reqs.into_iter().next().unwrap());
    assert_eq!(
        session.runspace_pool_state(),
        RunspacePoolState::Disconnecting
    );

    // The server faults the Disconnect request on the same connection.
    let outputs = session
        .accept_server_response(support::xml_response(conn_id, FAULT_ENVELOPE.to_owned()))
        .expect("a faulted Disconnect must not kill the session");
    assert_eq!(
        session.runspace_pool_state(),
        RunspacePoolState::Opened,
        "a faulted Disconnect must revert the pool to Opened"
    );
    assert!(
        !outputs
            .iter()
            .any(|o| matches!(o, ActiveSessionOutput::OperationSuccess)),
        "a faulted Disconnect must not report success, got: {outputs:?}"
    );
}

/// While disconnecting, faults from OTHER connections (the dying in-flight Receive)
/// must still be tolerated without aborting the disconnect.
#[test]
fn fault_on_other_connection_while_disconnecting_is_tolerated() {
    use ironposh_client_core::connector::{ActiveSessionOutput, UserOperation};
    use ironposh_client_core::pipeline::{PipelineCommand, PipelineSpec};
    use ironposh_client_core::runspace_pool::RunspacePoolState;

    let mut session = establish_active_session();

    // Park a pipeline request on its own connection.
    let out = session
        .accept_client_operation(UserOperation::InvokeWithSpec {
            uuid: uuid::Uuid::new_v4(),
            spec: PipelineSpec {
                commands: vec![PipelineCommand::new_script("Get-Date".to_owned())],
            },
        })
        .expect("invoke pipeline");
    let ActiveSessionOutput::SendBack(reqs) = out else {
        panic!("expected SendBack for invoke, got {out:?}");
    };
    let (_request, pipeline_conn_id) = support::expect_just_send(reqs.into_iter().next().unwrap());

    // Fire the Disconnect; it is carried by a different connection.
    let out = session
        .accept_client_operation(UserOperation::Disconnect)
        .expect("accept Disconnect operation");
    let ActiveSessionOutput::SendBack(reqs) = out else {
        panic!("expected SendBack for Disconnect, got {out:?}");
    };
    let (_request, disconnect_conn_id) =
        support::expect_just_send(reqs.into_iter().next().unwrap());
    assert_ne!(pipeline_conn_id, disconnect_conn_id);

    // The in-flight pipeline request faults during teardown → tolerated.
    let outputs = session
        .accept_server_response(support::xml_response(
            pipeline_conn_id,
            FAULT_ENVELOPE.to_owned(),
        ))
        .expect("teardown fault must be tolerated while disconnecting");
    assert_eq!(
        session.runspace_pool_state(),
        RunspacePoolState::Disconnecting,
        "a fault on another connection must not abort the disconnect"
    );
    assert!(
        outputs
            .iter()
            .all(|o| matches!(o, ActiveSessionOutput::Ignore)),
        "teardown fault must be ignored, got: {outputs:?}"
    );

    // The real DisconnectResponse still completes the disconnect.
    session
        .accept_server_response(support::xml_response(
            disconnect_conn_id,
            shell_op_response_xml("DisconnectResponse", "<rsp:DisconnectResponse/>"),
        ))
        .expect("accept DisconnectResponse");
    assert_eq!(
        session.runspace_pool_state(),
        RunspacePoolState::Disconnected
    );
}

/// Reconnect after a mid-pipeline disconnect must resume the pipeline's Receive,
/// not just the runspace pool stream.
#[test]
fn reconnect_resumes_active_pipeline_streams() {
    use ironposh_client_core::connector::{ActiveSessionOutput, UserOperation};
    use ironposh_client_core::pipeline::{PipelineCommand, PipelineSpec};
    use ironposh_client_core::runspace_pool::RunspacePoolState;

    let mut session = establish_active_session();

    // Start a pipeline that will survive the disconnect.
    let pipeline_id = uuid::Uuid::new_v4();
    let out = session
        .accept_client_operation(UserOperation::InvokeWithSpec {
            uuid: pipeline_id,
            spec: PipelineSpec {
                commands: vec![PipelineCommand::new_script("Get-Date".to_owned())],
            },
        })
        .expect("invoke pipeline");
    assert!(
        matches!(out, ActiveSessionOutput::SendBack(_)),
        "expected SendBack for invoke, got {out:?}"
    );

    // Disconnect mid-pipeline.
    let out = session
        .accept_client_operation(UserOperation::Disconnect)
        .expect("accept Disconnect operation");
    let ActiveSessionOutput::SendBack(reqs) = out else {
        panic!("expected SendBack for Disconnect, got {out:?}");
    };
    let (_request, conn_id) = support::expect_just_send(reqs.into_iter().next().unwrap());
    session
        .accept_server_response(support::xml_response(
            conn_id,
            shell_op_response_xml("DisconnectResponse", "<rsp:DisconnectResponse/>"),
        ))
        .expect("accept DisconnectResponse");

    // Reconnect.
    let out = session
        .accept_client_operation(UserOperation::Reconnect)
        .expect("accept Reconnect operation");
    let ActiveSessionOutput::SendBack(reqs) = out else {
        panic!("expected SendBack for Reconnect, got {out:?}");
    };
    let (_request, conn_id) = support::expect_just_send(reqs.into_iter().next().unwrap());
    let outputs = session
        .accept_server_response(support::xml_response(
            conn_id,
            shell_op_response_xml("ReconnectResponse", "<rsp:ReconnectResponse/>"),
        ))
        .expect("accept ReconnectResponse");
    assert_eq!(session.runspace_pool_state(), RunspacePoolState::Opened);

    // The post-reconnect Receive must include the surviving pipeline's stream.
    let resumed_streams: Vec<_> = outputs
        .iter()
        .filter_map(|o| match o {
            ActiveSessionOutput::PendingReceive { desired_streams } => Some(desired_streams),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(
        resumed_streams
            .iter()
            .any(|s| s.command_id() == Some(&pipeline_id)),
        "post-reconnect Receive must cover the surviving pipeline, got: {resumed_streams:?}"
    );
}

/// Connect mode (`new_connect`) must emit a WSMan Connect addressed at the
/// given shell whose `connectXml` payload defragments back to
/// [SessionCapability, ConnectRunspacePool].
#[test]
fn connect_mode_emits_wsman_connect() {
    use base64::Engine;
    use ironposh_psrp::{
        ConnectRunspacePool, MessageType, PsValue,
        fragmentation::{DefragmentResult, Defragmenter},
    };

    let shell_id: uuid::Uuid = "2d6534d0-6b12-40e3-b773-cba26459cfa8".parse().unwrap();
    let mut connector = Connector::new_connect(support::test_config(), shell_id);

    let result = connector.step(None).expect("idle step in connect mode");
    let ConnectorStepResult::SendBack { try_send } = result else {
        panic!("expected SendBack for Connect");
    };
    let (request, _conn) = support::expect_just_send(try_send);
    let xml = request
        .body
        .expect("connect has a body")
        .as_str()
        .expect("plaintext body in HttpInsecure mode")
        .to_owned();

    assert!(
        xml.contains("http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Connect"),
        "request must carry the Connect action URI, got: {xml}"
    );
    assert!(
        xml.contains(&shell_id.to_string().to_uppercase()),
        "request must carry the ShellId selector, got: {xml}"
    );
    assert!(
        xml.contains("<rsp:Connect"),
        "request must carry the rsp:Connect body element, got: {xml}"
    );

    // Extract and decode the connectXml payload.
    let re = regex::Regex::new(r"<connectXml[^>]*>([^<]+)</connectXml>").unwrap();
    let payload_b64 = &re
        .captures(&xml)
        .expect("Connect request must carry a connectXml payload")[1];
    let payload = base64::engine::general_purpose::STANDARD
        .decode(payload_b64)
        .expect("connectXml must be valid base64");

    let mut defragmenter = Defragmenter::new();
    let DefragmentResult::Complete(messages) = defragmenter
        .defragment(&payload)
        .expect("defragment connectXml payload")
    else {
        panic!("connectXml payload must defragment to complete messages");
    };

    assert_eq!(
        messages.len(),
        2,
        "connectXml must carry exactly SessionCapability + ConnectRunspacePool"
    );
    assert_eq!(messages[0].message_type, MessageType::SessionCapability);
    assert_eq!(messages[1].message_type, MessageType::ConnectRunspacepool);
    for message in &messages {
        assert_eq!(
            message.rpid, shell_id,
            "PSRP messages must use the shell id as the pool RPID"
        );
    }

    // The CONNECT_RUNSPACEPOOL payload must carry the runspace limits.
    let ps_value = messages[1]
        .parse_ps_message()
        .expect("parse ConnectRunspacePool payload");
    let PsValue::Object(obj) = ps_value else {
        panic!("expected ConnectRunspacePool as PsValue::Object");
    };
    let connect_runspace_pool =
        ConnectRunspacePool::try_from(obj).expect("decode ConnectRunspacePool");
    assert_eq!(connect_runspace_pool.min_runspaces, 1);
    assert_eq!(connect_runspace_pool.max_runspaces, 1);
}

/// Feeding a ConnectResponse (SessionCapability + RunspacePoolInitData) must
/// bring the connect-mode connector straight to Connected with an Opened pool.
#[test]
fn connect_mode_reaches_connected() {
    use ironposh_client_core::runspace_pool::RunspacePoolState;
    use ironposh_psrp::RunspacePoolInitData;

    let shell_id: uuid::Uuid = "2d6534d0-6b12-40e3-b773-cba26459cfa8".parse().unwrap();
    let mut connector = Connector::new_connect(support::test_config(), shell_id);

    // 1. Idle step emits the WSMan Connect request.
    let result = connector.step(None).expect("idle step in connect mode");
    let ConnectorStepResult::SendBack { try_send } = result else {
        panic!("expected SendBack for Connect");
    };
    let (_request, conn_id) = support::expect_just_send(try_send);

    // 2. Reply with a ConnectResponse carrying the server-side negotiation.
    let session_capability = SessionCapability {
        protocol_version: "2.3".to_owned(),
        ps_version: "2.0".to_owned(),
        serialization_version: "1.1.0.1".to_owned(),
        time_zone: None,
    };
    let init_data = RunspacePoolInitData {
        min_runspaces: 1,
        max_runspaces: 1,
    };
    let connect_response =
        support::connect_response_xml(shell_id, &[&session_capability, &init_data]);

    let result = connector
        .step(Some(support::xml_response(conn_id, connect_response)))
        .expect("accept ConnectResponse");

    // 3. The connector must land in Connected with an Opened pool and fire the
    //    initial pool-stream Receive.
    let ConnectorStepResult::Connected {
        active_session,
        send_this_one_async_or_you_stuck,
    } = result
    else {
        panic!(
            "expected Connected after ConnectResponse, got {}",
            result.name()
        );
    };

    assert_eq!(
        active_session.runspace_pool_state(),
        RunspacePoolState::Opened,
        "pool must be Opened after a successful Connect"
    );
    assert_eq!(
        active_session.shell_id().as_deref(),
        Some(shell_id.to_string().to_uppercase().as_str()),
        "the active session must target the reattached shell"
    );

    let (request, _conn) = support::expect_just_send(send_this_one_async_or_you_stuck);
    let receive_xml = request
        .body
        .expect("receive has a body")
        .as_str()
        .expect("plaintext body")
        .to_owned();
    assert!(
        receive_xml.contains("http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Receive"),
        "connector must fire a Receive after connecting to the shell, got: {receive_xml}"
    );
    assert!(
        receive_xml.contains(&shell_id.to_string().to_uppercase()),
        "post-connect Receive must target the reattached shell, got: {receive_xml}"
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
