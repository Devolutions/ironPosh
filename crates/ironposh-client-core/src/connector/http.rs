use std::{fmt::Display, net::IpAddr};

use base64::Engine;

use crate::connector::conntion_pool::{AuthenticatedHttpChannel, ConnectionId};

pub const ENCRYPTION_BOUNDARY: &str = "Encrypted Boundary";

#[derive(Debug, Clone)]
pub enum ServerAddress {
    Ip(IpAddr),
    Domain(String),
}

impl ServerAddress {
    pub fn parse(value: &str) -> Result<Self, crate::PwshCoreError> {
        if let Ok(ip) = value.parse::<IpAddr>() {
            Ok(ServerAddress::Ip(ip))
        } else if !value.trim().is_empty() {
            Ok(ServerAddress::Domain(value.to_string()))
        } else {
            Err(crate::PwshCoreError::InvalidServerAddress(
                "server address cannot be empty",
            ))
        }
    }
}

impl Display for ServerAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerAddress::Ip(ip) => write!(f, "{ip}"),
            ServerAddress::Domain(domain) => write!(f, "{domain}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Debug, Clone)]
pub enum HttpBody {
    Xml(String),
    Encrypted(Vec<u8>),
    Text(String),
    None,
}

impl HttpBody {
    pub fn is_encrypted(&self) -> bool {
        matches!(self, HttpBody::Encrypted(_))
    }

    pub(crate) fn empty() -> HttpBody {
        HttpBody::None
    }
}

impl HttpBody {
    pub fn content_type(&self) -> &'static str {
        match self {
            HttpBody::Xml(_) => "application/soap+xml; charset=utf-8",
            HttpBody::Encrypted(_) => {
                r#"multipart/encrypted;protocol="application/HTTP-SPNEGO-session-encrypted";boundary="Encrypted Boundary""#
            }
            HttpBody::Text(_) => "text/plain; charset=utf-8",
            HttpBody::None => "text/plain; charset=utf-8",
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            HttpBody::Xml(content) => content.is_empty(),
            HttpBody::Encrypted(content) => content.is_empty(),
            HttpBody::Text(content) => content.is_empty(),
            HttpBody::None => true,
        }
    }

    /// Returns the length of the body content in bytes
    pub fn len(&self) -> usize {
        match self {
            HttpBody::Xml(content) => content.len(),
            HttpBody::Encrypted(content) => content.len(),
            HttpBody::Text(content) => content.len(),
            HttpBody::None => 0,
        }
    }

    /// Returns the body content as a string reference
    pub fn as_str(&self) -> Result<&str, crate::PwshCoreError> {
        match self {
            HttpBody::Xml(content) => Ok(content),
            HttpBody::Encrypted(_) => Err(crate::PwshCoreError::InternalError(
                "Cannot convert binary encrypted content to &str".to_owned(),
            )),
            HttpBody::Text(content) => Ok(content),
            HttpBody::None => Ok(""),
        }
    }
}

#[derive(Debug)]
pub struct HttpRequestAction {
    pub connection_id: ConnectionId,
    pub request: HttpRequest,
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<HttpBody>,
    pub cookie: Option<String>,
}

impl HttpRequest {
    pub fn with_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.headers.extend(headers);
        self
    }
}

pub trait Body {
    fn body_type() -> &'static str;
    fn inner(&self) -> &str;
}

#[derive(Debug)]
pub struct Xml {
    inner: String,
}

impl Xml {
    pub fn new(content: String) -> Self {
        Self { inner: content }
    }
}

impl Body for Xml {
    fn body_type() -> &'static str {
        "application/soap+xml; charset=utf-8"
    }

    fn inner(&self) -> &str {
        &self.inner
    }
}

#[derive(Debug)]
pub struct Encrypted {
    inner: String,
}

impl Encrypted {
    pub fn new(content: String) -> Self {
        Self { inner: content }
    }
}

impl Body for Encrypted {
    fn body_type() -> &'static str {
        r#"multipart/encrypted;protocol="application/HTTP-SPNEGO-session-encrypted";boundary="Encrypted Boundary""#
    }

    fn inner(&self) -> &str {
        &self.inner
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: HttpBody,
}

