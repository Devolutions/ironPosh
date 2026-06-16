use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use ironposh_async::HttpClient;
use ironposh_client_core::{
    connector::{
        auth_sequence::SspiAuthSequence,
        authenticator::SecContextMaybeInit,
        config::TlsOptions,
        connection_pool::{ConnectionId, SecContextInited, TrySend},
        http::{
            HttpBody, HttpRequest, HttpRequestAction, HttpResponse, HttpResponseTargeted, Method,
        },
    },
    credentials::ClientUserName,
};
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, info, instrument};
use url::Url;
use uuid::Uuid;

use crate::{config::AuthMethod, http_client::ReqwestHttpClient};

type GatewayWs = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

#[derive(Debug, Clone)]
pub struct GatewaySession {
    pub websocket_url: Url,
    pub kdc_proxy_url: Option<Url>,
}

#[derive(Debug, Clone)]
pub struct GatewayTokenConfig {
    pub base_url: String,
    pub webapp_username: Option<String>,
    pub webapp_password: Option<String>,
    pub server: String,
    pub port: u16,
    pub https: bool,
    pub username: String,
    pub domain: String,
    pub auth_method: AuthMethod,
    pub kdc_address: Option<String>,
    pub kdc_proxy_url: Option<String>,
}

pub enum CliHttpClient {
    Direct(ReqwestHttpClient),
    Gateway(GatewayHttpViaWsClient),
}

impl HttpClient for CliHttpClient {
    async fn send_request(&self, try_send: TrySend) -> Result<HttpResponseTargeted> {
        match self {
            Self::Direct(client) => client.send_request(try_send).await,
            Self::Gateway(client) => Box::pin(client.send_request(try_send)).await,
        }
    }
}

pub struct GatewayHttpViaWsClient {
    websocket_url: Url,
    websocket: Arc<Mutex<Option<GatewayWs>>>,
}

impl GatewayHttpViaWsClient {
    pub fn new(websocket_url: Url) -> Self {
        let redacted_url = redact_gateway_url(&websocket_url);
        info!(
            gateway_url = %redacted_url,
            "creating Gateway HTTP-over-WebSocket client"
        );
        Self {
            websocket_url,
            websocket: Arc::new(Mutex::new(None)),
        }
    }

    #[instrument(skip(self, request), fields(method = ?request.method, url = %request.url))]
    // The websocket lock must be held across the whole request/response exchange.
    #[allow(clippy::significant_drop_tightening)]
    async fn send_http_request(
        &self,
        request: HttpRequest,
        conn_id: ConnectionId,
    ) -> Result<HttpResponse> {
        let bytes = serialize_http_request(&request)?;
        let mut websocket = self.websocket.lock().await;

        if websocket.is_none() {
            let redacted_url = redact_gateway_url(&self.websocket_url);
            info!(
                conn_id = conn_id.inner(),
                gateway_url = %redacted_url,
                "opening Gateway WebSocket"
            );
            let (stream, _) = connect_async(self.websocket_url.as_str())
                .await
                .context("failed to open Gateway WebSocket")?;
            *websocket = Some(stream);
        }

        let stream = websocket
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Gateway WebSocket missing after connection"))?;

        debug!(
            conn_id = conn_id.inner(),
            bytes_len = bytes.len(),
            "sending serialized HTTP request over Gateway WebSocket"
        );
        stream
            .send(Message::Binary(bytes.into()))
            .await
            .context("failed to send HTTP request over Gateway WebSocket")?;

        let mut decoder = HttpResponseDecoder::new(16 * 1024 * 1024);
        loop {
            let message = stream
                .next()
                .await
                .ok_or_else(|| anyhow::anyhow!("Gateway WebSocket closed"))?
                .context("Gateway WebSocket read failed")?;

            match message {
                Message::Binary(bytes) => {
                    if let Some(response) = decoder.feed(&bytes)? {
                        return Ok(response);
                    }
                }
                Message::Text(text) => {
                    return Err(anyhow::anyhow!(
                        "Gateway WebSocket returned text frame: {text}"
                    ));
                }
                Message::Close(frame) => {
                    return Err(anyhow::anyhow!(
                        "Gateway WebSocket closed while waiting for response: {frame:?}"
                    ));
                }
                Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
            }
        }
    }
}

