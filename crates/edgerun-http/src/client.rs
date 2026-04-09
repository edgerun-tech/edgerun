//! HTTP client

use crate::request::Request;
use crate::response::Response;
use crate::{Error, Result};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

/// HTTP client
pub struct Client {
    timeout: Option<Duration>,
}

impl Client {
    /// Create a new HTTP client
    pub fn new() -> Self {
        Client { timeout: None }
    }

    /// Set the timeout for requests
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Execute an HTTP request
    pub fn execute(&self, request: &Request) -> Result<Response> {
        let uri = request.uri();

        let host = uri
            .host()
            .ok_or_else(|| Error::InvalidUri("No host in URI".to_string()))?;

        let port = uri
            .port()
            .ok_or_else(|| Error::InvalidUri("No port in URI".to_string()))?;

        // Connect to the server
        let addr = format!("{}:{}", host, port);
        let sock_addr = SocketAddr::from_str(&addr)
            .map_err(|e| Error::InvalidUri(format!("Invalid address: {}", e)))?;
        let mut stream = if let Some(timeout) = self.timeout {
            TcpStream::connect_timeout(&sock_addr, timeout)?
        } else {
            TcpStream::connect(sock_addr)?
        };

        // For HTTPS, we'd need TLS support, but for now we'll just use plain TCP
        // A full TLS implementation would require native-tls or rustls
        if uri.is_https() {
            return Err(Error::ProtocolError(
                "HTTPS not supported without TLS feature".to_string(),
            ));
        }

        // Send the request
        let request_str = request.to_http_string();
        stream.write_all(request_str.as_bytes())?;
        stream.flush()?;

        // Read the response
        let mut reader = BufReader::new(stream);
        let mut response_str = String::new();

        // Read headers
        loop {
            let mut line = String::new();
            let bytes_read = reader.read_line(&mut line)?;
            if bytes_read == 0 {
                break;
            }
            response_str.push_str(&line);
            if line == "\r\n" || line == "\n" {
                break;
            }
        }

        // Read body if Content-Length is present
        if let Some(content_length) = reader
            .get_ref()
            .peek(&mut [0; 1])
            .ok()
            .filter(|&n| n > 0)
            .and_then(|_| {
                request
                    .headers()
                    .get("Content-Length")
                    .and_then(|h| h.as_str().parse::<usize>().ok())
            })
        {
            let mut body = vec![0u8; content_length];
            reader.read_exact(&mut body)?;
            response_str.push_str(&String::from_utf8_lossy(&body));
        }

        // Parse response
        Response::from_http(&response_str)
    }

    /// Send a GET request
    pub fn get(&self, uri: &str) -> Result<Response> {
        let request = Request::builder()
            .method(crate::Method::GET)
            .uri(uri)
            .build()?;
        self.execute(&request)
    }

    /// Send a POST request with a body
    pub fn post(&self, uri: &str, body: &[u8]) -> Result<Response> {
        let request = Request::builder()
            .method(crate::Method::POST)
            .uri(uri)
            .body(body.to_vec())
            .build()?;
        self.execute(&request)
    }

    /// Send a POST request with JSON body
    pub fn post_json(&self, uri: &str, json: &str) -> Result<Response> {
        let request = Request::builder()
            .method(crate::Method::POST)
            .uri(uri)
            .json_body(json)
            .build()?;
        self.execute(&request)
    }

    /// Send a PUT request with a body
    pub fn put(&self, uri: &str, body: &[u8]) -> Result<Response> {
        let request = Request::builder()
            .method(crate::Method::PUT)
            .uri(uri)
            .body(body.to_vec())
            .build()?;
        self.execute(&request)
    }

    /// Send a DELETE request
    pub fn delete(&self, uri: &str) -> Result<Response> {
        let request = Request::builder()
            .method(crate::Method::DELETE)
            .uri(uri)
            .build()?;
        self.execute(&request)
    }

    /// Send a PATCH request with a body
    pub fn patch(&self, uri: &str, body: &[u8]) -> Result<Response> {
        let request = Request::builder()
            .method(crate::Method::PATCH)
            .uri(uri)
            .body(body.to_vec())
            .build()?;
        self.execute(&request)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
