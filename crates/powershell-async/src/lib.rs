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
    fn send_request(
        &self,
        request: HttpRequest<String>,
    ) -> impl Future<Output = anyhow::Result<HttpResponse<String>>> + Send;
}
