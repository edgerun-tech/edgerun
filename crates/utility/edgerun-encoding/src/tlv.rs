//! TLV (Tag-Length-Value) encoding and decoding utilities.
//!
//! Consolidated from `edgerun-yubikey/src/lib.rs`.
//!
//! Supports:
//! - 1-byte length encoding (values ≤ 127)
//! - 2-byte length encoding (values 128–255, with 0x81 prefix)
//! - 3-byte length encoding (values ≥ 256, with 0x82 prefix)
//! - Parsing of nested TLV structures

use alloc::vec::Vec;
use core::fmt::Display;

/// Error type for TLV operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlvError {
    /// Input too short to parse a complete TLV structure.
    TruncatedInput,
    /// Length field indicates more data than available.
    LengthExceedsInput,
    /// Invalid or unsupported length encoding.
    InvalidLengthEncoding,
    /// Tag not found in TLV map.
    TagNotFound(u8),
}

impl Display for TlvError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TlvError::TruncatedInput => write!(f, "truncated TLV input"),
            TlvError::LengthExceedsInput => write!(f, "TLV length exceeds input"),
            TlvError::InvalidLengthEncoding => write!(f, "invalid TLV length encoding"),
            TlvError::TagNotFound(tag) => write!(f, "TLV tag not found: 0x{:02X}", tag),
        }
    }
}

/// Encode a TLV element.
///
/// Supports:
/// - 1-byte length for values ≤ 127 bytes
/// - 2-byte length (0x81 prefix) for values 128–255 bytes
/// - 3-byte length (0x82 prefix) for values ≥ 256 bytes
///
/// # Examples
/// ```
/// use edgerun_encoding::tlv::encode_tlv;
/// let tlv = encode_tlv(0x42, b"hello").unwrap();
/// assert_eq!(tlv, vec![0x42, 0x05, b'h', b'e', b'l', b'l', b'o']);
/// ```
pub fn encode_tlv(tag: u8, value: &[u8]) -> Result<Vec<u8>, TlvError> {
    let len = value.len();
    let mut result = Vec::with_capacity(1 + 3 + len); // tag + max length + value

    result.push(tag);

    if len <= 127 {
        result.push(len as u8);
    } else if len <= 255 {
        result.push(0x81);
        result.push(len as u8);
    } else if len <= 65535 {
        result.push(0x82);
        result.push((len >> 8) as u8);
        result.push((len & 0xFF) as u8);
    } else {
        // This shouldn't happen for typical TLV use cases
        return Err(TlvError::InvalidLengthEncoding);
    }

    result.extend_from_slice(value);
    Ok(result)
}

/// Parse a sequence of TLV elements into (tag, value) pairs.
///
/// Handles nested TLV structures by parsing sequentially until input is exhausted.
///
/// # Examples
/// ```
/// use edgerun_encoding::tlv::parse_tlv_map;
/// let data = vec![
///     0x01, 0x03, b'f', b'o', b'o',
///     0x02, 0x03, b'b', b'a', b'r',
/// ];
/// let parsed = parse_tlv_map(&data).unwrap();
/// assert_eq!(parsed, vec![(0x01, vec![b'f', b'o', b'o']), (0x02, vec![b'b', b'a', b'r'])]);
/// ```
pub fn parse_tlv_map(bytes: &[u8]) -> Result<Vec<(u8, Vec<u8>)>, TlvError> {
    let mut result = Vec::new();
    let mut offset = 0;

    while offset < bytes.len() {
        if offset + 1 > bytes.len() {
            return Err(TlvError::TruncatedInput);
        }

        let tag = bytes[offset];
        offset += 1;

        // Parse length
        if offset >= bytes.len() {
            return Err(TlvError::TruncatedInput);
        }

        let (length, len_bytes) = parse_tlv_length(&bytes[offset..])?;
        offset += len_bytes;

        // Parse value
        if offset + length > bytes.len() {
            return Err(TlvError::LengthExceedsInput);
        }

        let value = bytes[offset..offset + length].to_vec();
        offset += length;

        result.push((tag, value));
    }

    Ok(result)
}

/// Parse a TLV length field.
/// Returns `(length, bytes_consumed)`.
fn parse_tlv_length(bytes: &[u8]) -> Result<(usize, usize), TlvError> {
    if bytes.is_empty() {
        return Err(TlvError::TruncatedInput);
    }

    let first = bytes[0];

    if first & 0x80 == 0 {
        // Short form: length is in the byte itself (≤ 127)
        Ok((first as usize, 1))
    } else {
        let num_bytes = (first & 0x7F) as usize;
        if num_bytes == 0 || num_bytes > 2 {
            return Err(TlvError::InvalidLengthEncoding);
        }

        if bytes.len() < 1 + num_bytes {
            return Err(TlvError::TruncatedInput);
        }

        let mut length = 0usize;
        for i in 0..num_bytes {
            length = (length << 8) | (bytes[1 + i] as usize);
        }

        Ok((length, 1 + num_bytes))
    }
}

/// Parse a single TLV element from bytes.
///
/// Returns `(tag, value, bytes_consumed)`.
///
/// # Examples
/// ```
/// use edgerun_encoding::tlv::parse_single_tlv;
/// let data = vec![0x42, 0x05, b'h', b'e', b'l', b'l', b'o'];
/// let (tag, value, consumed) = parse_single_tlv(&data).unwrap();
/// assert_eq!(tag, 0x42);
/// assert_eq!(value, b"hello");
/// assert_eq!(consumed, 7);
/// ```
pub fn parse_single_tlv(bytes: &[u8]) -> Result<(u8, Vec<u8>, usize), TlvError> {
    if bytes.len() < 2 {
        return Err(TlvError::TruncatedInput);
    }

    let tag = bytes[0];
    let (length, len_bytes) = parse_tlv_length(&bytes[1..])?;
    let header_len = 1 + len_bytes;

    if header_len + length > bytes.len() {
        return Err(TlvError::LengthExceedsInput);
    }

    let value = bytes[header_len..header_len + length].to_vec();
    let total_consumed = header_len + length;

    Ok((tag, value, total_consumed))
}

