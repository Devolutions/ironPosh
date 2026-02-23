use std::rc::Rc;

use anyhow::Result;
use futures::{lock::Mutex, SinkExt};
use gloo_net::http::Request;
use gloo_net::websocket::futures::WebSocket;
use ironposh_async::HttpClient;
use ironposh_client_core::connector::{
    auth_sequence::SspiAuthSequence,
    authenticator::SecContextMaybeInit,
    conntion_pool::{ConnectionId, SecContextInited, TrySend},
    http::{HttpRequest, HttpRequestAction, HttpResponse, HttpResponseTargeted},
    NetworkProtocol, NetworkRequest,
};
use js_sys::Uint8Array;
use tracing::{debug, error, info, instrument};

use crate::{
    error::WasmError,
    http_convert::serialize_http_request,
    ws_http_decoder::{next_ws, HttpResponseDecoder},
};

// HTTP client implementation for WASM
//
// Uses a SINGLE WebSocket for all ConnectionIds. This is critical because
// Devolutions Gateway enforces jti-based token replay detection - reusing the
// same ASSOCIATION token on a second WebSocket while the first is still active
// causes the second to be rejected. By sharing a single WebSocket, all WinRM
// operations are serialized through one tunnel.
pub(crate) struct GatewayHttpViaWSClient {
    gateway_url: url::Url,
    websocket: Rc<Mutex<Option<WebsocketStream>>>,
    #[expect(dead_code)]
    token: String,
}

impl GatewayHttpViaWSClient {
    pub fn new(gateway_url: url::Url, token: String) -> Self {
        info!(
            gateway_url = %gateway_url,
            "creating new gateway HTTP via WebSocket client (single-connection mode)"
        );
        Self {
            gateway_url,
            websocket: Rc::new(Mutex::new(None)),
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
                    // 1) Initialize security context (may require KDC generator round-trips).
                    let init = {
                        let (seq, mut holder) = auth_sequence.prepare();
                        let res = match seq
                            .try_init_sec_context(auth_response.as_ref(), &mut holder)?
                        {
                            SecContextMaybeInit::Initialized(sec) => sec,
                            SecContextMaybeInit::RunGenerator {
                                mut packet,
                                mut generator_holder,
                            } => {
                                info!("running generator for KDC communication (wasm)");
                                loop {
                                    let kdc_resp = Self::send_kdc_network_request(&packet).await?;
                                    match SspiAuthSequence::resume(generator_holder, kdc_resp)? {
                                        SecContextMaybeInit::Initialized(sec) => break sec,
                                        SecContextMaybeInit::RunGenerator {
                                            packet: p2,
                                            generator_holder: g2,
                                        } => {
                                            packet = p2;
                                            generator_holder = g2;
                                        }
                                    }
                                }
                            }
                        };
                        res
                    };

                    // 2) Process initialized context → either Continue (send another token) or Done
                    match auth_sequence.process_sec_ctx_init(&init)? {
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
        info!(?con_id, "sending HTTP request via single WebSocket");

        // Acquire or create the shared WebSocket (single connection for all ConnectionIds)
        let stream = {
            let mut ws = self.websocket.lock().await;
            if let Some(s) = ws.as_ref() {
                debug!(?con_id, "reusing existing single WebSocket connection");
                s.clone()
            } else {
                info!(?con_id, gateway_url = %self.gateway_url, "creating single WebSocket connection");
                let stream = WebsocketStream::new(&self.gateway_url)?;
                *ws = Some(stream.clone());
                stream
            }
        };

        stream.send_http(req).await
    }

    async fn send_kdc_network_request(packet: &NetworkRequest) -> Result<Vec<u8>, WasmError> {
        info!(
            protocol = ?packet.protocol,
            url = %packet.url,
            "sending KDC network request via gateway"
        );
        match packet.protocol {
            NetworkProtocol::Http | NetworkProtocol::Https => {
                let body = Uint8Array::from(packet.data.as_slice());

                let response = Request::post(packet.url.as_str())
                    .header("keep-alive", "true")
                    .body(body)
                    .map_err(|e| WasmError::IOError(format!("Failed to build KDC request: {e}")))?;

                let response = response
                    .send()
                    .await
                    .map_err(|e| WasmError::IOError(format!("Failed to send KDC request: {e}")))?;

                if !response.ok() {
                    return Err(WasmError::Generic(format!(
                        "KDC proxy responded with status {} {}",
                        response.status(),
                        response.status_text()
                    )));
                }

                let reply = response
                    .binary()
                    .await
                    .map_err(|e| WasmError::IOError(format!("Failed to read KDC response: {e}")))?;

                Ok(reply)
            }
            unsupported => Err(WasmError::Generic(format!(
                "Unsupported KDC network protocol: {unsupported:?}"
            ))),
        }
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
            WasmError::IOError(format!("Failed to serialize HTTP request: {e}"))
        })?;

        debug!(
            method = ?request.method,
            url = %request.url,
            bytes_length = http_request_bytes.len(),
            "sending HTTP request over WebSocket"
        );

        let mut ws = self.ws.lock().await;

        if !matches!(
            ws.state(),
            gloo_net::websocket::State::Connecting | gloo_net::websocket::State::Open
        ) {
            let state = ws.state();
            error!(?state, "WebSocket is not open");
            return Err(WasmError::WebSocket(format!("WebSocket is not open: {state:?}")).into());
        }

        // Send the serialized HTTP request over WebSocket
        ws.send(gloo_net::websocket::Message::Bytes(http_request_bytes))
            .await
            .map_err(|e| {
                error!(?e, "failed to send HTTP request over WebSocket");
                WasmError::IOError(format!("Failed to send HTTP request over WebSocket: {e:?}"))
            })?;

        // Stream response frames using the decoder
        const MAX_RESPONSE_SIZE: usize = 16 * 1024 * 1024; // 16MB

        let mut decoder = HttpResponseDecoder::new(MAX_RESPONSE_SIZE);

        loop {
            let msg = next_ws(&mut ws).await.map_err(|e| {
                error!(?e, "WebSocket read error");
                WasmError::IOError(format!("WS read error: {e:?}"))
            })?;

            let bytes = match msg {
                gloo_net::websocket::Message::Bytes(b) => b,
                gloo_net::websocket::Message::Text(t) => {
                    error!(
                        text_length = t.len(),
                        "expected binary WebSocket message, got text"
                    );
                    return Err(WasmError::IOError(format!(
                        "Expected binary WS frame, got text (len={}): {}",
                        t.len(),
                        t
                    ))
                    .into());
                }
            };

            debug!(bytes_length = bytes.len(), "received WebSocket frame");

            if let Some(resp) = decoder.feed(&bytes)? {
                info!(
                    status_code = resp.status_code,
                    "HTTP response decoded successfully"
                );
                return Ok(resp);
            }
        }
    }
}

impl WebsocketStream {
    pub fn new(url: &url::Url) -> Result<Self, WasmError> {
        info!(
            url = %url,
            "opening WebSocket connection"
        );
        let ws = WebSocket::open(url.as_str()).map_err(|e| {
            error!(?e, url = %url, "failed to open WebSocket");
            WasmError::IOError(format!("Failed to open WebSocket: {e:?}"))
        })?;

        info!(url = %url, "WebSocket connection opened successfully");
        Ok(Self {
            ws: Rc::new(Mutex::new(ws)),
        })
    }
}
