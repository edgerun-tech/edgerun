//! HTTP request types

use crate::header::HeaderMap;
use crate::method::Method;
use crate::uri::Uri;
use crate::Result;
use std::fmt;

/// HTTP request
#[derive(Debug, Clone)]
pub struct Request {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
}

impl Request {
    /// Create a new request builder
    pub fn builder() -> RequestBuilder {
        RequestBuilder::new()
    }

    /// Get the method
    pub fn method(&self) -> Method {
        self.method
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
        let mut request = format!(
            "{} {} HTTP/1.1\r\n",
            self.method.as_str(),
            self.uri.request_target()
        );

        // Add Host header if not present
        if !self.headers.contains_key("Host") {
            if let Some(host) = self.uri.host() {
                if let Some(port) = self.uri.port() {
                    let default_port = if self.uri.is_https() { 443 } else { 80 };
                    if port != default_port {
                        request.push_str(&format!("Host: {}:{}\r\n", host, port));
                    } else {
                        request.push_str(&format!("Host: {}\r\n", host));
                    }
                }
            }
        }

        // Add Content-Length if body present and not set
        if self.body.is_some() && !self.headers.contains_key("Content-Length") {
            let len = self.body.as_ref().map(|b| b.len()).unwrap_or(0);
            request.push_str(&format!("Content-Length: {}\r\n", len));
        }

        // Add Connection header if not present
        if !self.headers.contains_key("Connection") {
            request.push_str("Connection: close\r\n");
        }

        request.push_str(&self.headers.to_http_string());
        request.push_str("\r\n");

        // Add body
        if let Some(body) = &self.body {
            request.push_str(&String::from_utf8_lossy(body));
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
}

impl RequestBuilder {
    /// Create a new request builder
    pub fn new() -> Self {
        RequestBuilder {
            method: Method::GET,
            uri: None,
            headers: HeaderMap::new(),
            body: None,
        }
    }

    /// Set the HTTP method
    pub fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    /// Set the URI
    pub fn uri(mut self, uri: impl AsRef<str>) -> Self {
        self.uri = Some(Uri::parse(uri.as_ref()).expect("Invalid URI"));
        self
    }

    /// Add a header
    pub fn header(
        mut self,
        name: impl Into<crate::HeaderName>,
        value: impl Into<crate::HeaderValue>,
    ) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Set the body
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Set the body as JSON (sets Content-Type header)
    pub fn json_body(mut self, json: &str) -> Self {
        self.headers.insert("Content-Type", "application/json");
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
        })
    }
}

impl Default for RequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}
