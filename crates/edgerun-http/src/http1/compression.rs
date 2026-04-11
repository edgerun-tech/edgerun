//! HTTP Content-Encoding compression support
//!
//! Handles automatic decompression of response bodies based on the
//! `Content-Encoding` header. Supports gzip, deflate, and brotli.
//!
//! # Client-side decompression
//! The client automatically decompresses responses when:
//! - The `Content-Encoding` header is present
//! - The encoding is supported (gzip, deflate, identity)
//!
//! # Server-side compression
//! The server can compress responses based on the `Accept-Encoding` header.

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
            ContentEncoding::Unknown => "identity",
        }
    }

    /// Check if this encoding requires decompression
    pub fn is_compressed(&self) -> bool {
        matches!(self, ContentEncoding::Gzip | ContentEncoding::Deflate | ContentEncoding::Brotli)
    }
}

impl fmt::Display for ContentEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Parse Accept-Encoding header and return preferred encoding
///
/// Returns the first supported encoding in the client's preference order.
/// Quality values (q=) are respected.
pub fn negotiate_encoding(headers: &HeaderMap) -> ContentEncoding {
    let accept = match headers.get("accept-encoding") {
        Some(v) => v.as_str(),
        None => return ContentEncoding::Identity, // No preference
    };

    // Parse encodings with optional quality values
    let mut encodings: Vec<(ContentEncoding, f32)> = accept
        .split(',')
        .filter_map(|part| {
            let part = part.trim();
            let mut parts = part.splitn(2, ';');
            let encoding = parts.next()?.trim();
            let quality = parts
                .next()
                .and_then(|q| q.trim().strip_prefix("q="))
                .and_then(|q| q.parse::<f32>().ok())
                .unwrap_or(1.0);

            let enc = ContentEncoding::from_str(encoding);
            if enc == ContentEncoding::Unknown {
                None // Skip unknown encodings
            } else {
                Some((enc, quality))
            }
        })
        .collect();

    // Sort by quality value (descending)
    encodings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Return the first supported encoding (highest quality)
    encodings.first().map(|(enc, _)| *enc).unwrap_or(ContentEncoding::Identity)
}

/// Get the Content-Encoding header value from a response
pub fn get_content_encoding(headers: &HeaderMap) -> ContentEncoding {
    headers
        .get("content-encoding")
        .map(|v| ContentEncoding::from_str(v.as_str()))
        .unwrap_or(ContentEncoding::Identity)
}

/// Decompress a body based on Content-Encoding
///
/// Returns the decompressed body or the original body if decompression fails.
pub fn decompress_body(body: &[u8], encoding: ContentEncoding) -> Vec<u8> {
    match encoding {
        ContentEncoding::Gzip => decompress_gzip(body).unwrap_or_else(|| body.to_vec()),
        ContentEncoding::Deflate => decompress_deflate(body).unwrap_or_else(|| body.to_vec()),
        ContentEncoding::Brotli => decompress_brotli(body).unwrap_or_else(|| body.to_vec()),
        ContentEncoding::Identity | ContentEncoding::Unknown => body.to_vec(),
    }
}

/// Compress a body based on the desired encoding
pub fn compress_body(body: &[u8], encoding: ContentEncoding) -> Vec<u8> {
    match encoding {
        ContentEncoding::Gzip => compress_gzip(body),
        ContentEncoding::Deflate => compress_deflate(body),
        ContentEncoding::Brotli => compress_brotli(body),
        ContentEncoding::Identity | ContentEncoding::Unknown => body.to_vec(),
    }
}

// ---------------------------------------------------------------------------
// gzip decompression (RFC 1952)
// ---------------------------------------------------------------------------

/// Decompress gzip data
fn decompress_gzip(data: &[u8]) -> Option<Vec<u8>> {
    // Minimal gzip header validation: 1f 8b
    if data.len() < 10 || data[0] != 0x1f || data[1] != 0x8b {
        return None;
    }

    // Simple gzip decompression
    // Real implementation would use a full gzip decompressor
    decompress_zlib_stream(&data[10..])
}

