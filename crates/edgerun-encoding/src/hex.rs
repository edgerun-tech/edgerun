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
        let first = hex_byte(s.as_bytes()[0])
            .ok_or(HexError::InvalidCharacter(s.chars().next().unwrap()))?;
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

// ---------------------------------------------------------------------------
// Hex bitmap parsing
// ---------------------------------------------------------------------------

/// Parse whitespace-separated hex values into a `Vec<u8>`.
///
/// Each token is parsed as a byte. Accepts optional `0x` prefix.
/// Invalid tokens are silently skipped.
///
/// # Examples
/// ```
/// use edgerun_encoding::hex::parse_hex_bitmap;
/// assert_eq!(parse_hex_bitmap("01 02 ab cd"), vec![0x01, 0x02, 0xab, 0xcd]);
/// assert_eq!(parse_hex_bitmap("0x01 0x02 0xff"), vec![0x01, 0x02, 0xff]);
/// ```
pub fn parse_hex_bitmap(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .filter_map(parse_hex_int::<u8>)
        .collect()
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
        assert_eq!(
            bytes_to_hex_prefixed(&[0x01, 0x02, 0xab, 0xcd]),
            "0x0102abcd"
        );
        assert_eq!(bytes_to_hex_prefixed(&[]), "0x");
    }

    #[test]
    fn test_hex_to_bytes() {
        assert_eq!(hex_to_bytes("00").unwrap(), vec![0x00]);
        assert_eq!(hex_to_bytes("ff").unwrap(), vec![0xff]);
        assert_eq!(
            hex_to_bytes("0102abcd").unwrap(),
            vec![0x01, 0x02, 0xab, 0xcd]
        );
        assert_eq!(
            hex_to_bytes("0x0102abcd").unwrap(),
            vec![0x01, 0x02, 0xab, 0xcd]
        );
        assert_eq!(
            hex_to_bytes("0X0102ABCD").unwrap(),
            vec![0x01, 0x02, 0xab, 0xcd]
        );
        assert_eq!(
            hex_to_bytes("102abcd").unwrap(),
            vec![0x01, 0x02, 0xab, 0xcd]
        );
        assert_eq!(hex_to_bytes("").unwrap(), vec![]);
    }

    #[test]
    fn test_hex_to_bytes_invalid() {
        assert!(matches!(
            hex_to_bytes("xyz"),
            Err(HexError::InvalidCharacter('x'))
        ));
        assert!(matches!(
            hex_to_bytes("0xxyz"),
            Err(HexError::InvalidCharacter('x'))
        ));
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

    #[test]
    fn test_bytes_to_hex_sep() {
        assert_eq!(bytes_to_hex_sep(&[0x00], ':'), "00");
        assert_eq!(bytes_to_hex_sep(&[0xab, 0xcd, 0xef], ':'), "ab:cd:ef");
        assert_eq!(bytes_to_hex_sep(&[], ':'), "");
        assert_eq!(bytes_to_hex_sep(&[0xff, 0x00], '-'), "ff-00");
    }

    #[test]
    fn test_parse_mac() {
        let mac = parse_mac("00:11:22:33:44:55").unwrap();
        assert_eq!(mac, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        assert!(parse_mac("00:11:22:33:44").is_none()); // too short
        assert!(parse_mac("00:11:22:33:44:55:66").is_none()); // too long
        assert!(parse_mac("00:11:22:33:44:zz").is_none()); // invalid
        assert!(parse_mac("").is_none());
    }

    #[test]
    fn test_parse_bdaddr() {
        // Bluetooth BDADDR is little-endian reversed
        let addr = parse_bdaddr("00:11:22:33:44:55").unwrap();
        assert_eq!(addr, [0x55, 0x44, 0x33, 0x22, 0x11, 0x00]);
        assert!(parse_bdaddr("invalid").is_none());
    }

    #[test]
    fn test_hex_to_bytes_fixed() {
        let arr: [u8; 4] = hex_to_bytes_fixed("0102abcd").unwrap();
        assert_eq!(arr, [0x01, 0x02, 0xab, 0xcd]);
        assert!(hex_to_bytes_fixed::<4>("0102ab").is_none()); // too short
        assert!(hex_to_bytes_fixed::<4>("0102abcd00").is_none()); // too long
    }

    #[test]
    fn test_parse_hex_int() {
        assert_eq!(parse_hex_int::<u8>("ff"), Some(0xff));
        assert_eq!(parse_hex_int::<u8>("0xff"), Some(0xff));
        assert_eq!(parse_hex_int::<u8>("0XFF"), Some(0xff));
        assert_eq!(parse_hex_int::<u16>("abcd"), Some(0xabcd));
        assert_eq!(parse_hex_int::<u32>("00001234"), Some(0x1234));
        assert_eq!(parse_hex_int::<u64>("deadbeef"), Some(0xdeadbeef));
        assert!(parse_hex_int::<u8>("xyz").is_none());
        assert!(parse_hex_int::<u8>("").is_none());
    }
}

// ---------------------------------------------------------------------------
// MAC address parsing (colon-separated hex)
// ---------------------------------------------------------------------------

/// Parse a colon-separated MAC address string into `[u8; 6]`.
///
/// Accepts lowercase or uppercase hex, exactly 6 octets separated by `:`.
///
/// # Examples
/// ```
/// use edgerun_encoding::hex::parse_mac;
/// let mac = parse_mac("00:11:22:33:44:55").unwrap();
/// assert_eq!(mac, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
/// ```
pub fn parse_mac(s: &str) -> Option<[u8; 6]> {
    let mut result = [0u8; 6];
    let parts = s.split(':');
    let mut i = 0;
    for part in parts {
        if i >= 6 {
            return None;
        }
        let bytes = part.as_bytes();
        if bytes.len() != 2 {
            return None;
        }
        let hi = hex_byte(bytes[0])?;
        let lo = hex_byte(bytes[1])?;
        result[i] = (hi << 4) | lo;
        i += 1;
    }
    if i == 6 {
        Some(result)
    } else {
        None
    }
}

/// Format `[u8; 6]` as a colon-separated MAC address string.
///
/// # Examples
/// ```
/// use edgerun_encoding::hex::format_mac;
/// assert_eq!(format_mac(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]), "00:11:22:33:44:55");
/// ```
pub fn format_mac(bytes: &[u8; 6]) -> String {
    bytes_to_hex_sep(bytes, ':')
}

/// Format the first six bytes as a colon-separated MAC/BDADDR string.
pub fn format_mac_bytes(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 6 {
        return None;
    }
    Some(bytes_to_hex_sep(&bytes[..6], ':'))
}

/// Parse a Bluetooth device address (BDADDR) string into `[u8; 6]`.
///
/// Same as `parse_mac` but reverses the bytes (Bluetooth uses little-endian
/// octet ordering for display).
///
/// # Examples
/// ```
/// use edgerun_encoding::hex::parse_bdaddr;
/// let addr = parse_bdaddr("00:11:22:33:44:55").unwrap();
/// assert_eq!(addr, [0x55, 0x44, 0x33, 0x22, 0x11, 0x00]);
/// ```
pub fn parse_bdaddr(s: &str) -> Option<[u8; 6]> {
    let mut mac = parse_mac(s)?;
    mac.reverse();
    Some(mac)
}

/// Format little-endian Bluetooth address bytes as display-order BDADDR.
pub fn format_bdaddr_le(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 6 {
        return None;
    }
    let mut reversed = [0u8; 6];
    reversed.copy_from_slice(&bytes[..6]);
    reversed.reverse();
    Some(bytes_to_hex_sep(&reversed, ':'))
}

// ---------------------------------------------------------------------------
// Fixed-size hex parsing
// ---------------------------------------------------------------------------

/// Decode a hex string into a fixed-size byte array `[u8; N]`.
///
/// The input must have exactly `N * 2` hex characters (no prefix, no odd length).
///
/// # Examples
/// ```
/// use edgerun_encoding::hex::hex_to_bytes_fixed;
/// let arr: [u8; 4] = hex_to_bytes_fixed("0102abcd").unwrap();
/// assert_eq!(arr, [0x01, 0x02, 0xab, 0xcd]);
/// ```
pub fn hex_to_bytes_fixed<const N: usize>(s: &str) -> Option<[u8; N]> {
    let bytes = s.as_bytes();
    if bytes.len() != N * 2 {
        return None;
    }
    let mut result = [0u8; N];
    for i in 0..N {
        let hi = hex_byte(bytes[i * 2])?;
        let lo = hex_byte(bytes[i * 2 + 1])?;
        result[i] = (hi << 4) | lo;
    }
    Some(result)
}

// ---------------------------------------------------------------------------
// Hex display formatting
// ---------------------------------------------------------------------------

/// Encode bytes to lowercase hex, separated by `sep`.
///
/// # Examples
/// ```
/// use edgerun_encoding::hex::bytes_to_hex_sep;
/// assert_eq!(bytes_to_hex_sep(&[0xab, 0xcd, 0xef], ':'), "ab:cd:ef");
/// ```
pub fn bytes_to_hex_sep(bytes: &[u8], sep: char) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let mut result = String::with_capacity(bytes.len() * 3 - 1);
    write!(result, "{:02x}", bytes[0]).unwrap();
    for &byte in &bytes[1..] {
        result.push(sep);
        write!(result, "{:02x}", byte).unwrap();
    }
    result
}

// ---------------------------------------------------------------------------
// Hex integer parsing (sysfs-style)
// ---------------------------------------------------------------------------

/// Parse a hex string into an unsigned integer.
///
/// Accepts optional `0x` or `0X` prefix. Returns `None` on invalid input
/// or if the value overflows the target type.
///
/// # Examples
/// ```
/// use edgerun_encoding::hex::parse_hex_int;
/// assert_eq!(parse_hex_int::<u8>("ff"), Some(0xff));
/// assert_eq!(parse_hex_int::<u16>("0xabcd"), Some(0xabcd));
/// assert_eq!(parse_hex_int::<u32>("0x00001234"), Some(0x1234));
/// assert!(parse_hex_int::<u8>("xyz").is_none());
/// ```
pub fn parse_hex_int<T>(s: &str) -> Option<T>
where
    T: TryFrom<u128>,
{
    let s = s.strip_prefix("0x").unwrap_or(s);
    let s = s.strip_prefix("0X").unwrap_or(s);
    if s.is_empty() {
        return None;
    }
    let mut result: u128 = 0;
    for &b in s.as_bytes() {
        let nibble = hex_byte(b)?;
        result = result.checked_mul(16)?.checked_add(nibble as u128)?;
    }
    T::try_from(result).ok()
}
