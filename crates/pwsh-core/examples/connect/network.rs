use tokio::sync::mpsc;
use tracing::{Instrument, error, info_span};

use crate::http_client::make_http_request;

/// Spawn network request handler task
pub fn spawn_network_handler(
    mut network_request_rx: mpsc::Receiver<pwsh_core::connector::http::HttpRequest<String>>,
    network_response_tx: mpsc::Sender<pwsh_core::connector::http::HttpResponse<String>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(
        async move {
            while let Some(request) = network_request_rx.recv().await {
                let network_response_tx = network_response_tx.clone();
                tokio::spawn(async move {
                    match make_http_request(&request).await {
                        Ok(response) => {
                            if let Err(e) = network_response_tx.send(response).await {
                                error!("Failed to send network response: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("HTTP request failed: {}", e);
                        }
                    }
                });
            }
        }
        .instrument(info_span!("NetworkRequestHandler")),
    )
}
