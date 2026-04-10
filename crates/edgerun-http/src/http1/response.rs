//! HTTP/1.1 response types (RFC 9112)

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

    /// Parse an HTTP/1.1 response from raw bytes.
    ///
    /// Handles:
    /// - Status line parsing
    /// - Header parsing until blank line (CRLF CRLF)
    /// - Body extraction based on:
    ///   - Content-Length header
    ///   - Chunked Transfer-Encoding (RFC 9112 §7.1)
    ///   - No body for HEAD responses, 1xx, 204, 304 responses
    pub fn from_bytes(raw: &[u8]) -> Result<Self> {
        let mut pos = 0;

        // Find end of status line (CRLF)
        let crlf = Self::find_crlf(raw, pos).ok_or_else(|| {
            crate::Error::InvalidResponse("No status line terminator".to_string())
        })?;
        let status_line = std::str::from_utf8(&raw[pos..crlf])
            .map_err(|_| crate::Error::InvalidResponse("Invalid UTF-8 in status line".to_string()))?;
        pos = crlf + 2;

        // Parse status line: HTTP-Version SP Status-Code SP Reason-Phrase
        let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
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

        // Parse headers until blank line (CRLF)
        let mut headers = HeaderMap::new();
        loop {
            if pos >= raw.len() {
                return Err(crate::Error::InvalidResponse(
                    "Unexpected end of response before headers".to_string(),
                ));
            }
            if pos + 1 < raw.len() && raw[pos] == b'\r' && raw[pos + 1] == b'\n' {
                // Blank line — end of headers
                pos += 2;
                break;
            }
            let crlf = Self::find_crlf(raw, pos).ok_or_else(|| {
                crate::Error::InvalidResponse("No header line terminator".to_string())
            })?;
            let line = std::str::from_utf8(&raw[pos..crlf])
                .map_err(|_| crate::Error::InvalidResponse("Invalid UTF-8 in header".to_string()))?;

            if let Some(colon_pos) = line.find(':') {
                let name = line[..colon_pos].trim();
                let value = line[colon_pos + 1..].trim();
                if !name.is_empty() {
                    headers.insert(name, value);
                }
            }
            pos = crlf + 2;
        }

        // Determine body length and extract body
        let status_code_val = status.as_u16();
        let is_head = false; // We don't know the request method here

        // Responses to HEAD requests and 1xx/204/304 responses MUST NOT have a body
        if is_head || status_code_val < 200 || status_code_val == 204 || status_code_val == 304 {
            return Ok(Response { status, headers, body: Vec::new() });
        }

        let remaining = &raw[pos..];

        // Check for chunked transfer encoding
        let transfer_encoding = headers
            .get("transfer-encoding")
            .map(|v| v.as_str().to_lowercase());
        let is_chunked = transfer_encoding.as_deref().map_or(false, |v| v.contains("chunked"));

        let body = if is_chunked {
            Self::parse_chunked_body(remaining)?
        } else if let Some(content_length) = headers.get("content-length") {
            let len = content_length
                .as_str()
                .parse::<usize>()
                .unwrap_or(remaining.len())
                .min(remaining.len());
            remaining[..len].to_vec()
        } else {
            // No Content-Length, no Transfer-Encoding — body extends to end of data
            remaining.to_vec()
        };

        Ok(Response { status, headers, body })
    }

    /// Create a response from HTTP response string (convenience wrapper).
    pub fn from_http(response: &str) -> Result<Self> {
        Self::from_bytes(response.as_bytes())
    }

    /// Find CRLF starting at position `pos`.
    fn find_crlf(data: &[u8], pos: usize) -> Option<usize> {
        data[pos..]
            .windows(2)
            .position(|w| w == b"\r\n")
            .map(|i| pos + i)
    }

    /// Parse a chunked transfer-encoded body (RFC 9112 §7.1).
    fn parse_chunked_body(mut data: &[u8]) -> Result<Vec<u8>> {
        let mut body = Vec::new();

        loop {
            // Find end of chunk size line
            let crlf = Self::find_crlf(data, 0).ok_or_else(|| {
                crate::Error::InvalidResponse("Incomplete chunked body".to_string())
            })?;

            // Parse chunk size (hex), ignoring chunk extensions
            let size_hex = std::str::from_utf8(&data[..crlf])
                .map_err(|_| crate::Error::InvalidResponse("Invalid chunk size".to_string()))?;

            // Chunk extensions are after the hex size, before any semicolon
            let size_str = size_hex.split(';').next().unwrap_or(size_hex).trim();
            let chunk_size = usize::from_str_radix(size_str, 16)
                .map_err(|_| crate::Error::InvalidResponse("Invalid chunk size".to_string()))?;

            data = &data[crlf + 2..];

            if chunk_size == 0 {
                // Last chunk — optional trailers follow
                // Find the final CRLF (trailers end with blank line)
                // For simplicity, we stop here — trailers are discarded
                break;
            }

            // Read chunk data
            if data.len() < chunk_size {
                return Err(crate::Error::InvalidResponse(
                    "Incomplete chunked body".to_string(),
                ));
            }

            body.extend_from_slice(&data[..chunk_size]);
            data = &data[chunk_size..];

            // Consume CRLF after chunk data
            if data.len() < 2 || data[0] != b'\r' || data[1] != b'\n' {
                return Err(crate::Error::InvalidResponse(
                    "Missing CRLF after chunk".to_string(),
                ));
            }
            data = &data[2..];
        }

        Ok(body)
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
            .map_err(|e| crate::Error::ProtocolError(format!("Invalid UTF-8: {e}")))
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
