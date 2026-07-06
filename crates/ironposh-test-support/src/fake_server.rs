//! Fake-server harness: drives the sans-IO Connector with canned HTTP responses.
//!
//! Uses `AuthenticatorConfig::Basic` + `TransportSecurity::HttpInsecure` so request
//! and response bodies stay plaintext XML (no SSPI, no encryption, zero network).

use base64::Engine;
use ironposh_client_core::connector::{
    config::{AuthenticatorConfig, TlsOptions},
    connection_pool::{ConnectionId, TrySend},
    http::{HttpBody, HttpRequest, HttpResponse, HttpResponseTargeted, ServerAddress},
    TransportSecurity, WinRmConfig,
};
use ironposh_psrp::{
    fragmentation::Fragment, ps_value::PsObjectWithType, Destination, HostDefaultData, HostInfo,
    PowerShellRemotingMessage, Size,
};
use ironposh_winrm::{
    cores::{Attribute, Namespace, StreamTag, Tag, Text},
    rsp::receive::{ReceiveResponseTag, ReceiveResponseValue},
    soap::{body::SoapBody, Envelope, SoapEnvelope},
};
use ironposh_xml::builder::Element;
use uuid::Uuid;

/// Basic auth + HttpInsecure config pointed at a fake server (never dialed).
pub fn test_config() -> WinRmConfig {
    let size = Size {
        width: 80,
        height: 25,
    };

    let host_data = HostDefaultData::builder()
        .buffer_size(size.clone())
        .window_size(size.clone())
        .max_window_size(size.clone())
        .max_physical_window_size(size)
        .build();

    let host_info = HostInfo::builder()
        .host_default_data(host_data)
        .use_runspace_host(true)
        .build();

    WinRmConfig {
        server: (ServerAddress::parse("127.0.0.1").unwrap(), 5985),
        transport: TransportSecurity::HttpInsecure,
        authentication: AuthenticatorConfig::Basic {
            username: "user".into(),
            password: "pass".into(),
        },
        host_info,
        operation_timeout_secs: Some(1.0),
        tls: TlsOptions::default(),
        configuration_name: None,
    }
}

/// Extract (request, connection_id) from a TrySend (Basic auth never hits the SSPI path).
pub fn expect_just_send(try_send: TrySend) -> (HttpRequest, ConnectionId) {
    match try_send {
        TrySend::JustSend { request, conn_id } => (request, conn_id),
        TrySend::AuthNeeded { .. } => panic!("expected JustSend, got AuthNeeded"),
    }
}

/// Build a 200 response carrying `xml`, targeted back at `conn_id`.
pub fn xml_response(conn_id: ConnectionId, xml: String) -> HttpResponseTargeted {
    HttpResponseTargeted::new(
        HttpResponse {
            status_code: 200,
            headers: vec![],
            body: HttpBody::Xml(xml),
            peer_cert_der: None,
        },
        conn_id,
        None,
    )
}

/// Parse the client's RunspacePool ID (== ShellId attribute) out of the Create request XML.
pub fn extract_shell_id(create_xml: &str) -> Uuid {
    let re = regex::Regex::new(r#"ShellId="([0-9a-fA-F-]{36})""#).unwrap();
    let captures = re
        .captures(create_xml)
        .expect("Create request must carry a ShellId attribute");
    captures[1].parse().expect("ShellId must be a UUID")
}

/// Build a ConnectResponse SOAP envelope whose `connectResponseXml` carries the
/// given server-to-client PSRP messages.
///
/// The messages ride as a base64 blob of concatenated single fragments,
/// mirroring how a real server answers a WSMan Connect to a disconnected
/// shell (MS-WSMV 3.1.4.15).
pub fn connect_response_xml(rpid: Uuid, messages: &[&dyn PsObjectWithType]) -> String {
    let mut payload = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        let remoting_message = PowerShellRemotingMessage::new(
            Destination::Client,
            message.message_type(),
            rpid,
            None,
            &message.to_ps_object(),
        )
        .expect("serialize PSRP message");

        let fragment = Fragment::new(index as u64 + 1, 0, remoting_message.pack(), true, true);
        payload.extend(fragment.pack());
    }

    let payload_b64 = base64::engine::general_purpose::STANDARD.encode(payload);

    format!(
        r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd"
    xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell"
    xmlns:p="http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd">
    <s:Header>
        <a:Action>http://schemas.microsoft.com/wbem/wsman/1/windows/shell/ConnectResponse</a:Action>
        <a:MessageID>uuid:6C334787-EF2C-40E4-992F-DE4599ED2505</a:MessageID>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
        <a:RelatesTo>uuid:87d0a667-c08e-4311-8d2d-069367f452d8</a:RelatesTo>
    </s:Header>
    <s:Body>
        <rsp:ConnectResponse>
            <connectResponseXml xmlns="http://schemas.microsoft.com/powershell">{payload_b64}</connectResponseXml>
        </rsp:ConnectResponse>
    </s:Body>
</s:Envelope>"#
    )
}

/// Build a CommandResponse SOAP envelope acknowledging a pipeline start.
pub fn command_response_xml(command_id: Uuid) -> String {
    format!(
        r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell">
    <s:Header>
        <a:Action>http://schemas.microsoft.com/wbem/wsman/1/windows/shell/CommandResponse</a:Action>
        <a:MessageID>uuid:{message_id}</a:MessageID>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
    </s:Header>
    <s:Body>
        <rsp:CommandResponse>
            <rsp:CommandId>{command_id}</rsp:CommandId>
        </rsp:CommandResponse>
    </s:Body>
</s:Envelope>"#,
        message_id = Uuid::new_v4(),
    )
}

