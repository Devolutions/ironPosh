use ironposh_client_core::connector::{conntion_pool::TrySend, http::HttpResponseTargeted};
use std::future::Future;

// Internal modules
mod connection;
mod host_calls;
mod session;

// Public API
pub mod client;

// Re-export the main client
pub use client::RemoteAsyncPowershellClient;

pub trait AsyncPowershellClient {
    fn open_task(&self, client: impl HttpClient) -> impl Future<Output = anyhow::Result<()>>
    where
        Self: Sized;

    fn send_command(&self, command: String) -> impl Future<Output = anyhow::Result<String>>;
}

pub trait HttpClient: Send + Sync + 'static {
    fn send_request(
        &self,
        try_send: TrySend,
    ) -> impl Future<Output = anyhow::Result<HttpResponseTargeted>> + Send;
}
