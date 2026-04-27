//! Protocol-agnostic HTTP response.

#[cfg(target_os = "none")]
use crate::prelude::v1::*;

use crate::header::HeaderMap;
use crate::status::StatusCode;
use std::fmt;

/// An HTTP response, protocol-agnostic.
///
/// Returned by [`crate::Handler`] implementations across HTTP/1.1, HTTP/2,
/// and HTTP/3 servers. Also used by all three client implementations.
#[derive(Debug, Clone)]
pub struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
    trailers: HeaderMap,
    streaming: bool,
}

impl Response {
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

    /// Create a streaming response with chunked transfer encoding.
    pub fn streaming(status: StatusCode) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            body: Vec::new(),
            trailers: HeaderMap::new(),
            streaming: true,
        }
    }

    /// Add a chunk to a streaming response body.
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

    pub fn from_http(raw: &str) -> crate::Result<Self> {
        let line_end = raw
            .find("\r\n")
            .ok_or_else(|| crate::Error::InvalidResponse("No status line".to_string()))?;
        let status_line = &raw[..line_end];
        let mut parts = status_line.splitn(3, ' ');
        let version = parts.next().unwrap_or_default();
        if !version.starts_with("HTTP/") {
            return Err(crate::Error::InvalidResponse(
                "Invalid status line".to_string(),
            ));
        }
        let status = parts
            .next()
            .ok_or_else(|| crate::Error::InvalidResponse("Missing status code".to_string()))?
            .parse::<u16>()
            .map_err(|_| crate::Error::InvalidResponse("Invalid status code".to_string()))?;
        let status =
            StatusCode::new(status).map_err(|_| crate::Error::InvalidStatusCode(status))?;

        let header_start = line_end + 2;
        let terminator = raw.find("\r\n\r\n").ok_or_else(|| {
            crate::Error::InvalidResponse("Missing header terminator".to_string())
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
        } else if headers
            .get("transfer-encoding")
            .map(|v| v.as_str().to_ascii_lowercase().contains("chunked"))
            .unwrap_or(false)
        {
            parse_chunked_body(raw_body, &mut trailers)?
        } else if let Some(cl) = headers.get("content-length") {
            let len = cl
                .as_str()
                .parse::<usize>()
                .map_err(|_| crate::Error::InvalidResponse("Invalid Content-Length".to_string()))?;
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

fn parse_chunked_body(raw: &str, trailers: &mut HeaderMap) -> crate::Result<Vec<u8>> {
    let mut pos = 0;
    let mut body = Vec::new();
    while pos < raw.len() {
        let line_end = raw[pos..]
            .find("\r\n")
            .ok_or_else(|| crate::Error::InvalidResponse("Invalid chunk".to_string()))?
            + pos;
        let size_text = raw[pos..line_end].split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|_| crate::Error::InvalidResponse("Invalid chunk size".to_string()))?;
        pos = line_end + 2;
        if size == 0 {
            if let Some(end) = raw[pos..].find("\r\n\r\n") {
                for line in raw[pos..pos + end].lines() {
                    if let Some(colon) = line.find(':') {
                        let _ = trailers.insert(line[..colon].trim(), line[colon + 1..].trim());
                    }
                }
            }
            break;
        }
        if pos + size > raw.len() {
            return Err(crate::Error::InvalidResponse("Truncated chunk".to_string()));
        }
        body.extend_from_slice(&raw.as_bytes()[pos..pos + size]);
        pos += size;
        if raw.get(pos..pos + 2) != Some("\r\n") {
            return Err(crate::Error::InvalidResponse(
                "Invalid chunk terminator".to_string(),
            ));
        }
        pos += 2;
    }
    Ok(body)
}