impl HttpClient for GatewayHttpViaWsClient {
    #[instrument(name = "gateway_http_request", level = "debug", skip(self, try_send))]
    async fn send_request(&self, try_send: TrySend) -> Result<HttpResponseTargeted> {
        match try_send {
            TrySend::JustSend { request, conn_id } => {
                let response = self.send_http_request(request, conn_id).await?;
                Ok(HttpResponseTargeted::new(response, conn_id, None))
            }
            TrySend::AuthNeeded { mut auth_sequence } => {
                info!("starting Gateway authentication sequence");
                let mut auth_response: Option<HttpResponse> = None;

                loop {
                    let (seq, mut holder) = auth_sequence.prepare();
                    let init = match seq
                        .try_init_sec_context(auth_response.as_ref(), &mut holder)?
                    {
                        SecContextMaybeInit::Initialized(sec) => sec,
                        SecContextMaybeInit::RunGenerator {
                            mut packet,
                            mut generator_holder,
                        } => {
                            info!("running generator for KDC communication");
                            loop {
                                // Gateway KDC-proxy TLS policy is the gateway deployment's own;
                                // client TLS flags are rejected with --gateway.
                                let kdc_response = ReqwestHttpClient::send_kdc_network_request(
                                    packet,
                                    &TlsOptions::default(),
                                )
                                .await
                                .context("failed to send KDC request during Gateway auth")?;

                                match SspiAuthSequence::resume(generator_holder, kdc_response)? {
                                    SecContextMaybeInit::Initialized(sec) => break sec,
                                    SecContextMaybeInit::RunGenerator {
                                        packet: next_packet,
                                        generator_holder: next_holder,
                                    } => {
                                        packet = next_packet;
                                        generator_holder = next_holder;
                                    }
                                }
                            }
                        }
                    };

                    match auth_sequence.process_sec_ctx_init(&init)? {
                        SecContextInited::Continue { request, sequence } => {
                            let HttpRequestAction {
                                connection_id,
                                request,
                            } = request;
                            let response = self.send_http_request(request, connection_id).await?;
                            auth_response = Some(response);
                            auth_sequence = sequence;
                        }
                        SecContextInited::SendRequest {
                            request,
                            authenticated_http_channel_cert,
                        } => {
                            let HttpRequestAction {
                                connection_id,
                                request,
                            } = request;
                            let response = self.send_http_request(request, connection_id).await?;
                            return Ok(HttpResponseTargeted::new(
                                response,
                                connection_id,
                                Some(authenticated_http_channel_cert),
                            ));
                        }
                    }
                }
            }
        }
    }
}

pub async fn create_gateway_session(config: &GatewayTokenConfig) -> Result<GatewaySession> {
    let http_base = to_http_base_url(&config.base_url)?;
    let ws_base = to_ws_base_url(&config.base_url)?;

    let webapp_username = config
        .webapp_username
        .clone()
        .or_else(|| std::env::var("VITE_GATEWAY_WEBAPP_USERNAME").ok())
        .unwrap_or_else(|| "admin".to_string());
    let webapp_password = config
        .webapp_password
        .clone()
        .or_else(|| std::env::var("VITE_GATEWAY_WEBAPP_PASSWORD").ok())
        .unwrap_or_else(|| "admin".to_string());

    let app_token = generate_app_token(&http_base, &webapp_username, &webapp_password).await?;

    let session_id = Uuid::new_v4();
    let (destination_scheme, protocol) = gateway_winrm_transport(config.https);
    let destination = format!("{destination_scheme}://{}:{}", config.server, config.port);
    let association_token = generate_session_token(
        &http_base,
        &app_token,
        json!({
            "content_type": "ASSOCIATION",
            "protocol": protocol,
            "destination": destination,
            "lifetime": 60,
            "session_id": session_id.to_string(),
        }),
    )
    .await?;

    let websocket_url = ws_base
        .join(&format!(
            "/jet/fwd/{destination_scheme}/{session_id}?token={association_token}"
        ))
        .context("failed to build Gateway WebSocket URL")?;

    let kdc_proxy_url = match config.auth_method {
        AuthMethod::Kerberos | AuthMethod::Negotiate => {
            if let Some(kdc_proxy_url) = &config.kdc_proxy_url {
                Some(kdc_proxy_url.parse().context("invalid KDC proxy URL")?)
            } else {
                let krb_realm = kerberos_realm(&config.username, &config.domain)?;
                let kdc_address = config
                    .kdc_address
                    .clone()
                    .unwrap_or_else(|| default_kdc_address(&config.server, &krb_realm));
                let kdc_token = generate_session_token(
                    &http_base,
                    &app_token,
                    json!({
                        "content_type": "KDC",
                        "krb_kdc": kdc_address,
                        "krb_realm": krb_realm,
                        "lifetime": 60,
                    }),
                )
                .await?;
                Some(
                    http_base
                        .join(&format!("/jet/KdcProxy/{kdc_token}"))
                        .context("failed to build KDC proxy URL")?,
                )
            }
        }
        AuthMethod::Basic | AuthMethod::Ntlm => None,
    };

    Ok(GatewaySession {
        websocket_url,
        kdc_proxy_url,
    })
}

