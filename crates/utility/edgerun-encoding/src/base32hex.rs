//! Base32hex encoding (RFC 4648 §7 — "Extended Hex" alphabet).
//!
//! Uses the alphabet `0-9a-v` (different from standard base32's `A-Z2-7`).
//! Primarily used for DNSSEC NSEC3 owner name construction.
//!
//! # Examples
//! ```
//! use edgerun_encoding::base32hex::encode_base32hex;
//! // Empty input produces empty output
//! assert_eq!(encode_base32hex(&[]), "");
//! ```

use alloc::string::String;

/// Encode bytes as base32hex (RFC 4648 extended hex alphabet: `0-9a-v`).
///
/// This is the encoding used for DNSSEC NSEC3 hashed owner names.
pub fn encode_base32hex(data: &[u8]) -> String {
    let mut result = String::with_capacity(base32hex_encoded_len(data.len()));
    let start = result.len();
    for _ in 0..base32hex_encoded_len(data.len()) {
        result.push('\0');
    }
    let bytes = unsafe { result.as_bytes_mut() };
    let written = encode_base32hex_into(data, &mut bytes[start..]).unwrap();
    result.truncate(start + written);
    result
}

/// Encoded length for unpadded base32hex.
pub const fn base32hex_encoded_len(input_len: usize) -> usize {
    (input_len * 8 + 4) / 5
}

/// Upper bound for decoded unpadded base32hex bytes.
pub const fn base32hex_decoded_bound(input_len: usize) -> usize {
    (input_len * 5) / 8
}

/// Encode bytes as base32hex into a caller-provided buffer.
pub fn encode_base32hex_into(data: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuv";
    let needed = base32hex_encoded_len(data.len());
    if out.len() < needed {
        return Err("output too short");
    }
    let mut bits = 0u64;
    let mut bit_len = 0;
    let mut written = 0;
    for &b in data {
        bits = (bits << 8) | (b as u64);
        bit_len += 8;
        while bit_len >= 5 {
            bit_len -= 5;
            let idx = (bits >> bit_len) & 0x1F;
            out[written] = ALPHABET[idx as usize];
            written += 1;
        }
    }
    if bit_len > 0 {
        let idx = (bits << (5 - bit_len)) & 0x1F;
        out[written] = ALPHABET[idx as usize];
        written += 1;
    }
    Ok(written)
}

/// Decode unpadded base32hex from bytes into a caller-provided buffer.
pub fn decode_base32hex_into(input: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    match input.len() % 8 {
        0 | 2 | 4 | 5 | 7 => {}
        _ => return Err("invalid base32hex length"),
    }
    let needed = base32hex_decoded_bound(input.len());
    if out.len() < needed {
        return Err("output too short");
    }

    let mut bits = 0u64;
    let mut bit_len = 0;
    let mut written = 0;
    for &byte in input {
        let value = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'v' => byte - b'a' + 10,
            b'A'..=b'V' => byte - b'A' + 10,
            _ => return Err("invalid base32hex character"),
        };
        bits = (bits << 5) | value as u64;
        bit_len += 5;
        while bit_len >= 8 {
            bit_len -= 8;
            out[written] = (bits >> bit_len) as u8;
            written += 1;
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(encode_base32hex(&[]), "");
    }

    #[test]
    fn single_byte() {
        // 0xAB = 10101011 → 10101 011xx → 'l'(21) + 'c'(12)
        assert_eq!(encode_base32hex(&[0xAB]), "lc");
    }

    #[test]
    fn sha1_hash_20_bytes() {
        // Typical SHA-1 output for NSEC3
        let hash = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13,
        ];
        let encoded = encode_base32hex(&hash);
        assert_eq!(encoded.len(), 32); // 20 bytes * 8/5 = 32 chars
        assert!(encoded.chars().all(|c| matches!(c, '0'..='9' | 'a'..='v')));
    }

    #[test]
    fn sha256_hash_32_bytes() {
        let hash = [0xFFu8; 32];
        let encoded = encode_base32hex(&hash);
        assert_eq!(encoded.len(), 52); // 32 * 8/5 = 51.2 → 52
        assert!(encoded.chars().all(|c| matches!(c, '0'..='9' | 'a'..='v')));
    }

    #[test]
    fn decode_rfc4648_vectors() {
        let mut out = [0u8; 8];
        let len = decode_base32hex_into(b"cpnmu", &mut out).unwrap();
        assert_eq!(&out[..len], b"foo");

        let len = decode_base32hex_into(b"CPNMUOG", &mut out).unwrap();
        assert_eq!(&out[..len], b"foob");

        assert_eq!(
            decode_base32hex_into(b"c", &mut out),
            Err("invalid base32hex length")
        );
        assert_eq!(
            decode_base32hex_into(b"cpnm!", &mut out),
            Err("invalid base32hex character")
        );
    }
}
