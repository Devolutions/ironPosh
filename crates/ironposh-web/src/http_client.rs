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
use tracing::info;

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
        Self {
            gateway_url,
            connection_map: Rc::new(Mutex::new(std::collections::HashMap::new())),
            token,
        }
    }
}

impl HttpClient for GatewayHttpViaWSClient {
    async fn send_request(&self, try_send: TrySend) -> anyhow::Result<HttpResponseTargeted> {
        match try_send {
            TrySend::JustSend { request, conn_id } => {
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
                                todo!("Kerbero not supported in WASM yet");
                            }
                        };

                    // 2) Process initialized context → either Continue (send another token) or Done
                    match auth_sequence.process_sec_ctx_init(init)? {
                        SecContextInited::Continue { request, sequence } => {
                            info!("continuing authentication sequence");
                            let HttpRequestAction {
                                connection_id,
                                request,
                            } = request;
                            let response = self.send_http_request(request, &connection_id).await?;
                            auth_response = Some(response);
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
                            let resp = self.send_http_request(request, &connection_id).await?;

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

impl GatewayHttpViaWSClient {
    async fn send_http_request(
        &self,
        req: HttpRequest,
        con_id: &ConnectionId,
    ) -> Result<HttpResponse> {
        let mut map = self.connection_map.lock().await;
        let stream = if let Some(stream) = map.get_mut(con_id) {
            stream
        } else {
            // Create new WebSocket connection
            let url = self.gateway_url.clone();

            let stream = WebsocketStream::new(url)?;
            map.insert(*con_id, stream);
            map.get_mut(con_id).unwrap()
        };

        stream.send_http(req).await
    }
}

pub struct WebsocketStream {
    ws: WebSocket,
}

unsafe impl Send for WebsocketStream {}

impl WebsocketStream {
    async fn send_http(&mut self, request: HttpRequest) -> Result<HttpResponse> {
        // The Gateway relays HTTP requests over WebSocket to TCP, so we can assume that we are working on TCP directly
        // Serialize the HTTP request to HTTP/1.1 wire format
        let http_request_bytes = serialize_http_request(&request)
            .map_err(|e| WasmError::IOError(format!("Failed to serialize HTTP request: {}", e)))?;

        info!(
            method = ?request.method,
            url = %request.url,
            bytes_length = http_request_bytes.len(),
            "sending HTTP request over WebSocket"
        );

        // Send the serialized HTTP request over WebSocket
        self.ws
            .send(gloo_net::websocket::Message::Bytes(http_request_bytes))
            .await
            .map_err(|e| {
                WasmError::IOError(format!(
                    "Failed to send HTTP request over WebSocket: {:?}",
                    e
                ))
            })?;

        // Wait for the response from the gateway
        let message = self.ws.next().await.ok_or(WasmError::IOError(
            "WebSocket closed before receiving response".to_string(),
        ))??;

        // Extract bytes from the WebSocket message
        let response_bytes = match message {
            gloo_net::websocket::Message::Bytes(bytes) => bytes,
            gloo_net::websocket::Message::Text(text) => {
                return Err(WasmError::IOError(format!(
                    "Expected binary WebSocket message, got text: {}",
                    text
                ))
                .into());
            }
        };

        info!(
            response_bytes_length = response_bytes.len(),
            "received HTTP response over WebSocket"
        );

        // Deserialize the HTTP response from HTTP/1.1 wire format
        let response = deserialize_http_response(&response_bytes).map_err(|e| {
            WasmError::IOError(format!("Failed to deserialize HTTP response: {}", e))
        })?;

        info!(
            status_code = response.status_code,
            "HTTP response deserialized successfully"
        );

        Ok(response)
    }
}

impl WebsocketStream {
    pub fn new(url: url::Url) -> Result<Self, WasmError> {
        let ws = WebSocket::open(url.as_str())
            .map_err(|e| WasmError::IOError(format!("Failed to open WebSocket: {:?}", e)))?;

        Ok(Self { ws })
    }
}

/// It's wasm, it will never be sent across threads, we are safe
unsafe impl Sync for GatewayHttpViaWSClient {}
/// It's wasm, it will never be sent across threads, we are safe
unsafe impl Send for GatewayHttpViaWSClient {}
