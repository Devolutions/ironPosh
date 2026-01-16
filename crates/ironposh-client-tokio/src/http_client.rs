use anyhow::Context;
use ironposh_async::HttpClient;
use ironposh_client_core::connector::{
    authenticator::SecContextMaybeInit,
    conntion_pool::SecContextInited,
    conntion_pool::TrySend,
    http::HttpRequestAction,
    http::{HttpBody, HttpRequest, HttpResponse, HttpResponseTargeted, Method},
};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, info, instrument};

pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl ReqwestHttpClient {
    pub fn new() -> Self {
        info!(
            connect_timeout_secs = 30,
            read_timeout_secs = 60,
            "initializing ReqwestHttpClient with native-tls"
        );
        Self {
            client: reqwest::Client::builder()
                .use_native_tls()
                .pool_max_idle_per_host(10)
                .connect_timeout(Duration::from_secs(30))
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to build reqwest client"),
        }
    }
}

impl ReqwestHttpClient {
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

impl HttpClient for ReqwestHttpClient {
    #[instrument(name = "http_request", level = "debug", skip(self, try_send))]
    async fn send_request(&self, try_send: TrySend) -> anyhow::Result<HttpResponseTargeted> {
        match try_send {
            // === Simple path: already have an idle, encrypted channel ===
            TrySend::JustSend { request, conn_id } => {
                info!(conn_id = conn_id.inner(), "sending on existing connection");
                let resp = Self::send_with_client(self.client.clone(), request).await?;
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
                            SecContextMaybeInit::RunGenerator { .. } => {
                                // For async client, we don't implement KDC communication yet
                                return Err(anyhow::anyhow!(
                                    "KDC generator not implemented in async client"
                                ));
                            }
                        };

                    // 2) Process initialized context → either Continue (send another token) or Done
                    match auth_sequence.process_sec_ctx_init(&init)? {
                        SecContextInited::Continue { request, sequence } => {
                            info!("continuing authentication sequence");
                            let HttpRequestAction {
                                connection_id: _,
                                request,
                            } = request;
                            let resp = Self::send_with_client(self.client.clone(), request).await?;
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
                            let resp = Self::send_with_client(self.client.clone(), request).await?;

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
