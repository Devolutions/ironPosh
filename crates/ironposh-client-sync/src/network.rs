use ironposh_client_core::connector::{
    conntion_pool::{ConnectionId, TrySend},
    http::{HttpResponse, HttpResponseTargeted},
};
use std::sync::{mpsc, Arc};
use std::thread;
use tracing::{error, info_span, instrument};

use crate::{auth_handler::AuthHandler, connection::HttpClient};

/// Network request handler (synchronous)
pub struct NetworkHandler {
    network_request_rx: mpsc::Receiver<TrySend>,
    network_response_tx: mpsc::Sender<HttpResponseTargeted>,
}

impl NetworkHandler {
    pub fn new(
        network_request_rx: mpsc::Receiver<TrySend>,
        network_response_tx: mpsc::Sender<HttpResponseTargeted>,
    ) -> Self {
        Self {
            network_request_rx,
            network_response_tx,
        }
    }

    pub fn run<T: HttpClient + Send + Sync + 'static>(&mut self, http_client: T) {
        let _span = info_span!("NetworkRequestHandler").entered();
        let client = Arc::new(http_client);

        while let Ok(request) = self.network_request_rx.recv() {
            let network_response_tx = self.network_response_tx.clone();
            let client = Arc::clone(&client);

            // Handle request in a separate thread to avoid blocking
            thread::spawn(move || match make_http_request(request, &*client) {
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

/// Makes an HTTP request based on a TrySend command, handling both JustSend and AuthNeeded cases
#[instrument(
    name = "network.make_http_request",
    level = "info",
    skip(request, client),
    fields(request_type = %match &request {
        TrySend::JustSend { .. } => "JustSend",
        TrySend::AuthNeeded { .. } => "AuthNeeded"
    }),
    err
)]
fn make_http_request(
    request: TrySend,
    client: &dyn HttpClient,
) -> Result<HttpResponseTargeted, anyhow::Error> {
    match request {
        TrySend::JustSend { .. } => {
            // Simple case: just send the HTTP request
            let response = client.send_request(request)?;
            Ok(response)
        }
        TrySend::AuthNeeded { auth_sequence } => {
            // Complex case: handle authentication sequence using the AuthHandler
            let (authenticated_channel, auth_request) =
                AuthHandler::handle_auth_sequence(client, auth_sequence)?;

            // Create a new TrySend for the authenticated request
            let auth_try_send = TrySend::JustSend {
                request: auth_request.request,
                conn_id: auth_request.connection_id,
            };

            let response = client.send_request(auth_try_send)?;
            Ok(response)
        }
    }
}
