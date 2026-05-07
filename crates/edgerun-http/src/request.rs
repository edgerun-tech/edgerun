//! Protocol-agnostic HTTP request.

use crate::header::HeaderMap;
use crate::method::Method;
use crate::middleware::Extensions;
use crate::uri::{Scheme, Uri};
use crate::Result;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use edgerun_protocols::http::{HttpMessageError, HttpRequest};

/// An HTTP request, protocol-agnostic.
///
/// Used by [`crate::Handler`] across HTTP/1.1, HTTP/2, and HTTP/3 servers.
#[derive(Clone)]
pub struct Request {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
    extensions: Extensions,
}

impl Request {
    /// Create a new request builder.
    pub fn builder() -> RequestBuilder {
        RequestBuilder::new()
    }

    /// Create a request with explicit fields (used internally by servers).
    pub(crate) fn new(method: Method, uri: Uri, headers: HeaderMap, body: Option<Vec<u8>>) -> Self {
        Self {
            method,
            uri,
            headers,
            body,
            extensions: Extensions::new(),
        }
    }

    /// Parse a request from raw HTTP/1.x bytes.
    pub fn from_http(raw: &str) -> Result<Self> {
        let request = HttpRequest::from_http(raw).map_err(map_http_message_error)?;
        Ok(Self {
            method: request.method().clone(),
            uri: request.uri().clone(),
            headers: request.headers().clone(),
            body: request.into_body(),
            extensions: Extensions::new(),
        })
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
    pub fn body_as_str(&self) -> Option<&str> {
        self.body
            .as_deref()
            .and_then(|body| core::str::from_utf8(body).ok())
    }
    pub fn into_body(self) -> Option<Vec<u8>> {
        self.body
    }

    /// Access the extensions map — used by middleware to pass data between layers.
    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    /// Mutate the extensions map — insert data for downstream middleware/handler.
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    /// Serialize the request as raw HTTP/1.1 bytes.
    pub fn to_http_bytes(&self) -> Vec<u8> {
        HttpRequest::new(
            self.method.clone(),
            self.uri.clone(),
            self.headers.clone(),
            self.body.clone(),
        )
        .to_http_bytes()
    }
}

fn map_http_message_error(err: HttpMessageError) -> crate::Error {
    match err {
        HttpMessageError::InvalidRequest(value) => crate::Error::InvalidRequest(value),
        HttpMessageError::InvalidResponse(value) => crate::Error::InvalidResponse(value),
        HttpMessageError::InvalidUri(value) => crate::Error::InvalidUri(value),
        HttpMessageError::InvalidStatusCode(value) => crate::Error::InvalidStatusCode(value),
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.method.as_str(), self.uri.request_target())
    }
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("method", &self.method)
            .field("uri", &self.uri)
            .field("headers", &self.headers)
            .field("body_len", &self.body.as_ref().map(|b| b.len()))
            .field("extensions", &self.extensions)
            .finish()
    }
}

/// Builder for [`Request`].
#[derive(Clone)]
pub struct RequestBuilder {
    method: Method,
    uri: Option<String>,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
    extensions: Extensions,
}

impl RequestBuilder {
    pub fn new() -> Self {
        Self {
            method: Method::GET,
            uri: None,
            headers: HeaderMap::new(),
            body: None,
            extensions: Extensions::new(),
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
        let _ = self.headers.insert(name, value);
        self
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        for (name, value) in headers.iter() {
            let _ = self.headers.insert(name.as_str(), value.as_str());
        }
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

    /// Attach an extension value to the request.
    pub fn extension<T: Send + 'static>(mut self, value: T) -> Self {
        self.extensions.insert(value);
        self
    }

    pub fn build(self) -> Result<Request> {
        let uri_str = self
            .uri
            .ok_or_else(|| crate::Error::InvalidRequest("No URI specified".to_string()))?;
        let uri = Uri::parse(&uri_str).map_err(crate::Error::InvalidUri)?;

        let mut headers = self.headers;
        if !headers.contains_key("Host") {
            if let Some(host) = uri.host() {
                let host_header = host_header_value(&uri, host);
                let _ = headers.insert("Host", &host_header);
            }
        }
        if !headers.contains_key("Connection") {
            let _ = headers.insert("Connection", "keep-alive");
        }
        if !headers.contains_key("User-Agent") {
            let _ = headers.insert("User-Agent", "edgerun-browser/0.1");
        }
        if let Some(ref body) = self.body {
            headers.remove("Content-Length");
            let _ = headers.insert("Content-Length", &body.len().to_string());
        }

        Ok(Request {
            method: self.method,
            uri,
            headers,
            body: self.body,
            extensions: self.extensions,
        })
    }
}

fn host_header_value(uri: &Uri, host: &str) -> String {
    let host = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    };

    match (uri.scheme(), uri.port()) {
        (Scheme::Http, Some(80)) | (Scheme::Https, Some(443)) | (_, None) => host,
        (_, Some(port)) => format!("{host}:{port}"),
    }
}

impl Default for RequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_replaces_existing_content_length_for_body() {
        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/acme/new-account")
            .header("Content-Length", "999")
            .body(b"{}".to_vec())
            .build()
            .unwrap();

        let lengths = request.headers().get_all("Content-Length");
        assert_eq!(lengths.len(), 1);
        assert_eq!(lengths[0].as_str(), "2");

        let raw = String::from_utf8(request.to_http_bytes()).unwrap();
        assert_eq!(raw.matches("Content-Length:").count(), 1);
    }
}
