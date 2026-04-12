//! Protocol-agnostic HTTP request.

use crate::header::HeaderMap;
use crate::method::Method;
use crate::uri::Uri;
use crate::Result;
use std::fmt;

/// An HTTP request, protocol-agnostic.
///
/// Used by [`crate::Handler`] across HTTP/1.1, HTTP/2, and HTTP/3 servers.
#[derive(Debug, Clone)]
pub struct Request {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
}

impl Request {
    /// Create a new request builder.
    pub fn builder() -> RequestBuilder {
        RequestBuilder::new()
    }

    /// Create a request with explicit fields (used internally by servers).
    pub(crate) fn new(method: Method, uri: Uri, headers: HeaderMap, body: Option<Vec<u8>>) -> Self {
        Self { method, uri, headers, body }
    }

    /// Parse a request from raw HTTP/1.x bytes.
    pub fn from_http(raw: &str) -> Result<Self> {
        let line_end = raw.find("\r\n").ok_or_else(|| {
            crate::Error::InvalidRequest("No request line".to_string())
        })?;

        let request_line = &raw[..line_end];
        let mut parts = request_line.splitn(3, ' ');
        let method_str = parts.next().ok_or_else(|| {
            crate::Error::InvalidRequest("Empty request line".to_string())
        })?;
        let target = parts.next().ok_or_else(|| {
            crate::Error::InvalidRequest("No request target".to_string())
        })?;

        let method = method_str.parse().map_err(|e: String| crate::Error::InvalidRequest(e))?;

        let header_end = raw[line_end..].find("\r\n\r\n")
            .map(|i| line_end + i)
            .unwrap_or(raw.len());

        let header_block = &raw[line_end + 2..header_end];
        let mut headers = HeaderMap::new();
        for line in header_block.lines() {
            if let Some(colon) = line.find(':') {
                let name = line[..colon].trim();
                let value = line[colon + 1..].trim();
                if !name.is_empty() {
                    let _ = headers.insert(name, value);
                }
            }
        }

        let uri = if target.starts_with("http://") || target.starts_with("https://") {
            Uri::parse(target).map_err(|e| crate::Error::InvalidUri(e.to_string()))?
        } else {
            let host = headers.get("Host").map(|v| v.as_str()).unwrap_or("localhost");
            let uri_str = if target.starts_with('/') {
                format!("http://{}{}", host, target)
            } else {
                format!("http://{}/{}", host, target)
            };
            Uri::parse(&uri_str).map_err(|e| crate::Error::InvalidUri(e.to_string()))?
        };

        let body = if header_end + 4 < raw.len() {
            let body_str = &raw[header_end + 4..];
            if let Some(cl) = headers.get("content-length") {
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

        Ok(Self { method, uri, headers, body })
    }

    pub fn method(&self) -> &Method { &self.method }
    pub fn uri(&self) -> &Uri { &self.uri }
    pub fn headers(&self) -> &HeaderMap { &self.headers }
    pub fn headers_mut(&mut self) -> &mut HeaderMap { &mut self.headers }
    pub fn body(&self) -> Option<&[u8]> { self.body.as_deref() }
    pub fn into_body(self) -> Option<Vec<u8>> { self.body }

    /// Serialize the request as raw HTTP/1.1 bytes.
    pub fn to_http_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.method.as_str().as_bytes());
        buf.push(b' ');
        buf.extend_from_slice(self.uri.request_target().as_bytes());
        buf.extend_from_slice(b" HTTP/1.1\r\n");

        for (name, value) in self.headers.iter() {
            buf.extend_from_slice(name.as_str().as_bytes());
            buf.push(b':');
            buf.push(b' ');
            buf.extend_from_slice(value.as_str().as_bytes());
            buf.extend_from_slice(b"\r\n");
        }

        if let Some(ref body) = self.body {
            let cl = body.len().to_string();
            buf.extend_from_slice(b"Content-Length: ");
            buf.extend_from_slice(cl.as_bytes());
            buf.extend_from_slice(b"\r\n");
        } else {
            buf.extend_from_slice(b"Content-Length: 0\r\n");
        }

        buf.extend_from_slice(b"\r\n");
        if let Some(ref body) = self.body {
            buf.extend_from_slice(body);
        }
        buf
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.method.as_str(), self.uri.request_target())
    }
}

/// Builder for [`Request`].
#[derive(Debug, Clone)]
pub struct RequestBuilder {
    method: Method,
    uri: Option<String>,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
}

impl RequestBuilder {
    pub fn new() -> Self {
        Self { method: Method::GET, uri: None, headers: HeaderMap::new(), body: None }
    }

    pub fn method(mut self, method: Method) -> Self { self.method = method; self }
    pub fn uri(mut self, uri: impl AsRef<str>) -> Self { self.uri = Some(uri.as_ref().to_string()); self }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        let _ = self.headers.insert(name, value);
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn json_body(mut self, json: &str) -> Self {
        let _ = self.headers.insert("Content-Type", "application/json");
        self.body = Some(json.as_bytes().to_vec());
        self
    }

    pub fn build(self) -> Result<Request> {
        let uri_str = self.uri.ok_or_else(|| {
            crate::Error::InvalidRequest("No URI specified".to_string())
        })?;
        let uri = Uri::parse(&uri_str).map_err(|e| crate::Error::InvalidUri(e.to_string()))?;

        let mut headers = self.headers;
        if let Some(ref body) = self.body {
            let _ = headers.insert("Content-Length", &body.len().to_string());
        }

        Ok(Request::new(self.method, uri, headers, self.body))
    }
}

impl Default for RequestBuilder {
    fn default() -> Self { Self::new() }
}
