//! Chunked transfer encoding parser (RFC 9112 §7.1).

use crate::http::{Error, Result};
use alloc::string::ToString;
use alloc::vec::Vec;

use edgerun_protocols::http::HeaderMap;
pub use edgerun_protocols::http::http1::chunked::ChunkedBodyError;

/// Parse a chunked transfer-encoded body without trailers (for requests).
pub fn parse_chunked_body(data: &[u8]) -> Result<Vec<u8>> {
    edgerun_protocols::http::http1::chunked::parse_chunked_body(data)
        .map_err(|err| Error::InvalidResponse(err.to_string()))
}

/// Parse a chunked transfer-encoded body with trailer headers (for responses).
pub fn parse_chunked_body_with_trailers(data: &[u8]) -> Result<(Vec<u8>, HeaderMap)> {
    edgerun_protocols::http::http1::chunked::parse_chunked_body_with_trailers(data)
        .map_err(|err| Error::InvalidResponse(err.to_string()))
}
