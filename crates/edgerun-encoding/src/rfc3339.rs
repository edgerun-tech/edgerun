//! RFC3339 timestamp parsing and formatting.
//!
//! Supports common RFC3339 variants:
//! - `2030-01-01T00:00:00Z`
//! - `2030-01-01T07:00:00+07:00`
//! - `2030-01-01T07:00:00-05:00`
//! - Fractional seconds: `2030-01-01T00:00:00.123456789Z`
//!
//! All operations are `no_std` compatible.

use alloc::format;
use alloc::string::String;

/// A UTC date-time with second + nanosecond precision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateTimeUtc {
    pub unix_secs: i64,
    pub nanos: u32,
}

impl DateTimeUtc {
    /// Unix epoch (1970-01-01T00:00:00Z).
    pub fn epoch() -> Self {
        Self { unix_secs: 0, nanos: 0 }
    }

    /// Current time from system clock.
    #[cfg(feature = "std")]
    pub fn now() -> Self {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
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

/// Parse an RFC3339 timestamp string into a `DateTimeUtc`.
///
/// # Examples
/// ```
/// use edgerun_encoding::rfc3339::parse_rfc3339;
/// let dt = parse_rfc3339("2030-01-01T00:00:00Z").unwrap();
/// assert_eq!(dt.unix_secs, 1893456000);
/// ```
pub fn parse_rfc3339(value: &str) -> Result<DateTimeUtc, ParseRfc3339Error> {
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

    // Fractional seconds
    let mut nanos = 0u32;
    let mut frac_index = 19;
    if s.get(19) == Some(&b'.') {
        let frac_start = 20;
        let mut frac_end = frac_start;
        while frac_end < s.len() && s[frac_end].is_ascii_digit() {
            frac_end += 1;
        }
        let mut val = 0u64;
        let mut scale = 1_000_000_000u64;
        for &b in &s[frac_start..frac_end] {
            val = val * 10 + (b - b'0') as u64;
            scale /= 10;
        }
        if scale > 0 {
            val *= scale;
        }
        nanos = (val % 1_000_000_000) as u32;
        frac_index = frac_end;
    }

    // Timezone offset
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

    let unix_secs = date_to_unix_secs(year, month, day, hour, minute, second) - tz_off_secs;

    Ok(DateTimeUtc { unix_secs, nanos })
}

/// RFC3339 parse error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseRfc3339Error;

impl core::fmt::Display for ParseRfc3339Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("invalid RFC3339 timestamp")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseRfc3339Error {}

/// Format a `DateTimeUtc` as RFC3339 in UTC (e.g. `2030-01-01T00:00:00Z`).
///
/// # Examples
/// ```
/// use edgerun_encoding::rfc3339::{DateTimeUtc, format_rfc3339_utc};
/// let dt = DateTimeUtc { unix_secs: 1893456000, nanos: 0 };
/// assert_eq!(format_rfc3339_utc(&dt), "2030-01-01T00:00:00Z");
/// ```
pub fn format_rfc3339_utc(dt: &DateTimeUtc) -> String {
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

/// Canonicalize an RFC3339 timestamp string to UTC form.
/// Returns `None` if the input is invalid.
///
/// # Examples
/// ```
/// use edgerun_encoding::rfc3339::canonical_time_string;
/// assert_eq!(
///     canonical_time_string("2030-01-01T07:00:00+07:00"),
///     Some("2030-01-01T00:00:00Z".to_string())
/// );
/// ```
pub fn canonical_time_string(value: &str) -> Option<String> {
    parse_rfc3339(value).ok().map(|t| format_rfc3339_utc(&t))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

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
    for y in 1970..year {
        secs += if is_leap_year(y) { 366 } else { 365 };
    }
    for m in 1..month {
        secs += days_in_month(year, m) as i64;
    }
    secs += (day - 1) as i64;
    secs = secs * 86400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64;
    secs
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn parse_utc_simple() {
        let dt = parse_rfc3339("2030-01-01T00:00:00Z").unwrap();
        assert_eq!(dt.unix_secs, 1893456000);
        assert_eq!(dt.nanos, 0);
    }

    #[test]
    fn parse_with_offset() {
        let dt = parse_rfc3339("2030-01-01T07:00:00+07:00").unwrap();
        assert_eq!(dt.unix_secs, 1893456000);
    }

    #[test]
    fn parse_negative_offset() {
        let dt = parse_rfc3339("2029-12-31T19:00:00-05:00").unwrap();
        assert_eq!(dt.unix_secs, 1893456000);
    }

    #[test]
    fn parse_fractional_seconds() {
        let dt = parse_rfc3339("2030-01-01T00:00:00.123456789Z").unwrap();
        assert_eq!(dt.nanos, 123_456_789);
    }

    #[test]
    fn format_roundtrip() {
        let dt = parse_rfc3339("2030-01-01T00:00:00Z").unwrap();
        let s = format_rfc3339_utc(&dt);
        assert_eq!(s, "2030-01-01T00:00:00Z");
    }

    #[test]
    fn canonical_normalizes_offset() {
        assert_eq!(
            canonical_time_string("2030-01-01T07:00:00+07:00"),
            Some("2030-01-01T00:00:00Z".to_string())
        );
    }

    #[test]
    fn invalid_short_string() {
        assert!(parse_rfc3339("2030-01-01").is_err());
    }

    #[test]
    fn invalid_bad_separator() {
        assert!(parse_rfc3339("2030-01-01 00:00:00Z").is_err());
    }

    #[test]
    fn epoch_roundtrip() {
        let dt = DateTimeUtc::epoch();
        let s = format_rfc3339_utc(&dt);
        assert_eq!(s, "1970-01-01T00:00:00Z");
    }

    #[test]
    fn duration_between_two_times() {
        let a = parse_rfc3339("2030-01-01T00:00:00Z").unwrap();
        let b = parse_rfc3339("2030-01-01T00:00:01Z").unwrap();
        assert!((a.duration_secs(&b) - (-1.0)).abs() < 1e-9);
    }
}