/// Build a nested TLV structure from a tree of (tag, children) pairs.
///
/// If `children` is empty, encodes as a simple TLV.
/// If `children` has entries, recursively encodes them as the value.
///
/// # Examples
/// ```
/// use edgerun_encoding::tlv::encode_nested_tlv;
/// // Nested TLV with two children
/// let children: Vec<(u8, Vec<u8>)> = vec![
///     (0x01, vec![b'a', b'b']),
///     (0x02, vec![b'c', b'd']),
/// ];
/// let tlv = encode_nested_tlv(0x80, &children).unwrap();
/// assert_eq!(tlv[0], 0x80);
/// ```
pub fn encode_nested_tlv(tag: u8, children: &[(u8, Vec<u8>)]) -> Result<Vec<u8>, TlvError> {
    if children.is_empty() {
        // This shouldn't happen — use encode_tlv for single values
        return encode_tlv(tag, &[]);
    }

    // Build value from children
    let mut value = Vec::new();
    for (child_tag, child_value) in children {
        let encoded = encode_tlv(*child_tag, child_value)?;
        value.extend_from_slice(&encoded);
    }

    encode_tlv(tag, &value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_encode_tlv_short() {
        let tlv = encode_tlv(0x42, b"hello").unwrap();
        assert_eq!(tlv, vec![0x42, 0x05, b'h', b'e', b'l', b'l', b'o']);
    }

    #[test]
    fn test_encode_tlv_medium() {
        let value = vec![0u8; 200];
        let tlv = encode_tlv(0x01, &value).unwrap();
        assert_eq!(tlv[0], 0x01);
        assert_eq!(tlv[1], 0x81);
        assert_eq!(tlv[2], 200);
        assert_eq!(tlv.len(), 3 + 200);
    }

    #[test]
    fn test_encode_tlv_long() {
        let value = vec![0u8; 300];
        let tlv = encode_tlv(0x01, &value).unwrap();
        assert_eq!(tlv[0], 0x01);
        assert_eq!(tlv[1], 0x82);
        assert_eq!(tlv[2], 0x01);
        assert_eq!(tlv[3], 0x2C); // 300 = 0x012C
        assert_eq!(tlv.len(), 4 + 300);
    }

    #[test]
    fn test_parse_tlv_map() {
        let data = vec![0x01, 0x03, b'f', b'o', b'o', 0x02, 0x03, b'b', b'a', b'r'];
        let parsed = parse_tlv_map(&data).unwrap();
        assert_eq!(
            parsed,
            vec![
                (0x01, vec![b'f', b'o', b'o']),
                (0x02, vec![b'b', b'a', b'r']),
            ]
        );
    }

    #[test]
    fn test_parse_tlv_map_nested() {
        // Nested TLV: outer tag 0x80 containing two inner TLVs
        let inner1 = encode_tlv(0x01, b"foo").unwrap();
        let inner2 = encode_tlv(0x02, b"bar").unwrap();
        let mut inner_data = Vec::new();
        inner_data.extend_from_slice(&inner1);
        inner_data.extend_from_slice(&inner2);

        let outer = encode_tlv(0x80, &inner_data).unwrap();
        let parsed = parse_tlv_map(&outer).unwrap();

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, 0x80);
        // Parse inner
        let inner_parsed = parse_tlv_map(&parsed[0].1).unwrap();
        assert_eq!(
            inner_parsed,
            vec![
                (0x01, vec![b'f', b'o', b'o']),
                (0x02, vec![b'b', b'a', b'r']),
            ]
        );
    }

    #[test]
    fn test_parse_single_tlv() {
        let data = vec![0x42, 0x05, b'h', b'e', b'l', b'l', b'o'];
        let (tag, value, consumed) = parse_single_tlv(&data).unwrap();
        assert_eq!(tag, 0x42);
        assert_eq!(value, b"hello");
        assert_eq!(consumed, 7);
    }

    #[test]
    fn test_parse_single_tlv_truncated() {
        assert!(matches!(
            parse_single_tlv(&[0x42]),
            Err(TlvError::TruncatedInput)
        ));
        assert!(matches!(
            parse_single_tlv(&[0x42, 0x05, b'h']),
            Err(TlvError::LengthExceedsInput)
        ));
    }

    #[test]
    fn test_tlv_roundtrip() {
        let original = vec![
            (0x01u8, vec![b'a', b'b', b'c']),
            (0x02u8, vec![b'd', b'e', b'f']),
        ];

        let mut encoded = Vec::new();
        for (tag, value) in &original {
            encoded.extend_from_slice(&encode_tlv(*tag, value).unwrap());
        }

        let parsed = parse_tlv_map(&encoded).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_encode_nested_tlv() {
        let children = vec![(0x01u8, vec![b'a', b'b']), (0x02u8, vec![b'c', b'd'])];
        let nested = encode_nested_tlv(0x80, &children).unwrap();

        // Parse outer
        let (tag, value, _) = parse_single_tlv(&nested).unwrap();
        assert_eq!(tag, 0x80);

        // Parse inner
        let inner = parse_tlv_map(&value).unwrap();
        assert_eq!(inner, children);
    }
}