async fn generate_app_token(http_base: &Url, username: &str, password: &str) -> Result<String> {
    let url = http_base
        .join("/jet/webapp/app-token")
        .context("failed to build app-token URL")?;
    let token = Client::new()
        .post(url)
        .basic_auth(username, Some(password))
        .json(&json!({
            "content_type": "WEBAPP",
            "subject": username,
            "lifetime": 7200,
        }))
        .send()
        .await
        .context("failed to request Gateway app token")?
        .error_for_status()
        .context("Gateway app-token request failed")?
        .text()
        .await
        .context("failed to read Gateway app token")?;

    Ok(token)
}

async fn generate_session_token(
    http_base: &Url,
    app_token: &str,
    body: serde_json::Value,
) -> Result<String> {
    let url = http_base
        .join("/jet/webapp/session-token")
        .context("failed to build session-token URL")?;
    let token = Client::new()
        .post(url)
        .bearer_auth(app_token)
        .json(&body)
        .send()
        .await
        .context("failed to request Gateway session token")?
        .error_for_status()
        .context("Gateway session-token request failed")?
        .text()
        .await
        .context("failed to read Gateway session token")?;

    Ok(token)
}

fn gateway_winrm_transport(https: bool) -> (&'static str, &'static str) {
    if https {
        ("tls", "winrm-https-pwsh")
    } else {
        ("tcp", "winrm-http-pwsh")
    }
}

fn default_kdc_address(server: &str, domain: &str) -> String {
    if !domain.is_empty() && !server.contains('.') {
        format!("tcp://{server}.{domain}:88")
    } else {
        format!("tcp://{server}:88")
    }
}

fn kerberos_realm(username: &str, domain: &str) -> Result<String> {
    let domain = domain.trim();
    if !domain.is_empty() {
        return Ok(domain.to_string());
    }

    let username = ClientUserName::parse(username)?;
    Ok(username.domain_name().unwrap_or_default().to_string())
}

/// Split `scheme://rest` into a lowercased scheme and the remainder. URL schemes are
/// case-insensitive (RFC 3986), so this must not be matched case-sensitively — otherwise
/// `HTTPS://` would be treated as scheme-less and silently downgraded to a plaintext hop.
/// Returns `None` for scheme-less input (e.g. `host:7171`).
fn split_scheme(url: &str) -> Option<(String, &str)> {
    url.split_once("://")
        .map(|(scheme, rest)| (scheme.to_ascii_lowercase(), rest))
}

fn to_http_base_url(raw_url: &str) -> Result<Url> {
    let trimmed = raw_url.trim().trim_end_matches('/');
    let normalized = match split_scheme(trimmed) {
        Some((scheme, rest)) => match scheme.as_str() {
            "ws" => format!("http://{rest}"),
            "wss" => format!("https://{rest}"),
            "http" | "https" => format!("{scheme}://{rest}"),
            other => anyhow::bail!("unsupported Gateway URL scheme: {other}"),
        },
        None => format!("http://{trimmed}"),
    };

    normalized.parse().context("invalid Gateway HTTP base URL")
}

fn to_ws_base_url(raw_url: &str) -> Result<Url> {
    let trimmed = raw_url.trim().trim_end_matches('/');
    let normalized = match split_scheme(trimmed) {
        Some((scheme, rest)) => match scheme.as_str() {
            "http" => format!("ws://{rest}"),
            "https" => format!("wss://{rest}"),
            "ws" | "wss" => format!("{scheme}://{rest}"),
            other => anyhow::bail!("unsupported Gateway URL scheme: {other}"),
        },
        None => format!("ws://{trimmed}"),
    };

    normalized
        .parse()
        .context("invalid Gateway WebSocket base URL")
}

