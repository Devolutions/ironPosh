use std::rc::Rc;

use anyhow::Result;
use futures::{lock::Mutex, SinkExt, StreamExt};
use gloo_net::websocket::futures::WebSocket;
use ironposh_async::HttpClient;
use ironposh_client_core::connector::{
    authenticator::SecContextMaybeInit,
    conntion_pool::{ConnectionId, SecContextInited, TrySend},
    http::{HttpRequest, HttpRequestAction, HttpResponse, HttpResponseTargeted},
};
use tracing::{debug, error, info, instrument};

use crate::{
    error::WasmError,
    http_convert::{deserialize_http_response, serialize_http_request},
};

// HTTP client implementation for WASM
pub(crate) struct GatewayHttpViaWSClient {
    gateway_url: url::Url,
    connection_map: Rc<Mutex<std::collections::HashMap<ConnectionId, WebsocketStream>>>,
    // We here assume that the token is short lived but can be reused for multiple connections in parallel
    token: String,
}

impl GatewayHttpViaWSClient {
    pub fn new(gateway_url: url::Url, token: String) -> Self {
        info!(
            gateway_url = %gateway_url,
            "creating new gateway HTTP via WebSocket client"
        );
        Self {
            gateway_url,
            connection_map: Rc::new(Mutex::new(std::collections::HashMap::new())),
            token,
        }
    }
}

/// It's wasm, it will never be sent across threads, we are safe
unsafe impl Send for GatewayHttpViaWSClient {}
unsafe impl Sync for GatewayHttpViaWSClient {}

impl HttpClient for GatewayHttpViaWSClient {
    #[instrument(skip(self, try_send), fields(conn_id = ?try_send.get_connection_id()))]
    async fn send_request(&self, try_send: TrySend) -> anyhow::Result<HttpResponseTargeted> {
        debug!("preparing to send HTTP request");
        match try_send {
            TrySend::JustSend { request, conn_id } => {
                debug!(
                    ?conn_id,
                    method = ?request.method,
                    url = %request.url,
                    "sending HTTP request without authentication"
                );
                let response = self.send_http_request(request, &conn_id).await;
                response.map(|resp| HttpResponseTargeted::new(resp, conn_id, None))
            }
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
                                error!("Kerberos authentication not supported in WASM");
                                todo!("Kerbero not supported in WASM yet");
                            }
                        };

                    // 2) Process initialized context → either Continue (send another token) or Done
                    match auth_sequence.process_sec_ctx_init(init)? {
                        SecContextInited::Continue { request, sequence } => {
                            let HttpRequestAction {
                                connection_id,
                                request,
                            } = request;
                            info!(?connection_id, "continuing authentication sequence");
                            let response = self.send_http_request(request, &connection_id).await?;
                            debug!(
                                status_code = response.status_code,
                                "received authentication response"
                            );
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
                            info!(
                                ?connection_id,
                                "authentication sequence complete, sending final encrypted request"
                            );

                            // Send the final (sealed) request
                            let resp = self.send_http_request(request, &connection_id).await?;

                            // Return targeted response WITH the provider attached
                            info!(
                                ?connection_id,
                                status_code = resp.status_code,
                                "authentication sequence successful"
                            );
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

impl GatewayHttpViaWSClient {
    #[instrument(skip(self, req), fields(method = ?req.method, url = %req.url))]
    async fn send_http_request(
        &self,
        req: HttpRequest,
        con_id: &ConnectionId,
    ) -> Result<HttpResponse> {
        info!(?con_id, "sending HTTP request via WebSocket");

        // -- Acquire or create the per-connection stream (map lock held briefly)
        let stream_rc = {
            let mut map = self.connection_map.lock().await;
            if let Some(s) = map.get(con_id) {
                debug!(?con_id, "reusing existing WebSocket connection");
                s.clone()
            } else {
                info!(?con_id, gateway_url = %self.gateway_url, "creating new WebSocket connection");
                let stream = WebsocketStream::new(self.gateway_url.clone())?;
                map.insert(*con_id, stream.clone());
                stream
            }
        }; // Drop the map lock here

        // -- Now it's safe to await without blocking other connections
        stream_rc.send_http(req).await
    }
}

#[derive(Debug, Clone)]
pub struct WebsocketStream {
    ws: Rc<Mutex<WebSocket>>,
}

unsafe impl Send for WebsocketStream {}

impl WebsocketStream {
    #[instrument(skip(self, request), fields(method = ?request.method, url = %request.url))]
    async fn send_http(&self, request: HttpRequest) -> Result<HttpResponse> {
        // The Gateway relays HTTP requests over WebSocket to TCP, so we can assume that we are working on TCP directly
        // Serialize the HTTP request to HTTP/1.1 wire format
        info!(?request.method, url = %request.url, "serializing HTTP request");
        let http_request_bytes = serialize_http_request(&request).map_err(|e| {
            error!(?e, "failed to serialize HTTP request");
            WasmError::IOError(format!("Failed to serialize HTTP request: {}", e))
        })?;

        debug!(
            method = ?request.method,
            url = %request.url,
            bytes_length = http_request_bytes.len(),
            "sending HTTP request over WebSocket"
        );

        let mut ws = self.ws.lock().await;

        // Send the serialized HTTP request over WebSocket
        ws.send(gloo_net::websocket::Message::Bytes(http_request_bytes))
            .await
            .map_err(|e| {
                error!(?e, "failed to send HTTP request over WebSocket");
                WasmError::IOError(format!(
                    "Failed to send HTTP request over WebSocket: {:?}",
                    e
                ))
            })?;

        // Wait for the response from the gateway
        let message = ws.next().await.ok_or_else(|| {
            error!("WebSocket closed before receiving response");
            WasmError::IOError("WebSocket closed before receiving response".to_string())
        })??;

        // Extract bytes from the WebSocket message
        let response_bytes = match message {
            gloo_net::websocket::Message::Bytes(bytes) => bytes,
            gloo_net::websocket::Message::Text(text) => {
                error!(
                    text_length = text.len(),
                    "expected binary WebSocket message, got text"
                );
                return Err(WasmError::IOError(format!(
                    "Expected binary WebSocket message, got text: {}",
                    text
                ))
                .into());
            }
        };

        debug!(
            response_bytes_length = response_bytes.len(),
            "received HTTP response over WebSocket"
        );

        // Deserialize the HTTP response from HTTP/1.1 wire format
        let response = deserialize_http_response(&response_bytes).map_err(|e| {
            error!(?e, "failed to deserialize HTTP response");
            WasmError::IOError(format!("Failed to deserialize HTTP response: {}", e))
        })?;

        debug!(
            status_code = response.status_code,
            "HTTP response deserialized successfully"
        );

        Ok(response)
    }
}

impl WebsocketStream {
    pub fn new(url: url::Url) -> Result<Self, WasmError> {
        info!(
            url = %url,
            "opening WebSocket connection"
        );
        let ws = WebSocket::open(url.as_str()).map_err(|e| {
            error!(?e, url = %url, "failed to open WebSocket");
            WasmError::IOError(format!("Failed to open WebSocket: {:?}", e))
        })?;

        info!(url = %url, "WebSocket connection opened successfully");
        Ok(Self {
            ws: Rc::new(Mutex::new(ws)),
        })
    }
}
