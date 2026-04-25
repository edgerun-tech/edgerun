//! Binary string field helpers for protobuf-like manual serialization.
//!
//! Consolidated from identical implementations in:
//! - `edgerun-remote-capability/src/adapters/wifi.rs`
//! - `edgerun-remote-capability/src/adapters/bluetooth.rs`
//! - `edgerun-remote-capability/src/adapters/camera.rs`
//! - `edgerun-remote-capability/src/adapters/microphone.rs`
//! - `edgerun-remote-capability/src/adapters/speaker.rs`
//! - `edgerun-remote-capability/src/adapters/input.rs`
//!
//! Provides helpers for encoding/decoding string fields in binary protocols
//! using cursor-based serialization (length-prefixed UTF-8 strings).

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::str;

/// Error type for string field operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringFieldError {
    /// Invalid UTF-8 in string field.
    InvalidUtf8,
    /// Buffer too short to read string field.
    TruncatedInput,
    /// String field length prefix exceeds buffer.
    LengthExceedsInput,
}

impl core::fmt::Display for StringFieldError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StringFieldError::InvalidUtf8 => write!(f, "invalid UTF-8 in string field"),
            StringFieldError::TruncatedInput => write!(f, "buffer too short for string field"),
            StringFieldError::LengthExceedsInput => write!(f, "string field length exceeds buffer"),
        }
    }
}

/// Encode a string field with a 1-byte length prefix.
///
/// Fails if the string exceeds 255 bytes.
///
/// # Examples
/// ```
/// use edgerun_encoding::string_field::encode_string_field;
/// let encoded = encode_string_field("hello").unwrap();
/// assert_eq!(encoded, vec![0x05, b'h', b'e', b'l', b'l', b'o']);
/// ```
pub fn encode_string_field(value: &str) -> Result<Vec<u8>, StringFieldError> {
    let bytes = value.as_bytes();
    if bytes.len() > 255 {
        return Err(StringFieldError::LengthExceedsInput);
    }

    let mut result = Vec::with_capacity(1 + bytes.len());
    result.push(bytes.len() as u8);
    result.extend_from_slice(bytes);
    Ok(result)
}

/// Decode a string field with a 1-byte length prefix.
///
/// Advances the cursor past the length byte and string bytes.
///
/// # Examples
/// ```
/// use edgerun_encoding::string_field::decode_string_field;
/// let data = vec![0x05, b'h', b'e', b'l', b'l', b'o'];
/// let mut cursor = 0;
/// let s = decode_string_field(&data, &mut cursor).unwrap();
/// assert_eq!(s, "hello");
/// assert_eq!(cursor, 6);
/// ```
pub fn decode_string_field(bytes: &[u8], cursor: &mut usize) -> Result<String, StringFieldError> {
    if *cursor >= bytes.len() {
        return Err(StringFieldError::TruncatedInput);
    }

    let len = bytes[*cursor] as usize;
    *cursor += 1;

    if *cursor + len > bytes.len() {
        return Err(StringFieldError::TruncatedInput);
    }

    let s = str::from_utf8(&bytes[*cursor..*cursor + len])
        .map_err(|_| StringFieldError::InvalidUtf8)?
        .to_string();

    *cursor += len;
    Ok(s)
}

/// Encode an optional string field with a presence byte.
///
/// Format: `[presence: u8][length: u8][bytes...]` where presence is 1 if Some, 0 if None.
///
/// # Examples
/// ```
/// use edgerun_encoding::string_field::encode_optional_string_field;
/// let encoded = encode_optional_string_field(Some("hello")).unwrap();
/// assert_eq!(encoded, vec![0x01, 0x05, b'h', b'e', b'l', b'l', b'o']);
///
/// let encoded = encode_optional_string_field(None).unwrap();
/// assert_eq!(encoded, vec![0x00]);
/// ```
pub fn encode_optional_string_field(value: Option<&str>) -> Result<Vec<u8>, StringFieldError> {
    let mut result = Vec::new();

    match value {
        Some(s) => {
            result.push(1);
            result.extend_from_slice(&encode_string_field(s)?);
        }
        None => {
            result.push(0);
        }
    }

    Ok(result)
}

