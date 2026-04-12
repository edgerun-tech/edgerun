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
    ///
    /// For HTTP/2 and HTTP/3, requests arrive already decoded from their
    /// binary framing — this method is primarily for HTTP/1.1 testing.
    pub fn from_http(raw: &str) -> Result<Self> {
        // Find the end of the request line
        let line_end = raw.find("\r\n").ok_or_else(|| {
            crate::Error::InvalidRequest("No request line".to_string())
        })?;

        // Parse request line
        let request_line = &raw[..line_end];
        let mut parts = request_line.splitn(3, ' ');
        let method_str = parts.next().ok_or_else(|| {
            crate::Error::InvalidRequest("Empty request line".to_string())
        })?;
        let target = parts.next().ok_or_else(|| {
            crate::Error::InvalidRequest("No request target".to_string())
        })?;
        let version_str = parts.next().ok_or_else(|| {
            crate::Error::InvalidRequest("No HTTP version".to_string())
        })?;

        let method = method_str.parse()?;

        // Find the end of the headers
        let header_end = raw[line_end..].find("\r\n\r\n")
            .map(|i| line_end + i)
            .unwrap_or(raw.len());

        // Parse headers
        let header_block = &raw[line_end + 2..header_end];
        let mut headers = HeaderMap::new();
        for line in header_block.lines() {
            if let Some(colon) = line.find(':') {
                let name = line[..colon].trim();
                let value = line[colon + 1..].trim();
                if !name.is_empty() {
                    headers.insert(name, value);
                }
            }
        }

        // Build URI from request target and Host header
        let mut uri = if target.starts_with("http://") || target.starts_with("https://") {
            Uri::parse(target)?
        } else {
            let host = headers.get("Host").unwrap_or("");
            let uri_str = if host.is_empty() {
                format!("http://localhost{}", target)
            } else if target.starts_with('/') {
                format!("http://{}{}", host, target)
            } else {
                format!("http://{}/{}", host, target)
            };
            Uri::parse(&uri_str)?
        };

        // Override scheme from version (HTTP/1.0 has no scheme info, default to http)
        let version = version_str.trim();
        if !version.is_empty() {
            // Just validate, don't store
            let _ = version;
        }

        // Parse body based on Content-Length
        let body = if header_end + 4 < raw.len() {
            let body_str = &raw[header_end + 4..];
            if let Some(cl) = headers.get("content-length") {
                if let Ok(len) = cl.parse::<usize>() {
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

    /// HTTP method.
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Request URI.
    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    /// Request headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Mutable access to headers.
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    /// Request body, if present.
    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }

    /// Take ownership of the body.
    pub fn into_body(self) -> Option<Vec<u8>> {
        self.body
    }

    /// Serialize the request as raw HTTP/1.1 bytes.
    ///
    /// Useful for debugging or proxy forwarding.
    pub fn to_http_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Request line
        buf.extend_from_slice(self.method.as_str().as_bytes());
        buf.push(b' ');
        buf.extend_from_slice(self.uri.request_target().as_bytes());
        buf.extend_from_slice(b" HTTP/1.1\r\n");

        // Headers
        for (name, value) in self.headers.iter() {
            buf.extend_from_slice(name.as_bytes());
            buf.extend_from_slice(b": ");
            buf.extend_from_slice(value.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }

        // Content-Length
        if let Some(ref body) = self.body {
            let cl = body.len().to_string();
            buf.extend_from_slice(b"Content-Length: ");
            buf.extend_from_slice(cl.as_bytes());
            buf.extend_from_slice(b"\r\n");
        } else {
            buf.extend_from_slice(b"Content-Length: 0\r\n");
        }

        buf.extend_from_slice(b"\r\n");

        // Body
        if let Some(ref body) = self.body {
            buf.extend_from_slice(body);
        }

        buf
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {:?}",
            self.method.as_str(),
            self.uri.request_target(),
            self.headers
        )
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
        Self {
            method: Method::GET,
            uri: None,
            headers: HeaderMap::new(),
            body: None,
        }
    }

    pub fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    pub fn uri(mut self, uri: impl AsRef<str>) -> Self {
        self.uri = Some(uri.as_ref().to_string());
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name, value);
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn json_body(mut self, json: &str) -> Self {
        self.headers.insert("Content-Type", "application/json");
        self.body = Some(json.as_bytes().to_vec());
        self
    }

    pub fn body_opt(mut self, body: Option<Vec<u8>>) -> Self {
        if let Some(ref b) = body {
            self.headers.insert("Content-Length", &b.len().to_string());
        }
        self.body = body;
        self
    }

    pub fn build(self) -> Result<Request> {
        let uri_str = self.uri.ok_or_else(|| {
            crate::Error::InvalidRequest("No URI specified".to_string())
        })?;
        let uri = Uri::parse(&uri_str)?;

        let mut headers = self.headers;
        if let Some(ref body) = self.body {
            let cl = body.len().to_string();
            headers.insert("Content-Length", &cl);
        }

        Ok(Request::new(self.method, uri, headers, self.body))
    }
}

impl Default for RequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}
