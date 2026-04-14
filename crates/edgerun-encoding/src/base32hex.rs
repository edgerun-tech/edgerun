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
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuv";
    let mut result = String::with_capacity(data.len() * 8 / 5 + 1);
    let mut bits = 0u64;
    let mut bit_len = 0;
    for &b in data {
        bits = (bits << 8) | (b as u64);
        bit_len += 8;
        while bit_len >= 5 {
            bit_len -= 5;
            let idx = (bits >> bit_len) & 0x1F;
            result.push(ALPHABET[idx as usize] as char);
        }
    }
    if bit_len > 0 {
        let idx = (bits << (5 - bit_len)) & 0x1F;
        result.push(ALPHABET[idx as usize] as char);
    }
    result
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
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
            0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
            0x10, 0x11, 0x12, 0x13,
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
}
