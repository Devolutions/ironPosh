use pwsh_core::connector::http::{HttpRequest, HttpResponse};
use std::future::Future;

pub mod remote_client;

pub trait AsyncPowershellClient {
    fn open_task(&self, client: impl HttpClient) -> impl Future<Output = anyhow::Result<()>>
    where
        Self: Sized;

    fn send_command(&self, command: String) -> impl Future<Output = anyhow::Result<String>>;
}

pub trait HttpClient: Send + Sync + 'static {
    // Asynchronous method to send an HTTP request and receive a response
    fn send_request(
        &self,
        request: HttpRequest<String>,
    ) -> impl Future<Output = anyhow::Result<HttpResponse<String>>> + Send;

    // The reason to use callback style is that
    // Some HTTP Request is long hanging, we don't want to block the current async task
    fn send_request_callback<F>(&self, request: HttpRequest<String>, callback: F)
    where
        F: FnOnce(anyhow::Result<HttpResponse<String>>) + Send + 'static;
}
