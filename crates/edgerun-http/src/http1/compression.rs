//! HTTP Content-Encoding compression support
//!
//! Handles automatic decompression of response bodies based on the
//! `Content-Encoding` header. Supports gzip, deflate, and brotli.
//!
//! # Client-side decompression
//! The client automatically decompresses responses when:
//! - The `Content-Encoding` header is present
//! - The encoding is supported (gzip, deflate, br, identity)

use crate::HeaderMap;
use std::fmt;

/// Supported content encodings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentEncoding {
    /// No encoding (identity)
    Identity,
    /// gzip (RFC 1952)
    Gzip,
    /// zlib/deflate (RFC 1950)
    Deflate,
    /// brotli (RFC 7932)
    Brotli,
    /// Unknown/unsupported encoding
    Unknown,
}

impl ContentEncoding {
    /// Parse from Content-Encoding header value
    pub fn from_str(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "gzip" => ContentEncoding::Gzip,
            "deflate" => ContentEncoding::Deflate,
            "br" => ContentEncoding::Brotli,
            "identity" => ContentEncoding::Identity,
            _ => ContentEncoding::Unknown,
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
    "br, gzip, deflate"
}

/// Decompress response body based on Content-Encoding header
pub fn decompress_body(body: &[u8], headers: &HeaderMap) -> Option<Vec<u8>> {
    let encoding = headers
        .get("content-encoding")
        .map(|v| ContentEncoding::from_str(v.as_str()))
        .unwrap_or(ContentEncoding::Identity);

    match encoding {
        ContentEncoding::Gzip => decompress_gzip(body),
        ContentEncoding::Deflate => decompress_deflate(body),
        ContentEncoding::Brotli => decompress_brotli(body),
        ContentEncoding::Identity => Some(body.to_vec()),
        ContentEncoding::Unknown => Some(body.to_vec()),
    }
}

// ---------------------------------------------------------------------------
// gzip compression/decompression (RFC 1952)
// ---------------------------------------------------------------------------

/// Compress to gzip format
fn compress_gzip(data: &[u8]) -> Vec<u8> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::with_capacity(data.len()), Compression::default());
    encoder.write_all(data).ok();
    encoder.finish().unwrap_or_default()
}

/// Decompress gzip data
fn decompress_gzip(data: &[u8]) -> Option<Vec<u8>> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    if data.len() < 10 || data[0] != 0x1f || data[1] != 0x8b {
        return None;
    }

    let mut decoder = GzDecoder::new(data);
    let mut result = Vec::with_capacity(data.len() * 2);
    decoder.read_to_end(&mut result).ok()?;
    Some(result)
}

// ---------------------------------------------------------------------------
// deflate/zlib compression/decompression (RFC 1950)
// ---------------------------------------------------------------------------

/// Compress to zlib/deflate format
fn compress_deflate(data: &[u8]) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::with_capacity(data.len()), Compression::default());
    encoder.write_all(data).ok();
    encoder.finish().unwrap_or_default()
}

/// Decompress zlib/deflate data
fn decompress_deflate(data: &[u8]) -> Option<Vec<u8>> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    if data.len() < 2 {
        return None;
    }

    let mut decoder = ZlibDecoder::new(data);
    let mut result = Vec::with_capacity(data.len() * 2);
    decoder.read_to_end(&mut result).ok()?;
    Some(result)
}

// ---------------------------------------------------------------------------
// brotli compression/decompression (RFC 7932)
// ---------------------------------------------------------------------------

/// Compress to brotli format
fn compress_brotli(data: &[u8]) -> Vec<u8> {
    use brotli::enc::backward_references::BrotliEncoderMode;
    use brotli::enc::BrotliEncoderParams;
    let mut out = Vec::with_capacity(data.len());
    let mut params = BrotliEncoderParams::default();
    params.mode = BrotliEncoderMode::BROTLI_MODE_GENERIC;
    params.quality = 4; // Moderate compression
    brotli::BrotliCompress(&mut std::io::Cursor::new(data), &mut out, &params).unwrap_or_default();
    out
}

/// Decompress brotli data
fn decompress_brotli(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() * 2);
    brotli::BrotliDecompress(&mut std::io::Cursor::new(data), &mut out).ok()?;
    Some(out)
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

    #[test]
    fn test_deflate_roundtrip() {
        let original = b"Hello, World! This is a test of zlib compression.";
        let compressed = compress_deflate(original);
        assert!(compressed.len() > 2);

        let decompressed = decompress_deflate(&compressed).expect("deflate decompress failed");
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_brotli_roundtrip() {
        let original = b"Hello, World! This is a test of brotli compression.";
        let compressed = compress_brotli(original);
        assert!(!compressed.is_empty());

        let decompressed = decompress_brotli(&compressed).expect("brotli decompress failed");
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_decompress_body_identity() {
        let headers = HeaderMap::new();
        let body = b"plain text";
        let result = decompress_body(body, &headers);
        assert_eq!(result, Some(body.to_vec()));
    }

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
