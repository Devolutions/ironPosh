use anyhow::{Context, Result};
use ironposh_client_core::connector::http::{HttpBody, HttpRequest, HttpResponse, Method};

/// Serialize an HttpRequest to HTTP/1.1 wire format as bytes
pub fn serialize_http_request(request: &HttpRequest) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();

    // Request line: METHOD PATH HTTP/1.1
    let method_str = match request.method {
        Method::Get => "GET",
        Method::Post => "POST",
        Method::Put => "PUT",
        Method::Delete => "DELETE",
    };

    // Parse URL to get path and host
    let url = url::Url::parse(&request.url).context("Failed to parse request URL")?;

    let path = if url.query().is_some() {
        format!("{}?{}", url.path(), url.query().unwrap())
    } else {
        url.path().to_string()
    };

    // Request line
    let request_line = format!("{method_str} {path} HTTP/1.1\r\n");
    buffer.extend_from_slice(request_line.as_bytes());

    // Host header (required for HTTP/1.1)
    if let Some(host) = url.host_str() {
        let host_header = url.port().map_or_else(
            || format!("Host: {host}\r\n"),
            |port| format!("Host: {host}:{port}\r\n"),
        );
        buffer.extend_from_slice(host_header.as_bytes());
    }

    // Headers
    for (name, value) in &request.headers {
        if name.eq_ignore_ascii_case("host") || name.eq_ignore_ascii_case("content-length") {
            continue;
        }
        let header_line = format!("{name}: {value}\r\n");
        buffer.extend_from_slice(header_line.as_bytes());
    }

    // Cookie header if present
    if let Some(cookie) = &request.cookie {
        let cookie_line = format!("Cookie: {cookie}\r\n");
        buffer.extend_from_slice(cookie_line.as_bytes());
    }

    // Body handling
    if let Some(body) = &request.body {
        match body {
            HttpBody::Text(text) => {
                let content_length = format!("Content-Length: {}\r\n", text.len());
                buffer.extend_from_slice(content_length.as_bytes());
                buffer.extend_from_slice(b"\r\n");
                buffer.extend_from_slice(text.as_bytes());
                return Ok(buffer);
            }
            HttpBody::Encrypted(bytes) => {
                let content_length = format!("Content-Length: {}\r\n", bytes.len());
                buffer.extend_from_slice(content_length.as_bytes());
                buffer.extend_from_slice(b"\r\n");
                buffer.extend_from_slice(bytes);
                return Ok(buffer);
            }
            HttpBody::Xml(xml) => {
                let content_length = format!("Content-Length: {}\r\n", xml.len());
                buffer.extend_from_slice(content_length.as_bytes());
                buffer.extend_from_slice(b"\r\n");
                buffer.extend_from_slice(xml.as_bytes());
                return Ok(buffer);
            }
            HttpBody::None => {
                // No body content
            }
        }
    }

    // Some servers (notably WinRM/WSMan) require Content-Length for POST/PUT even
    // when there is no body.
    if matches!(request.method, Method::Post | Method::Put)
        && matches!(&request.body, None | Some(HttpBody::None))
    {
        buffer.extend_from_slice(b"Content-Length: 0\r\n");
    }

    // No body - just end headers
    buffer.extend_from_slice(b"\r\n");

    Ok(buffer)
}

/// Deserialize HTTP/1.1 wire format bytes to HttpResponse
pub fn deserialize_http_response(bytes: &[u8]) -> Result<HttpResponse> {
    // Find the end of headers (double CRLF)
    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .context("Failed to find end of headers in HTTP response")?;

    let header_section = &bytes[..header_end];
    let body_start = header_end + 4;

    // Parse headers as UTF-8
    let header_str = std::str::from_utf8(header_section)
        .context("Failed to parse HTTP response headers as UTF-8")?;

    let mut lines = header_str.lines();

    // Parse status line: HTTP/1.1 200 OK
    let status_line = lines
        .next()
        .context("Missing status line in HTTP response")?;

    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .context("Failed to parse status code from status line")?;

    // Parse headers
    let mut headers = Vec::new();
    let mut content_type = None;

    for line in lines {
        if line.is_empty() {
            continue;
        }

        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_string();
            let value = value.trim().to_string();

            // Track content-type for body parsing
            if name.eq_ignore_ascii_case("content-type") {
                content_type = Some(value.clone());
            }

            headers.push((name, value));
        }
    }

    // Parse body
    let body_bytes = &bytes[body_start..];

    // Determine if we should treat as encrypted based on content-type
    let body = if let Some(ct) = &content_type {
        if ct.contains("application/soap+xml")
            || ct.contains("application/octet-stream")
            || ct.contains("multipart/encrypted")
        {
            // Could be encrypted SOAP - check if it's valid UTF-8
            std::str::from_utf8(body_bytes).map_or_else(
                |_| HttpBody::Encrypted(body_bytes.to_vec()),
                |text| HttpBody::Text(text.to_string()),
            )
        } else {
            // Text-based content type
            let text = std::str::from_utf8(body_bytes)
                .context("Failed to parse response body as UTF-8")?;
            HttpBody::Text(text.to_string())
        }
    } else {
        // No content-type, try to parse as UTF-8, otherwise treat as encrypted
        std::str::from_utf8(body_bytes).map_or_else(
            |_| HttpBody::Encrypted(body_bytes.to_vec()),
            |text| HttpBody::Text(text.to_string()),
        )
    };

    Ok(HttpResponse {
        status_code,
        headers,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_simple_get() {
        let request = HttpRequest {
            method: Method::Get,
            url: "http://example.com/path".to_string(),
            headers: vec![("User-Agent".to_string(), "test".to_string())],
            cookie: None,
            body: None,
        };

        let bytes = serialize_http_request(&request).unwrap();
        let result = String::from_utf8(bytes).unwrap();

        assert!(result.starts_with("GET /path HTTP/1.1\r\n"));
        assert!(result.contains("Host: example.com\r\n"));
        assert!(result.contains("User-Agent: test\r\n"));
    }

    #[test]
    fn test_serialize_post_with_body() {
        let request = HttpRequest {
            method: Method::Post,
            url: "http://example.com/api".to_string(),
            headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
            cookie: None,
            body: Some(HttpBody::Text("test body".to_string())),
        };

        let bytes = serialize_http_request(&request).unwrap();
        let result = String::from_utf8(bytes).unwrap();

        assert!(result.starts_with("POST /api HTTP/1.1\r\n"));
        assert!(result.contains("Content-Length: 9\r\n"));
        assert!(result.ends_with("test body"));
    }

    #[test]
    fn test_deserialize_response() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\r\nhello";

        let response = deserialize_http_response(raw).unwrap();

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body.as_str().unwrap(), "hello");
    }
}
