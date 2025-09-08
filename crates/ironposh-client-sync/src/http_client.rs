use crate::connection::{HttpClient, KeepAlive};
use std::cell::RefCell;
use tracing::{debug, error, info, info_span, instrument};

pub struct UreqHttpClient {
    client: RefCell<Option<ureq::Agent>>,
}

impl UreqHttpClient {
    pub fn new() -> Self {
        UreqHttpClient {
            client: RefCell::new(None),
        }
    }

    fn get_or_create_client(&self) -> ureq::Agent {
        let mut client_ref = self.client.borrow_mut();
        if let Some(agent) = client_ref.as_ref() {
            agent.clone()
        } else {
            let agent = ureq::AgentBuilder::new()
                .timeout_connect(std::time::Duration::from_secs(30))
                .timeout_read(std::time::Duration::from_secs(60))
                .build();
            *client_ref = Some(agent.clone());
            agent
        }
    }

    fn make_request_with_agent(
        &self,
        agent: &ureq::Agent,
        request: &ironposh_client_core::connector::http::HttpRequest<String>,
    ) -> Result<ironposh_client_core::connector::http::HttpResponse<String>, anyhow::Error> {
        let span = info_span!("http.request", method=?request.method, url=%request.url);
        let _enter = span.enter();

        info!("sending request");

        // Build the HTTP client request
        let mut ureq_request = match request.method {
            ironposh_client_core::connector::http::Method::Post => agent.post(&request.url),
            ironposh_client_core::connector::http::Method::Get => agent.get(&request.url),
            ironposh_client_core::connector::http::Method::Put => agent.put(&request.url),
            ironposh_client_core::connector::http::Method::Delete => agent.delete(&request.url),
        };

        // Add headers
        for (name, value) in &request.headers {
            ureq_request = ureq_request.set(name, value);
        }

        // Add cookie if present
        if let Some(cookie) = &request.cookie {
            ureq_request = ureq_request.set("Cookie", cookie);
        }

        debug!(
            headers_count = request.headers.len(),
            has_cookie = request.cookie.is_some(),
            "request configured"
        );

        // Make the request
        let response_result = if let Some(body) = &request.body {
            debug!(body_length = body.len(), "sending with body");
            ureq_request.send_string(body)
        } else {
            debug!("sending without body");
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
                let body = response.into_string().map_err(|e| {
                    error!(error=%e, "failed to read response body");
                    e
                })?;
                (status, headers, body)
            }
            Err(ureq::Error::Status(status, response)) => {
                // Handle status codes like 401 which are expected in authentication flows
                debug!(status=%status, "received status response");
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
                error!(error=%e, "request failed");
                return Err(e.into());
            }
        };

        info!(status_code=%status_code, response_body_length=response_body.len(), "response received");

        // Return as HttpResponse with actual response data
        Ok(ironposh_client_core::connector::http::HttpResponse {
            status_code: status_code,
            headers,
            body: Some(response_body),
        })
    }
}

impl HttpClient for UreqHttpClient {
    #[instrument(name="http_client.send_request", level="info", skip(self, request), fields(method=?request.method, url=%request.url, keep_alive=?keep_alive), err)]
    fn send_request(
        &self,
        request: ironposh_client_core::connector::http::HttpRequest<String>,
        keep_alive: KeepAlive,
    ) -> Result<ironposh_client_core::connector::http::HttpResponse<String>, anyhow::Error> {
        match keep_alive {
            KeepAlive::Must => {
                info!("using persistent client");
                let agent = self.get_or_create_client();
                self.make_request_with_agent(&agent, &request)
            }
            KeepAlive::NotNecessary => {
                info!("using one-time client");
                make_http_request(&request)
            }
        }
    }
}

/// Make an HTTP request using ureq (synchronous)
#[instrument(name="http.request_oneshot", level="info", skip(request), fields(method=?request.method, url=%request.url), err)]
pub fn make_http_request(
    request: &ironposh_client_core::connector::http::HttpRequest<String>,
) -> Result<ironposh_client_core::connector::http::HttpResponse<String>, anyhow::Error> {
    info!("sending one-time request");

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

    debug!(
        headers_count = request.headers.len(),
        has_cookie = request.cookie.is_some(),
        "request configured"
    );

    // Make the request
    let response_result = if let Some(body) = &request.body {
        debug!(body_length = body.len(), "sending with body");
        ureq_request.send_string(body)
    } else {
        debug!("sending without body");
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
            let body = response.into_string().map_err(|e| {
                error!(error=%e, "failed to read response body");
                e
            })?;
            (status, headers, body)
        }
        Err(ureq::Error::Status(status, response)) => {
            // Handle status codes like 401 which are expected in authentication flows
            debug!(status=%status, "received status response");
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
            error!(error=%e, "request failed");
            return Err(e.into());
        }
    };

    info!(status_code=%status_code, response_body_length=response_body.len(), "response received");

    // Return as HttpResponse with actual response data
    Ok(ironposh_client_core::connector::http::HttpResponse {
        status_code: status_code,
        headers,
        body: Some(response_body),
    })
}
