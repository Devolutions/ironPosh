use ironposh_client_core::connector::{conntion_pool::TrySend, http::HttpResponseTargeted};
use std::sync::mpsc;
use tracing::{error, info, instrument};

use crate::connection::HttpClient;

/// Network request handler that maintains persistent HTTP connections
/// Processes requests sequentially to ensure proper connection reuse and state management
pub struct NetworkHandler<T: HttpClient> {
    network_request_rx: mpsc::Receiver<TrySend>,
    network_response_tx: mpsc::Sender<HttpResponseTargeted>,
    http_client: T,
}

impl<T: HttpClient> NetworkHandler<T> {
    pub fn new(
        network_request_rx: mpsc::Receiver<TrySend>,
        network_response_tx: mpsc::Sender<HttpResponseTargeted>,
        http_client: T,
    ) -> Self {
        Self {
            network_request_rx,
            network_response_tx,
            http_client,
        }
    }

    /// Main event loop that processes network requests sequentially
    /// This ensures proper connection reuse and maintains authentication state
    #[instrument(
        name = "network.handler.run",
        level = "info",
        skip(self),
        fields(processed_requests = 0u64)
    )]
    pub fn run(&mut self) {
        let span = tracing::Span::current();
        let mut processed_requests = 0u64;

        info!("network handler started, waiting for requests");

        while let Ok(request) = self.network_request_rx.recv() {
            processed_requests += 1;
            span.record("processed_requests", processed_requests);

            let request_type = match &request {
                TrySend::JustSend { conn_id, .. } => {
                    format!("JustSend(conn_id={})", conn_id.inner())
                }
                TrySend::AuthNeeded { .. } => "AuthNeeded".to_string(),
            };

            info!(
                request_type = %request_type,
                request_number = processed_requests,
                "processing network request"
            );

            match self.process_request(request) {
                Ok(response) => {
                    info!(
                        response_status = response.response().status_code,
                        response_body_length = response.response().body.len(),
                        "request processed successfully, sending response"
                    );

                    if let Err(e) = self.network_response_tx.send(response) {
                        error!(
                            error = %e,
                            "failed to send network response, channel may be disconnected"
                        );
                        // If we can't send responses, no point in continuing
                        break;
                    }
                }
                Err(e) => {
                    error!(
                        error = %e,
                        request_type = %request_type,
                        "HTTP request failed"
                    );
                    // For now, we continue processing other requests even if one fails
                    // In the future, we might want to implement retry logic or circuit breakers
                }
            }
        }

        info!(
            total_processed = processed_requests,
            "network handler shutting down, request channel closed"
        );
    }

    /// Process a single network request using the persistent HTTP client
    #[instrument(
        name = "network.process_request",
        level = "info",
        skip(self, request),
        fields(
            request_type = %match &request {
                TrySend::JustSend { conn_id, .. } => format!("JustSend({})", conn_id.inner()),
                TrySend::AuthNeeded { .. } => "AuthNeeded".to_string()
            }
        ),
        err
    )]
    fn process_request(&mut self, request: TrySend) -> Result<HttpResponseTargeted, anyhow::Error> {
        // The HTTP client now handles both JustSend and AuthNeeded cases internally
        // This includes managing authentication sequences, KDC communication, and connection reuse
        self.http_client.send_request(request)
    }
}
