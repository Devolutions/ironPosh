use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use url::Url;

use crate::error::WasmError;
pub struct WebsocketStream {
    tx: futures::stream::SplitSink<WebSocket, Message>,
    rx: futures::stream::SplitStream<WebSocket>,
}

impl WebsocketStream {
    pub fn new(url: Url) -> Result<Self, WasmError> {
        let s = url.as_str().to_string();
        let ws = WebSocket::open(&s)
            .map_err(|e| WasmError::IOError(format!("Failed to open WebSocket: {e:?}")))?;
        let (tx, rx) = ws.split(); // ← consumes, no Clone needed
        Ok(Self { tx, rx })
    }

    /// Sends text. If the socket is still CONNECTING, this `await` blocks until it's OPEN.
    pub async fn send_text(&mut self, s: impl Into<String>) -> Result<(), WasmError> {
        self.tx
            .send(Message::Text(s.into()))
            .await
            .map_err(|e| WasmError::WebSocket(format!("send failed: {e:?}")))
    }

    pub async fn send_bytes(&mut self, bytes: impl AsRef<[u8]>) -> Result<(), WasmError> {
        self.tx
            .send(Message::Bytes(bytes.as_ref().to_vec()))
            .await
            .map_err(|e| WasmError::WebSocket(format!("send failed: {e:?}")))
    }

    /// Waits for the next incoming message (or end-of-stream).
    pub async fn recv(&mut self) -> Option<Result<Message, WasmError>> {
        match self.rx.next().await {
            Some(Ok(m)) => Some(Ok(m)),
            Some(Err(e)) => Some(Err(WasmError::WebSocket(format!("recv error: {e:?}")))),
            None => None, // closed
        }
    }

    /// Graceful close (sends a close frame).
    pub async fn close(mut self) -> Result<(), WasmError> {
        self.tx
            .close()
            .await
            .map_err(|e| WasmError::WebSocket(format!("close failed: {e:?}")))
    }
}
