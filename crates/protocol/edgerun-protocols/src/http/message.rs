//! HTTP/1 message parsing and serialization.

use super::chunked::{has_chunked_transfer_coding, parse_body, parse_body_with_trailers};
use super::{HeaderMap, Method, StatusCode, Uri};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMessageError {
    InvalidRequest(String),
    InvalidResponse(String),
    InvalidUri(String),
    InvalidStatusCode(u16),
}

impl fmt::Display for HttpMessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpMessageError::InvalidRequest(value) => write!(f, "Invalid request: {value}"),
            HttpMessageError::InvalidResponse(value) => write!(f, "Invalid response: {value}"),
            HttpMessageError::InvalidUri(value) => write!(f, "Invalid URI: {value}"),
            HttpMessageError::InvalidStatusCode(value) => {
                write!(f, "Invalid status code: {value}")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
}

impl HttpRequest {
    pub fn new(method: Method, uri: Uri, headers: HeaderMap, body: Option<Vec<u8>>) -> Self {
        Self {
            method,
            uri,
            headers,
            body,
        }
    }

    pub fn from_http(raw: &str) -> Result<Self, HttpMessageError> {
        let line_end = raw
            .find("\r\n")
            .ok_or_else(|| HttpMessageError::InvalidRequest("No request line".to_string()))?;

        let request_line = &raw[..line_end];
        let mut parts = request_line.splitn(3, ' ');
        let method_str = parts
            .next()
            .ok_or_else(|| HttpMessageError::InvalidRequest("Empty request line".to_string()))?;
        let target = parts
            .next()
            .ok_or_else(|| HttpMessageError::InvalidRequest("No request target".to_string()))?;
        let version = parts
            .next()
            .ok_or_else(|| HttpMessageError::InvalidRequest("No HTTP version".to_string()))?;
        if !version.starts_with("HTTP/") {
            return Err(HttpMessageError::InvalidRequest(
                "Invalid HTTP version".to_string(),
            ));
        }

        let method = method_str
            .parse()
            .map_err(HttpMessageError::InvalidRequest)?;

        let header_start = line_end + 2;
        let terminator = raw.find("\r\n\r\n").ok_or_else(|| {
            HttpMessageError::InvalidRequest("Missing header terminator".to_string())
        })?;
        let header_end = terminator.max(header_start);

        let mut headers = HeaderMap::new();
        for line in raw[header_start..header_end].lines() {
            if let Some(colon) = line.find(':') {
                let name = line[..colon].trim();
                let value = line[colon + 1..].trim();
                if !name.is_empty() {
                    let _ = headers.insert(name, value);
                }
            }
        }

        let uri = if target.starts_with("http://") || target.starts_with("https://") {
            Uri::parse(target).map_err(HttpMessageError::InvalidUri)?
        } else if target == "*" {
            let host = headers
                .get("Host")
                .map(|v| v.as_str())
                .unwrap_or("localhost");
            Uri::parse(&format!("http://{host}/")).map_err(HttpMessageError::InvalidUri)?
        } else {
            let host = headers
                .get("Host")
                .map(|v| v.as_str())
                .unwrap_or("localhost");
            let uri_str = if target.starts_with('/') {
                format!("http://{}{}", host, target)
            } else {
                format!("http://{}/{}", host, target)
            };
            Uri::parse(&uri_str).map_err(HttpMessageError::InvalidUri)?
        };

        let body_start = terminator + 4;
        let body = if body_start < raw.len() {
            let body_str = &raw[body_start..];
            if has_chunked_transfer_coding(&headers) {
                Some(
                    parse_body(body_str.as_bytes())
                        .map_err(|err| HttpMessageError::InvalidRequest(err.to_string()))?,
                )
            } else if let Some(cl) = headers.get("content-length") {
                if let Ok(len) = cl.as_str().parse::<usize>() {
                    Some(body_str[..len.min(body_str.len())].as_bytes().to_vec())
                } else {
                    Some(body_str.as_bytes().to_vec())
                }
            } else {
                Some(body_str.as_bytes().to_vec())
            }
        } else {
            None
        };

        Ok(Self::new(method, uri, headers, body))
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }

    pub fn into_body(self) -> Option<Vec<u8>> {
        self.body
    }

    pub fn to_http_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.method.as_str().as_bytes());
        buf.push(b' ');
        buf.extend_from_slice(self.uri.request_target().as_bytes());
        buf.extend_from_slice(b" HTTP/1.1\r\n");

        let mut has_content_length = false;
        for (name, value) in self.headers.iter() {
            if name.as_str().eq_ignore_ascii_case("content-length") {
                has_content_length = true;
            }
            buf.extend_from_slice(name.as_str().as_bytes());
            buf.extend_from_slice(b": ");
            buf.extend_from_slice(value.as_str().as_bytes());
            buf.extend_from_slice(b"\r\n");
        }

        if !has_content_length {
            if let Some(ref body) = self.body {
                let cl = body.len().to_string();
                buf.extend_from_slice(b"Content-Length: ");
                buf.extend_from_slice(cl.as_bytes());
                buf.extend_from_slice(b"\r\n");
            }
        }

        buf.extend_from_slice(b"\r\n");
        if let Some(ref body) = self.body {
            buf.extend_from_slice(body);
        }
        buf
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
    trailers: HeaderMap,
    streaming: bool,
}

