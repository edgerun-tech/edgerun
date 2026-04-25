//! Chunked transfer encoding parser (RFC 9112 §7.1).
//!
//! Shared between request and response parsing to avoid duplicating
//! the hex-size-line parsing logic.

use crate::header::HeaderMap;
use crate::{Error, Result};

/// Find CRLF starting at position `pos`.
fn find_crlf(data: &[u8], pos: usize) -> Option<usize> {
    data[pos..]
        .windows(2)
        .position(|w| w == b"\r\n")
        .map(|i| pos + i)
}

/// Parse a chunked transfer-encoded body without trailers (for requests).
///
/// Requests do not have trailer headers per RFC 9112 §6.3.
pub fn parse_chunked_body(data: &[u8]) -> Result<Vec<u8>> {
    let (body, _trailers) = parse_chunked_body_with_trailers(data)?;
    Ok(body)
}

/// Parse a chunked transfer-encoded body with trailer headers (for responses).
///
/// Returns (body, trailer_headers).
pub fn parse_chunked_body_with_trailers(mut data: &[u8]) -> Result<(Vec<u8>, HeaderMap)> {
    let mut body = Vec::new();

    loop {
        let crlf = find_crlf(data, 0)
            .ok_or_else(|| Error::InvalidResponse("Incomplete chunked body".to_string()))?;

        let size_hex = std::str::from_utf8(&data[..crlf])
            .map_err(|_| Error::InvalidResponse("Invalid chunk size".to_string()))?;

        let size_str = size_hex.split(';').next().unwrap_or(size_hex).trim();
        let chunk_size = usize::from_str_radix(size_str, 16)
            .map_err(|_| Error::InvalidResponse("Invalid chunk size".to_string()))?;

        data = &data[crlf + 2..];

        if chunk_size == 0 {
            // Last chunk — parse trailer headers (RFC 9112 §6.3)
            let trailers = parse_trailers(data)?;
            return Ok((body, trailers));
        }

        if data.len() < chunk_size {
            return Err(Error::InvalidResponse(
                "Incomplete chunked body".to_string(),
            ));
        }

        body.extend_from_slice(&data[..chunk_size]);
        data = &data[chunk_size..];

        if data.len() < 2 || data[0] != b'\r' || data[1] != b'\n' {
            return Err(Error::InvalidResponse(
                "Missing CRLF after chunk".to_string(),
            ));
        }
        data = &data[2..];
    }
}

/// Parse trailer headers after the last chunk (RFC 9112 §6.3).
fn parse_trailers(data: &[u8]) -> Result<HeaderMap> {
    let mut trailers = HeaderMap::new();
    let mut pos = 0;

    while pos < data.len() {
        let line_end = data[pos..]
            .windows(2)
            .position(|w| w == b"\r\n")
            .map(|i| pos + i);

        match line_end {
            Some(end) if end == pos => {
                // Blank line — end of trailers
                break;
            }
            Some(end) => {
                let line = std::str::from_utf8(&data[pos..end])
                    .map_err(|_| Error::InvalidResponse("Invalid UTF-8 in trailer".to_string()))?;

                if let Some(colon) = line.find(':') {
                    let name = line[..colon].trim();
                    let value = line[colon + 1..].trim();
                    if !name.is_empty() {
                        let _ = trailers.insert(name, value);
                    }
                }
                pos = end + 2;
            }
            None => break,
        }
    }

    Ok(trailers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_chunked_body() {
        // "5\r\n" + "Hello" + "\r\n" + "0\r\n" + "\r\n"
        let data = b"5\r\nHello\r\n0\r\n\r\n";
        let (body, trailers) = parse_chunked_body_with_trailers(data).unwrap();
        assert_eq!(body, b"Hello");
        assert!(trailers.is_empty());
    }

    #[test]
    fn test_multiple_chunks() {
        let data = b"3\r\nHel\r\n8\r\nlo World\r\n0\r\n\r\n";
        let (body, _) = parse_chunked_body_with_trailers(data).unwrap();
        assert_eq!(body, b"Hello World");
    }

    #[test]
    fn test_chunked_with_trailers() {
        let data = b"5\r\nHello\r\n0\r\nX-Foo: bar\r\n\r\n";
        let (body, trailers) = parse_chunked_body_with_trailers(data).unwrap();
        assert_eq!(body, b"Hello");
        assert_eq!(trailers.get("x-foo").map(|v| v.as_str()), Some("bar"));
    }

    #[test]
    fn test_chunk_extensions() {
        // Chunk size with extension: "5;foo=bar\r\n"
        let data = b"5;foo=bar\r\nHello\r\n0\r\n\r\n";
        let (body, _) = parse_chunked_body_with_trailers(data).unwrap();
        assert_eq!(body, b"Hello");
    }

    #[test]
    fn test_incomplete_chunked_body() {
        let data = b"5\r\nHel";
        assert!(parse_chunked_body_with_trailers(data).is_err());
    }
}
