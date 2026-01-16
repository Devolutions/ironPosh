use crate::connection::HttpClient;
use ironposh_client_core::connector::http::{HttpBody, HttpRequest, HttpResponse};
use std::{
    collections::HashMap,
    io::Read,
    sync::{Arc, Mutex},
};
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
            response
                .into_reader()
                .read_to_end(&mut bytes)
                .map_err(|e| {
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
#[derive(Clone)]
pub struct UreqHttpClient {
    agents: Arc<Mutex<HashMap<u32, ureq::Agent>>>,
    connect_timeout: std::time::Duration,
    read_timeout: std::time::Duration,
}

impl UreqHttpClient {
    pub fn new() -> Self {
        info!(
            connect_timeout_secs = 30,
            read_timeout_secs = 60,
            "initializing UreqHttpClient with connection pooling"
        );
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
            connect_timeout: std::time::Duration::from_secs(30),
            read_timeout: std::time::Duration::from_secs(60),
        }
    }

    #[instrument(level = "debug", skip(self), fields(conn_id))]
    fn get_or_create_agent(&self, conn_id: u32) -> ureq::Agent {
        let mut map = self.agents.lock().unwrap();
        if let Some(a) = map.get(&conn_id) {
            info!(conn_id, "reusing existing HTTP agent for connection");
            return a.clone();
        }
        // New per-connection agent (isolates connection pooling to this conn_id)
        info!(
            conn_id,
            total_agents = map.len(),
            "creating new HTTP agent for connection"
        );
        let tls_connector = std::sync::Arc::new(
            native_tls::TlsConnector::new().expect("failed to create TLS connector"),
        );
        let agent = ureq::AgentBuilder::new()
            .tls_connector(tls_connector)
            .timeout_connect(self.connect_timeout)
            .timeout_read(self.read_timeout)
            .build();
        map.insert(conn_id, agent.clone());
        info!(
            conn_id,
            total_agents = map.len(),
            "HTTP agent created and cached"
        );
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

        let agent_pool_size = self.agents.lock().unwrap().len();
        info!(agent_pool_size, "sending request with pooled agent");

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

        info!(
            status_code,
            response_body_length = response_body.len(),
            "response received"
        );

        Ok(HttpResponse {
            status_code,
            headers,
            body: response_body,
        })
    }
}

impl HttpClient for UreqHttpClient {
    #[instrument(
        name = "http_client.send_request",
        level = "info",
        skip(self, try_send),
        err
    )]
    fn send_request(
        &self,
        try_send: ironposh_client_core::connector::conntion_pool::TrySend,
    ) -> Result<ironposh_client_core::connector::http::HttpResponseTargeted, anyhow::Error> {
        use crate::kerberos::send_packet;
        use anyhow::Context;
        use ironposh_client_core::connector::{
            auth_sequence::SspiAuthSequence,
            authenticator::SecContextMaybeInit,
            conntion_pool::{SecContextInited, TrySend},
            http::{HttpRequestAction, HttpResponseTargeted},
        };

        match try_send {
            // === Simple path: already have an idle, encrypted channel ===
            TrySend::JustSend { request, conn_id } => {
                info!(conn_id = conn_id.inner(), "sending on existing connection");
                let agent = self.get_or_create_agent(conn_id.inner());
                let resp = self.make_request_with_agent(&agent, &request, conn_id.inner())?;
                // No provider attached on steady-state sends
                Ok(HttpResponseTargeted::new(resp, conn_id, None))
            }

            // === Auth path: drive the per-connection FSM, then send first sealed request ===
            TrySend::AuthNeeded { mut auth_sequence } => {
                info!("starting authentication sequence");
                let mut auth_response: Option<ironposh_client_core::connector::http::HttpResponse> =
                    None;

                loop {
                    // 1) Initialize security context (may require KDC generator)
                    let (seq, mut holder) = auth_sequence.prepare();
                    let init =
                        match seq.try_init_sec_context(auth_response.as_ref(), &mut holder)? {
                            SecContextMaybeInit::Initialized(sec) => sec,
                            SecContextMaybeInit::RunGenerator {
                                mut packet,
                                mut generator_holder,
                            } => {
                                info!("running generator for KDC communication");
                                loop {
                                    let kdc_resp = send_packet(packet).context(
                                        "failed to send packet to KDC during authentication",
                                    )?;
                                    match SspiAuthSequence::resume(generator_holder, kdc_resp)? {
                                        SecContextMaybeInit::RunGenerator {
                                            packet: p2,
                                            generator_holder: g2,
                                        } => {
                                            packet = p2;
                                            generator_holder = g2;
                                        }
                                        SecContextMaybeInit::Initialized(sec) => break sec,
                                    }
                                }
                            }
                        };

                    // 2) Process initialized context → either Continue (send another token) or Done
                    match auth_sequence.process_sec_ctx_init(&init)? {
                        SecContextInited::Continue { request, sequence } => {
                            info!("continuing authentication sequence");
                            // send challenge-response step on the same conn_id
                            let HttpRequestAction {
                                connection_id,
                                request,
                            } = request;
                            let agent = self.get_or_create_agent(connection_id.inner());
                            let resp = self.make_request_with_agent(
                                &agent,
                                &request,
                                connection_id.inner(),
                            )?;
                            auth_response = Some(resp); // feed back into try_init_sec_context
                            auth_sequence = sequence; // keep looping
                        }

                        SecContextInited::SendRequest {
                            request,
                            authenticated_http_channel_cert,
                        } => {
                            info!(
                                "authentication sequence complete, sending final encrypted request"
                            );
                            // We have: (a) the final encrypted HttpRequest to send, and
                            // (b) the EncryptionProvider to INSTALL on this conn_id for the *response*.
                            let HttpRequestAction {
                                connection_id,
                                request,
                            } = request;

                            // 3) Send the final (sealed) request
                            let agent = self.get_or_create_agent(connection_id.inner());
                            let resp = self.make_request_with_agent(
                                &agent,
                                &request,
                                connection_id.inner(),
                            )?;

                            // 4) Return targeted response WITH the provider attached.
                            //    Pool::accept will install it on PreAuth and decrypt the body.
                            info!("authentication sequence successful");
                            return Ok(HttpResponseTargeted::new(
                                resp,
                                connection_id,
                                Some(authenticated_http_channel_cert),
                            ));
                        }
                    }
                }
            }
        }
    }
}
