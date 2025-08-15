use base64::Engine;
use std::{fmt::Display, net::IpAddr};

#[derive(Debug, Clone)]
pub enum ServerAddress {
    Ip(IpAddr),
    Domain(String),
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
pub struct HttpRequest<T> {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<T>,
    pub cookie: Option<String>,
}

impl<T> HttpRequest<T> {
    pub fn with_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.headers.extend(headers);
        self
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse<T> {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: Option<T>,
}

#[derive(Debug)]
pub struct HttpBuilder {
    pub(crate) server: ServerAddress,
    pub(crate) port: u16,
    pub(crate) scheme: crate::connector::Scheme,
    pub(crate) authentication: crate::connector::Authentication,
    pub(crate) cookie: Option<String>,
}

impl HttpBuilder {
    pub fn new(
        server: ServerAddress,
        port: u16,
        scheme: crate::connector::Scheme,
        authentication: crate::connector::Authentication,
    ) -> Self {
        Self {
            server,
            port,
            scheme,
            authentication,
            cookie: None,
        }
    }

    pub fn with_cookie(mut self, cookie: String) -> Self {
        self.cookie = Some(cookie);
        self
    }

    fn build_url(&self, path: &str) -> String {
        let scheme_str = match self.scheme {
            crate::connector::Scheme::Http => "http",
            crate::connector::Scheme::Https => "https",
        };

        let server_str = match &self.server {
            ServerAddress::Ip(ip) => ip.to_string(),
            ServerAddress::Domain(domain) => domain.clone(),
        };

        format!("{}://{}:{}{}", scheme_str, server_str, self.port, path)
    }

    fn build_auth_header(&self) -> String {
        match &self.authentication {
            crate::connector::Authentication::Basic { username, password } => {
                let credentials = format!("{username}:{password}");
                let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
                format!("Basic {encoded}")
            }
        }
    }

    fn build_host_header(&self) -> String {
        match &self.server {
            ServerAddress::Ip(ip) => format!("{}:{}", ip, self.port),
            ServerAddress::Domain(domain) => format!("{}:{}", domain, self.port),
        }
    }

    fn build_headers(&self, body: Option<&str>) -> Vec<(String, String)> {
        let mut headers = vec![
            ("Host".to_string(), self.build_host_header()),
            (
                "Content-Type".to_string(),
                "application/soap+xml; charset=utf-8".to_string(),
            ),
            ("Authorization".to_string(), self.build_auth_header()),
        ];

        if let Some(body_content) = body {
            headers.push(("Content-Length".to_string(), body_content.len().to_string()));
        } else {
            headers.push(("Content-Length".to_string(), "0".to_string()));
        }

        if let Some(cookie) = &self.cookie {
            headers.push(("Cookie".to_string(), cookie.clone()));
        }

        headers
    }

    pub fn post(&self, path: &str, body: String) -> HttpRequest<String> {
        HttpRequest {
            method: Method::Post,
            url: self.build_url(path),
            headers: self.build_headers(Some(&body)),
            body: Some(body),
            cookie: self.cookie.clone(),
        }
    }

    pub fn get(&self, path: &str) -> HttpRequest<String> {
        HttpRequest {
            method: Method::Get,
            url: self.build_url(path),
            headers: self.build_headers(None),
            body: None,
            cookie: self.cookie.clone(),
        }
    }

    pub fn put(&self, path: &str, body: String) -> HttpRequest<String> {
        HttpRequest {
            method: Method::Put,
            url: self.build_url(path),
            headers: self.build_headers(Some(&body)),
            body: Some(body),
            cookie: self.cookie.clone(),
        }
    }

    pub fn delete(&self, path: &str) -> HttpRequest<String> {
        HttpRequest {
            method: Method::Delete,
            url: self.build_url(path),
            headers: self.build_headers(None),
            body: None,
            cookie: self.cookie.clone(),
        }
    }
}
