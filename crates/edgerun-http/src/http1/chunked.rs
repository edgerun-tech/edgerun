//! Chunked transfer encoding parser (RFC 9112 §7.1).
//!
//! Shared between request and response parsing to avoid duplicating
//! the hex-size-line parsing logic.

use crate::header::HeaderMap;
use crate::{Error, Result};
use alloc::string::ToString;
use alloc::vec::Vec;

/// Parse a chunked transfer-encoded body without trailers (for requests).
///
/// Requests do not have trailer headers per RFC 9112 §6.3.
pub fn parse_chunked_body(data: &[u8]) -> Result<Vec<u8>> {
    crate::chunked::parse_body(data).map_err(|err| Error::InvalidResponse(err.to_string()))
}

/// Parse a chunked transfer-encoded body with trailer headers (for responses).
///
/// Returns (body, trailer_headers).
pub fn parse_chunked_body_with_trailers(data: &[u8]) -> Result<(Vec<u8>, HeaderMap)> {
    crate::chunked::parse_body_with_trailers(data)
        .map_err(|err| Error::InvalidResponse(err.to_string()))
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
