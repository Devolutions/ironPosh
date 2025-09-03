use tracing::{debug, info};
use crate::connection::HttpClient;

pub struct UreqHttpClient;

impl HttpClient for UreqHttpClient {
    fn send_request(
        &self,
        request: ironposh_client_core::connector::http::HttpRequest<String>,
    ) -> Result<ironposh_client_core::connector::http::HttpResponse<String>, anyhow::Error> {
        make_http_request(&request)
    }
}

/// Make an HTTP request using ureq (synchronous)
pub fn make_http_request(
    request: &ironposh_client_core::connector::http::HttpRequest<String>,
) -> Result<ironposh_client_core::connector::http::HttpResponse<String>, anyhow::Error> {
    info!("Making HTTP request to: {}", request.url);
    debug!("Request headers: {:?}", request.headers);
    debug!(
        "Request body length: {:?}",
        request.body.as_ref().map(|b| b.len())
    );

    // Build the HTTP client request
    let mut ureq_request = match request.method {
        ironposh_client_core::connector::http::Method::Post => ureq::post(&request.url),
        ironposh_client_core::connector::http::Method::Get => ureq::get(&request.url),
        ironposh_client_core::connector::http::Method::Put => ureq::put(&request.url),
        ironposh_client_core::connector::http::Method::Delete => ureq::delete(&request.url),
    };

    // Add headers
    for (name, value) in &request.headers {
        ureq_request = ureq_request.set(name, value);
    }

    // Add cookie if present
    if let Some(cookie) = &request.cookie {
        ureq_request = ureq_request.set("Cookie", cookie);
    }

    // Make the request
    let response = if let Some(body) = &request.body {
        ureq_request.send_string(body)?
    } else {
        ureq_request.call()?
    };

    // Read response
    let response_body = response.into_string()?;
    debug!("Response body length: {}", response_body.len());

    // Return as HttpResponse with proper response format
    Ok(ironposh_client_core::connector::http::HttpResponse {
        status_code: 200,
        headers: vec![],
        body: Some(response_body),
    })
}
