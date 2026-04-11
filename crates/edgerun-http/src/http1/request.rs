//! HTTP/1.x request types

use crate::header::HeaderMap;
use crate::method::Method;
use crate::uri::Uri;
use crate::Result;
use std::fmt;

use super::version::HttpVersion;

/// HTTP request
#[derive(Debug, Clone)]
pub struct Request {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
    version: HttpVersion,
}

impl Request {
    /// Create a new request builder
    pub fn builder() -> RequestBuilder {
        RequestBuilder::new()
    }

    /// Parse an HTTP/1.x request from a raw string.
    ///
    /// Handles the request line, headers, and body based on Content-Length.
    /// Supports all request target forms: origin-form, absolute-form,
    /// authority-form, and asterisk-form.
    ///
    /// Supports both HTTP/1.0 and HTTP/1.1.
    pub fn from_http(raw: &str) -> Result<Self> {
        // Find the end of the request line
        let line_end = raw.find("\r\n").ok_or_else(|| {
            crate::Error::InvalidRequest("No request line".to_string())
        })?;

        // Parse request line: METHOD SP REQUEST-TARGET SP HTTP-VERSION
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

        let method: Method = method_str.parse().map_err(crate::Error::InvalidRequest)?;
        let uri = Uri::parse(target).map_err(crate::Error::InvalidRequest)?;
        let version = HttpVersion::from_str(version_str)?;

        // Parse headers and body
        let after_line = &raw[line_end + 2..];
        let (headers, body) = Self::parse_headers_and_body(after_line)?;

        Ok(Request {
            method,
            uri,
            headers,
            body: Some(body).filter(|b| !b.is_empty()),
            version,
        })
    }

    /// Parse headers (until blank line) and body (based on Content-Length or Transfer-Encoding).
    fn parse_headers_and_body(raw: &str) -> Result<(HeaderMap, Vec<u8>)> {
        let mut headers = HeaderMap::new();
        let mut pos = 0;
        let bytes = raw.as_bytes();

        // Parse headers until blank line
        while pos < bytes.len() {
            let line_end = bytes[pos..]
                .windows(2)
                .position(|w| w == b"\r\n")
                .map(|i| pos + i);

            match line_end {
                Some(end) if end == pos => {
                    pos = end + 2;
                    break;
                }
                Some(end) => {
                    let line = std::str::from_utf8(&bytes[pos..end])
                        .map_err(|_| crate::Error::InvalidRequest("Invalid UTF-8 in headers".to_string()))?;

                    if let Some(colon) = line.find(':') {
                        let name = line[..colon].trim();
                        let value = line[colon + 1..].trim();
                        if !name.is_empty() {
                            let _ = headers.insert(name, value); // skip invalid headers
                        }
                    }
                    pos = end + 2;
                }
                None => {
                    break;
                }
            }
        }

        let remaining = &bytes[pos..];

        // Determine body handling
        let transfer_encoding = headers
            .get("transfer-encoding")
            .map(|v| v.as_str().to_lowercase());
        let is_chunked = transfer_encoding.as_deref().map_or(false, |v| v.contains("chunked"));

        let body = if is_chunked {
            Self::parse_chunked_body(remaining)?
        } else if let Some(cl) = headers.get("content-length") {
            let len = cl.as_str()
                .parse::<usize>()
                .ok()
                .unwrap_or(remaining.len())
                .min(remaining.len());
            remaining[..len].to_vec()
        } else {
            remaining.to_vec()
        };

        Ok((headers, body))
    }

    /// Parse a chunked transfer-encoded body (RFC 9112 §7.1).
    fn parse_chunked_body(mut data: &[u8]) -> Result<Vec<u8>> {
        let mut body = Vec::new();

        loop {
            let crlf = Self::find_crlf(data, 0).ok_or_else(|| {
                crate::Error::InvalidRequest("Incomplete chunked body".to_string())
            })?;

            let size_hex = std::str::from_utf8(&data[..crlf])
                .map_err(|_| crate::Error::InvalidRequest("Invalid chunk size".to_string()))?;

            let size_str = size_hex.split(';').next().unwrap_or(size_hex).trim();
            let chunk_size = usize::from_str_radix(size_str, 16)
                .map_err(|_| crate::Error::InvalidRequest("Invalid chunk size".to_string()))?;

            data = &data[crlf + 2..];

            if chunk_size == 0 {
                break;
            }

            if data.len() < chunk_size {
                return Err(crate::Error::InvalidRequest(
                    "Incomplete chunked body".to_string(),
                ));
            }

            body.extend_from_slice(&data[..chunk_size]);
            data = &data[chunk_size..];

            if data.len() < 2 || data[0] != b'\r' || data[1] != b'\n' {
                return Err(crate::Error::InvalidRequest(
                    "Missing CRLF after chunk".to_string(),
                ));
            }
            data = &data[2..];
        }

        Ok(body)
    }

