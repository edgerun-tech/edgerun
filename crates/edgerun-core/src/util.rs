use prost_types::Timestamp;

// ---------------------------------------------------------------------------
// Minimal RFC3339 timestamp handling — replaces `time` crate
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateTimeUtc {
    pub unix_secs: i64,
    pub nanos: u32,
}

impl DateTimeUtc {
    pub fn now() -> Self {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();
        Self {
            unix_secs: d.as_secs() as i64,
            nanos: d.subsec_nanos(),
        }
    }

    /// Subtract another DateTimeUtc, returning duration in seconds (f64).
    pub fn duration_secs(&self, other: &Self) -> f64 {
        (self.unix_secs - other.unix_secs) as f64
            + (self.nanos as f64 - other.nanos as f64) / 1_000_000_000.0
    }
}

/// Parse an RFC3339 timestamp string into a DateTimeUtc.
/// Supported formats:
///   `2030-01-01T00:00:00Z`
///   `2030-01-01T07:00:00+07:00`
///   `2030-01-01T07:00:00-05:00`
pub fn parse_rfc3339(value: &str) -> Result<DateTimeUtc, ParseRfc3339Error> {
    // Expected format: YYYY-MM-DDTHH:MM:SS[+-]HH:MM or ...Z
    let s = value.as_bytes();
    if s.len() < 20 {
        return Err(ParseRfc3339Error);
    }

    fn d2(s: &[u8], off: usize) -> Result<u32, ParseRfc3339Error> {
        if s[off].is_ascii_digit() && s[off + 1].is_ascii_digit() {
            Ok(((s[off] - b'0') as u32) * 10 + (s[off + 1] - b'0') as u32)
        } else {
            Err(ParseRfc3339Error)
        }
    }
    fn d4(s: &[u8], off: usize) -> Result<u32, ParseRfc3339Error> {
        Ok(d2(s, off)? * 100 + d2(s, off + 2)?)
    }

    let year = d4(s, 0)?;
    if s[4] != b'-' { return Err(ParseRfc3339Error); }
    let month = d2(s, 5)?;
    if s[7] != b'-' { return Err(ParseRfc3339Error); }
    let day = d2(s, 8)?;
    if s[10] != b'T' { return Err(ParseRfc3339Error); }
    let hour = d2(s, 11)?;
    if s[13] != b':' { return Err(ParseRfc3339Error); }
    let minute = d2(s, 14)?;
    if s[16] != b':' { return Err(ParseRfc3339Error); }
    let second = d2(s, 17)?;

    // Seconds can have fractional part
    let mut nanos = 0u32;
    let mut frac_index = 19;
    if s.get(19) == Some(&b'.') {
        let frac_start = 20;
        let mut frac_end = frac_start;
        while frac_end < s.len() && s[frac_end].is_ascii_digit() {
            frac_end += 1;
        }
        let frac_str = &s[frac_start..frac_end];
        // Pad or truncate to nanoseconds
        let mut val = 0u64;
        let mut scale = 1_000_000_000u64;
        for &b in frac_str {
            val = val * 10 + (b - b'0') as u64;
            scale /= 10;
        }
        if scale > 0 {
            val *= scale;
        }
        nanos = (val % 1_000_000_000) as u32;
        frac_index = frac_end;
    }

    // Timezone
    let tz_off_secs = if s.get(frac_index) == Some(&b'Z') {
        0i64
    } else if frac_index + 6 <= s.len() && (s[frac_index] == b'+' || s[frac_index] == b'-') {
        let sign = if s[frac_index] == b'+' { 1i64 } else { -1i64 };
        let tz_h = d2(s, frac_index + 1)? as i64;
        if s[frac_index + 3] != b':' { return Err(ParseRfc3339Error); }
        let tz_m = d2(s, frac_index + 4)? as i64;
        sign * (tz_h * 3600 + tz_m * 60)
    } else {
        return Err(ParseRfc3339Error);
    };

    // Convert to unix timestamp (simplified — good enough for validation purposes)
    let unix_secs = date_to_unix_secs(year, month, day, hour, minute, second) - tz_off_secs;

    Ok(DateTimeUtc { unix_secs, nanos })
}

#[derive(Debug)]
pub struct ParseRfc3339Error;

impl std::fmt::Display for ParseRfc3339Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid RFC3339 timestamp")
    }
}

impl std::error::Error for ParseRfc3339Error {}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 0,
    }
}

fn date_to_unix_secs(year: u32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> i64 {
    let mut secs: i64 = 0;
    // Days from 1970 to year
    for y in 1970..year {
        secs += if is_leap_year(y) { 366 } else { 365 };
    }
    // Days in current year
    for m in 1..month {
        secs += days_in_month(year, m) as i64;
    }
    secs += (day - 1) as i64;
    secs = secs * 86400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64;
    secs
}

pub fn timestamp_parts(value: &str) -> Result<(i64, i32), ParseRfc3339Error> {
    let t = parse_rfc3339(value)?;
    Ok((t.unix_secs, t.nanos as i32))
}

pub fn parse_timestamp_value(value: &str) -> Result<Timestamp, ParseRfc3339Error> {
    let (seconds, nanos) = timestamp_parts(value)?;
    Ok(Timestamp { seconds, nanos })
}

/// Format a DateTimeUtc as RFC3339 in UTC (e.g. `2030-01-01T00:00:00Z`).
fn format_rfc3339_utc(dt: &DateTimeUtc) -> String {
    // Convert unix_secs back to date components (simplified)
    let mut remaining = dt.unix_secs;
    let mut year = 1970u32;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        let secs_in_year = days_in_year as i64 * 86400;
        if remaining < secs_in_year {
            break;
        }
        remaining -= secs_in_year;
        year += 1;
    }
    let mut month = 1u32;
    loop {
        let dim = days_in_month(year, month);
        let secs_in_month = dim as i64 * 86400;
        if remaining < secs_in_month {
            break;
        }
        remaining -= secs_in_month;
        month += 1;
    }
    let day = (remaining / 86400) as u32 + 1;
    let rem_secs = remaining % 86400;
    let hour = (rem_secs / 3600) as u32;
    let minute = ((rem_secs % 3600) / 60) as u32;
    let second = (rem_secs % 60) as u32;

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, hour, minute, second)
}

/// Convert `SystemTime` to a `prost_types::Timestamp`.
/// Shared utility — replaces duplicated conversion code in session.rs,
/// capability-policy, stream, and node lib.rs.
pub fn system_time_to_prost(time: std::time::SystemTime) -> prost_types::Timestamp {
    let duration = time.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    prost_types::Timestamp {
        seconds: duration.as_secs() as i64,
        nanos: duration.subsec_nanos() as i32,
    }
}

pub fn canonical_time_string(value: &str) -> Option<String> {
    parse_rfc3339(value).ok().map(|t| format_rfc3339_utc(&t))
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
