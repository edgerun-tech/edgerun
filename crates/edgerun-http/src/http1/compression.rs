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
#[cfg(feature = "http-compression")]
use alloc::boxed::Box;
use alloc::string::String;
#[cfg(feature = "http-compression")]
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;
#[cfg(feature = "http-compression")]
use edgerun_encoding::crc32::crc32;

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
        "br, gzip, deflate"
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
        ContentEncoding::Brotli => Some(compress_brotli(body)),
        ContentEncoding::Gzip => Some(compress_gzip(body)),
        ContentEncoding::Deflate => Some(compress_deflate(body)),
        ContentEncoding::Identity | ContentEncoding::Unknown => None,
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
        if accepts_encoding(accept_encoding, "br") {
            ContentEncoding::Brotli
        } else if accepts_encoding(accept_encoding, "gzip") {
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
        #[cfg(feature = "http-compression")]
        ContentEncoding::Brotli => decompress_brotli(body),
        #[cfg(not(feature = "http-compression"))]
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
    let mut out = Vec::with_capacity(data.len() + 18);
    out.extend_from_slice(&[0x1f, 0x8b, 0x08, 0x00, 0, 0, 0, 0, 0x00, 0xff]);
    out.extend_from_slice(&miniz_oxide::deflate::compress_to_vec(data, 6));
    out.extend_from_slice(&crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

/// Decompress gzip data
#[cfg(feature = "http-compression")]
fn decompress_gzip(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 18 || data[0] != 0x1f || data[1] != 0x8b || data[2] != 8 {
        return None;
    }

    let flags = data[3];
    if flags & 0xe0 != 0 {
        return None;
    }

    let mut pos = 10;
    if flags & 0x04 != 0 {
        if pos + 2 > data.len() {
            return None;
        }
        let xlen = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
        pos = pos.checked_add(2 + xlen)?;
    }
    if flags & 0x08 != 0 {
        pos = skip_zero_terminated(data, pos)?;
    }
    if flags & 0x10 != 0 {
        pos = skip_zero_terminated(data, pos)?;
    }
    if flags & 0x02 != 0 {
        pos = pos.checked_add(2)?;
    }
    if pos + 8 > data.len() {
        return None;
    }

    let footer = data.len() - 8;
    let result = miniz_oxide::inflate::decompress_to_vec(&data[pos..footer]).ok()?;
    let expected_crc = u32::from_le_bytes(data[footer..footer + 4].try_into().ok()?);
    let expected_len = u32::from_le_bytes(data[footer + 4..].try_into().ok()?);
    if expected_crc != crc32(&result) || expected_len != result.len() as u32 {
        return None;
    }
    Some(result)
}

// ---------------------------------------------------------------------------
// deflate/zlib compression/decompression (RFC 1950)
// ---------------------------------------------------------------------------

/// Compress to zlib/deflate format
#[cfg(feature = "http-compression")]
fn compress_deflate(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(data, 6)
}

/// Decompress zlib/deflate data
#[cfg(feature = "http-compression")]
fn decompress_deflate(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 2 {
        return None;
    }

    miniz_oxide::inflate::decompress_to_vec_zlib(data).ok()
}

// ---------------------------------------------------------------------------
// brotli compression/decompression (RFC 7932)
// ---------------------------------------------------------------------------

/// Compress to brotli format
#[cfg(feature = "http-compression")]
fn compress_brotli(data: &[u8]) -> Vec<u8> {
    use brotli::enc::backward_references::BrotliEncoderMode;
    use brotli::enc::BrotliEncoderParams;

    let mut out = Vec::with_capacity(data.len());
    let mut params = BrotliEncoderParams::default();
    params.mode = BrotliEncoderMode::BROTLI_MODE_GENERIC;
    params.quality = 4; // Moderate compression

    let mut reader = SliceReader::new(data);
    let mut writer = VecWriter::new(&mut out);
    let mut input = [0u8; 4096];
    let mut output = [0u8; 4096];
    let mut callback =
        |_: &mut brotli::interface::PredictionModeContextMap<brotli::InputReferenceMut>,
         _: &mut [brotli::interface::StaticCommand],
         _: brotli::InputPair,
         _: &mut HeapAllocator| {};

    brotli::BrotliCompressCustomIo(
        &mut reader,
        &mut writer,
        &mut input,
        &mut output,
        &params,
        HeapAllocator,
        &mut callback,
        (),
    )
    .ok();
    out
}

/// Decompress brotli data
#[cfg(feature = "http-compression")]
fn decompress_brotli(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() * 2);
    let mut reader = SliceReader::new(data);
    let mut writer = VecWriter::new(&mut out);
    let mut input = [0u8; 4096];
    let mut output = [0u8; 4096];

    brotli::BrotliDecompressCustomIo(
        &mut reader,
        &mut writer,
        &mut input,
        &mut output,
        HeapAllocator,
        HeapAllocator,
        HeapAllocator,
        (),
    )
    .ok()?;
    Some(out)
}

#[cfg(feature = "http-compression")]
fn skip_zero_terminated(data: &[u8], mut pos: usize) -> Option<usize> {
    while pos < data.len() {
        let byte = data[pos];
        pos += 1;
        if byte == 0 {
            return Some(pos);
        }
    }
    None
}

#[cfg(feature = "http-compression")]
struct SliceReader<'a> {
    data: &'a [u8],
    pos: usize,
}

#[cfg(feature = "http-compression")]
impl<'a> SliceReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
}

#[cfg(feature = "http-compression")]
impl brotli::CustomRead<()> for SliceReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        let remaining = &self.data[self.pos..];
        let len = remaining.len().min(buf.len());
        buf[..len].copy_from_slice(&remaining[..len]);
        self.pos += len;
        Ok(len)
    }
}

#[cfg(feature = "http-compression")]
struct VecWriter<'a> {
    out: &'a mut Vec<u8>,
}

#[cfg(feature = "http-compression")]
impl<'a> VecWriter<'a> {
    fn new(out: &'a mut Vec<u8>) -> Self {
        Self { out }
    }
}

#[cfg(feature = "http-compression")]
impl brotli::CustomWrite<()> for VecWriter<'_> {
    fn write(&mut self, data: &[u8]) -> Result<usize, ()> {
        self.out.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> Result<(), ()> {
        Ok(())
    }
}

#[derive(Default)]
#[cfg(feature = "http-compression")]
struct Rebox<T> {
    b: Box<[T]>,
}

#[cfg(feature = "http-compression")]
impl<T> brotli::SliceWrapper<T> for Rebox<T> {
    fn slice(&self) -> &[T] {
        &self.b
    }
}

#[cfg(feature = "http-compression")]
impl<T> brotli::SliceWrapperMut<T> for Rebox<T> {
    fn slice_mut(&mut self) -> &mut [T] {
        &mut self.b
    }
}

#[derive(Clone, Copy, Default)]
#[cfg(feature = "http-compression")]
struct HeapAllocator;

#[cfg(feature = "http-compression")]
impl<T: Clone + Default> brotli::Allocator<T> for HeapAllocator {
    type AllocatedMemory = Rebox<T>;

    fn alloc_cell(&mut self, len: usize) -> Self::AllocatedMemory {
        Rebox {
            b: vec![T::default(); len].into_boxed_slice(),
        }
    }

    fn free_cell(&mut self, _data: Self::AllocatedMemory) {}
}

#[cfg(feature = "http-compression")]
impl brotli::enc::BrotliAlloc for HeapAllocator {}

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

    #[cfg(feature = "http-compression")]
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
