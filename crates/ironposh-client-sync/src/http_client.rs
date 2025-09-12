use crate::connection::HttpClient;
use ironposh_client_core::connector::http::{HttpBody, HttpRequest, HttpResponse};
use std::{cell::RefCell, collections::HashMap, io::Read};
use tracing::{debug, error, info, info_span, instrument};

/// Decide how to read the body based on Content-Type.
fn determine_body_type_from_headers(
    headers: &[(String, String)],
) -> fn(ureq::Response) -> Result<HttpBody, anyhow::Error> {
    let content_type = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.to_lowercase())
        .unwrap_or_default();

    if content_type.contains("multipart/encrypted") {
        |response| {
            debug!("reading encrypted response as binary data");
            let mut bytes = Vec::new();
            response.into_reader().read_to_end(&mut bytes).map_err(|e| {
                error!(error=%e, "failed to read binary response body");
                anyhow::Error::from(e)
            })?;
            Ok(HttpBody::Encrypted(bytes))
        }
    } else if content_type.contains("application/soap+xml") {
        |response| {
            debug!("reading XML response as text");
            let text = response.into_string().map_err(|e| {
                error!(error=%e, "failed to read XML response body");
                anyhow::Error::from(e)
            })?;
            Ok(HttpBody::Xml(text))
        }
    } else {
        |response| {
            debug!("reading response as text");
            let text = response.into_string().map_err(|e| {
                error!(error=%e, "failed to read text response body");
                anyhow::Error::from(e)
            })?;
            Ok(HttpBody::Text(text))
        }
    }
}

/// ureq-based implementation that maintains one Agent per `connection_id`.
pub struct UreqHttpClient {
    agents: RefCell<HashMap<u32, ureq::Agent>>,
    connect_timeout: std::time::Duration,
    read_timeout: std::time::Duration,
}

impl UreqHttpClient {
    pub fn new() -> Self {
        Self {
            agents: RefCell::new(HashMap::new()),
            connect_timeout: std::time::Duration::from_secs(30),
            read_timeout: std::time::Duration::from_secs(60),
        }
    }

    pub fn with_timeouts(connect: std::time::Duration, read: std::time::Duration) -> Self {
        Self {
            agents: RefCell::new(HashMap::new()),
            connect_timeout: connect,
            read_timeout: read,
        }
    }

    fn get_or_create_agent(&self, conn_id: u32) -> ureq::Agent {
        let mut map = self.agents.borrow_mut();
        if let Some(a) = map.get(&conn_id) {
            return a.clone();
        }
        // New per-connection agent (isolates connection pooling to this conn_id)
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(self.connect_timeout)
            .timeout_read(self.read_timeout)
            .build();
        map.insert(conn_id, agent.clone());
        agent
    }

    fn make_request_with_agent(
        &self,
        agent: &ureq::Agent,
        request: &HttpRequest,
        conn_id: u32,
    ) -> Result<HttpResponse, anyhow::Error> {
        let span = info_span!("http.request", conn_id, method=?request.method, url=%request.url);
        let _enter = span.enter();

        info!("sending request");

        // Build method
        let mut ureq_req = match request.method {
            ironposh_client_core::connector::http::Method::Post => agent.post(&request.url),
            ironposh_client_core::connector::http::Method::Get => agent.get(&request.url),
            ironposh_client_core::connector::http::Method::Put => agent.put(&request.url),
            ironposh_client_core::connector::http::Method::Delete => agent.delete(&request.url),
        };

        // Headers
        for (name, value) in &request.headers {
            ureq_req = ureq_req.set(name, value);
        }
        // We want persistent connection behavior per conn_id
        ureq_req = ureq_req.set("Connection", "Keep-Alive");

        // Cookies if present
        if let Some(cookie) = &request.cookie {
            ureq_req = ureq_req.set("Cookie", cookie);
        }

        debug!(
            headers_count = request.headers.len(),
            has_cookie = request.cookie.is_some(),
            "request configured"
        );

        // Send
        let resp_res = if let Some(body) = &request.body {
            debug!(body_length = body.len(), "sending with body");
            match body {
                HttpBody::Encrypted(bytes) => ureq_req.send_bytes(bytes),
                _ => ureq_req.send_string(body.as_str()?),
            }
        } else {
            debug!("sending without body");
            ureq_req.call()
        };

        // Read response
        let (status_code, headers, response_body) = match resp_res {
            Ok(resp) => {
                let status = resp.status();
                let headers: Vec<(String, String)> = resp
                    .headers_names()
                    .iter()
                    .filter_map(|n| resp.header(n).map(|v| (n.clone(), v.to_string())))
                    .collect();

                let rdr = determine_body_type_from_headers(&headers);
                let body = rdr(resp)?;
                (status, headers, body)
            }
            Err(ureq::Error::Status(status, resp)) => {
                debug!(status, "received status response");
                let headers: Vec<(String, String)> = resp
                    .headers_names()
                    .iter()
                    .filter_map(|n| resp.header(n).map(|v| (n.clone(), v.to_string())))
                    .collect();
                let rdr = determine_body_type_from_headers(&headers);
                let body = rdr(resp).unwrap_or(HttpBody::Text(String::new()));
                (status, headers, body)
            }
            Err(e) => {
                error!(error=%e, "request failed");
                return Err(e.into());
            }
        };

        info!(status_code, response_body_length=response_body.len(), "response received");

        Ok(HttpResponse {
            status_code,
            headers,
            body: Some(response_body),
        })
    }
}

impl HttpClient for UreqHttpClient {
    #[instrument(
        name="http_client.send_request",
        level="info",
        skip(self, request),
        fields(conn_id = conn_id, method=?request.method, url=%request.url),
        err
    )]
    fn send_request(
        &self,
        request: HttpRequest,
        conn_id: u32,
    ) -> Result<HttpResponse, anyhow::Error> {
        let agent = self.get_or_create_agent(conn_id);
        self.make_request_with_agent(&agent, &request, conn_id)
    }
}