/// Decode an optional string field with a presence byte.
///
/// # Examples
/// ```
/// use edgerun_encoding::string_field::decode_optional_string_field;
/// let data = vec![0x01, 0x05, b'h', b'e', b'l', b'l', b'o'];
/// let mut cursor = 0;
/// let s = decode_optional_string_field(&data, &mut cursor).unwrap();
/// assert_eq!(s, Some("hello".to_string()));
/// assert_eq!(cursor, 7);
/// ```
pub fn decode_optional_string_field(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<Option<String>, StringFieldError> {
    if *cursor >= bytes.len() {
        return Err(StringFieldError::TruncatedInput);
    }

    let present = bytes[*cursor];
    *cursor += 1;

    if present == 0 {
        Ok(None)
    } else {
        decode_string_field(bytes, cursor).map(Some)
    }
}

/// Encode a string field with a 2-byte length prefix (little-endian).
///
/// Supports strings up to 65535 bytes.
///
/// # Examples
/// ```
/// use edgerun_encoding::string_field::encode_string_field_u16;
/// let encoded = encode_string_field_u16("hello").unwrap();
/// assert_eq!(encoded, vec![0x05, 0x00, b'h', b'e', b'l', b'l', b'o']);
/// ```
pub fn encode_string_field_u16(value: &str) -> Result<Vec<u8>, StringFieldError> {
    let bytes = value.as_bytes();
    if bytes.len() > 65535 {
        return Err(StringFieldError::LengthExceedsInput);
    }

    let mut result = Vec::with_capacity(2 + bytes.len());
    result.push((bytes.len() & 0xFF) as u8);
    result.push(((bytes.len() >> 8) & 0xFF) as u8);
    result.extend_from_slice(bytes);
    Ok(result)
}

/// Decode a string field with a 2-byte length prefix (little-endian).
///
/// # Examples
/// ```
/// use edgerun_encoding::string_field::decode_string_field_u16;
/// let data = vec![0x05, 0x00, b'h', b'e', b'l', b'l', b'o'];
/// let mut cursor = 0;
/// let s = decode_string_field_u16(&data, &mut cursor).unwrap();
/// assert_eq!(s, "hello");
/// assert_eq!(cursor, 7);
/// ```
pub fn decode_string_field_u16(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<String, StringFieldError> {
    if *cursor + 2 > bytes.len() {
        return Err(StringFieldError::TruncatedInput);
    }

    let len = bytes[*cursor] as usize | ((bytes[*cursor + 1] as usize) << 8);
    *cursor += 2;

    if *cursor + len > bytes.len() {
        return Err(StringFieldError::TruncatedInput);
    }

    let s = str::from_utf8(&bytes[*cursor..*cursor + len])
        .map_err(|_| StringFieldError::InvalidUtf8)?
        .to_string();

    *cursor += len;
    Ok(s)
}

/// Encode a fixed-length byte array (useful for IDs, hashes, etc.).
///
/// # Examples
/// ```
/// use edgerun_encoding::string_field::encode_bytes;
/// let encoded = encode_bytes(b"hello").unwrap();
/// assert_eq!(encoded, vec![0x05, b'h', b'e', b'l', b'l', b'o']);
/// ```
pub fn encode_bytes(value: &[u8]) -> Result<Vec<u8>, StringFieldError> {
    if value.len() > 255 {
        return Err(StringFieldError::LengthExceedsInput);
    }

    let mut result = Vec::with_capacity(1 + value.len());
    result.push(value.len() as u8);
    result.extend_from_slice(value);
    Ok(result)
}

/// Decode a fixed-length byte array.
///
/// # Examples
/// ```
/// use edgerun_encoding::string_field::decode_bytes;
/// let data = vec![0x05, b'h', b'e', b'l', b'l', b'o'];
/// let mut cursor = 0;
/// let b = decode_bytes(&data, &mut cursor).unwrap();
/// assert_eq!(b, b"hello");
/// assert_eq!(cursor, 6);
/// ```
pub fn decode_bytes(bytes: &[u8], cursor: &mut usize) -> Result<Vec<u8>, StringFieldError> {
    if *cursor >= bytes.len() {
        return Err(StringFieldError::TruncatedInput);
    }

    let len = bytes[*cursor] as usize;
    *cursor += 1;

    if *cursor + len > bytes.len() {
        return Err(StringFieldError::TruncatedInput);
    }

    let result = bytes[*cursor..*cursor + len].to_vec();
    *cursor += len;
    Ok(result)
}

// ─── u32-LE prefixed variants (for edgerun-remote-capability wire format) ─────

/// Encode a string field with a 4-byte little-endian length prefix.
///
/// This is the wire format used by `edgerun-remote-capability` adapters.
/// Supports strings up to 4 GiB.
///
/// Writes directly into the provided output buffer (cursor-based).
///
/// # Examples
/// ```
/// use edgerun_encoding::string_field::encode_string_field_u32;
/// let mut out = Vec::new();
/// encode_string_field_u32("hello", &mut out).unwrap();
/// assert_eq!(out, vec![0x05, 0x00, 0x00, 0x00, b'h', b'e', b'l', b'l', b'o']);
/// ```
pub fn encode_string_field_u32(value: &str, out: &mut Vec<u8>) -> Result<(), StringFieldError> {
    let bytes = value.as_bytes();
    if bytes.len() > u32::MAX as usize {
        return Err(StringFieldError::LengthExceedsInput);
    }

    let len = bytes.len() as u32;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}

/// Decode a string field with a 4-byte little-endian length prefix.
///
/// This is the wire format used by `edgerun-remote-capability` adapters.
///
/// # Examples
/// ```
/// use edgerun_encoding::string_field::decode_string_field_u32;
/// let data = vec![0x05, 0x00, 0x00, 0x00, b'h', b'e', b'l', b'l', b'o'];
/// let mut cursor = 0;
/// let s = decode_string_field_u32(&data, &mut cursor).unwrap();
/// assert_eq!(s, "hello");
/// assert_eq!(cursor, 9);
/// ```
pub fn decode_string_field_u32(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<String, StringFieldError> {
    if *cursor + 4 > bytes.len() {
        return Err(StringFieldError::TruncatedInput);
    }

    let len = u32::from_le_bytes(
        bytes[*cursor..*cursor + 4]
            .try_into()
            .map_err(|_| StringFieldError::TruncatedInput)?,
    ) as usize;
    *cursor += 4;

    if *cursor + len > bytes.len() {
        return Err(StringFieldError::TruncatedInput);
    }

    let s = str::from_utf8(&bytes[*cursor..*cursor + len])
        .map_err(|_| StringFieldError::InvalidUtf8)?
        .to_string();

    *cursor += len;
    Ok(s)
}

/// Encode an optional string field with u32-LE length prefix and presence byte.
///
/// Format: `[presence: u8][length: u32 LE][bytes...]`
///
/// # Examples
/// ```
/// use edgerun_encoding::string_field::encode_optional_string_field_u32;
/// let mut out = Vec::new();
/// encode_optional_string_field_u32(Some("hello"), &mut out).unwrap();
/// assert_eq!(out, vec![0x01, 0x05, 0x00, 0x00, 0x00, b'h', b'e', b'l', b'l', b'o']);
/// ```
pub fn encode_optional_string_field_u32(
    value: Option<&str>,
    out: &mut Vec<u8>,
) -> Result<(), StringFieldError> {
    match value {
        Some(s) => {
            out.push(1);
            encode_string_field_u32(s, out)?;
        }
        None => {
            out.push(0);
        }
    }
    Ok(())
}

/// Decode an optional string field with u32-LE length prefix and presence byte.
///
/// # Examples
/// ```
/// use edgerun_encoding::string_field::decode_optional_string_field_u32;
/// let data = vec![0x01, 0x05, 0x00, 0x00, 0x00, b'h', b'e', b'l', b'l', b'o'];
/// let mut cursor = 0;
/// let s = decode_optional_string_field_u32(&data, &mut cursor).unwrap();
/// assert_eq!(s, Some("hello".to_string()));
/// assert_eq!(cursor, 10);
/// ```
pub fn decode_optional_string_field_u32(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<Option<String>, StringFieldError> {
    if *cursor >= bytes.len() {
        return Err(StringFieldError::TruncatedInput);
    }

    let present = bytes[*cursor] != 0;
    *cursor += 1;

    if present {
        decode_string_field_u32(bytes, cursor).map(Some)
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn test_encode_string_field() {
        assert_eq!(
            encode_string_field("hello").unwrap(),
            vec![0x05, b'h', b'e', b'l', b'l', b'o']
        );
        assert_eq!(encode_string_field("").unwrap(), vec![0x00]);
    }

    #[test]
    fn test_encode_string_field_too_long() {
        let long_string = "a".repeat(256);
        assert!(matches!(
            encode_string_field(&long_string),
            Err(StringFieldError::LengthExceedsInput)
        ));
    }

    #[test]
    fn test_decode_string_field() {
        let data = vec![0x05, b'h', b'e', b'l', b'l', b'o'];
        let mut cursor = 0;
        let s = decode_string_field(&data, &mut cursor).unwrap();
        assert_eq!(s, "hello");
        assert_eq!(cursor, 6);
    }

    #[test]
    fn test_decode_string_field_truncated() {
        let data = vec![0x05, b'h', b'e'];
        let mut cursor = 0;
        assert!(matches!(
            decode_string_field(&data, &mut cursor),
            Err(StringFieldError::TruncatedInput)
        ));
    }

    #[test]
    fn test_decode_string_field_invalid_utf8() {
        let data = vec![0x02, 0xff, 0xfe];
        let mut cursor = 0;
        assert!(matches!(
            decode_string_field(&data, &mut cursor),
            Err(StringFieldError::InvalidUtf8)
        ));
    }

    #[test]
    fn test_string_field_roundtrip() {
        let test_strings: Vec<&str> = vec!["", "hello", "世界"];
        for s in &test_strings {
            let encoded = encode_string_field(s).unwrap();
            let mut cursor = 0;
            let decoded = decode_string_field(&encoded, &mut cursor).unwrap();
            assert_eq!(*s, decoded);
            assert_eq!(cursor, encoded.len());
        }
    }

    #[test]
    fn test_encode_optional_string_field_some() {
        let encoded = encode_optional_string_field(Some("hello")).unwrap();
        assert_eq!(encoded, vec![0x01, 0x05, b'h', b'e', b'l', b'l', b'o']);
    }

    #[test]
    fn test_encode_optional_string_field_none() {
        let encoded = encode_optional_string_field(None).unwrap();
        assert_eq!(encoded, vec![0x00]);
    }

    #[test]
    fn test_decode_optional_string_field_some() {
        let data = vec![0x01, 0x05, b'h', b'e', b'l', b'l', b'o'];
        let mut cursor = 0;
        let s = decode_optional_string_field(&data, &mut cursor).unwrap();
        assert_eq!(s, Some("hello".to_string()));
        assert_eq!(cursor, 7);
    }

    #[test]
    fn test_decode_optional_string_field_none() {
        let data = vec![0x00];
        let mut cursor = 0;
        let s = decode_optional_string_field(&data, &mut cursor).unwrap();
        assert_eq!(s, None);
        assert_eq!(cursor, 1);
    }

    #[test]
    fn test_encode_string_field_u16() {
        assert_eq!(
            encode_string_field_u16("hello").unwrap(),
            vec![0x05, 0x00, b'h', b'e', b'l', b'l', b'o']
        );
    }

    #[test]
    fn test_decode_string_field_u16() {
        let data = vec![0x05, 0x00, b'h', b'e', b'l', b'l', b'o'];
        let mut cursor = 0;
        let s = decode_string_field_u16(&data, &mut cursor).unwrap();
        assert_eq!(s, "hello");
        assert_eq!(cursor, 7);
    }

    #[test]
    fn test_encode_bytes() {
        assert_eq!(
            encode_bytes(b"hello").unwrap(),
            vec![0x05, b'h', b'e', b'l', b'l', b'o']
        );
    }

    #[test]
    fn test_decode_bytes() {
        let data = vec![0x05, b'h', b'e', b'l', b'l', b'o'];
        let mut cursor = 0;
        let b = decode_bytes(&data, &mut cursor).unwrap();
        assert_eq!(b, b"hello");
        assert_eq!(cursor, 6);
    }

    #[test]
    fn test_bytes_roundtrip() {
        let test_cases: Vec<Vec<u8>> =
            vec![vec![], vec![0x00], vec![0xff, 0xfe, 0xfd], vec![0u8; 200]];
        for b in &test_cases {
            let encoded = encode_bytes(b).unwrap();
            let mut cursor = 0;
            let decoded = decode_bytes(&encoded, &mut cursor).unwrap();
            assert_eq!(*b, decoded);
            assert_eq!(cursor, encoded.len());
        }
    }

    #[test]
    fn test_encode_string_field_u32() {
        let mut out = Vec::new();
        encode_string_field_u32("hello", &mut out).unwrap();
        assert_eq!(
            out,
            vec![0x05, 0x00, 0x00, 0x00, b'h', b'e', b'l', b'l', b'o']
        );
    }

    #[test]
    fn test_decode_string_field_u32() {
        let data = vec![0x05, 0x00, 0x00, 0x00, b'h', b'e', b'l', b'l', b'o'];
        let mut cursor = 0;
        let s = decode_string_field_u32(&data, &mut cursor).unwrap();
        assert_eq!(s, "hello");
        assert_eq!(cursor, 9);
    }

    #[test]
    fn test_string_field_u32_roundtrip() {
        let test_strings: Vec<&str> = vec!["", "hello", "世界", "a".repeat(1000).leak()];
        for s in &test_strings {
            let mut out = Vec::new();
            encode_string_field_u32(s, &mut out).unwrap();
            let mut cursor = 0;
            let decoded = decode_string_field_u32(&out, &mut cursor).unwrap();
            assert_eq!(*s, decoded);
            assert_eq!(cursor, out.len());
        }
    }

    #[test]
    fn test_encode_optional_string_field_u32_some() {
        let mut out = Vec::new();
        encode_optional_string_field_u32(Some("hello"), &mut out).unwrap();
        assert_eq!(
            out,
            vec![0x01, 0x05, 0x00, 0x00, 0x00, b'h', b'e', b'l', b'l', b'o']
        );
    }

    #[test]
    fn test_encode_optional_string_field_u32_none() {
        let mut out = Vec::new();
        encode_optional_string_field_u32(None, &mut out).unwrap();
        assert_eq!(out, vec![0x00]);
    }

    #[test]
    fn test_decode_optional_string_field_u32_some() {
        let data = vec![0x01, 0x05, 0x00, 0x00, 0x00, b'h', b'e', b'l', b'l', b'o'];
        let mut cursor = 0;
        let s = decode_optional_string_field_u32(&data, &mut cursor).unwrap();
        assert_eq!(s, Some("hello".to_string()));
        assert_eq!(cursor, 10);
    }

    #[test]
    fn test_decode_optional_string_field_u32_none() {
        let data = vec![0x00];
        let mut cursor = 0;
        let s = decode_optional_string_field_u32(&data, &mut cursor).unwrap();
        assert_eq!(s, None);
        assert_eq!(cursor, 1);
    }
}
