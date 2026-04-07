use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use unicode_normalization::UnicodeNormalization;

use prost_types::Timestamp;

pub fn parse_rfc3339(value: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(value, &Rfc3339)
}

pub fn timestamp_parts(value: &str) -> Result<(i64, i32), time::error::Parse> {
    let t = parse_rfc3339(value)?;
    Ok((t.unix_timestamp(), t.nanosecond() as i32))
}

pub fn parse_timestamp_value(value: &str) -> Result<Timestamp, time::error::Parse> {
    let (seconds, nanos) = timestamp_parts(value)?;
    Ok(Timestamp { seconds, nanos })
}

// ---------------------------------------------------------------------------
// Hex encoding/decoding — replaces `hex` crate
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum HexError {
    InvalidChar,
    OddLength,
}

impl std::fmt::Display for HexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid hexadecimal string")
    }
}

impl std::error::Error for HexError {}

/// Encode bytes as lowercase hex (no prefix). Replaces `hex::encode`.
pub fn bytes_to_hex(value: &[u8]) -> String {
    let mut out = String::with_capacity(value.len() * 2);
    for &b in value {
        out.push(HEX_DIGITS[(b >> 4) as usize] as char);
        out.push(HEX_DIGITS[(b & 0x0f) as usize] as char);
    }
    out
}

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Decode hex string (no prefix expected). Replaces `hex::decode`.
pub fn hex_to_bytes(value: &str) -> Result<Vec<u8>, HexError> {
    let trimmed = value.strip_prefix("0x").unwrap_or(value);
    if trimmed.len() % 2 != 0 {
        return decode_hex_with_odd_prefix_handling(trimmed);
    }
    let mut out = Vec::with_capacity(trimmed.len() / 2);
    let bytes = trimmed.as_bytes();
    for chunk in bytes.chunks_exact(2) {
        let hi = hex_byte(chunk[0]).ok_or(HexError::InvalidChar)?;
        let lo = hex_byte(chunk[1]).ok_or(HexError::InvalidChar)?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn decode_hex_with_odd_prefix_handling(s: &str) -> Result<Vec<u8>, HexError> {
    let padded = format!("0{}", s);
    hex_to_bytes(&padded)
}

fn hex_byte(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Decode hex, panic on error.
pub fn must_hex_to_bytes(value: &str) -> Vec<u8> {
    hex_to_bytes(value).unwrap()
}

/// Encode bytes as lowercase hex with `0x` prefix.
pub fn bytes_to_hex_prefixed(value: &[u8]) -> String {
    format!("0x{}", bytes_to_hex(value))
}

pub fn nfc(text: &str) -> String {
    text.nfc().collect()
}

pub fn canonical_time_string(value: &str) -> Option<String> {
    parse_rfc3339(value)
        .ok()
        .and_then(|t| t.to_offset(time::UtcOffset::UTC).format(&Rfc3339).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_bytes_accepts_odd_length_prefixed_values() {
        assert_eq!(hex_to_bytes("0xabc").unwrap(), vec![0x0a, 0xbc]);
    }

    #[test]
    fn canonical_time_string_normalizes_offset_to_utc() {
        assert_eq!(
            canonical_time_string("2030-01-01T07:00:00+07:00").as_deref(),
            Some("2030-01-01T00:00:00Z")
        );
    }

    #[test]
    fn nfc_normalizes_combining_forms() {
        let composed = nfc("e\u{301}");
        assert_eq!(composed, "é");
    }
}
