use anyhow::Context;
use powershell_async::HttpClient;
use pwsh_core::connector::http::{HttpRequest, HttpResponse, Method};
use reqwest::Client;
use tracing::instrument;

pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl ReqwestHttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                // TODO: Make these configurable
                .pool_max_idle_per_host(10)
                .build()
                .expect("Failed to build reqwest client"),
        }
    }
}

impl ReqwestHttpClient {
    async fn send_with_client(
        client: Client,
        request: HttpRequest<String>,
    ) -> anyhow::Result<HttpResponse<String>> {
        tracing::info!(
            method = ?request.method,
            url = %request.url,
            headers_count = request.headers.len(),
            body_length = request.body.as_ref().map(|b| b.len()).unwrap_or(0),
            "Starting HTTP request"
        );

        let mut req_builder = match request.method {
            Method::Get => client.get(&request.url),
            Method::Post => client.post(&request.url),
            Method::Put => client.put(&request.url),
            Method::Delete => client.delete(&request.url),
        };

        // Add headers
        for (key, value) in &request.headers {
            req_builder = req_builder.header(key, value);
        }

        // Add body if present
        if let Some(body) = &request.body {
            req_builder = req_builder.body(body.clone());
        }

        tracing::info!("Sending HTTP request to server");
        let response = req_builder
            .send()
            .await
            .context("Failed to send HTTP request")?;

        tracing::info!(
            status_code = response.status().as_u16(),
            "Received HTTP response"
        );

        let status_code = response.status().as_u16();
        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        tracing::info!("Reading response body");
        let body = response
            .text()
            .await
            .context("Failed to read response body")?;

        tracing::info!(
            body_length = body.len(),
            "HTTP request completed successfully"
        );

        Ok(HttpResponse {
            status_code,
            headers,
            body: Some(body),
        })
    }
}

impl HttpClient for ReqwestHttpClient {
    #[instrument(name = "http_request", level = "debug", skip(self, request))]
    async fn send_request(
        &self,
        request: HttpRequest<String>,
    ) -> anyhow::Result<HttpResponse<String>> {
        Self::send_with_client(self.client.clone(), request).await
    }

    fn send_request_callback<F>(&self, request: HttpRequest<String>, callback: F)
    where
        F: FnOnce(anyhow::Result<HttpResponse<String>>) + Send + 'static,
    {
        let client = self.client.clone();
        tokio::spawn(async move {
            let result = Self::send_with_client(client, request).await;
            callback(result);
        });
    }
}
