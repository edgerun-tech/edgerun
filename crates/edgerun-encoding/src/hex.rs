//! Hex encoding and decoding utilities.
//!
//! Consolidated from `edgerun-core/src/util.rs` and `edgerun-virtual-disk/src/bin/block-client.rs`.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

/// Error type for hex decoding operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HexError {
    /// Input string has invalid length (must be even after prefix removal).
    InvalidLength,
    /// Input string contains invalid hex characters.
    InvalidCharacter(char),
}

impl core::fmt::Display for HexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            HexError::InvalidLength => write!(f, "invalid hex length"),
            HexError::InvalidCharacter(c) => write!(f, "invalid hex character: {}", c),
        }
    }
}

/// Decode a single hex nibble (byte character) to its numeric value.
fn hex_byte(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Decode a hex string to bytes, handling optional `0x` prefix and odd-length strings.
fn decode_hex_with_odd_prefix_handling(s: &str) -> Result<Vec<u8>, HexError> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let s = s.strip_prefix("0X").unwrap_or(s);

    // Handle odd-length strings by prepending a '0'
    let (s, mut result) = if !s.len().is_multiple_of(2) {
        let mut result = Vec::with_capacity(s.len() / 2 + 1);
        let first = hex_byte(s.as_bytes()[0]).ok_or(HexError::InvalidCharacter(s.chars().next().unwrap()))?;
        result.push(first);
        (&s[1..], result)
    } else {
        (s, Vec::with_capacity(s.len() / 2))
    };

    for chunk in s.as_bytes().chunks_exact(2) {
        let high = hex_byte(chunk[0]).ok_or(HexError::InvalidCharacter(chunk[0] as char))?;
        let low = hex_byte(chunk[1]).ok_or(HexError::InvalidCharacter(chunk[1] as char))?;
        result.push((high << 4) | low);
    }

    Ok(result)
}

/// Encode bytes to a lowercase hex string without prefix.
///
/// # Examples
/// ```
/// use edgerun_encoding::hex::bytes_to_hex;
/// assert_eq!(bytes_to_hex(&[0x01, 0x02, 0xab, 0xcd]), "0102abcd");
/// ```
pub fn bytes_to_hex(value: &[u8]) -> String {
    let mut result = String::with_capacity(value.len() * 2);
    for &byte in value {
        write!(result, "{:02x}", byte).unwrap();
    }
    result
}

/// Encode bytes to a lowercase hex string with `0x` prefix.
///
/// # Examples
/// ```
/// use edgerun_encoding::hex::bytes_to_hex_prefixed;
/// assert_eq!(bytes_to_hex_prefixed(&[0x01, 0x02, 0xab, 0xcd]), "0x0102abcd");
/// ```
pub fn bytes_to_hex_prefixed(value: &[u8]) -> String {
    let mut result = String::with_capacity(value.len() * 2 + 2);
    result.push_str("0x");
    for &byte in value {
        write!(result, "{:02x}", byte).unwrap();
    }
    result
}

/// Decode a hex string to bytes. Accepts optional `0x` prefix and handles odd-length strings.
///
/// # Examples
/// ```
/// use edgerun_encoding::hex::hex_to_bytes;
/// assert_eq!(hex_to_bytes("0102abcd").unwrap(), vec![0x01, 0x02, 0xab, 0xcd]);
/// assert_eq!(hex_to_bytes("0x0102abcd").unwrap(), vec![0x01, 0x02, 0xab, 0xcd]);
/// assert_eq!(hex_to_bytes("102abcd").unwrap(), vec![0x01, 0x02, 0xab, 0xcd]); // odd length
/// ```
pub fn hex_to_bytes(value: &str) -> Result<Vec<u8>, HexError> {
    decode_hex_with_odd_prefix_handling(value)
}

/// Decode a hex string to bytes, panicking on invalid input.
///
/// Use only in test code or where invalid hex is a logic error.
///
/// # Panics
/// Panics if the input is not valid hex.
pub fn must_hex_to_bytes(value: &str) -> Vec<u8> {
    hex_to_bytes(value).expect("invalid hex string")
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_bytes_to_hex() {
        assert_eq!(bytes_to_hex(&[0x00]), "00");
        assert_eq!(bytes_to_hex(&[0xff]), "ff");
        assert_eq!(bytes_to_hex(&[0x01, 0x02, 0xab, 0xcd]), "0102abcd");
        assert_eq!(bytes_to_hex(&[]), "");
    }

    #[test]
    fn test_bytes_to_hex_prefixed() {
        assert_eq!(bytes_to_hex_prefixed(&[0x00]), "0x00");
        assert_eq!(bytes_to_hex_prefixed(&[0xff]), "0xff");
        assert_eq!(bytes_to_hex_prefixed(&[0x01, 0x02, 0xab, 0xcd]), "0x0102abcd");
        assert_eq!(bytes_to_hex_prefixed(&[]), "0x");
    }

    #[test]
    fn test_hex_to_bytes() {
        assert_eq!(hex_to_bytes("00").unwrap(), vec![0x00]);
        assert_eq!(hex_to_bytes("ff").unwrap(), vec![0xff]);
        assert_eq!(hex_to_bytes("0102abcd").unwrap(), vec![0x01, 0x02, 0xab, 0xcd]);
        assert_eq!(hex_to_bytes("0x0102abcd").unwrap(), vec![0x01, 0x02, 0xab, 0xcd]);
        assert_eq!(hex_to_bytes("0X0102ABCD").unwrap(), vec![0x01, 0x02, 0xab, 0xcd]);
        assert_eq!(hex_to_bytes("102abcd").unwrap(), vec![0x01, 0x02, 0xab, 0xcd]);
        assert_eq!(hex_to_bytes("").unwrap(), vec![]);
    }

    #[test]
    fn test_hex_to_bytes_invalid() {
        assert!(matches!(hex_to_bytes("xyz"), Err(HexError::InvalidCharacter('x'))));
        assert!(matches!(hex_to_bytes("0xxyz"), Err(HexError::InvalidCharacter('x'))));
    }

    #[test]
    fn test_roundtrip() {
        let original = vec![0x00, 0x01, 0xab, 0xff, 0x12, 0x34];
        let encoded = bytes_to_hex(&original);
        let decoded = hex_to_bytes(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_roundtrip_prefixed() {
        let original = vec![0x00, 0x01, 0xab, 0xff, 0x12, 0x34];
        let encoded = bytes_to_hex_prefixed(&original);
        let decoded = hex_to_bytes(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    #[should_panic]
    fn test_must_hex_to_bytes_panics() {
        must_hex_to_bytes("invalid");
    }
}
