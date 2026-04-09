//! HTTP response types

use crate::header::HeaderMap;
use crate::status::StatusCode;
use crate::Result;
use std::fmt;

/// HTTP response
#[derive(Debug, Clone)]
pub struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl Response {
    /// Create a new response
    pub fn new(status: StatusCode) -> Self {
        Response {
            status,
            headers: HeaderMap::new(),
            body: Vec::new(),
        }
    }

    /// Create a response from HTTP response string
    pub fn from_http(response: &str) -> Result<Self> {
        let mut lines = response.lines();

        // Parse status line
        let status_line = lines
            .next()
            .ok_or_else(|| crate::Error::InvalidResponse("No status line".to_string()))?;

        let parts: Vec<&str> = status_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(crate::Error::InvalidResponse(
                "Invalid status line".to_string(),
            ));
        }

        let status_code = parts[1]
            .parse::<u16>()
            .map_err(|_| crate::Error::InvalidResponse("Invalid status code".to_string()))?;

        let status = StatusCode::new(status_code)
            .map_err(crate::Error::InvalidResponse)?;

        // Parse headers
        let mut headers = HeaderMap::new();
        let mut body_start = false;
        let mut body_lines = Vec::new();

        for line in lines {
            if line.is_empty() {
                body_start = true;
                continue;
            }

            if !body_start {
                if let Some(colon_pos) = line.find(':') {
                    let name = line[..colon_pos].trim();
                    let value = line[colon_pos + 1..].trim();
                    if !name.is_empty() {
                        headers.insert(name, value);
                    }
                }
            } else {
                body_lines.push(line);
            }
        }

        // Body is everything after the empty line
        let body = if body_start {
            body_lines.join("\n").into_bytes()
        } else {
            Vec::new()
        };

        Ok(Response {
            status,
            headers,
            body,
        })
    }

    /// Get the status code
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Get the headers
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Get the body
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Get the body as a string (if UTF-8)
    pub fn body_as_string(&self) -> Result<&str> {
        std::str::from_utf8(&self.body)
            .map_err(|e| crate::Error::ProtocolError(format!("Invalid UTF-8: {}", e)))
    }

    /// Set the headers
    pub fn set_headers(&mut self, headers: HeaderMap) {
        self.headers = headers;
    }

    /// Set the body
    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = body;
    }

    /// Check if the response is successful (2xx)
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HTTP/1.1 {}", self.status)
    }
}