impl HttpResponse {
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            body: Vec::new(),
            trailers: HeaderMap::new(),
            streaming: false,
        }
    }

    pub fn from_parts(status: StatusCode, headers: HeaderMap, body: Vec<u8>) -> Self {
        Self {
            status,
            headers,
            body,
            trailers: HeaderMap::new(),
            streaming: false,
        }
    }

    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        let body = body.into();
        if !self.streaming && !self.headers.contains_key("Content-Length") {
            let _ = self
                .headers
                .insert("Content-Length", &body.len().to_string());
        }
        self.body = body;
        self
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn body_as_string(&self) -> Option<String> {
        String::from_utf8(self.body.clone()).ok()
    }

    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    pub fn trailers(&self) -> &HeaderMap {
        &self.trailers
    }

    pub fn is_streaming(&self) -> bool {
        self.streaming
    }

    pub fn streaming(status: StatusCode) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            body: Vec::new(),
            trailers: HeaderMap::new(),
            streaming: true,
        }
    }

    pub fn add_chunk(&mut self, chunk: impl Into<Vec<u8>>) {
        let chunk = chunk.into();
        if self.streaming && !chunk.is_empty() {
            self.body.extend_from_slice(&chunk);
        }
    }

    pub fn text(status: StatusCode, body: &str) -> Self {
        Self::new(status)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_body(body)
    }

    pub fn json(status: StatusCode, body: &str) -> Self {
        Self::new(status)
            .with_header("Content-Type", "application/json")
            .with_body(body)
    }

    pub fn html(status: StatusCode, body: &str) -> Self {
        Self::new(status)
            .with_header("Content-Type", "text/html; charset=utf-8")
            .with_body(body)
    }

    pub fn not_found() -> Self {
        Self::text(StatusCode::new(404).unwrap(), "404 Not Found")
    }

    pub fn internal_error() -> Self {
        Self::text(StatusCode::new(500).unwrap(), "500 Internal Server Error")
    }

    pub fn internal_error_msg(msg: &str) -> Self {
        Self::text(StatusCode::new(500).unwrap(), msg)
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        let _ = self.headers.insert(name, value);
        self
    }

    pub fn from_http(raw: &str) -> Result<Self, HttpMessageError> {
        let line_end = raw
            .find("\r\n")
            .ok_or_else(|| HttpMessageError::InvalidResponse("No status line".to_string()))?;
        let status_line = &raw[..line_end];
        let mut parts = status_line.splitn(3, ' ');
        let version = parts.next().unwrap_or_default();
        if !version.starts_with("HTTP/") {
            return Err(HttpMessageError::InvalidResponse(
                "Invalid status line".to_string(),
            ));
        }
        let status = parts
            .next()
            .ok_or_else(|| HttpMessageError::InvalidResponse("Missing status code".to_string()))?
            .parse::<u16>()
            .map_err(|_| HttpMessageError::InvalidResponse("Invalid status code".to_string()))?;
        let status =
            StatusCode::new(status).map_err(|_| HttpMessageError::InvalidStatusCode(status))?;

        let header_start = line_end + 2;
        let terminator = raw.find("\r\n\r\n").ok_or_else(|| {
            HttpMessageError::InvalidResponse("Missing header terminator".to_string())
        })?;
        let header_end = terminator.max(header_start);
        let mut headers = HeaderMap::new();
        for line in raw[header_start..header_end].lines() {
            if let Some(colon) = line.find(':') {
                let name = line[..colon].trim();
                let value = line[colon + 1..].trim();
                if !name.is_empty() {
                    let _ = headers.insert(name, value);
                }
            }
        }

        let body_start = terminator + 4;
        let raw_body = raw.get(body_start..).unwrap_or_default();
        let mut trailers = HeaderMap::new();
        let body = if (100..200).contains(&status.as_u16())
            || status.as_u16() == 204
            || status.as_u16() == 304
        {
            Vec::new()
        } else if has_chunked_transfer_coding(&headers) {
            let (body, parsed_trailers) = parse_body_with_trailers(raw_body.as_bytes())
                .map_err(|err| HttpMessageError::InvalidResponse(err.to_string()))?;
            trailers = parsed_trailers;
            body
        } else if let Some(cl) = headers.get("content-length") {
            let len = cl.as_str().parse::<usize>().map_err(|_| {
                HttpMessageError::InvalidResponse("Invalid Content-Length".to_string())
            })?;
            raw_body.as_bytes()[..len.min(raw_body.len())].to_vec()
        } else {
            raw_body.as_bytes().to_vec()
        };

        Ok(Self {
            status,
            headers,
            body,
            trailers,
            streaming: false,
        })
    }
}
