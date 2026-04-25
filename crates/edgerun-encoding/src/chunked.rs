//! Chunked transfer encoding (RFC 9112 §7.1).
//!
//! Wire format per chunk: `<hex-size>[;extension]\r\n<data>\r\n`
//! Terminating chunk: `0\r\n<trailer-headers>\r\n\r\n`

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Error type for chunked encoding operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkedError {
    /// Incomplete chunked body (truncated mid-chunk).
    Incomplete,
    /// Invalid hex chunk size.
    InvalidChunkSize,
    /// Missing CRLF after chunk data.
    MissingCrlf,
    /// Invalid UTF-8 in chunk extension.
    InvalidUtf8,
}

impl core::fmt::Display for ChunkedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChunkedError::Incomplete => write!(f, "incomplete chunked body"),
            ChunkedError::InvalidChunkSize => write!(f, "invalid chunk size"),
            ChunkedError::MissingCrlf => write!(f, "missing CRLF after chunk"),
            ChunkedError::InvalidUtf8 => write!(f, "invalid UTF-8"),
        }
    }
}

/// Decode a chunked transfer-encoded body from bytes.
///
/// Returns `(body_bytes, trailer_bytes)`.
/// Each chunk is: `<hex-size>[;extension]\r\n<data>\r\n`
/// Ends with: `0\r\n<trailers>\r\n\r\n`
///
/// # Examples
/// ```
/// use edgerun_encoding::chunked::decode_chunked;
/// let data = b"5\r\nHello\r\n0\r\n\r\n";
/// let (body, trailers) = decode_chunked(data).unwrap();
/// assert_eq!(body, b"Hello");
/// assert!(trailers.is_empty());
/// ```
pub fn decode_chunked(data: &[u8]) -> Result<(Vec<u8>, Vec<u8>), ChunkedError> {
    let mut body = Vec::new();
    let mut pos = 0;

    loop {
        let crlf = find_crlf(data, pos).ok_or(ChunkedError::Incomplete)?;
        let size_line =
            core::str::from_utf8(&data[pos..crlf]).map_err(|_| ChunkedError::InvalidUtf8)?;

        let size_str = size_line.split(';').next().unwrap_or(size_line).trim();
        let chunk_size =
            usize::from_str_radix(size_str, 16).map_err(|_| ChunkedError::InvalidChunkSize)?;

        pos = crlf + 2;

        if chunk_size == 0 {
            // Last chunk. If only \r\n remains, no trailers.
            let remaining = &data[pos..];
            if remaining == b"\r\n" {
                return Ok((body, Vec::new()));
            }
            let trailer_end = find_double_crlf(data, pos).unwrap_or(data.len());
            let trailers = data[pos..trailer_end].to_vec();
            return Ok((body, trailers));
        }

        let data_end = pos + chunk_size;
        if data_end > data.len() {
            return Err(ChunkedError::Incomplete);
        }
        body.extend_from_slice(&data[pos..data_end]);
        pos = data_end;

        if data.len() < pos + 2 || data[pos] != b'\r' || data[pos + 1] != b'\n' {
            return Err(ChunkedError::MissingCrlf);
        }
        pos += 2;
    }
}

/// Encode bytes as a single chunk: `<hex-size>\r\n<data>\r\n`.
///
/// # Examples
/// ```
/// use edgerun_encoding::chunked::encode_chunk;
/// assert_eq!(encode_chunk(b"Hello"), b"5\r\nHello\r\n");
/// ```
pub fn encode_chunk(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + data.len());
    out.extend_from_slice(format!("{:x}\r\n", data.len()).as_bytes());
    out.extend_from_slice(data);
    out.extend_from_slice(b"\r\n");
    out
}

/// Encode bytes as a complete chunked body (single chunk + terminator).
///
/// # Examples
/// ```
/// use edgerun_encoding::chunked::encode_chunked;
/// assert_eq!(encode_chunked(b"Hello"), b"5\r\nHello\r\n0\r\n\r\n");
/// ```
pub fn encode_chunked(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return b"0\r\n\r\n".to_vec();
    }
    let mut out = encode_chunk(data);
    out.extend_from_slice(b"0\r\n\r\n");
    out
}

/// Encode multiple chunks followed by the terminating chunk.
pub fn encode_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in chunks {
        out.extend_from_slice(&encode_chunk(chunk));
    }
    out.extend_from_slice(b"0\r\n\r\n");
    out
}

/// Encode data as chunked body with trailer headers.
pub fn encode_chunked_with_trailers(data: &[u8], trailers: &[(String, String)]) -> Vec<u8> {
    let mut out = if data.is_empty() {
        Vec::new()
    } else {
        encode_chunk(data)
    };
    out.extend_from_slice(b"0\r\n");
    for (name, value) in trailers {
        out.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    out.extend_from_slice(b"\r\n");
    out
}

fn find_crlf(data: &[u8], pos: usize) -> Option<usize> {
    data[pos..]
        .windows(2)
        .position(|w| w == b"\r\n")
        .map(|i| pos + i)
}

fn find_double_crlf(data: &[u8], pos: usize) -> Option<usize> {
    data[pos..]
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| pos + i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_simple() {
        let (body, trailers) = decode_chunked(b"5\r\nHello\r\n0\r\n\r\n").unwrap();
        assert_eq!(body, b"Hello");
        assert!(trailers.is_empty());
    }

    #[test]
    fn test_decode_multiple() {
        let (body, _) = decode_chunked(b"3\r\nHel\r\n8\r\nlo World\r\n0\r\n\r\n").unwrap();
        assert_eq!(body, b"Hello World");
    }

    #[test]
    fn test_decode_with_trailers() {
        let (body, trailers) = decode_chunked(b"5\r\nHello\r\n0\r\nX-Foo: bar\r\n\r\n").unwrap();
        assert_eq!(body, b"Hello");
        assert_eq!(trailers, b"X-Foo: bar");
    }

    #[test]
    fn test_decode_extension() {
        let (body, _) = decode_chunked(b"5;foo=bar\r\nHello\r\n0\r\n\r\n").unwrap();
        assert_eq!(body, b"Hello");
    }

    #[test]
    fn test_decode_incomplete() {
        assert!(decode_chunked(b"5\r\nHel").is_err());
    }

    #[test]
    fn test_encode_chunk() {
        assert_eq!(encode_chunk(b"Hello"), b"5\r\nHello\r\n");
    }

    #[test]
    fn test_encode_chunked() {
        assert_eq!(encode_chunked(b"Hello"), b"5\r\nHello\r\n0\r\n\r\n");
        assert_eq!(encode_chunked(b""), b"0\r\n\r\n");
    }

    #[test]
    fn test_roundtrip() {
        let original = b"Hello World";
        let encoded = encode_chunked(original);
        let (decoded, _) = decode_chunked(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_encode_multiple() {
        let result = encode_chunks(&[b"Hel", b"lo World"]);
        assert_eq!(result, b"3\r\nHel\r\n8\r\nlo World\r\n0\r\n\r\n");
    }
}
