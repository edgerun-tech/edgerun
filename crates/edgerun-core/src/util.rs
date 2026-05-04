use crate::prelude::v1::*;

use crate::protocol::Timestamp;

// ---------------------------------------------------------------------------
// RFC3339 timestamp handling — delegates to edgerun-encoding
// ---------------------------------------------------------------------------

pub use edgerun_encoding::rfc3339::{
    canonical_time_string, format_rfc3339_utc, parse_rfc3339, DateTimeUtc, ParseRfc3339Error,
};

/// Parse an RFC3339 timestamp into `(seconds, nanos)` for `crate::protocol::Timestamp`.
pub fn timestamp_parts(value: &str) -> Result<(i64, i32), ParseRfc3339Error> {
    let t = parse_rfc3339(value)?;
    Ok((t.unix_secs, t.nanos as i32))
}

/// Parse an RFC3339 timestamp into a `crate::protocol::Timestamp`.
pub fn parse_timestamp_value(value: &str) -> Result<Timestamp, ParseRfc3339Error> {
    let (seconds, nanos) = timestamp_parts(value)?;
    Ok(Timestamp { seconds, nanos })
}

/// Convert `SystemTime` to a `crate::protocol::Timestamp`.
pub fn system_time_to_prost(time: std::time::SystemTime) -> crate::protocol::Timestamp {
    let duration = time
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    crate::protocol::Timestamp {
        seconds: duration.as_secs() as i64,
        nanos: duration.subsec_nanos() as i32,
    }
}

/// Current Unix time in seconds.
pub fn now_unix_secs_i64() -> i64 {
    #[cfg(not(target_os = "none"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    #[cfg(target_os = "none")]
    {
        0
    }
}

/// Current Unix time in milliseconds.
pub fn now_unix_millis_i64() -> i64 {
    #[cfg(not(target_os = "none"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }

    #[cfg(target_os = "none")]
    {
        0
    }
}

/// Current Unix time in microseconds.
pub fn now_unix_micros_u64() -> u64 {
    #[cfg(not(target_os = "none"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }

    #[cfg(target_os = "none")]
    {
        0
    }
}

/// Current Unix time as a protobuf timestamp.
pub fn now_prost_timestamp() -> crate::protocol::Timestamp {
    #[cfg(not(target_os = "none"))]
    {
        system_time_to_prost(std::time::SystemTime::now())
    }

    #[cfg(target_os = "none")]
    {
        crate::protocol::Timestamp {
            seconds: 0,
            nanos: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Hex encoding/decoding — delegates to edgerun-encoding
// ---------------------------------------------------------------------------

/// Hex decoding error (compatibility alias for edgerun-encoding).
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

fn map_hex_error(e: edgerun_encoding::hex::HexError) -> HexError {
    match e {
        edgerun_encoding::hex::HexError::InvalidCharacter(_)
        | edgerun_encoding::hex::HexError::InvalidLength => HexError::InvalidChar,
    }
}

/// Encode bytes as lowercase hex (no prefix). Replaces `hex::encode`.
pub fn bytes_to_hex(value: &[u8]) -> String {
    edgerun_encoding::hex::bytes_to_hex(value)
}

/// Decode hex string (no prefix expected). Replaces `hex::decode`.
pub fn hex_to_bytes(value: &str) -> Result<Vec<u8>, HexError> {
    edgerun_encoding::hex::hex_to_bytes(value).map_err(map_hex_error)
}

/// Decode hex, panic on error.
pub fn must_hex_to_bytes(value: &str) -> Vec<u8> {
    edgerun_encoding::hex::must_hex_to_bytes(value)
}

/// Encode bytes as lowercase hex with `0x` prefix.
pub fn bytes_to_hex_prefixed(value: &[u8]) -> String {
    edgerun_encoding::hex::bytes_to_hex_prefixed(value)
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
}