/// Build a `w:TimedOut` WS-Management fault envelope — what a real server
/// returns when a Receive exhausts its OperationTimeout with no data.
pub fn timeout_fault_xml() -> String {
    format!(
        r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd">
    <s:Header>
        <a:Action>http://schemas.dmtf.org/wbem/wsman/1/wsman/fault</a:Action>
        <a:MessageID>uuid:{message_id}</a:MessageID>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
    </s:Header>
    <s:Body>
        <s:Fault>
            <s:Code>
                <s:Value>s:Receiver</s:Value>
                <s:Subcode>
                    <s:Value>w:TimedOut</s:Value>
                </s:Subcode>
            </s:Code>
            <s:Reason>
                <s:Text xml:lang="en-US">The WS-Management service cannot complete the operation within the time specified in OperationTimeout.</s:Text>
            </s:Reason>
        </s:Fault>
    </s:Body>
</s:Envelope>"#,
        message_id = Uuid::new_v4(),
    )
}

/// Build a SignalResponse SOAP envelope. `relates_to` must echo the Signal
/// request's MessageID (the client correlates signal acks through it).
pub fn signal_response_xml(relates_to: &str) -> String {
    format!(
        r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell">
    <s:Header>
        <a:Action>http://schemas.microsoft.com/wbem/wsman/1/windows/shell/SignalResponse</a:Action>
        <a:MessageID>uuid:{message_id}</a:MessageID>
        <a:RelatesTo>{relates_to}</a:RelatesTo>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
    </s:Header>
    <s:Body>
        <rsp:SignalResponse/>
    </s:Body>
</s:Envelope>"#,
        message_id = Uuid::new_v4(),
    )
}

/// Build a ReceiveResponse SOAP envelope carrying pipeline-scoped PSRP messages
/// (streams tagged with `CommandId`, PSRP `pid` set to the command id).
///
/// `object_id_start` seeds the PSRP fragment object ids so ids stay unique
/// across successive responses in one session. When `done` is set, a
/// `CommandState Done` element is appended — mirroring how a real server closes
/// out a finished pipeline.
pub fn pipeline_receive_response_xml(
    rpid: Uuid,
    command_id: Uuid,
    messages: &[&dyn PsObjectWithType],
    done: bool,
    object_id_start: u64,
) -> String {
    use std::fmt::Write as _;

    let mut streams = String::new();
    for (index, message) in messages.iter().enumerate() {
        let remoting_message = PowerShellRemotingMessage::new(
            Destination::Client,
            message.message_type(),
            rpid,
            Some(command_id),
            &message.to_ps_object(),
        )
        .expect("serialize PSRP message");

        let fragment = Fragment::new(
            object_id_start + index as u64,
            0,
            remoting_message.pack(),
            true,
            true,
        );
        let payload = base64::engine::general_purpose::STANDARD.encode(fragment.pack());
        write!(
            streams,
            r#"<rsp:Stream Name="stdout" CommandId="{command_id}">{payload}</rsp:Stream>"#
        )
        .expect("write stream XML");
    }

    let command_state = if done {
        format!(
            r#"<rsp:CommandState CommandId="{command_id}" State="http://schemas.microsoft.com/wbem/wsman/1/windows/shell/CommandState/Done"><rsp:ExitCode>0</rsp:ExitCode></rsp:CommandState>"#
        )
    } else {
        String::new()
    };

    format!(
        r#"<s:Envelope xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell">
    <s:Header>
        <a:Action>http://schemas.microsoft.com/wbem/wsman/1/windows/shell/ReceiveResponse</a:Action>
        <a:MessageID>uuid:{message_id}</a:MessageID>
        <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
    </s:Header>
    <s:Body>
        <rsp:ReceiveResponse>{streams}{command_state}</rsp:ReceiveResponse>
    </s:Body>
</s:Envelope>"#,
        message_id = Uuid::new_v4(),
    )
}

/// Build a ReceiveResponse SOAP envelope carrying the given server-to-client PSRP
/// messages as single-fragment `stdout` streams (no command id => runspace pool stream).
pub fn receive_response_xml(rpid: Uuid, messages: &[&dyn PsObjectWithType]) -> String {
    let base64_fragments: Vec<String> = messages
        .iter()
        .enumerate()
        .map(|(index, message)| {
            let remoting_message = PowerShellRemotingMessage::new(
                Destination::Client,
                message.message_type(),
                rpid,
                None,
                &message.to_ps_object(),
            )
            .expect("serialize PSRP message");

            let fragment = Fragment::new(index as u64 + 1, 0, remoting_message.pack(), true, true);
            base64::engine::general_purpose::STANDARD.encode(fragment.pack())
        })
        .collect();

    let streams: Vec<Tag<'_, Text<'_>, StreamTag>> = base64_fragments
        .iter()
        .map(|fragment| {
            Tag::from_name(StreamTag)
                .with_value(Text::from(fragment.as_str()))
                .with_attribute(Attribute::Name("stdout".into()))
        })
        .collect();

    let receive_response_value = ReceiveResponseValue::builder()
        .streams(streams)
        .command_state(None)
        .build();

    let receive_response_tag = Tag::from_name(ReceiveResponseTag)
        .with_value(receive_response_value)
        .with_declaration(Namespace::WsmanShell);

    let body = SoapBody::builder()
        .receive_response(receive_response_tag)
        .build();

    let envelope = SoapEnvelope::builder().body(body).build();

    let soap = Envelope::new(envelope)
        .with_declaration(Namespace::SoapEnvelope2003)
        .with_declaration(Namespace::WsAddressing2004)
        .with_declaration(Namespace::DmtfWsmanSchema)
        .with_declaration(Namespace::MsWsmanSchema);

    let element: Element<'_> = soap.into_element();
    element
        .to_xml_string()
        .expect("serialize ReceiveResponse envelope")
}
