use ironposh_client_core::connector::{conntion_pool::TrySend, http::HttpResponseTargeted};
use std::sync::{mpsc, Arc};
use std::thread;
use tracing::{error, info, instrument, warn};

use crate::connection::HttpClient;

/// Network request handler that maintains persistent HTTP connections
/// Processes requests concurrently to handle WinRM long polling without blocking other requests
/// Each request gets its own thread while sharing the same HTTP client for connection reuse
pub struct NetworkHandler<T: HttpClient + Clone + Send + 'static> {
    network_request_rx: mpsc::Receiver<TrySend>,
    network_response_tx: mpsc::Sender<HttpResponseTargeted>,
    http_client: Arc<T>,
}

impl<T: HttpClient + Clone + Send + Sync + 'static> NetworkHandler<T> {
    pub fn new(
        network_request_rx: mpsc::Receiver<TrySend>,
        network_response_tx: mpsc::Sender<HttpResponseTargeted>,
        http_client: T,
    ) -> Self {
        Self {
            network_request_rx,
            network_response_tx,
            http_client: Arc::new(http_client),
        }
    }

    /// Main event loop that dispatches network requests to concurrent worker threads
    /// This allows WinRM long polling operations to run without blocking other requests
    #[instrument(
        name = "network.handler.run",
        level = "info",
        skip(self),
        fields(processed_requests = 0u64, active_requests = 0u64)
    )]
    pub fn run(&mut self) {
        info!("network handler started, waiting for requests");
        let mut active_request_count = 0u64;

        while let Ok(request) = self.network_request_rx.recv() {
            let request_type = match &request {
                TrySend::JustSend { conn_id, .. } => {
                    format!("JustSend(conn_id={})", conn_id.inner())
                }
                TrySend::AuthNeeded { .. } => "AuthNeeded".to_string(),
            };

            info!(
                request_type = %request_type,
                active_requests = active_request_count,
                "dispatching network request to worker thread"
            );

            // Clone the necessary components for the worker thread
            let http_client = Arc::clone(&self.http_client);
            let response_tx = self.network_response_tx.clone();
            active_request_count += 1;

            // Spawn a worker thread for this request to handle potential long polling
            let request_type_for_thread = request_type.clone();
            thread::spawn(move || {
                Self::handle_request_in_thread(
                    request,
                    http_client,
                    response_tx,
                    request_type_for_thread,
                );
            });

            info!(
                request_type = %request_type,
                active_requests = active_request_count,
                "request dispatched to worker thread"
            );
        }

        info!("network handler shutting down, request channel closed");
    }

    /// Handle a single request in a dedicated worker thread
    /// This allows long polling operations to complete without blocking the dispatcher
    #[instrument(
        name = "network.worker_thread",
        level = "info",
        skip(http_client, response_tx, request),
        fields(request_type = %request_type)
    )]
    fn handle_request_in_thread(
        request: TrySend,
        http_client: Arc<T>,
        response_tx: mpsc::Sender<HttpResponseTargeted>,
        request_type: String,
    ) {
        info!("worker thread started for request");

        // No mutex needed - send_request only needs &self, not &mut self
        let result = http_client.send_request(request);

        match result {
            Ok(response) => {
                info!(
                    response_status = response.response().status_code,
                    response_body_length = response.response().body.len(),
                    "request processed successfully, sending response"
                );

                if let Err(e) = response_tx.send(response) {
                    error!(
                        error = %e,
                        "failed to send network response, main thread may have shut down"
                    );
                }
            }
            Err(e) => {
                error!(
                    error = %e,
                    request_type = %request_type,
                    "HTTP request failed in worker thread"
                );
                // TODO: Consider implementing retry logic or error recovery here
                // For now, we just log the error and let the main loop handle the timeout
            }
        }

        info!("worker thread completed");
    }
}
