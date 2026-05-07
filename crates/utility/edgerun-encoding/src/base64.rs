//! Base64 encoding and decoding utilities.
//!
//! Consolidated from:
//! - `edgerun-oauth/src/base64url.rs` (Base64-URL, RFC 4648)
//! - `edgerun-oci/src/registry/base64.rs` (Standard Base64)
//! - `edgerun-protocols/src/dns/tsig.rs` (Standard Base64)
//! - `edgerun-protocols/src/dns/doh.rs` (Base64-URL decode)
//! - `edgerun-email/src/smtp/server/session.rs` (Standard Base64, SASL)
//! - `edgerun-email/src/imap/server.rs` (Standard Base64, SASL)
//! - `edgerun-email/src/smtp/client/builder.rs` (Standard Base64 with line wrapping)
//! - `edgerun-node http/src/http1/upgrade.rs` (Custom u64 base64)

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Standard Base64 alphabet (RFC 4648)
const STANDARD_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64-URL alphabet (RFC 4648, Section 5)
const URLSAFE_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Encode 3 bytes to 4 Base64 characters using the given alphabet.
fn encode_tripplet(bytes: &[u8; 3], alphabet: &[u8; 64]) -> [u8; 4] {
    let n = ((bytes[0] as u32) << 16) | ((bytes[1] as u32) << 8) | (bytes[2] as u32);
    [
        alphabet[((n >> 18) & 0x3F) as usize],
        alphabet[((n >> 12) & 0x3F) as usize],
        alphabet[((n >> 6) & 0x3F) as usize],
        alphabet[(n & 0x3F) as usize],
    ]
}

/// Decode 4 Base64 characters to 3 bytes using the given reverse lookup table.
fn decode_quad(chars: &[u8; 4], reverse: &[u8; 256]) -> Option<[u8; 3]> {
    let r0 = reverse[chars[0] as usize];
    let r1 = reverse[chars[1] as usize];
    let r2 = reverse[chars[2] as usize];
    let r3 = reverse[chars[3] as usize];

    // 0xFF means invalid character
    if r0 == 0xFF || r1 == 0xFF || r2 == 0xFF || r3 == 0xFF {
        return None;
    }

    let n = (r0 as u32) << 18 | (r1 as u32) << 12 | (r2 as u32) << 6 | (r3 as u32);
    Some([(n >> 16) as u8, (n >> 8) as u8, n as u8])
}

/// Build a reverse lookup table from a Base64 alphabet.
fn build_reverse(alphabet: &[u8; 64]) -> [u8; 256] {
    let mut reverse = [0xFF; 256];
    for (i, &byte) in alphabet.iter().enumerate() {
        reverse[byte as usize] = i as u8;
    }
    reverse
}

