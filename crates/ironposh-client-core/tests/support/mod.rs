//! Fake-server harness: drives the sans-IO Connector with canned HTTP responses.
//!
//! Uses `AuthenticatorConfig::Basic` + `TransportSecurity::HttpInsecure` so request
//! and response bodies stay plaintext XML (no SSPI, no encryption, zero network).

// Shared across integration-test binaries; each binary uses a subset of helpers,
// and unused ones would otherwise fail CI under -D warnings.
#![allow(dead_code)]

use base64::Engine;
use ironposh_client_core::connector::{
    TransportSecurity, WinRmConfig,
    config::{AuthenticatorConfig, TlsOptions},
    connection_pool::{ConnectionId, TrySend},
    http::{HttpBody, HttpRequest, HttpResponse, HttpResponseTargeted, ServerAddress},
};
use ironposh_psrp::{
    Destination, HostDefaultData, HostInfo, PowerShellRemotingMessage, Size,
    fragmentation::Fragment, ps_value::PsObjectWithType,
};
use ironposh_winrm::{
    cores::{Attribute, Namespace, ReceiveResponse, Tag, Text, tag_name::{Envelope, Stream}},
    rsp::receive::ReceiveResponseValue,
    soap::{SoapEnvelope, body::SoapBody},
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

    let streams: Vec<Tag<'_, Text<'_>, Stream>> = base64_fragments
        .iter()
        .map(|fragment| {
            Tag::from_name(Stream)
                .with_value(Text::from(fragment.as_str()))
                .with_attribute(Attribute::Name("stdout".into()))
        })
        .collect();

    let receive_response_value = ReceiveResponseValue::builder()
        .streams(streams)
        .command_state(None)
        .build();

    let receive_response_tag = Tag::from_name(ReceiveResponse)
        .with_value(receive_response_value)
        .with_declaration(Namespace::WsmanShell);

    let body = SoapBody::builder()
        .receive_response(receive_response_tag)
        .build();

    let envelope = SoapEnvelope::builder().body(body).build();

    let soap = Tag::<SoapEnvelope<'_>, Envelope>::new(envelope)
        .with_declaration(Namespace::SoapEnvelope2003)
        .with_declaration(Namespace::WsAddressing2004)
        .with_declaration(Namespace::DmtfWsmanSchema)
        .with_declaration(Namespace::MsWsmanSchema);

    let element: Element<'_> = soap.into_element();
    element
        .to_xml_string()
        .expect("serialize ReceiveResponse envelope")
}
