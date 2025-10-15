use futures::StreamExt;
use gloo_net::websocket::{futures::WebSocket, Message as WsMessage};
use ironposh_client_core::connector::http::{HttpBody, HttpResponse};
use tracing::{debug, error, info, warn};

use crate::error::WasmError;

#[derive(Debug)]
pub enum ProtocolError {
    Incomplete(&'static str),
    Malformed(&'static str),
    TooLarge,
    Timeout,
    UnexpectedTextFrame,
    WebsocketError(gloo_net::websocket::WebSocketError),
}

impl From<ProtocolError> for WasmError {
    fn from(e: ProtocolError) -> Self {
        match e {
            ProtocolError::Incomplete(s) => Self::IOError(format!("Incomplete: {s}")),
            ProtocolError::Malformed(s) => Self::IOError(format!("Malformed: {s}")),
            ProtocolError::TooLarge => Self::IOError("Response too large".to_string()),
            ProtocolError::Timeout => Self::IOError("Timeout".to_string()),
            ProtocolError::UnexpectedTextFrame => {
                Self::IOError("Unexpected text frame".to_string())
            }
            ProtocolError::WebsocketError(e) => Self::IOError(format!("WebSocket error: {e:?}")),
        }
    }
}

pub async fn next_ws(ws: &mut WebSocket) -> Result<WsMessage, ProtocolError> {
    let next = ws
        .next()
        .await
        .ok_or(ProtocolError::Incomplete("websocket closed"))?
        .map_err(ProtocolError::WebsocketError)?;

    Ok(next)
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BodyMode {
    None,
    Fixed(usize),
    Chunked,
}

pub struct HttpResponseDecoder {
    max_size: usize,
    /// Raw buffer of *received frames concatenated*
    buf: Vec<u8>,
    hdr_end: Option<usize>, // index of '\r\n\r\n' end (exclusive of CRLFCRLF)
    status: Option<u16>,
    mode: Option<BodyMode>,

    // for chunked decoding
    /// decoded body bytes (chunk-meta stripped)
    decoded: Vec<u8>,
    /// cursor where chunked parsing starts (after headers)
    chunk_cursor: usize,
    /// if currently waiting for a chunk of this size (in bytes)
    cur_chunk_remaining: Option<usize>,
    /// have we seen the terminal 0-sized chunk yet?
    chunk_done: bool,
    /// after terminal chunk: we must read trailers until CRLFCRLF
    trailers_complete: bool,
}

impl HttpResponseDecoder {
    pub fn new(max_response_size: usize) -> Self {
        debug!(max_response_size, "creating new HTTP response decoder");
        Self {
            max_size: max_response_size,
            buf: Vec::new(),
            hdr_end: None,
            status: None,
            mode: None,
            decoded: Vec::new(),
            chunk_cursor: 0,
            cur_chunk_remaining: None,
            chunk_done: false,
            trailers_complete: false,
        }
    }

    #[expect(clippy::too_many_lines)]
    pub fn feed(&mut self, bytes: &[u8]) -> Result<Option<HttpResponse>, WasmError> {
        if self.buf.len() + bytes.len() > self.max_size {
            error!(
                current_size = self.buf.len(),
                new_bytes = bytes.len(),
                max_size = self.max_size,
                "HTTP response too large"
            );
            return Err(WasmError::IOError("HTTP response too large".into()));
        }
        self.buf.extend_from_slice(bytes);
        debug!(
            total_buffered = self.buf.len(),
            new_bytes = bytes.len(),
            "fed bytes to decoder"
        );

        // 1) Parse headers if not done
        if self.hdr_end.is_none() {
            if let Some(hend) = find_header_end(&self.buf) {
                self.hdr_end = Some(hend);
                let (status, mode) = parse_status_and_body_mode(&self.buf[..hend])?;
                info!(?status, ?mode, "parsed HTTP response status and body mode");
                self.status = Some(status);
                self.mode = Some(mode);

                // no-body statuses
                if (100..200).contains(&status) || status == 204 || status == 304 {
                    info!(?status, "no-body status code, completing immediately");
                    let (headers, _) = parse_headers(&self.buf[..hend])?;
                    return Ok(Some(HttpResponse {
                        status_code: status,
                        headers,
                        body: HttpBody::None,
                    }));
                }

                // chunked init
                if mode == BodyMode::Chunked {
                    self.chunk_cursor = hend + 4;
                    debug!(chunk_cursor = self.chunk_cursor, "initialized chunked mode");
                }
            } else {
                debug!("need more data for headers");
                // need more for headers
                return Ok(None);
            }
        }

        let hend = self.hdr_end.unwrap();
        let status = self.status.unwrap();
        match self.mode.unwrap() {
            BodyMode::None => {
                debug!("body mode None, completing");
                let (headers, _) = parse_headers(&self.buf[..hend])?;
                Ok(Some(HttpResponse {
                    status_code: status,
                    headers,
                    body: HttpBody::None,
                }))
            }
            BodyMode::Fixed(clen) => {
                let body = &self.buf[hend + 4..];
                if body.len() < clen {
                    debug!(
                        current = body.len(),
                        expected = clen,
                        "need more data for fixed-length body"
                    );
                    return Ok(None); // need more
                }
                if body.len() > clen {
                    error!(
                        current = body.len(),
                        expected = clen,
                        "body longer than Content-Length"
                    );
                    return Err(WasmError::IOError("Body longer than Content-Length".into()));
                }
                info!(body_length = clen, "fixed-length body complete");
                let (headers, content_type) = parse_headers(&self.buf[..hend])?;
                let http_body = classify_body(body, content_type.as_deref())?;
                Ok(Some(HttpResponse {
                    status_code: status,
                    headers,
                    body: http_body,
                }))
            }
            BodyMode::Chunked => {
                // parse chunk stream incrementally from chunk_cursor..buf.len()
                loop {
                    if self.chunk_done {
                        if self.trailers_complete {
                            info!(
                                decoded_body_size = self.decoded.len(),
                                "chunked transfer complete"
                            );
                            let (mut headers, content_type) = parse_headers(&self.buf[..hend])?;
                            // Optional: merge trailers into headers
                            if let Some(tr) = parse_trailers(&self.buf, self.chunk_cursor)? {
                                debug!(trailer_count = tr.len(), "parsed trailers");
                                headers.extend(tr);
                            }
                            let http_body = classify_body(&self.decoded, content_type.as_deref())?;
                            return Ok(Some(HttpResponse {
                                status_code: status,
                                headers,
                                body: http_body,
                            }));
                        }
                        // need CRLFCRLF after trailers
                        if let Some(off) = find_double_crlf_from(&self.buf, self.chunk_cursor) {
                            debug!(offset = off, "found trailers end marker");
                            self.chunk_cursor = off; // points after CRLFCRLF
                            self.trailers_complete = true;
                            // loop will complete at top
                            continue;
                        }
                        debug!("need more data for trailers completion");
                        return Ok(None);
                    }

                    // If mid-chunk, ensure we have enough to consume chunk data + CRLF
                    if let Some(rem) = self.cur_chunk_remaining {
                        let available = self.buf.len().saturating_sub(self.chunk_cursor);
                        if available < rem + 2 {
                            debug!(remaining = rem, available, "need more data for chunk body");
                            return Ok(None); // need more
                        }
                        let start = self.chunk_cursor;
                        let end = start + rem;
                        self.decoded.extend_from_slice(&self.buf[start..end]);
                        debug!(chunk_size = rem, "decoded chunk");

                        // expect CRLF
                        if &self.buf[end..end + 2] != b"\r\n" {
                            error!("chunk data not followed by CRLF");
                            return Err(WasmError::IOError(
                                "Chunk data not followed by CRLF".into(),
                            ));
                        }

                        self.chunk_cursor = end + 2;
                        self.cur_chunk_remaining = None; // finished current chunk, proceed to parse next size
                        continue;
                    }

                    // Need a chunk size line: "<hex>[;ext]*\r\n"
                    if let Some(crlf) = find_crlf_from(&self.buf, self.chunk_cursor) {
                        let line = &self.buf[self.chunk_cursor..crlf];
                        let size_hex = match split_on_semicolon(line) {
                            Some((hex, _ext)) => hex,
                            None => line,
                        };
                        let size_str = std::str::from_utf8(size_hex)
                            .map_err(|_| WasmError::IOError("Chunk size line not utf8".into()))?
                            .trim();
                        let size = usize::from_str_radix(size_str, 16).map_err(|e| {
                            error!(?e, size_str, "invalid chunk size");
                            WasmError::IOError("Invalid chunk size".into())
                        })?;

                        self.chunk_cursor = crlf + 2;
                        if size == 0 {
                            debug!("received terminal chunk (size 0)");
                            // terminal chunk: after this comes trailers then CRLFCRLF
                            self.chunk_done = true;
                            // fall through and let outer loop look for trailers completion
                        }
                        debug!(chunk_size = size, "parsed chunk size");
                        self.cur_chunk_remaining = Some(size);
                        // loop will try to consume chunk data next
                    } else {
                        debug!("need more data for chunk size line");
                        return Ok(None); // need more for size line
                    }
                }
            }
        }
    }
}

/* ------------ helpers ------------- */

fn parse_status_and_body_mode(hdr: &[u8]) -> Result<(u16, BodyMode), WasmError> {
    let s = std::str::from_utf8(hdr).map_err(|_| WasmError::IOError("hdr utf8".into()))?;
    let mut lines = s.lines();
    let status = lines
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse::<u16>().ok())
        .ok_or_else(|| WasmError::IOError("Bad status line".into()))?;

    let mut content_len: Option<usize> = None;
    let mut chunked = false;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some((n, v)) = line.split_once(':') {
            let n = n.trim().to_ascii_lowercase();
            let v = v.trim();
            if n == "content-length" {
                if let Ok(n) = v.parse::<usize>() {
                    content_len = Some(n);
                }
            } else if n == "transfer-encoding" && v.to_ascii_lowercase().contains("chunked") {
                chunked = true;
            }
        }
    }

    let mode = if (100..200).contains(&status) || status == 204 || status == 304 {
        BodyMode::None
    } else if chunked {
        BodyMode::Chunked
    } else if let Some(cl) = content_len {
        BodyMode::Fixed(cl)
    } else {
        // Over WebSockets, we cannot infer by connection close => reject
        warn!("HTTP response has no Content-Length or Transfer-Encoding: chunked");
        return Err(WasmError::IOError(
            "No Content-Length or chunked; unsupported over WS".into(),
        ));
    };

    Ok((status, mode))
}

#[expect(clippy::type_complexity)]
fn parse_headers(hdr: &[u8]) -> Result<(Vec<(String, String)>, Option<String>), WasmError> {
    let s = std::str::from_utf8(hdr).map_err(|_| WasmError::IOError("hdr utf8".into()))?;
    let mut lines = s.lines();
    let _status_line = lines
        .next()
        .ok_or_else(|| WasmError::IOError("no status line".into()))?;
    let mut headers = Vec::new();
    let mut content_type = None;

    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some((n, v)) = line.split_once(':') {
            let n_t = n.trim().to_string();
            let v_t = v.trim().to_string();
            if n_t.eq_ignore_ascii_case("content-type") {
                content_type = Some(v_t.clone());
            }
            headers.push((n_t, v_t));
        }
    }
    Ok((headers, content_type))
}

