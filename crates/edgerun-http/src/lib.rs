//! A dependency-free HTTP client and type system built exclusively with Rust's standard library.
//!
//! # Features
//! - HTTP/1.1 request and response types
//! - All standard HTTP methods
//! - Header management
//! - URI parsing and validation
//! - Basic HTTP client with TCP connectivity
//!
//! # Example
//! ```no_run
//! use edgerun_http::{Client, Request, Method};
//!
//! let client = Client::new();
//! let request = Request::builder()
//!     .method(Method::GET)
//!     .uri("http://example.com/api/data")
//!     .header("Accept", "application/json")
//!     .build()
//!     .unwrap();
//!
//! let response = client.execute(&request).unwrap();
//! println!("Status: {}", response.status());
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod client;
pub mod header;
pub mod method;
pub mod request;
pub mod response;
pub mod status;
pub mod uri;

pub use client::Client;
pub use header::{HeaderMap, HeaderName, HeaderValue};
pub use method::Method;
pub use request::Request;
pub use response::Response;
pub use status::StatusCode;
pub use uri::Uri;

/// Result type for HTTP operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for HTTP operations
#[derive(Debug)]
pub enum Error {
    /// Invalid URI
    InvalidUri(String),
    /// Invalid header
    InvalidHeader(String),
    /// Network error
    Network(std::io::Error),
    /// Invalid HTTP method
    InvalidMethod(String),
    /// Invalid status code
    InvalidStatusCode(u16),
    /// Request timeout
    Timeout,
    /// HTTP protocol error
    ProtocolError(String),
    /// Invalid response
    InvalidResponse(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidUri(msg) => write!(f, "Invalid URI: {}", msg),
            Error::InvalidHeader(msg) => write!(f, "Invalid header: {}", msg),
            Error::Network(err) => write!(f, "Network error: {}", err),
            Error::InvalidMethod(msg) => write!(f, "Invalid method: {}", msg),
            Error::InvalidStatusCode(code) => {
                write!(f, "Invalid status code: {}", code)
            }
            Error::Timeout => write!(f, "Request timeout"),
            Error::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            Error::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Network(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Network(err)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Client, HeaderMap, Method, Request, Response, StatusCode, Uri};

    #[test]
    fn test_uri_parse_http() {
        let uri = Uri::parse("http://example.com").unwrap();
        assert_eq!(uri.host(), Some("example.com"));
        assert_eq!(uri.port(), Some(80));
        assert_eq!(uri.path(), "/");
        assert!(!uri.is_https());
    }

    #[test]
    fn test_uri_parse_https() {
        let uri = Uri::parse("https://example.com/api/data").unwrap();
        assert_eq!(uri.host(), Some("example.com"));
        assert_eq!(uri.port(), Some(443));
        assert_eq!(uri.path(), "/api/data");
        assert!(uri.is_https());
    }

    #[test]
    fn test_uri_parse_with_port() {
        let uri = Uri::parse("http://localhost:8080/api").unwrap();
        assert_eq!(uri.host(), Some("localhost"));
        assert_eq!(uri.port(), Some(8080));
        assert_eq!(uri.path(), "/api");
    }

    #[test]
    fn test_uri_parse_with_query() {
        let uri = Uri::parse("http://example.com/search?q=rust&lang=en").unwrap();
        assert_eq!(uri.path(), "/search");
        assert_eq!(uri.query(), Some("q=rust&lang=en"));
    }

    #[test]
    fn test_uri_parse_with_fragment() {
        let uri = Uri::parse("http://example.com/page#section1").unwrap();
        assert_eq!(uri.path(), "/page");
        assert_eq!(uri.fragment(), Some("section1"));
    }

    #[test]
    fn test_uri_parse_full() {
        let uri =
            Uri::parse("http://example.com:8080/api?key=value#frag").unwrap();
        assert_eq!(uri.host(), Some("example.com"));
        assert_eq!(uri.port(), Some(8080));
        assert_eq!(uri.path(), "/api");
        assert_eq!(uri.query(), Some("key=value"));
        assert_eq!(uri.fragment(), Some("frag"));
    }

    #[test]
    fn test_uri_request_target() {
        let uri = Uri::parse("http://example.com/api?q=test").unwrap();
        assert_eq!(uri.request_target(), "/api?q=test");

        let uri_no_query = Uri::parse("http://example.com/api").unwrap();
        assert_eq!(uri_no_query.request_target(), "/api");
    }

    #[test]
    fn test_method_display() {
        assert_eq!(Method::GET.to_string(), "GET");
        assert_eq!(Method::POST.to_string(), "POST");
        assert_eq!(Method::PUT.to_string(), "PUT");
        assert_eq!(Method::DELETE.to_string(), "DELETE");
    }

    #[test]
    fn test_method_from_str() {
        assert_eq!("GET".parse::<Method>().unwrap(), Method::GET);
        assert_eq!("post".parse::<Method>().unwrap(), Method::POST);
        assert!("INVALID".parse::<Method>().is_err());
    }

    #[test]
    fn test_method_has_body() {
        assert!(!Method::GET.has_body());
        assert!(Method::POST.has_body());
        assert!(Method::PUT.has_body());
        assert!(!Method::DELETE.has_body());
        assert!(Method::PATCH.has_body());
    }

    #[test]
    fn test_status_code_display() {
        let status = StatusCode::new(200).unwrap();
        assert_eq!(status.to_string(), "200 OK");
        assert_eq!(status.reason(), "OK");
    }

    #[test]
    fn test_status_code_categories() {
        assert!(StatusCode::new(200).unwrap().is_success());
        assert!(StatusCode::new(201).unwrap().is_success());
        assert!(StatusCode::new(301).unwrap().is_redirection());
        assert!(StatusCode::new(404).unwrap().is_client_error());
        assert!(StatusCode::new(500).unwrap().is_server_error());
        assert!(StatusCode::new(100).unwrap().is_informational());
    }

    #[test]
    fn test_status_code_constants() {
        assert_eq!(StatusCode::OK.as_u16(), 200);
        assert_eq!(StatusCode::NOT_FOUND.as_u16(), 404);
        assert_eq!(StatusCode::INTERNAL_SERVER_ERROR.as_u16(), 500);
    }

    #[test]
    fn test_status_code_invalid() {
        assert!(StatusCode::new(0).is_err());
        assert!(StatusCode::new(999).is_err());
    }

    #[test]
    fn test_header_map_insert_get() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json");
        headers.insert("Accept", "text/html");

        assert_eq!(
            headers.get("Content-Type").unwrap().as_str(),
            "application/json"
        );
        assert_eq!(
            headers.get("Accept").unwrap().as_str(),
            "text/html"
        );
    }