    /// Find CRLF starting at position `pos`.
    fn find_crlf(data: &[u8], pos: usize) -> Option<usize> {
        data[pos..]
            .windows(2)
            .position(|w| w == b"\r\n")
            .map(|i| pos + i)
    }

    /// Get the HTTP version
    pub fn version(&self) -> &HttpVersion {
        &self.version
    }

    /// Get the method
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Get the URI
    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    /// Get the headers
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Get the body
    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }

    /// Get the body as a string (if UTF-8)
    pub fn body_as_str(&self) -> Result<&str> {
        match &self.body {
            Some(body) => std::str::from_utf8(body)
                .map_err(|e| crate::Error::ProtocolError(format!("Invalid UTF-8: {}", e))),
            None => Ok(""),
        }
    }

    /// Format the request as an HTTP string
    pub fn to_http_string(&self) -> String {
        String::from_utf8_lossy(&self.to_http_bytes()).into_owned()
    }

    /// Format the request as HTTP bytes (binary-safe).
    pub fn to_http_bytes(&self) -> Vec<u8> {
        let mut request = format!(
            "{} {} {}\r\n",
            self.method.as_str(),
            self.uri.request_target(),
            self.version.as_str()
        ).into_bytes();

        // Add Host header if not present
        if !self.headers.contains_key("Host") {
            if let Some(host) = self.uri.host() {
                if let Some(port) = self.uri.port() {
                    let default_port = if self.uri.is_https() { 443 } else { 80 };
                    if port != default_port {
                        let _ = std::io::Write::write_fmt(&mut request, format_args!("Host: {}:{}\r\n", host, port));
                    } else {
                        let _ = std::io::Write::write_fmt(&mut request, format_args!("Host: {}\r\n", host));
                    }
                }
            }
        }

        // Add Content-Length if body present and not set
        if self.body.is_some() && !self.headers.contains_key("Content-Length") {
            let len = self.body.as_ref().map(|b| b.len()).unwrap_or(0);
            let _ = std::io::Write::write_fmt(&mut request, format_args!("Content-Length: {}\r\n", len));
        }

        // Add Connection header based on version default if not present
        if !self.headers.contains_key("Connection") {
            match self.version.default_connection_behavior() {
                super::version::ConnectionDefault::Close => {
                    request.extend_from_slice(b"Connection: close\r\n");
                }
                super::version::ConnectionDefault::KeepAlive => {
                    request.extend_from_slice(b"Connection: keep-alive\r\n");
                }
            }
        }

        request.extend_from_slice(self.headers.to_http_string().as_bytes());
        request.extend_from_slice(b"\r\n");

        // Add body
        if let Some(body) = &self.body {
            request.extend_from_slice(body);
        }

        request
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}",
            self.method,
            self.uri
        )
    }
}

/// Request builder
#[derive(Debug, Clone)]
pub struct RequestBuilder {
    method: Method,
    uri: Option<Uri>,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
    version: HttpVersion,
}

impl RequestBuilder {
    /// Create a new request builder
    pub fn new() -> Self {
        RequestBuilder {
            method: Method::GET,
            uri: None,
            headers: HeaderMap::new(),
            body: None,
            version: HttpVersion::default(),
        }
    }

    /// Set the HTTP method
    pub fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    /// Set the HTTP version
    pub fn version(mut self, version: HttpVersion) -> Self {
        self.version = version;
        self
    }

    /// Set the URI
    pub fn uri(mut self, uri: impl AsRef<str>) -> Self {
        self.uri = Some(Uri::parse(uri.as_ref()).expect("Invalid URI"));
        self
    }

    /// Add a header. Panics if the name or value is invalid.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name, value).expect("Invalid header name or value");
        self
    }

    /// Set the body
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Set the body as JSON (sets Content-Type header)
    pub fn json_body(mut self, json: &str) -> Self {
        let _ = self.headers.insert("Content-Type", "application/json");
        self.body = Some(json.as_bytes().to_vec());
        self
    }

    /// Build the request
    pub fn build(self) -> Result<Request> {
        let uri = self
            .uri
            .ok_or_else(|| crate::Error::ProtocolError("URI is required".to_string()))?;

        Ok(Request {
            method: self.method,
            uri,
            headers: self.headers,
            body: self.body,
            version: self.version,
        })
    }
}

impl Default for RequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}
