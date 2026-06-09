use anyhow::Context;
use ironposh_async::HttpClient;
use ironposh_client_core::connector::{
    auth_sequence::SspiAuthSequence,
    authenticator::SecContextMaybeInit,
    config::TlsOptions,
    connection_pool::TrySend,
    connection_pool::{ConnectionId, SecContextInited},
    http::HttpRequestAction,
    http::{HttpBody, HttpRequest, HttpResponse, HttpResponseTargeted, Method},
    NetworkProtocol, NetworkRequest,
};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info, instrument};

/// Build a reqwest client honoring the given [`TlsOptions`] (native-tls backend).
pub fn build_reqwest_client(tls: &TlsOptions) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .use_native_tls()
        // IMPORTANT: keep each logical `ConnectionId` on its own reqwest client to
        // reduce the chance of SSPI contexts being mixed across TCP connections.
        .pool_max_idle_per_host(1)
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .danger_accept_invalid_certs(tls.accept_invalid_certs)
        .danger_accept_invalid_hostnames(tls.accept_invalid_hostnames);

    if let Some(pem) = &tls.extra_ca_pem {
        let cert = reqwest::Certificate::from_pem(pem).context("invalid extra CA PEM")?;
        builder = builder.add_root_certificate(cert);
    }

    builder.build().context("failed to build reqwest client")
}

pub struct ReqwestHttpClient {
    tls: TlsOptions,
    clients_by_conn: Mutex<HashMap<u32, reqwest::Client>>,
}

impl ReqwestHttpClient {
    pub fn new() -> Self {
        Self::with_tls_options(TlsOptions::default())
    }

    pub fn with_tls_options(tls: TlsOptions) -> Self {
        info!(
            connect_timeout_secs = 30,
            read_timeout_secs = 60,
            accept_invalid_certs = tls.accept_invalid_certs,
            accept_invalid_hostnames = tls.accept_invalid_hostnames,
            has_extra_ca_pem = tls.extra_ca_pem.is_some(),
            "initializing ReqwestHttpClient with native-tls"
        );
        Self {
            tls,
            clients_by_conn: Mutex::new(HashMap::new()),
        }
    }

    fn client_for_conn(&self, conn_id: ConnectionId) -> Client {
        let mut clients = self
            .clients_by_conn
            .lock()
            .expect("ReqwestHttpClient.clients_by_conn mutex poisoned");

        clients
            .entry(conn_id.inner())
            .or_insert_with(|| {
                build_reqwest_client(&self.tls).expect("Failed to build reqwest client")
            })
            .clone()
    }
}

impl ReqwestHttpClient {
    #[instrument(
        name = "kdc_request",
        level = "debug",
        skip(packet),
        fields(protocol = ?packet.protocol, url = tracing::field::Empty, data_len = packet.data.len())
    )]
    pub(crate) async fn send_kdc_network_request(
        packet: NetworkRequest,
    ) -> anyhow::Result<Vec<u8>> {
        let redacted_url = redact_network_url(&packet.url);
        tracing::Span::current().record("url", redacted_url.as_str());
        info!(
            protocol = ?packet.protocol,
            url = %redacted_url,
            data_len = packet.data.len(),
            "sending KDC network request"
        );

        match packet.protocol {
            NetworkProtocol::Tcp => Self::send_kdc_tcp_packet(packet).await,
            NetworkProtocol::Http | NetworkProtocol::Https => {
                Self::send_kdc_http_packet(packet).await
            }
            NetworkProtocol::Udp => todo!("UDP protocol not implemented for Kerberos"),
        }
    }

    #[instrument(
        name = "kdc_tcp",
        level = "debug",
        skip(packet),
        fields(host = packet.url.host_str(), port = packet.url.port())
    )]
    async fn send_kdc_tcp_packet(packet: NetworkRequest) -> anyhow::Result<Vec<u8>> {
        let host = packet
            .url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Missing host in KDC URL"))?;
        let port = packet
            .url
            .port()
            .ok_or_else(|| anyhow::anyhow!("Missing port in KDC URL"))?;

        info!(host = %host, port, "opening TCP connection to KDC");
        let mut stream = tokio::net::TcpStream::connect((host, port))
            .await
            .context("failed to establish TCP connection to KDC")?;

        stream
            .write_all(&packet.data)
            .await
            .context("failed to write packet data to KDC")?;
        stream
            .flush()
            .await
            .context("failed to flush TCP stream to KDC")?;

        let response_len = stream
            .read_u32()
            .await
            .context("failed to read response length from KDC")?;

        let mut response_data = vec![0_u8; response_len as usize + 4];
        response_data[..4].copy_from_slice(&response_len.to_be_bytes());

        stream
            .read_exact(&mut response_data[4..])
            .await
            .context("failed to read response data from KDC")?;

        info!(
            response_len = response_data.len(),
            "received TCP response from KDC"
        );

        Ok(response_data)
    }

    #[instrument(
        name = "kdc_http",
        level = "debug",
        skip(packet),
        fields(protocol = ?packet.protocol, url = tracing::field::Empty)
    )]
    async fn send_kdc_http_packet(packet: NetworkRequest) -> anyhow::Result<Vec<u8>> {
        tracing::Span::current().record("url", redact_network_url(&packet.url).as_str());
        let response = build_reqwest_client(&TlsOptions::default())?
            .post(packet.url.clone())
            .header("keep-alive", "true")
            .body(packet.data)
            .send()
            .await
            .context("failed to send KDC HTTP request")?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "KDC HTTP request failed with status {status}"
            ));
        }

        let bytes = response
            .bytes()
            .await
            .context("failed to read KDC HTTP response body")?;

        info!(
            response_len = bytes.len(),
            "received HTTP response from KDC"
        );

        Ok(bytes.to_vec())
    }

    async fn send_with_client(
        client: Client,
        request: HttpRequest,
    ) -> anyhow::Result<HttpResponse> {
        tracing::info!(
            method = ?request.method,
            url = %request.url,
            headers_count = request.headers.len(),
            body_length = request.body.as_ref().map_or(0, HttpBody::len),
            "Starting HTTP request"
        );

        let mut req_builder = match request.method {
            Method::Get => client.get(&request.url),
            Method::Post => client.post(&request.url),
            Method::Put => client.put(&request.url),
            Method::Delete => client.delete(&request.url),
        };

        // Add headers
        for (key, value) in &request.headers {
            req_builder = req_builder.header(key, value);
        }

        // Add body if present
        if let Some(body) = &request.body {
            match body {
                HttpBody::Encrypted(bytes) => {
                    debug!(body_length = bytes.len(), "sending encrypted body as bytes");
                    req_builder = req_builder.body(bytes.clone());
                }
                _ => {
                    req_builder = req_builder.body(body.as_str()?.to_string());
                }
            }
        }

        tracing::info!("Sending HTTP request to server");
        let response = req_builder
            .send()
            .await
            .context("Failed to send HTTP request")?;

        let status_code = response.status().as_u16();
        tracing::info!(status_code, "Received HTTP response");

        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Determine body type from Content-Type header
        let content_type = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.to_lowercase())
            .unwrap_or_default();

        tracing::info!("Reading response body");
        let body = if content_type.contains("multipart/encrypted") {
            debug!("reading encrypted response as binary data");
            let bytes = response
                .bytes()
                .await
                .context("Failed to read binary response body")?;
            HttpBody::Encrypted(bytes.to_vec())
        } else if content_type.contains("application/soap+xml") {
            debug!("reading XML response as text");
            let text = response
                .text()
                .await
                .context("Failed to read XML response body")?;
            HttpBody::Xml(text)
        } else {
            debug!("reading response as text");
            let text = response
                .text()
                .await
                .context("Failed to read text response body")?;
            HttpBody::Text(text)
        };

        tracing::info!(
            body_length = body.len(),
            "HTTP request completed successfully"
        );

        Ok(HttpResponse {
            status_code,
            headers,
            body,
        })
    }
}

