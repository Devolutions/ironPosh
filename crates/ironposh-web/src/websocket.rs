use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use tracing::{debug, error, info};
use url::Url;

use crate::error::WasmError;
pub struct WebsocketStream {
    tx: futures::stream::SplitSink<WebSocket, Message>,
    rx: futures::stream::SplitStream<WebSocket>,
}

impl WebsocketStream {
    pub fn new(url: Url) -> Result<Self, WasmError> {
        info!(url = %url, "opening WebSocket stream");
        let s = url.as_str().to_string();
        let ws = WebSocket::open(&s).map_err(|e| {
            error!(?e, url = %url, "failed to open WebSocket");
            WasmError::IOError(format!("Failed to open WebSocket: {e:?}"))
        })?;
        let (tx, rx) = ws.split(); // ← consumes, no Clone needed
        info!(url = %url, "WebSocket stream opened successfully");
        Ok(Self { tx, rx })
    }

    /// Sends text. If the socket is still CONNECTING, this `await` blocks until it's OPEN.
    pub async fn send_text(&mut self, s: impl Into<String>) -> Result<(), WasmError> {
        let text = s.into();
        debug!(
            text_length = text.len(),
            "sending text message over WebSocket"
        );
        self.tx.send(Message::Text(text)).await.map_err(|e| {
            error!(?e, "failed to send text message over WebSocket");
            WasmError::WebSocket(format!("send failed: {e:?}"))
        })
    }

    pub async fn send_bytes(&mut self, bytes: impl AsRef<[u8]>) -> Result<(), WasmError> {
        let bytes_vec = bytes.as_ref().to_vec();
        debug!(
            bytes_length = bytes_vec.len(),
            "sending binary message over WebSocket"
        );
        self.tx.send(Message::Bytes(bytes_vec)).await.map_err(|e| {
            error!(?e, "failed to send binary message over WebSocket");
            WasmError::WebSocket(format!("send failed: {e:?}"))
        })
    }

    /// Waits for the next incoming message (or end-of-stream).
    pub async fn recv(&mut self) -> Option<Result<Message, WasmError>> {
        match self.rx.next().await {
            Some(Ok(m)) => {
                debug!(
                    message_type = match &m {
                        Message::Text(_) => "text",
                        Message::Bytes(_) => "bytes",
                    },
                    "received message from WebSocket"
                );
                Some(Ok(m))
            }
            Some(Err(e)) => {
                error!(?e, "WebSocket receive error");
                Some(Err(WasmError::WebSocket(format!("recv error: {e:?}"))))
            }
            None => {
                info!("WebSocket stream closed");
                None
            } // closed
        }
    }

    /// Graceful close (sends a close frame).
    pub async fn close(mut self) -> Result<(), WasmError> {
        info!("closing WebSocket stream");
        self.tx.close().await.map_err(|e| {
            error!(?e, "failed to close WebSocket");
            WasmError::WebSocket(format!("close failed: {e:?}"))
        })?;
        info!("WebSocket stream closed successfully");
        Ok(())
    }
}