fn parse_trailers(buf: &[u8], from: usize) -> Result<Option<Vec<(String, String)>>, WasmError> {
    if let Some(end) = find_double_crlf_from(buf, from) {
        let trailer_section = &buf[from..end - 4]; // exclude the CRLFCRLF
        if trailer_section.is_empty() {
            return Ok(None);
        }
        let s = std::str::from_utf8(trailer_section)
            .map_err(|_| WasmError::IOError("trailer utf8".into()))?;
        let mut trailers = Vec::new();
        for line in s.lines() {
            if line.is_empty() {
                continue;
            }
            if let Some((n, v)) = line.split_once(':') {
                trailers.push((n.trim().to_string(), v.trim().to_string()));
            }
        }
        Ok(Some(trailers))
    } else {
        Ok(None)
    }
}

fn classify_body(body_bytes: &[u8], content_type: Option<&str>) -> Result<HttpBody, WasmError> {
    if let Some(ct) = content_type {
        if ct.contains("application/soap+xml")
            || ct.contains("application/octet-stream")
            || ct.contains("multipart/encrypted")
        {
            std::str::from_utf8(body_bytes).map_or_else(
                |_| Ok(HttpBody::Encrypted(body_bytes.to_vec())),
                |text| Ok(HttpBody::Text(text.to_string())),
            )
        } else {
            let text = std::str::from_utf8(body_bytes).map_err(|_| {
                WasmError::IOError("Body not valid UTF-8 for text content-type".into())
            })?;
            Ok(HttpBody::Text(text.to_string()))
        }
    } else if let Ok(text) = std::str::from_utf8(body_bytes) {
        Ok(HttpBody::Text(text.to_string()))
    } else {
        Ok(HttpBody::Encrypted(body_bytes.to_vec()))
    }
}

fn find_crlf_from(buf: &[u8], from: usize) -> Option<usize> {
    buf[from..]
        .windows(2)
        .position(|w| w == b"\r\n")
        .map(|p| p + from)
}

fn find_double_crlf_from(buf: &[u8], from: usize) -> Option<usize> {
    buf[from..]
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + from + 4)
}

fn split_on_semicolon(line: &[u8]) -> Option<(&[u8], &[u8])> {
    line.iter()
        .position(|&b| b == b';')
        .map(|i| (&line[..i], &line[i + 1..]))
}