    #[test]
    fn test_header_map_case_insensitive() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json");
        assert_eq!(
            headers.get("Content-Type").unwrap().as_str(),
            "application/json"
        );
        assert_eq!(
            headers.get("CONTENT-TYPE").unwrap().as_str(),
            "application/json"
        );
    }

    #[test]
    fn test_header_map_get_all() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", "text/html");
        headers.insert("Accept", "application/json");

        let all = headers.get_all("Accept");
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_header_map_contains_key() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json");
        assert!(headers.contains_key("Content-Type"));
        assert!(!headers.contains_key("Accept"));
    }

    #[test]
    fn test_request_builder() {
        let request = Request::builder()
            .method(Method::GET)
            .uri("http://example.com/api")
            .header("Accept", "application/json")
            .build()
            .unwrap();

        assert_eq!(request.method(), Method::GET);
        assert_eq!(request.uri().host(), Some("example.com"));
        assert!(request.headers().contains_key("Accept"));
    }

    #[test]
    fn test_request_with_body() {
        let request = Request::builder()
            .method(Method::POST)
            .uri("http://example.com/api")
            .body(b"test data".to_vec())
            .build()
            .unwrap();

        assert_eq!(request.body(), Some(&b"test data"[..]));
    }

    #[test]
    fn test_request_json_body() {
        let request = Request::builder()
            .method(Method::POST)
            .uri("http://example.com/api")
            .json_body(r#"{"key":"value"}"#)
            .build()
            .unwrap();

        assert_eq!(
            request.headers().get("Content-Type").unwrap().as_str(),
            "application/json"
        );
        assert_eq!(
            request.body_as_str().unwrap(),
            r#"{"key":"value"}"#
        );
    }

    #[test]
    fn test_request_to_http_string() {
        let request = Request::builder()
            .method(Method::GET)
            .uri("http://example.com/api")
            .build()
            .unwrap();

        let http_str = request.to_http_string();
        assert!(http_str.starts_with("GET /api HTTP/1.1\r\n"));
        assert!(http_str.contains("Host: example.com\r\n"));
    }

    #[test]
    fn test_response_from_http() {
        let http_response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 13\r\n\r\nHello, World!";
        let response = Response::from_http(http_response).unwrap();

        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(
            response.headers().get("Content-Type").unwrap().as_str(),
            "text/html"
        );
        assert_eq!(response.body_as_string().unwrap(), "Hello, World!");
    }

    #[test]
    fn test_response_is_success() {
        let response_200 = Response::new(StatusCode::OK);
        assert!(response_200.is_success());

        let response_404 = Response::new(StatusCode::NOT_FOUND);
        assert!(!response_404.is_success());
    }

    #[test]
    fn test_response_set_body() {
        let mut response = Response::new(StatusCode::OK);
        response.set_body(b"test body".to_vec());
        assert_eq!(response.body_as_string().unwrap(), "test body");
    }

    #[test]
    fn test_client_new() {
        let client = Client::new();
        // Client should be created successfully
        drop(client);
    }

    #[test]
    fn test_uri_display() {
        let uri = Uri::parse("http://example.com/api?q=test").unwrap();
        assert_eq!(uri.to_string(), "http://example.com/api?q=test");
    }

    #[test]
    fn test_header_map_len_is_empty_clear() {
        let mut headers = HeaderMap::new();
        assert!(headers.is_empty());
        assert_eq!(headers.len(), 0);

        headers.insert("Content-Type", "application/json");
        assert!(!headers.is_empty());
        assert_eq!(headers.len(), 1);

        headers.clear();
        assert!(headers.is_empty());
        assert_eq!(headers.len(), 0);
    }

    #[test]
    fn test_uri_invalid() {
        assert!(Uri::parse("").is_err());
    }
}
