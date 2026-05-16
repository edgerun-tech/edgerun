//! HTTP Content-Encoding compression support
//!
//! Handles automatic decompression of response bodies based on the
//! `Content-Encoding` header. Supports gzip and deflate.
//!
//! # Client-side decompression
//! The client automatically decompresses responses when:
//! - The `Content-Encoding` header is present
//! - The encoding is supported (gzip, deflate, identity)

use crate::http::HeaderMap;
#[cfg(feature = "http-compression")]
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
#[cfg(feature = "http-compression")]
use edgerun_encoding::compression;

/// Supported content encodings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentEncoding {
    /// No encoding (identity)
    Identity,
    /// gzip (RFC 1952)
    Gzip,
    /// zlib/deflate (RFC 1950)
    Deflate,
    /// brotli (RFC 7932), parsed but unsupported by this crate.
    Brotli,
    /// Unknown/unsupported encoding
    Unknown,
}

impl ContentEncoding {
    /// Parse from Content-Encoding header value
    pub fn from_str(value: &str) -> Self {
        if value.eq_ignore_ascii_case("gzip") {
            ContentEncoding::Gzip
        } else if value.eq_ignore_ascii_case("deflate") {
            ContentEncoding::Deflate
        } else if value.eq_ignore_ascii_case("br") {
            ContentEncoding::Brotli
        } else if value.eq_ignore_ascii_case("identity") {
            ContentEncoding::Identity
        } else {
            ContentEncoding::Unknown
        }
    }

    /// Format as header value
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentEncoding::Identity => "identity",
            ContentEncoding::Gzip => "gzip",
            ContentEncoding::Deflate => "deflate",
            ContentEncoding::Brotli => "br",
            ContentEncoding::Unknown => "unknown",
        }
    }
}

/// Build Accept-Encoding header value
pub fn accept_encoding_value() -> &'static str {
    #[cfg(feature = "http-compression")]
    {
        "gzip, deflate"
    }
    #[cfg(not(feature = "http-compression"))]
    {
        "identity"
    }
}

/// Compress response body for a negotiated content encoding.
#[cfg(feature = "http-compression")]
pub fn compress_body(body: &[u8], encoding: ContentEncoding) -> Option<Vec<u8>> {
    match encoding {
        ContentEncoding::Gzip => Some(compress_gzip(body)),
        ContentEncoding::Deflate => Some(compress_deflate(body)),
        ContentEncoding::Identity | ContentEncoding::Brotli | ContentEncoding::Unknown => None,
    }
}

/// Compress response body for a negotiated content encoding.
#[cfg(not(feature = "http-compression"))]
pub fn compress_body(_body: &[u8], _encoding: ContentEncoding) -> Option<Vec<u8>> {
    None
}

/// Pick the best response encoding from an Accept-Encoding header.
pub fn preferred_response_encoding(accept_encoding: &str) -> ContentEncoding {
    #[cfg(feature = "http-compression")]
    {
        if accepts_encoding(accept_encoding, "gzip") {
            ContentEncoding::Gzip
        } else if accepts_encoding(accept_encoding, "deflate") {
            ContentEncoding::Deflate
        } else {
            ContentEncoding::Identity
        }
    }
    #[cfg(not(feature = "http-compression"))]
    {
        let _ = accept_encoding;
        ContentEncoding::Identity
    }
}

fn accepts_encoding(header: &str, encoding: &str) -> bool {
    header.split(',').any(|part| {
        let mut pieces = part.trim().split(';');
        let token = pieces.next().unwrap_or("").trim();
        if !token.eq_ignore_ascii_case(encoding) && token != "*" {
            return false;
        }
        pieces.all(|piece| {
            let piece = piece.trim();
            !piece.starts_with("q=") || piece[2..].parse::<f32>().map(|q| q > 0.0).unwrap_or(true)
        })
    })
}

/// Decompress response body based on Content-Encoding header
pub fn decompress_body(body: &[u8], headers: &HeaderMap) -> Option<Vec<u8>> {
    let encoding = headers
        .get("content-encoding")
        .map(|v| ContentEncoding::from_str(v.as_str()))
        .unwrap_or(ContentEncoding::Identity);

    match encoding {
        #[cfg(feature = "http-compression")]
        ContentEncoding::Gzip => decompress_gzip(body),
        #[cfg(not(feature = "http-compression"))]
        ContentEncoding::Gzip => None,
        #[cfg(feature = "http-compression")]
        ContentEncoding::Deflate => decompress_deflate(body),
        #[cfg(not(feature = "http-compression"))]
        ContentEncoding::Deflate => None,
        ContentEncoding::Brotli => None,
        ContentEncoding::Identity => Some(body.to_vec()),
        ContentEncoding::Unknown => Some(body.to_vec()),
    }
}

// ---------------------------------------------------------------------------
// gzip compression/decompression (RFC 1952)
// ---------------------------------------------------------------------------

/// Compress to gzip format
#[cfg(feature = "http-compression")]
fn compress_gzip(data: &[u8]) -> Vec<u8> {
    compression::gzip_compress(data, 6)
}

/// Decompress gzip data
#[cfg(feature = "http-compression")]
fn decompress_gzip(data: &[u8]) -> Option<Vec<u8>> {
    compression::gzip_decompress(data).ok()
}

// ---------------------------------------------------------------------------
// deflate/zlib compression/decompression (RFC 1950)
// ---------------------------------------------------------------------------

/// Compress to zlib/deflate format
#[cfg(feature = "http-compression")]
fn compress_deflate(data: &[u8]) -> Vec<u8> {
    compression::zlib_compress(data, 6)
}

/// Decompress zlib/deflate data
#[cfg(feature = "http-compression")]
fn decompress_deflate(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 2 {
        return None;
    }

    compression::zlib_decompress(data).ok()
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

impl fmt::Display for ContentEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "http-compression")]
    #[test]
    fn test_gzip_roundtrip() {
        let original = b"Hello, World! This is a test of gzip compression.";
        let compressed = compress_gzip(original);
        assert!(compressed.len() > 10);
        assert_eq!(compressed[0], 0x1f);
        assert_eq!(compressed[1], 0x8b);

        let decompressed = decompress_gzip(&compressed).expect("gzip decompress failed");
        assert_eq!(decompressed, original);
    }

    #[cfg(feature = "http-compression")]
    #[test]
    fn test_deflate_roundtrip() {
        let original = b"Hello, World! This is a test of zlib compression.";
        let compressed = compress_deflate(original);
        assert!(compressed.len() > 2);

        let decompressed = decompress_deflate(&compressed).expect("deflate decompress failed");
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_decompress_body_identity() {
        let headers = HeaderMap::new();
        let body = b"plain text";
        let result = decompress_body(body, &headers);
        assert_eq!(result, Some(body.to_vec()));
    }

    #[cfg(feature = "http-compression")]
    #[test]
    fn test_decompress_body_gzip() {
        let original = b"test gzip content";
        let compressed = compress_gzip(original);

        let mut headers = HeaderMap::new();
        headers.insert("content-encoding", "gzip");

        let decompressed = decompress_body(&compressed, &headers).expect("decompress failed");
        assert_eq!(decompressed, original);
    }
}
