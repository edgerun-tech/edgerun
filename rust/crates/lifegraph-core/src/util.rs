use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use unicode_normalization::UnicodeNormalization;

use crate::protocol::Timestamp;

pub fn parse_rfc3339(value: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(value, &Rfc3339)
}

pub fn timestamp_parts(value: &str) -> Result<(i64, i64), time::error::Parse> {
    let t = parse_rfc3339(value)?;
    Ok((t.unix_timestamp(), t.nanosecond() as i64))
}

pub fn parse_timestamp_value(value: &str) -> Result<Timestamp, time::error::Parse> {
    let (seconds, nanos) = timestamp_parts(value)?;
    Ok(Timestamp { seconds, nanos })
}

pub fn hex_to_bytes(value: &str) -> Result<Vec<u8>, hex::FromHexError> {
    let trimmed = value.strip_prefix("0x").unwrap_or(value);
    let owned;
    let s = if trimmed.len() % 2 != 0 {
        owned = format!("0{trimmed}");
        owned.as_str()
    } else {
        trimmed
    };
    hex::decode(s)
}

pub fn must_hex_to_bytes(value: &str) -> Vec<u8> {
    hex_to_bytes(value).unwrap()
}

pub fn bytes_to_hex(value: &[u8]) -> String {
    format!("0x{}", hex::encode(value))
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