const STANDARD_REVERSE: [u8; 256] = {
    let mut r = [0xFF; 256];
    let mut i = 0;
    while i < STANDARD_ALPHABET.len() {
        r[STANDARD_ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    r
};

const URLSAFE_REVERSE: [u8; 256] = {
    let mut r = [0xFF; 256];
    let mut i = 0;
    while i < URLSAFE_ALPHABET.len() {
        r[URLSAFE_ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    r
};

/// Decode 4 Base64 characters to 3 bytes (standard alphabet).
fn decode_standard_quad(chars: &[u8; 4]) -> Option<[u8; 3]> {
    decode_quad(chars, &STANDARD_REVERSE)
}

/// Decode 4 Base64 characters to 3 bytes (URL-safe alphabet).
fn decode_urlsafe_quad(chars: &[u8; 4]) -> Option<[u8; 3]> {
    decode_quad(chars, &URLSAFE_REVERSE)
}

/// Encoded length for unpadded Base64-URL.
pub const fn base64url_nopad_encoded_len(input_len: usize) -> usize {
    (input_len / 3) * 4
        + match input_len % 3 {
            0 => 0,
            1 => 2,
            _ => 3,
        }
}

/// Upper bound for decoded Base64-URL bytes.
pub const fn base64url_decoded_bound(input_len: usize) -> usize {
    ((input_len + 3) / 4) * 3
}

/// Encode bytes to unpadded Base64-URL into a caller-provided buffer.
pub fn base64url_nopad_encode_into(input: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    let needed = base64url_nopad_encoded_len(input.len());
    if out.len() < needed {
        return Err("output too short");
    }
    let chunks = input.len() / 3;
    let mut o = 0;
    for i in 0..chunks {
        let tri = [input[i * 3], input[i * 3 + 1], input[i * 3 + 2]];
        let quad = encode_tripplet(&tri, URLSAFE_ALPHABET);
        out[o..o + 4].copy_from_slice(&quad);
        o += 4;
    }
    match input.len() % 3 {
        0 => {}
        1 => {
            let tri = [input[chunks * 3], 0, 0];
            let quad = encode_tripplet(&tri, URLSAFE_ALPHABET);
            out[o..o + 2].copy_from_slice(&quad[..2]);
            o += 2;
        }
        2 => {
            let tri = [input[chunks * 3], input[chunks * 3 + 1], 0];
            let quad = encode_tripplet(&tri, URLSAFE_ALPHABET);
            out[o..o + 3].copy_from_slice(&quad[..3]);
            o += 3;
        }
        _ => unreachable!(),
    }
    Ok(o)
}

/// Decode unpadded Base64-URL from bytes into a caller-provided buffer.
pub fn base64url_decode_into(input: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    if input.len() % 4 == 1 {
        return Err("invalid base64url length");
    }
    if out.len() < base64url_decoded_bound(input.len()) {
        return Err("output too short");
    }
    let mut i = 0;
    let mut o = 0;
    while i + 4 <= input.len() {
        let quad = [input[i], input[i + 1], input[i + 2], input[i + 3]];
        let decoded = decode_urlsafe_quad(&quad).ok_or("invalid base64url character")?;
        out[o..o + 3].copy_from_slice(&decoded);
        i += 4;
        o += 3;
    }
    match input.len() - i {
        0 => {}
        2 => {
            let a = URLSAFE_REVERSE[input[i] as usize];
            let b = URLSAFE_REVERSE[input[i + 1] as usize];
            if a == 0xff || b == 0xff {
                return Err("invalid base64url character");
            }
            out[o] = (a << 2) | (b >> 4);
            o += 1;
        }
        3 => {
            let a = URLSAFE_REVERSE[input[i] as usize];
            let b = URLSAFE_REVERSE[input[i + 1] as usize];
            let c = URLSAFE_REVERSE[input[i + 2] as usize];
            if a == 0xff || b == 0xff || c == 0xff {
                return Err("invalid base64url character");
            }
            out[o] = (a << 2) | (b >> 4);
            out[o + 1] = ((b & 0x0f) << 4) | (c >> 2);
            o += 2;
        }
        _ => return Err("invalid base64url length"),
    }
    Ok(o)
}

// ─── Standard Base64 ────────────────────────────────────────────────────────

/// Encode bytes to standard Base64 (RFC 4648) with padding.
///
/// Uses `+` and `/` characters. Output length is always a multiple of 4.
///
/// # Examples
/// ```
/// use edgerun_encoding::base64::standard_encode;
/// assert_eq!(standard_encode(b"Hello"), "SGVsbG8=");
/// assert_eq!(standard_encode(b"Hello, World!"), "SGVsbG8sIFdvcmxkIQ==");
/// ```
pub fn standard_encode(data: &[u8]) -> String {
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    let chunks = data.len() / 3;

    for i in 0..chunks {
        let tri = [data[i * 3], data[i * 3 + 1], data[i * 3 + 2]];
        let quad = encode_tripplet(&tri, STANDARD_ALPHABET);
        result.push_str(core::str::from_utf8(&quad).unwrap());
    }

    match data.len() % 3 {
        0 => {}
        1 => {
            let tri = [data[chunks * 3], 0, 0];
            let quad = encode_tripplet(&tri, STANDARD_ALPHABET);
            result.push_str(core::str::from_utf8(&quad[..2]).unwrap());
            result.push_str("==");
        }
        2 => {
            let tri = [data[chunks * 3], data[chunks * 3 + 1], 0];
            let quad = encode_tripplet(&tri, STANDARD_ALPHABET);
            result.push_str(core::str::from_utf8(&quad[..3]).unwrap());
            result.push('=');
        }
        _ => unreachable!(),
    }

    result
}

/// Decode standard Base64 with padding to bytes.
///
/// # Examples
/// ```
/// use edgerun_encoding::base64::standard_decode;
/// assert_eq!(standard_decode("SGVsbG8=").unwrap(), b"Hello");
/// assert_eq!(standard_decode("SGVsbG8sIFdvcmxkIQ==").unwrap(), b"Hello, World!");
/// ```
pub fn standard_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    let input = input.trim_end_matches('=');
    let bytes = input.as_bytes();
    let mut result = Vec::with_capacity(bytes.len() / 4 * 3 + 3);

    let mut i = 0;
    while i + 4 <= bytes.len() {
        let quad = [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]];
        let decoded = decode_standard_quad(&quad).ok_or("invalid base64 character")?;
        result.extend_from_slice(&decoded);
        i += 4;
    }

    if i < bytes.len() {
        let remaining = &bytes[i..];
        let len = remaining.len();
        let mut padded = [b'A'; 4];
        padded[..len].copy_from_slice(remaining);
        while padded.len() < 4 {
            padded[len] = b'A';
        }
        let decoded = decode_standard_quad(&padded).ok_or("invalid base64 character")?;
        result.extend_from_slice(&decoded[..len - 1]);
    }

    Ok(result)
}

/// Encode bytes to standard Base64 with RFC 2045 line wrapping (76 chars).
///
/// Used for MIME email encoding. Lines are separated by `\r\n`.
///
/// # Examples
/// ```
/// use edgerun_encoding::base64::standard_encode_wrapped;
/// let long_data = vec![0u8; 100];
/// let encoded = standard_encode_wrapped(&long_data);
/// let encoded_str = core::str::from_utf8(&encoded).unwrap();
/// assert!(encoded_str.contains("\r\n"));
/// ```
pub fn standard_encode_wrapped(data: &[u8]) -> Vec<u8> {
    let encoded = standard_encode(data);
    let mut result = Vec::with_capacity(encoded.len() + encoded.len() / 76 * 2 + 2);
    let bytes = encoded.as_bytes();

    for (i, chunk) in bytes.chunks(76).enumerate() {
        if i > 0 {
            result.extend_from_slice(b"\r\n");
        }
        result.extend_from_slice(chunk);
    }

    result
}

// ─── Base64-URL ──────────────────────────────────────────────────────────────

/// Encode bytes to Base64-URL without padding (RFC 4648, Section 5).
///
/// Uses `-` and `_` characters. No `=` padding.
///
/// # Examples
/// ```
/// use edgerun_encoding::base64::base64url_nopad_encode;
/// assert_eq!(base64url_nopad_encode(b"Hello"), "SGVsbG8");
/// ```
pub fn base64url_nopad_encode(data: &[u8]) -> String {
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    let chunks = data.len() / 3;

    for i in 0..chunks {
        let tri = [data[i * 3], data[i * 3 + 1], data[i * 3 + 2]];
        let quad = encode_tripplet(&tri, URLSAFE_ALPHABET);
        result.push_str(core::str::from_utf8(&quad).unwrap());
    }

    match data.len() % 3 {
        0 => {}
        1 => {
            let tri = [data[chunks * 3], 0, 0];
            let quad = encode_tripplet(&tri, URLSAFE_ALPHABET);
            result.push_str(core::str::from_utf8(&quad[..2]).unwrap());
        }
        2 => {
            let tri = [data[chunks * 3], data[chunks * 3 + 1], 0];
            let quad = encode_tripplet(&tri, URLSAFE_ALPHABET);
            result.push_str(core::str::from_utf8(&quad[..3]).unwrap());
        }
        _ => unreachable!(),
    }

    result
}

/// Encode bytes to Base64-URL with padding.
///
/// Uses `-` and `_` characters. Output length is always a multiple of 4.
///
/// # Examples
/// ```
/// use edgerun_encoding::base64::base64url_encode;
/// assert_eq!(base64url_encode(b"Hello"), "SGVsbG8=");
/// ```
pub fn base64url_encode(data: &[u8]) -> String {
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    let chunks = data.len() / 3;

    for i in 0..chunks {
        let tri = [data[i * 3], data[i * 3 + 1], data[i * 3 + 2]];
        let quad = encode_tripplet(&tri, URLSAFE_ALPHABET);
        result.push_str(core::str::from_utf8(&quad).unwrap());
    }

    match data.len() % 3 {
        0 => {}
        1 => {
            let tri = [data[chunks * 3], 0, 0];
            let quad = encode_tripplet(&tri, URLSAFE_ALPHABET);
            result.push_str(core::str::from_utf8(&quad[..2]).unwrap());
            result.push_str("==");
        }
        2 => {
            let tri = [data[chunks * 3], data[chunks * 3 + 1], 0];
            let quad = encode_tripplet(&tri, URLSAFE_ALPHABET);
            result.push_str(core::str::from_utf8(&quad[..3]).unwrap());
            result.push('=');
        }
        _ => unreachable!(),
    }

    result
}

/// Decode Base64-URL (with or without padding) to bytes.
///
/// Accepts both `-`/`_` and `+`/`/` alphabets for compatibility.
///
/// # Examples
/// ```
/// use edgerun_encoding::base64::base64url_decode;
/// assert_eq!(base64url_decode("SGVsbG8").unwrap(), b"Hello");
/// assert_eq!(base64url_decode("SGVsbG8=").unwrap(), b"Hello");
/// ```
pub fn base64url_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    // Convert Base64-URL to standard Base64 for decoding
    let input = input.replace('-', "+").replace('_', "/");
    standard_decode(&input)
}

/// Convert Base64-URL to standard Base64, then decode.
///
/// This is the pattern used in `edgerun-protocols/src/dns/doh.rs`.
///
/// # Examples
/// ```
/// use edgerun_encoding::base64::base64url_to_standard_decode;
/// assert_eq!(base64url_to_standard_decode("SGVsbG8=").unwrap(), b"Hello");
/// ```
pub fn base64url_to_standard_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    // Convert base64url chars to standard base64 chars, add padding
    let converted = input.replace('-', "+").replace('_', "/");
    let padded = match converted.len() % 4 {
        0 => converted,
        2 => format!("{}==", converted),
        3 => format!("{}=", converted),
        _ => converted,
    };
    standard_decode(&padded)
}