fn serialize_http_request(request: &HttpRequest) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let method = match request.method {
        Method::Get => "GET",
        Method::Post => "POST",
        Method::Put => "PUT",
        Method::Delete => "DELETE",
    };
    let url = Url::parse(&request.url).context("failed to parse request URL")?;
    let path = url.query().map_or_else(
        || url.path().to_string(),
        |query| format!("{}?{query}", url.path()),
    );

    buffer.extend_from_slice(format!("{method} {path} HTTP/1.1\r\n").as_bytes());

    if let Some(host) = url.host_str() {
        let host = url
            .port()
            .map_or_else(|| host.to_string(), |port| format!("{host}:{port}"));
        buffer.extend_from_slice(format!("Host: {host}\r\n").as_bytes());
    }

    for (name, value) in &request.headers {
        if name.eq_ignore_ascii_case("host") || name.eq_ignore_ascii_case("content-length") {
            continue;
        }
        buffer.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }

    if let Some(cookie) = &request.cookie {
        buffer.extend_from_slice(format!("Cookie: {cookie}\r\n").as_bytes());
    }

    if let Some(body) = &request.body {
        match body {
            HttpBody::Text(text) | HttpBody::Xml(text) => {
                buffer.extend_from_slice(format!("Content-Length: {}\r\n", text.len()).as_bytes());
                buffer.extend_from_slice(b"\r\n");
                buffer.extend_from_slice(text.as_bytes());
                return Ok(buffer);
            }
            HttpBody::Encrypted(bytes) => {
                buffer.extend_from_slice(format!("Content-Length: {}\r\n", bytes.len()).as_bytes());
                buffer.extend_from_slice(b"\r\n");
                buffer.extend_from_slice(bytes);
                return Ok(buffer);
            }
            HttpBody::None => {}
        }
    }

    if matches!(request.method, Method::Post | Method::Put)
        && matches!(&request.body, None | Some(HttpBody::None))
    {
        buffer.extend_from_slice(b"Content-Length: 0\r\n");
    }

    buffer.extend_from_slice(b"\r\n");
    Ok(buffer)
}

fn header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

struct HttpResponseDecoder {
    max_size: usize,
    buffer: Vec<u8>,
}

impl HttpResponseDecoder {
    fn new(max_size: usize) -> Self {
        Self {
            max_size,
            buffer: Vec::new(),
        }
    }

