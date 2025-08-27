use ironposh_client_core::connector::http::{HttpRequest, HttpResponse};
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

    // Channel-based method for long-hanging requests
    // Sends the result over the provided channel when finished
    fn send_request_with_channel(
        &self, 
        request: HttpRequest<String>, 
        tx: futures::channel::mpsc::Sender<anyhow::Result<HttpResponse<String>>>
    );
}