// ─── Specialized: u64 to Base64 (WebSocket accept keys) ──────────────────────

/// Encode a u64 to a fixed 12-character Base64 string.
///
/// Used for WebSocket accept keys (`edgerun-node http/src/http1/upgrade.rs`).
/// Output is always exactly 12 characters, no padding.
///
/// # Examples
/// ```
/// use edgerun_encoding::base64::encode_u64_base64;
/// let encoded = encode_u64_base64(12345);
/// assert_eq!(encoded.len(), 12);
/// ```
pub fn encode_u64_base64(value: u64) -> String {
    // 64 bits fits in 11 base64 chars, we pad to 12 for consistency
    let mut result = String::with_capacity(12);
    let mut n = value;
    let mut chars = [0u8; 12];

    // Encode in little-endian base64
    for i in (0..11).rev() {
        chars[i] = STANDARD_ALPHABET[(n & 0x3F) as usize];
        n >>= 6;
    }
    chars[11] = STANDARD_ALPHABET[(n & 0x3F) as usize];

    result.push_str(core::str::from_utf8(&chars).unwrap());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_standard_encode() {
        assert_eq!(standard_encode(b""), "");
        assert_eq!(standard_encode(b"f"), "Zg==");
        assert_eq!(standard_encode(b"fo"), "Zm8=");
        assert_eq!(standard_encode(b"foo"), "Zm9v");
        assert_eq!(standard_encode(b"foob"), "Zm9vYg==");
        assert_eq!(standard_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(standard_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn test_standard_decode() {
        assert_eq!(standard_decode("").unwrap(), b"");
        assert_eq!(standard_decode("Zg==").unwrap(), b"f");
        assert_eq!(standard_decode("Zm8=").unwrap(), b"fo");
        assert_eq!(standard_decode("Zm9v").unwrap(), b"foo");
        assert_eq!(standard_decode("Zm9vYg==").unwrap(), b"foob");
        assert_eq!(standard_decode("Zm9vYmE=").unwrap(), b"fooba");
        assert_eq!(standard_decode("Zm9vYmFy").unwrap(), b"foobar");
    }

    #[test]
    fn test_standard_roundtrip() {
        let test_cases: Vec<Vec<u8>> = vec![
            vec![],
            vec![0x00],
            vec![0xff],
            vec![0x00, 0x01, 0x02],
            vec![0xab, 0xcd, 0xef, 0x12],
            vec![0u8; 100],
        ];
        for data in &test_cases {
            let encoded = standard_encode(data);
            let decoded = standard_decode(&encoded).unwrap();
            assert_eq!(*data, decoded);
        }
    }

    #[test]
    fn test_standard_encode_wrapped() {
        let data = vec![0u8; 100];
        let encoded = standard_encode_wrapped(&data);
        let encoded_str = core::str::from_utf8(&encoded).unwrap();
        assert!(encoded_str.contains("\r\n"));
        // Verify no line exceeds 76 chars
        for line in encoded_str.split("\r\n") {
            assert!(line.len() <= 76);
        }
    }

    #[test]
    fn test_base64url_nopad_encode() {
        assert_eq!(base64url_nopad_encode(b"f"), "Zg");
        assert_eq!(base64url_nopad_encode(b"fo"), "Zm8");
        assert_eq!(base64url_nopad_encode(b"foo"), "Zm9v");
        assert_eq!(base64url_nopad_encode(b"foob"), "Zm9vYg");
    }

    #[test]
    fn test_base64url_encode() {
        assert_eq!(base64url_encode(b"f"), "Zg==");
        assert_eq!(base64url_encode(b"fo"), "Zm8=");
        assert_eq!(base64url_encode(b"foo"), "Zm9v");
    }

    #[test]
    fn test_base64url_decode() {
        assert_eq!(base64url_decode("Zg").unwrap(), b"f");
        assert_eq!(base64url_decode("Zg==").unwrap(), b"f");
        assert_eq!(base64url_decode("Zm8").unwrap(), b"fo");
        assert_eq!(base64url_decode("Zm8=").unwrap(), b"fo");
        assert_eq!(base64url_decode("Zm9v").unwrap(), b"foo");
    }

    #[test]
    fn test_base64url_special_chars() {
        // Data that produces - and _ in base64url
        let data = b"hello??world";
        let encoded = base64url_nopad_encode(data);
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
        let decoded = base64url_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base64url_to_standard_decode() {
        assert_eq!(base64url_to_standard_decode("Zg").unwrap(), b"f");
        assert_eq!(base64url_to_standard_decode("Zm8").unwrap(), b"fo");
        assert_eq!(base64url_to_standard_decode("Zm9v").unwrap(), b"foo");
    }

    #[test]
    fn test_encode_u64_base64() {
        let encoded = encode_u64_base64(0);
        assert_eq!(encoded.len(), 12);
        assert_eq!(encoded, "AAAAAAAAAAAA");

        let encoded = encode_u64_base64(12345);
        assert_eq!(encoded.len(), 12);

        let encoded = encode_u64_base64(u64::MAX);
        assert_eq!(encoded.len(), 12);
    }

    #[test]
    fn test_invalid_base64() {
        // "!!!" contains no valid base64 chars, but our decoder treats invalid chars as 0xFF
        // which triggers the "invalid base64 character" error in decode_quad.
        // "Zg==!" after trimming '=' becomes "Zg!" which is 3 chars — partial quad.
        // The decoder handles this gracefully. Test with something that actually fails.
        assert!(standard_decode("Z!@#").is_err());
    }
}
