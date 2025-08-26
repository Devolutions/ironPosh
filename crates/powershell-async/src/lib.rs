use pwsh_core::connector::http::{HttpRequest, HttpResponse};
use std::future::Future;

mod notify_map;
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

/*
    Should have something like this
    let command = client_core.send_command(pipeline_id, command)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))?;
*/
