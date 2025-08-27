use std::sync::mpsc;
use std::thread;
use tracing::{error, info_span};

use crate::http_client::make_http_request;

/// Network request handler (synchronous)
pub struct NetworkHandler {
    network_request_rx: mpsc::Receiver<ironposh_client_core::connector::http::HttpRequest<String>>,
    network_response_tx: mpsc::Sender<ironposh_client_core::connector::http::HttpResponse<String>>,
}

impl NetworkHandler {
    pub fn new(
        network_request_rx: mpsc::Receiver<
            ironposh_client_core::connector::http::HttpRequest<String>,
        >,
        network_response_tx: mpsc::Sender<
            ironposh_client_core::connector::http::HttpResponse<String>,
        >,
    ) -> Self {
        Self {
            network_request_rx,
            network_response_tx,
        }
    }

    pub fn run(&mut self) {
        let _span = info_span!("NetworkRequestHandler").entered();

        while let Ok(request) = self.network_request_rx.recv() {
            let network_response_tx = self.network_response_tx.clone();

            // Handle request in a separate thread to avoid blocking
            thread::spawn(move || match make_http_request(&request) {
                Ok(response) => {
                    if let Err(e) = network_response_tx.send(response) {
                        error!("Failed to send network response: {}", e);
                    }
                }
                Err(e) => {
                    error!("HTTP request failed: {}", e);
                }
            });
        }
    }
}
