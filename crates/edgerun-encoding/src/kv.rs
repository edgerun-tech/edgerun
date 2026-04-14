//! Key-value and list parsing utilities.
//!
//! Consolidated from `edgerun-node/src/config.rs`, `edgerun-node/src/lib.rs`.
//!
//! All operations are `no_std` compatible.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Parse a `key: value` line into `(key, value)`.
///
/// Splits on the first `:` and trims both sides.
///
/// # Examples
/// ```
/// use edgerun_encoding::kv::parse_kv_colon;
/// assert_eq!(
///     parse_kv_colon("stream_id: abc123"),
///     Some(("stream_id".into(), "abc123".into()))
/// );
/// ```
pub fn parse_kv_colon(line: &str) -> Option<(String, String)> {
    let colon = line.find(':')?;
    let key = line[..colon].trim().to_string();
    let val = line[colon + 1..].trim().to_string();
    Some((key, val))
}

/// Parse a bracketed list like `[item1, item2, "quoted item"]`.
///
/// Returns an empty `Vec` for `[]`, empty string, or non-bracket input.
/// Inner items are trimmed and unquoted.
///
/// # Examples
/// ```
/// use edgerun_encoding::kv::parse_bracket_list;
/// let items = parse_bracket_list("[foo, bar, \"baz qux\"]");
/// assert_eq!(items, vec!["foo", "bar", "baz qux"]);
/// ```
pub fn parse_bracket_list(val: &str) -> Vec<String> {
    let val = val.trim();
    if val == "[]" || val.is_empty() {
        return Vec::new();
    }
    if val.starts_with('[') && val.ends_with(']') {
        let inner = &val[1..val.len() - 1];
        if inner.trim().is_empty() {
            return Vec::new();
        }
        return inner.split(',').map(|s| unquote(s.trim())).collect();
    }
    Vec::new()
}

/// Remove surrounding quotes from a string.
///
/// Strips matching `"..."` or `'...'` pairs. Returns the trimmed inner value.
///
/// # Examples
/// ```
/// use edgerun_encoding::kv::unquote;
/// assert_eq!(unquote("\"hello\""), "hello");
/// assert_eq!(unquote("'world'"), "world");
/// assert_eq!(unquote("plain"), "plain");
/// ```
pub fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 {
        let bytes = s.as_bytes();
        let first = bytes[0];
        let last = bytes[s.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

/// Format a number of bytes as a human-readable string.
///
/// Uses binary prefixes (B, KB, MB, GB, TB).
///
/// # Examples
/// ```
/// use edgerun_encoding::kv::format_bytes;
/// assert_eq!(format_bytes(0), "0.0 B");
/// assert_eq!(format_bytes(1024), "1.0 KB");
/// assert_eq!(format_bytes(1048576), "1.0 MB");
/// assert_eq!(format_bytes(1073741824), "1.0 GB");
/// ```
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut val = bytes as f64;
    let mut idx = 0;
    while val >= 1024.0 && idx < UNITS.len() - 1 {
        val /= 1024.0;
        idx += 1;
    }
    alloc::format!("{:.1} {}", val, UNITS[idx])
}

/// Format an `Option<u8>` as a 2-digit lowercase hex string.
///
/// Returns empty string for `None`.
///
/// # Examples
/// ```
/// use edgerun_encoding::kv::format_hex_u8;
/// assert_eq!(format_hex_u8(Some(0xab)), "ab");
/// assert_eq!(format_hex_u8(None), "");
/// ```
pub fn format_hex_u8(value: Option<u8>) -> String {
    value.map(|v| alloc::format!("{v:02x}")).unwrap_or_default()
}

/// Format an `Option<u16>` as a 4-digit lowercase hex string.
///
/// Returns empty string for `None`.
///
/// # Examples
/// ```
/// use edgerun_encoding::kv::format_hex_u16;
/// assert_eq!(format_hex_u16(Some(0xabcd)), "abcd");
/// assert_eq!(format_hex_u16(None), "");
/// ```
pub fn format_hex_u16(value: Option<u16>) -> String {
    value.map(|v| alloc::format!("{v:04x}")).unwrap_or_default()
}

/// Format an `Option<u32>` as a 6-digit lowercase hex string.
///
/// Returns empty string for `None`.
///
/// # Examples
/// ```
/// use edgerun_encoding::kv::format_hex_u32;
/// assert_eq!(format_hex_u32(Some(0xabcdef)), "abcdef");
/// assert_eq!(format_hex_u32(None), "");
/// ```
pub fn format_hex_u32(value: Option<u32>) -> String {
    value.map(|v| alloc::format!("{v:06x}")).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_parse_kv_colon_basic() {
        assert_eq!(
            parse_kv_colon("stream_id: abc123"),
            Some(("stream_id".into(), "abc123".into()))
        );
    }

    #[test]
    fn test_parse_kv_colon_no_space() {
        assert_eq!(
            parse_kv_colon("key:value"),
            Some(("key".into(), "value".into()))
        );
    }

    #[test]
    fn test_parse_kv_colon_no_colon() {
        assert_eq!(parse_kv_colon("no_colon"), None);
    }

    #[test]
    fn test_parse_kv_colon_multiple_colons() {
        assert_eq!(
            parse_kv_colon("host: http://example.com:8080"),
            Some(("host".into(), "http://example.com:8080".into()))
        );
    }

    #[test]
    fn test_parse_bracket_list_basic() {
        let items = parse_bracket_list("[foo, bar, baz]");
        assert_eq!(items, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn test_parse_bracket_list_quoted() {
        let items = parse_bracket_list("[\"foo bar\", 'baz']");
        assert_eq!(items, vec!["foo bar", "baz"]);
    }

    #[test]
    fn test_parse_bracket_list_empty() {
        assert!(parse_bracket_list("[]").is_empty());
        assert!(parse_bracket_list("").is_empty());
        assert!(parse_bracket_list("not_a_list").is_empty());
    }

    #[test]
    fn test_unquote_double() {
        assert_eq!(unquote("\"hello\""), "hello");
    }

    #[test]
    fn test_unquote_single() {
        assert_eq!(unquote("'world'"), "world");
    }

    #[test]
    fn test_unquote_plain() {
        assert_eq!(unquote("plain"), "plain");
    }

    #[test]
    fn test_unquote_trimmed() {
        assert_eq!(unquote("  \"spaced\"  "), "spaced");
    }

    #[test]
    fn test_format_bytes_zero() {
        assert_eq!(format_bytes(0), "0.0 B");
    }

    #[test]
    fn test_format_bytes_kb() {
        assert_eq!(format_bytes(1024), "1.0 KB");
    }

    #[test]
    fn test_format_bytes_mb() {
        assert_eq!(format_bytes(1048576), "1.0 MB");
    }

    #[test]
    fn test_format_bytes_gb() {
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }

    #[test]
    fn test_format_hex_u8() {
        assert_eq!(format_hex_u8(Some(0xab)), "ab");
        assert_eq!(format_hex_u8(None), "");
    }

    #[test]
    fn test_format_hex_u16() {
        assert_eq!(format_hex_u16(Some(0xabcd)), "abcd");
        assert_eq!(format_hex_u16(None), "");
    }

    #[test]
    fn test_format_hex_u32() {
        assert_eq!(format_hex_u32(Some(0xabcdef)), "abcdef");
        assert_eq!(format_hex_u32(None), "");
    }
}