fn redact_network_url(url: &url::Url) -> String {
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

impl HttpClient for ReqwestHttpClient {
    #[instrument(name = "http_request", level = "debug", skip(self, try_send))]
    async fn send_request(&self, try_send: TrySend) -> anyhow::Result<HttpResponseTargeted> {
        match try_send {
            // === Simple path: already have an idle, encrypted channel ===
            TrySend::JustSend { request, conn_id } => {
                info!(conn_id = conn_id.inner(), "sending on existing connection");
                let client = self.client_for_conn(conn_id);
                let resp = Self::send_with_client(client, request).await?;
                // No provider attached on steady-state sends
                Ok(HttpResponseTargeted::new(resp, conn_id, None))
            }

            // === Auth path: drive the per-connection FSM, then send first sealed request ===
            TrySend::AuthNeeded { mut auth_sequence } => {
                info!("starting authentication sequence");
                let mut auth_response: Option<HttpResponse> = None;

                loop {
                    // 1) Initialize security context
                    let (seq, mut holder) = auth_sequence.prepare();
                    let init =
                        match seq.try_init_sec_context(auth_response.as_ref(), &mut holder)? {
                            SecContextMaybeInit::Initialized(sec) => sec,
                            SecContextMaybeInit::RunGenerator {
                                mut packet,
                                mut generator_holder,
                            } => {
                                info!("running generator for KDC communication");
                                loop {
                                    let kdc_resp =
                                        Self::send_kdc_network_request(packet).await.context(
                                            "failed to send packet to KDC during authentication",
                                        )?;
                                    match SspiAuthSequence::resume(generator_holder, kdc_resp)? {
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

                    // 2) Process initialized context → either Continue (send another token) or Done
                    match auth_sequence.process_sec_ctx_init(&init)? {
                        SecContextInited::Continue { request, sequence } => {
                            info!("continuing authentication sequence");
                            let HttpRequestAction {
                                connection_id,
                                request,
                            } = request;
                            let client = self.client_for_conn(connection_id);
                            let resp = Self::send_with_client(client, request).await?;
                            auth_response = Some(resp);
                            auth_sequence = sequence;
                        }

                        SecContextInited::SendRequest {
                            request,
                            authenticated_http_channel_cert,
                        } => {
                            info!(
                                "authentication sequence complete, sending final encrypted request"
                            );
                            let HttpRequestAction {
                                connection_id,
                                request,
                            } = request;

                            // Send the final (sealed) request
                            let client = self.client_for_conn(connection_id);
                            let resp = Self::send_with_client(client, request).await?;

                            // Return targeted response WITH the provider attached
                            info!("authentication sequence successful");
                            return Ok(HttpResponseTargeted::new(
                                resp,
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

#[cfg(test)]
mod tls_tests {
    use super::*;

    #[test]
    fn builds_with_default_options() {
        build_reqwest_client(&TlsOptions::default()).expect("default TLS options must build");
    }

    #[test]
    fn builds_with_insecure_options() {
        let tls = TlsOptions {
            accept_invalid_certs: true,
            ..TlsOptions::default()
        };
        build_reqwest_client(&tls).expect("insecure TLS options must build");
    }

    #[test]
    fn rejects_garbage_ca_pem() {
        let tls = TlsOptions {
            extra_ca_pem: Some(b"not a pem".to_vec()),
            ..TlsOptions::default()
        };
        assert!(build_reqwest_client(&tls).is_err());
    }
}