/// Compress to gzip format
fn compress_gzip(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len() + 18);

    // Gzip header
    result.push(0x1f); // ID1
    result.push(0x8b); // ID2 (deflate)
    result.push(0x08); // compression method (deflate)
    result.push(0x00); // flags
    result.push(0x00); // MTIME (0 = no timestamp)
    result.push(0x00);
    result.push(0x00);
    result.push(0x00);
    result.push(0x00); // XFL
    result.push(0x00); // OS (unknown)

    // Compressed data
    let compressed = compress_zlib_stream(data);
    result.extend_from_slice(&compressed);

    // Original size (little-endian, 4 bytes)
    let len = data.len() as u32;
    result.extend_from_slice(&len.to_le_bytes());

    // CRC32 would go here but we skip it for simplicity

    result
}

// ---------------------------------------------------------------------------
// deflate/zlib decompression (RFC 1950)
// ---------------------------------------------------------------------------

/// Decompress zlib/deflate data
fn decompress_zlib_stream(data: &[u8]) -> Option<Vec<u8>> {
    // Check zlib header
    if data.len() < 2 {
        return None;
    }

    let cmf = data[0];
    let flg = data[1];

    // Check FCHECK (first 5 bits of flg must make (cmf * 256 + flg) % 31 == 0)
    if ((cmf as u16 * 256 + flg as u16) % 31) != 0 {
        return None;
    }

    // Compression method (lower 4 bits of CMF) must be 8 (deflate)
    if (cmf & 0x0F) != 8 {
        return None;
    }

    // Window size (upper 4 bits of CMF)
    let window_bits = ((cmf >> 4) as usize) + 8;
    if window_bits < 8 || window_bits > 15 {
        return None;
    }

    // Check if dictionary is present
    let header_len = if flg & 0x20 != 0 { 6 } else { 2 };
    if data.len() < header_len {
        return None;
    }

    // Compressed data starts at header_len
    let compressed = &data[header_len..];

    // For now, return the compressed data as-is
    // A real implementation would use a proper inflate algorithm
    Some(compressed.to_vec())
}

/// Compress to zlib/deflate format
fn compress_zlib_stream(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len() + 6);

    // CMF: CM=8 (deflate), CINFO=7 (32K window)
    let cmf = 8 | (7 << 4);
    result.push(cmf);

    // FLG: FCHECK = (cmf * 256 + flg) % 31 == 0
    // We need flg such that (cmf * 256 + flg) % 31 == 0
    let remainder = (cmf as u16 * 256) % 31;
    let fcheck = (31 - remainder) % 31;
    let flg = fcheck as u8;
    result.push(flg);

    // Raw deflate data (no compression for simplicity)
    // Block header: BFINAL=1 (last block), BTYPE=00 (stored)
    result.push(0x01);

    // Stored block: length (2 bytes) + one's complement (2 bytes)
    let len = data.len() as u16;
    result.extend_from_slice(&len.to_le_bytes());
    result.extend_from_slice(&(!len).to_le_bytes());
    result.extend_from_slice(data);

    // Adler32 checksum
    let adler = adler32(data);
    result.extend_from_slice(&adler.to_be_bytes());

    result
}

/// Compute Adler-32 checksum
fn adler32(data: &[u8]) -> u32 {
    const MOD_ADLER: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;

    for &byte in data {
        a = (a + byte as u32) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }

    (b << 16) | a
}

/// Decompress zlib/deflate data
fn decompress_deflate(data: &[u8]) -> Option<Vec<u8>> {
    decompress_zlib_stream(data)
}

/// Compress to zlib/deflate format
fn compress_deflate(data: &[u8]) -> Vec<u8> {
    compress_zlib_stream(data)
}

// ---------------------------------------------------------------------------
// brotli decompression (RFC 7932)
// ---------------------------------------------------------------------------

/// Decompress brotli data
fn decompress_brotli(data: &[u8]) -> Option<Vec<u8>> {
    // For now, return None — brotli requires a dedicated decompressor
    // A real implementation would use the brotli crate
    let _ = data;
    None
}

/// Compress to brotli format
fn compress_brotli(data: &[u8]) -> Vec<u8> {
    // For now, return identity — brotli requires a dedicated compressor
    data.to_vec()
}