/// A targeted HTTP response that includes both the response data and the connection it came from.
/// This struct is opaque and immutable, ensuring type safety for response handling.
#[derive(Debug)]
pub struct HttpResponseTargeted {
    pub(crate) response: HttpResponse,
    pub(crate) connection_id: ConnectionId,
    pub(crate) authenticated: Option<AuthenticatedHttpChannel>,
}

impl HttpResponseTargeted {
    /// Creates a new HttpResponseTargeted from an HttpResponse and ConnectionId.
    /// This is the only way to construct this struct, ensuring controlled creation.
    pub fn new(
        response: HttpResponse,
        connection_id: ConnectionId,
        authentication_cert: Option<AuthenticatedHttpChannel>,
    ) -> Self {
        Self {
            response,
            connection_id,
            authenticated: authentication_cert,
        }
    }

    /// Gets a reference to the HTTP response data
    pub fn response(&self) -> &HttpResponse {
        &self.response
    }

    /// Gets the connection ID this response came from
    pub fn connection_id(&self) -> ConnectionId {
        self.connection_id
    }

    /// Destructures into the contained response and connection ID
    pub fn into_parts(self) -> (HttpResponse, ConnectionId) {
        (self.response, self.connection_id)
    }
}

#[derive(Debug)]
pub struct HttpBuilder {
    pub(crate) server: ServerAddress,
    pub(crate) port: u16,
    pub(crate) scheme: crate::connector::Scheme,
    // pub(crate) authentication: crate::connector::Authentication,
    pub(crate) cookie: Option<String>,
    pub(crate) headers: Vec<(String, String)>,
}

impl HttpBuilder {
    pub fn new(server: ServerAddress, port: u16, scheme: crate::connector::Scheme) -> Self {
        Self {
            server,
            port,
            scheme,
            headers: vec![],
            cookie: None,
        }
    }

    pub fn with_cookie(&mut self, cookie: String) {
        self.cookie = Some(cookie);
    }

    pub fn with_auth_header(&mut self, header: String) {
        self.headers.push(("Authorization".to_string(), header));
    }

    /// Adds `Authorization: Basic <base64(username:password)>`.
    /// WARNING: never log the resulting header value.
    pub fn with_basic(&mut self, username: &str, password: &str) -> &mut Self {
        let creds = format!("{username}:{password}");
        let b64 = base64::engine::general_purpose::STANDARD.encode(creds.as_bytes());
        self.with_auth_header(format!("Basic {b64}"));
        self
    }

    fn build_url(&self) -> String {
        let scheme_str = match self.scheme {
            crate::connector::Scheme::Http => "http",
            crate::connector::Scheme::Https => "https",
        };

        let server_str = match &self.server {
            ServerAddress::Ip(ip) => ip.to_string(),
            ServerAddress::Domain(domain) => domain.clone(),
        };

        format!(
            "{}://{}:{}{}?PSVersion=7.4.11",
            scheme_str, server_str, self.port, "/wsman"
        )
    }

    fn build_host_header(&self) -> String {
        match &self.server {
            ServerAddress::Ip(ip) => format!("{}:{}", ip, self.port),
            ServerAddress::Domain(domain) => format!("{}:{}", domain, self.port),
        }
    }

    fn build_headers(&mut self, body: Option<&HttpBody>) -> Vec<(String, String)> {
        let mut headers = vec![("Host".to_string(), self.build_host_header())];

        if let Some(body_content) = body {
            headers.push((
                "Content-Type".to_string(),
                body_content.content_type().to_string(),
            ));
            headers.push(("Content-Length".to_string(), body_content.len().to_string()));
        }

        if let Some(cookie) = &self.cookie {
            headers.push(("Cookie".to_string(), cookie.clone()));
        }

        headers.append(&mut self.headers);

        headers
    }

    pub fn post(&mut self, body: HttpBody) -> HttpRequest {
        HttpRequest {
            method: Method::Post,
            url: self.build_url(),
            headers: self.build_headers(Some(&body)),
            body: Some(body),
            cookie: self.cookie.clone(),
        }
    }
}