    fn feed(&mut self, bytes: &[u8]) -> Result<Option<HttpResponse>> {
        if self.buffer.len() + bytes.len() > self.max_size {
            return Err(anyhow::anyhow!("Gateway HTTP response too large"));
        }
        self.buffer.extend_from_slice(bytes);

        let Some(header_end) = header_end(&self.buffer) else {
            return Ok(None);
        };

        let header_bytes = &self.buffer[..header_end];
        let header_text = std::str::from_utf8(header_bytes)
            .context("Gateway HTTP response headers were not UTF-8")?;
        let mut lines = header_text.lines();
        let status_line = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("Gateway HTTP response missing status line"))?;
        let status_code = status_line
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| anyhow::anyhow!("Gateway HTTP response missing status code"))?
            .parse::<u16>()
            .context("Gateway HTTP response status code was invalid")?;

        let mut headers = Vec::new();
        let mut content_length = None;
        let mut content_type = None;
        let mut transfer_encoding = None;
        for line in lines {
            if let Some((name, value)) = line.split_once(':') {
                let name = name.trim().to_string();
                let value = value.trim().to_string();
                if name.eq_ignore_ascii_case("content-length") {
                    // Duplicate framing headers are ambiguous (a desync/smuggling vector)
                    // on the reused WebSocket; refuse rather than last-wins.
                    if content_length.is_some() {
                        return Err(anyhow::anyhow!(
                            "Gateway HTTP response carried multiple Content-Length headers"
                        ));
                    }
                    content_length = Some(
                        value
                            .parse::<usize>()
                            .context("Gateway HTTP response Content-Length was invalid")?,
                    );
                }
                if name.eq_ignore_ascii_case("content-type") {
                    content_type = Some(value.clone());
                }
                if name.eq_ignore_ascii_case("transfer-encoding") {
                    if transfer_encoding.is_some() {
                        return Err(anyhow::anyhow!(
                            "Gateway HTTP response carried multiple Transfer-Encoding headers"
                        ));
                    }
                    transfer_encoding = Some(value.clone());
                }
                headers.push((name, value));
            }
        }

        // A response carrying both Content-Length and Transfer-Encoding is ambiguous
        // (RFC 7230 §3.3.3) and a classic desync/smuggling vector. Since the same
        // WebSocket is reused for subsequent requests, refuse rather than guess which
        // framing to honor.
        if content_length.is_some() && transfer_encoding.is_some() {
            return Err(anyhow::anyhow!(
                "Gateway HTTP response carried both Content-Length and Transfer-Encoding"
            ));
        }

        let body_start = header_end + 4;
        let body_len = self.buffer.len() - body_start;
        let body_bytes: std::borrow::Cow<'_, [u8]> = if let Some(expected_len) = content_length {
            if body_len < expected_len {
                return Ok(None);
            }
            if body_len > expected_len {
                return Err(anyhow::anyhow!(
                    "Gateway HTTP response body exceeded Content-Length"
                ));
            }
            std::borrow::Cow::Borrowed(&self.buffer[body_start..body_start + expected_len])
        } else if let Some(transfer_encoding) = transfer_encoding.as_deref() {
            if !transfer_encoding_is_chunked(transfer_encoding) {
                return Err(anyhow::anyhow!(
                    "Gateway HTTP response used unsupported Transfer-Encoding: {transfer_encoding}"
                ));
            }
            let Some(body) = decode_chunked_body(&self.buffer[body_start..])? else {
                return Ok(None);
            };
            std::borrow::Cow::Owned(body)
        } else if response_status_forbids_body(status_code) {
            if body_len > 0 {
                return Err(anyhow::anyhow!(
                    "Gateway HTTP response included a body for status {status_code}"
                ));
            }
            std::borrow::Cow::Borrowed(&[])
        } else {
            return Err(anyhow::anyhow!(
                "Gateway HTTP response missing Content-Length; close-delimited bodies are unsupported"
            ));
        };

        let body = classify_body(&body_bytes, content_type.as_deref())?;

        Ok(Some(HttpResponse {
            status_code,
            headers,
            body,
        }))
    }
}

fn transfer_encoding_is_chunked(value: &str) -> bool {
    let mut codings = value
        .split(',')
        .map(str::trim)
        .filter(|coding| !coding.is_empty());
    let Some(coding) = codings.next() else {
        return false;
    };
    coding.eq_ignore_ascii_case("chunked") && codings.next().is_none()
}

fn find_crlf(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|window| window == b"\r\n")
}

fn decode_chunked_body(bytes: &[u8]) -> Result<Option<Vec<u8>>> {
    let mut pos = 0;
    let mut decoded = Vec::new();

    loop {
        let Some(line_end) = find_crlf(&bytes[pos..]) else {
            return Ok(None);
        };
        let line = &bytes[pos..pos + line_end];
        let line = std::str::from_utf8(line).context("Gateway HTTP chunk size was not UTF-8")?;
        let size_text = line.split(';').next().unwrap_or_default().trim();
        if size_text.is_empty() {
            return Err(anyhow::anyhow!("Gateway HTTP chunk size was missing"));
        }
        let size =
            usize::from_str_radix(size_text, 16).context("Gateway HTTP chunk size was invalid")?;
        pos += line_end + 2;

        if size == 0 {
            if bytes.len() < pos + 2 {
                return Ok(None);
            }
            let end = if bytes[pos..].starts_with(b"\r\n") {
                pos + 2
            } else if let Some(trailer_end) = header_end(&bytes[pos..]) {
                pos + trailer_end + 4
            } else {
                return Ok(None);
            };
            if bytes.len() > end {
                return Err(anyhow::anyhow!(
                    "Gateway HTTP response had trailing data after chunked body"
                ));
            }
            return Ok(Some(decoded));
        }

        let data_end = pos
            .checked_add(size)
            .ok_or_else(|| anyhow::anyhow!("Gateway HTTP chunk size overflowed"))?;
        let chunk_end = data_end
            .checked_add(2)
            .ok_or_else(|| anyhow::anyhow!("Gateway HTTP chunk size overflowed"))?;
        if bytes.len() < chunk_end {
            return Ok(None);
        }
        if bytes.get(data_end..chunk_end) != Some(&b"\r\n"[..]) {
            return Err(anyhow::anyhow!(
                "Gateway HTTP chunk missing CRLF terminator"
            ));
        }
        decoded.extend_from_slice(&bytes[pos..data_end]);
        pos = chunk_end;
    }
}