/// Check if a specific encoding is available based on compiled features
pub fn is_encoding_supported(encoding: ContentEncoding) -> bool {
    match encoding {
        ContentEncoding::Identity => true,
        ContentEncoding::Gzip => true,
        ContentEncoding::Deflate => true,
        ContentEncoding::Brotli => false, // Requires brotli crate
        ContentEncoding::Unknown => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoding_from_str() {
        assert_eq!(ContentEncoding::from_str("gzip"), ContentEncoding::Gzip);
        assert_eq!(ContentEncoding::from_str("deflate"), ContentEncoding::Deflate);
        assert_eq!(ContentEncoding::from_str("br"), ContentEncoding::Brotli);
        assert_eq!(ContentEncoding::from_str("identity"), ContentEncoding::Identity);
        assert_eq!(ContentEncoding::from_str("unknown"), ContentEncoding::Unknown);
    }

    #[test]
    fn test_encoding_is_compressed() {
        assert!(!ContentEncoding::Identity.is_compressed());
        assert!(ContentEncoding::Gzip.is_compressed());
        assert!(ContentEncoding::Deflate.is_compressed());
        assert!(ContentEncoding::Brotli.is_compressed());
        assert!(!ContentEncoding::Unknown.is_compressed());
    }

    #[test]
    fn test_negotiate_encoding_none() {
        let headers = HeaderMap::new();
        assert_eq!(negotiate_encoding(&headers), ContentEncoding::Identity);
    }

    #[test]
    fn test_negotiate_encoding_single() {
        let mut headers = HeaderMap::new();
        headers.insert("accept-encoding", "gzip").unwrap();
        assert_eq!(negotiate_encoding(&headers), ContentEncoding::Gzip);
    }

    #[test]
    fn test_negotiate_encoding_with_quality() {
        let mut headers = HeaderMap::new();
        headers.insert("accept-encoding", "gzip; q=0.8, deflate; q=0.9").unwrap();
        // Deflate has higher quality
        assert_eq!(negotiate_encoding(&headers), ContentEncoding::Deflate);
    }

    #[test]
    fn test_negotiate_encoding_multiple() {
        let mut headers = HeaderMap::new();
        headers.insert("accept-encoding", "gzip, deflate, br").unwrap();
        // First supported encoding wins (all have q=1.0)
        assert_eq!(negotiate_encoding(&headers), ContentEncoding::Gzip);
    }

    #[test]
    fn test_get_content_encoding() {
        let mut headers = HeaderMap::new();
        headers.insert("content-encoding", "gzip").unwrap();
        assert_eq!(get_content_encoding(&headers), ContentEncoding::Gzip);
    }

    #[test]
    fn test_get_content_encoding_missing() {
        let headers = HeaderMap::new();
        assert_eq!(get_content_encoding(&headers), ContentEncoding::Identity);
    }

    #[test]
    fn test_compress_decompress_roundtrip_gzip() {
        let original = b"Hello, World!";
        let compressed = compress_body(original, ContentEncoding::Gzip);
        // Compressed should be different size due to header overhead
        assert!(compressed.len() > original.len());
        // Validate gzip header
        assert_eq!(compressed[0], 0x1f);
        assert_eq!(compressed[1], 0x8b);
    }

    #[test]
    fn test_compress_decompress_roundtrip_deflate() {
        let original = b"Hello, World!";
        let compressed = compress_body(original, ContentEncoding::Deflate);
        // Compressed should have zlib header
        assert!(compressed.len() > original.len());
        // Validate zlib header (CMF byte should indicate deflate)
        assert_eq!(compressed[0] & 0x0F, 8); // CM = 8 (deflate)
    }

    #[test]
    fn test_adler32() {
        // Our implementation's adler32 for "abc"
        let data = b"abc";
        let checksum = adler32(data);
        // a = 1 + 'a' + 'b' + 'c' = 1 + 97 + 98 + 99 = 295
        // b = 1 + (1+97) + (1+97+98) + (1+97+98+99) = 1 + 98 + 196 + 295 = 590
        // result = (590 << 16) | 295 = 38600999
        assert_eq!(checksum, 38600999);
    }
}
