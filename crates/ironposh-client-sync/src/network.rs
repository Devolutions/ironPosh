use ironposh_client_core::connector::{
    conntion_pool::{ConnectionId, TrySend},
    http::HttpResponse,
};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use tracing::{error, info_span};

use crate::connection::HttpClient;

/// Network request handler (synchronous)
pub struct NetworkHandler {
    network_request_rx: mpsc::Receiver<TrySend>,
    network_response_tx: mpsc::Sender<(HttpResponse, ConnectionId)>,
    http_client: Arc<dyn HttpClient + Send>,
}

impl NetworkHandler {
    pub fn new<T: HttpClient + Send + 'static>(
        network_request_rx: mpsc::Receiver<TrySend>,
        network_response_tx: mpsc::Sender<(HttpResponse, ConnectionId)>,
        http_client: T,
    ) -> Self {
        Self {
            network_request_rx,
            network_response_tx,
            http_client: Arc::new(http_client),
        }
    }

    pub fn run(&mut self) {
        let _span = info_span!("NetworkRequestHandler").entered();

        while let Ok(request) = self.network_request_rx.recv() {
            let network_response_tx = self.network_response_tx.clone();

            // Handle request in a separate thread to avoid blocking
            let client = Arc::clone(&self.http_client);
            thread::spawn(move || {
                match request {
                    TrySend::JustSend { request, conn_id } => {
                        let response = client
                            .lock()
                            .unwrap()
                            .send_request(request, conn_id.inner());
                        match response {
                            Ok(resp) => {
                                let _ = network_response_tx.send((resp, conn_id));
                            }
                            Err(e) => {
                                error!(error=%e, "failed to send network request");
                                // In a real implementation, you might want to send an error response back
                            }
                        }
                    }
                    TrySend::AuthNeeded { auth_sequence } => {
                        // In a real implementation, you would handle the auth sequence here
                        error!("AuthNeeded requests are not supported in this simplified example");
                    }
                }
            });
        }
    }
}