fn response_status_forbids_body(status_code: u16) -> bool {
    (100..200).contains(&status_code) || matches!(status_code, 204 | 304)
}

fn classify_body(bytes: &[u8], content_type: Option<&str>) -> Result<HttpBody> {
    let Some(content_type) = content_type else {
        return std::str::from_utf8(bytes).map_or_else(
            |_| Ok(HttpBody::Encrypted(bytes.to_vec())),
            |text| Ok(HttpBody::Text(text.to_string())),
        );
    };

    let lower = content_type.to_ascii_lowercase();
    if lower.contains("multipart/encrypted") {
        Ok(HttpBody::Encrypted(bytes.to_vec()))
    } else if lower.contains("application/soap+xml") {
        let text = std::str::from_utf8(bytes).context("SOAP response was not UTF-8")?;
        Ok(HttpBody::Xml(text.to_string()))
    } else {
        std::str::from_utf8(bytes).map_or_else(
            |_| Ok(HttpBody::Encrypted(bytes.to_vec())),
            |text| Ok(HttpBody::Text(text.to_string())),
        )
    }
}

pub fn redact_gateway_url(url: &Url) -> String {
    let mut redacted = url.clone();
    if redacted.path_segments().is_some_and(|mut segments| {
        segments.any(|segment| segment.eq_ignore_ascii_case("KdcProxy"))
    }) {
        redacted.set_path("/jet/KdcProxy/<redacted>");
    }
    if redacted.query().is_some() {
        redacted.set_query(Some("<redacted>"));
    }
    redacted.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kerberos_realm_prefers_explicit_domain() {
        let realm = kerberos_realm("Administrator@ad.it-help.ninja", "EXAMPLE.COM").expect("realm");

        assert_eq!(realm, "EXAMPLE.COM");
    }

    #[test]
    fn kerberos_realm_uses_upn_suffix_when_domain_is_empty() {
        let realm = kerberos_realm("Administrator@ad.it-help.ninja", "").expect("realm");

        assert_eq!(realm, "ad.it-help.ninja");
    }

    #[test]
    fn kerberos_realm_allows_plain_username_without_domain() {
        let realm = kerberos_realm("Administrator", "").expect("realm");

        assert_eq!(realm, "");
    }

    #[test]
    fn gateway_winrm_transport_uses_tls_for_https() {
        assert_eq!(gateway_winrm_transport(true), ("tls", "winrm-https-pwsh"));
    }

    #[test]
    fn gateway_winrm_transport_uses_tcp_for_http() {
        assert_eq!(gateway_winrm_transport(false), ("tcp", "winrm-http-pwsh"));
    }

    #[test]
    fn decoder_rejects_body_without_framing() {
        let mut decoder = HttpResponseDecoder::new(1024);

        let err = decoder
            .feed(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\npartial")
            .expect_err("missing framing must fail");

        assert!(err.to_string().contains("missing Content-Length"));
    }

    #[test]
    fn decoder_waits_for_complete_chunked_body() {
        let mut decoder = HttpResponseDecoder::new(1024);

        assert!(
            decoder
                .feed(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: text/plain\r\n\r\n5\r\nhe")
                .expect("partial chunk")
                .is_none()
        );
        let response = decoder
            .feed(b"llo\r\n0\r\n\r\n")
            .expect("complete chunk")
            .expect("response");

        assert_eq!(response.status_code, 200);
        assert!(matches!(response.body, HttpBody::Text(ref text) if text == "hello"));
    }

    #[test]
    fn decoder_accepts_no_body_status_without_content_length() {
        let mut decoder = HttpResponseDecoder::new(1024);

        let response = decoder
            .feed(b"HTTP/1.1 204 No Content\r\n\r\n")
            .expect("no-body status")
            .expect("response");

        assert_eq!(response.status_code, 204);
        assert!(matches!(response.body, HttpBody::Text(ref text) if text.is_empty()));
    }
}
