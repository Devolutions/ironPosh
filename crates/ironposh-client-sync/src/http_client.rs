use crate::connection::HttpClient;
use tracing::{debug, info};

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
    let response_result = if let Some(body) = &request.body {
        ureq_request.send_string(body)
    } else {
        ureq_request.call()
    };

    // Handle the response, including potential 401 authentication challenges
    let (status_code, headers, response_body) = match response_result {
        Ok(response) => {
            let status = response.status();
            let headers: Vec<(String, String)> = response
                .headers_names()
                .iter()
                .filter_map(|name| {
                    response
                        .header(name)
                        .map(|value| (name.clone(), value.to_string()))
                })
                .collect();
            let body = response.into_string()?;
            (status, headers, body)
        }
        Err(ureq::Error::Status(status, response)) => {
            // Handle status codes like 401 which are expected in authentication flows
            let headers: Vec<(String, String)> = response
                .headers_names()
                .iter()
                .filter_map(|name| {
                    response
                        .header(name)
                        .map(|value| (name.clone(), value.to_string()))
                })
                .collect();
            let body = response.into_string().unwrap_or_default();
            (status, headers, body)
        }
        Err(e) => {
            // For other errors (network issues, etc.), propagate them
            return Err(e.into());
        }
    };

    debug!("Response status: {}", status_code);
    debug!("Response headers: {:?}", headers);
    debug!("Response body length: {}", response_body.len());

    // Return as HttpResponse with actual response data
    Ok(ironposh_client_core::connector::http::HttpResponse {
        status_code: status_code as u16,
        headers,
        body: Some(response_body),
    })
}
