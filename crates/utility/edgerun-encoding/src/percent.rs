//! Percent/URL encoding and decoding utilities.
//!
//! Consolidated from `edgerun-oauth/src/types.rs`, `edgerun-oauth/src/client.rs`,
//! and `edgerun-oauth/src/server.rs`.
//!
//! Implements RFC 3986 percent-encoding for form-urlencoded data.

use alloc::string::String;
use alloc::vec::Vec;

/// Check if a byte is an unreserved character per RFC 3986.
/// Unreserved characters: A-Z, a-z, 0-9, '-', '.', '_', '~'
fn is_unreserved(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~')
}

/// Encode a byte to percent-encoded format (e.g., `0x20` → `"%20"`).
fn percent_encode_byte(b: u8) -> [u8; 3] {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    [b'%', HEX[(b >> 4) as usize], HEX[(b & 0x0F) as usize]]
}

/// Percent-encode a string according to RFC 3986.
///
/// Unreserved characters (A-Z, a-z, 0-9, '-', '.', '_', '~') are passed through.
/// All other characters are percent-encoded as `%XX`.
///
/// This consolidates the duplicate implementations in:
/// - `edgerun-oauth/src/types.rs`
/// - `edgerun-oauth/src/client.rs`
///
/// # Examples
/// ```
/// use edgerun_encoding::percent::percent_encode;
/// assert_eq!(percent_encode("hello world"), "hello%20world");
/// assert_eq!(percent_encode("foo@bar.com"), "foo%40bar.com");
/// assert_eq!(percent_encode("already-safe_123"), "already-safe_123");
/// ```
pub fn percent_encode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result = String::with_capacity(bytes.len() * 3);

    for &b in bytes {
        if is_unreserved(b) {
            result.push(b as char);
        } else {
            let encoded = percent_encode_byte(b);
            result.push_str(core::str::from_utf8(&encoded).unwrap());
        }
    }

    result
}

/// Percent-encode path segments while preserving `/` separators.
///
/// This is useful for URI paths where each segment must be escaped but the
/// hierarchy separators must remain intact.
pub fn percent_encode_path_segments(path: &str) -> String {
    path.split('/')
        .map(percent_encode)
        .collect::<Vec<_>>()
        .join("/")
}

/// Percent-encode a value while preserving its first `:` separator.
///
/// This covers URI path values such as digest references where the colon is a
/// structural separator rather than data inside either side.
pub fn percent_encode_colon_pair(value: &str) -> String {
    if let Some((left, right)) = value.split_once(':') {
        alloc::format!("{}:{}", percent_encode(left), percent_encode(right))
    } else {
        percent_encode(value)
    }
}

/// Percent-decode a string, handling `%XX` sequences and `+` as space.
///
/// This is the inverse of `percent_encode`, with the additional handling
/// of `+` as space (common in form-urlencoded data).
///
/// # Examples
/// ```
/// use edgerun_encoding::percent::percent_decode;
/// assert_eq!(percent_decode("hello%20world"), "hello world");
/// assert_eq!(percent_decode("foo+bar"), "foo bar");
/// assert_eq!(percent_decode("hello%40world"), "hello@world");
/// ```
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'+' {
            result.push(b' ');
            i += 1;
        } else if bytes[i] == b'%' && i + 2 < bytes.len() {
            let high = hex_char_to_nibble(bytes[i + 1]);
            let low = hex_char_to_nibble(bytes[i + 2]);
            if let (Some(h), Some(l)) = (high, low) {
                result.push((h << 4) | l);
                i += 3;
            } else {
                // Invalid percent encoding, pass through
                result.push(bytes[i]);
                i += 1;
            }
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }

    String::from_utf8(result).unwrap_or_default()
}

/// Convert a hex character to its nibble value.
fn hex_char_to_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Parse form-urlencoded data into key-value pairs.
///
/// Input format: `key1=value1&key2=value2`
/// Keys and values are percent-decoded.
///
/// # Examples
/// ```
/// use edgerun_encoding::percent::parse_form_urlencoded;
/// let pairs = parse_form_urlencoded("name=John%20Doe&age=30");
/// assert_eq!(pairs.get("name"), Some(&"John Doe".to_string()));
/// assert_eq!(pairs.get("age"), Some(&"30".to_string()));
/// ```
pub fn parse_form_urlencoded(body: &str) -> alloc::collections::BTreeMap<String, String> {
    let mut result = alloc::collections::BTreeMap::new();

    for pair in body.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            result.insert(percent_decode(key), percent_decode(value));
        } else if !pair.is_empty() {
            result.insert(percent_decode(pair), String::new());
        }
    }

    result
}

/// Encode a key-value pair as form-urlencoded data.
///
/// Both key and value are percent-encoded.
///
/// # Examples
/// ```
/// use edgerun_encoding::percent::url_encode_pair;
/// assert_eq!(url_encode_pair("name", "John Doe"), "name=John%20Doe");
/// assert_eq!(url_encode_pair("foo", "bar baz"), "foo=bar%20baz");
/// ```
pub fn url_encode_pair(key: &str, value: &str) -> String {
    alloc::format!("{}={}", percent_encode(key), percent_encode(value))
}

/// Encode multiple key-value pairs as form-urlencoded data.
///
/// # Examples
/// ```
/// use edgerun_encoding::percent::url_encode_pairs;
/// let params = vec![("name", "John"), ("age", "30")];
/// assert_eq!(url_encode_pairs(&params), "name=John&age=30");
/// ```
pub fn url_encode_pairs(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| url_encode_pair(k, v))
        .collect::<Vec<_>>()
        .join("&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn test_percent_encode() {
        assert_eq!(percent_encode("hello"), "hello");
        assert_eq!(percent_encode("hello world"), "hello%20world");
        assert_eq!(percent_encode("foo@bar.com"), "foo%40bar.com");
        assert_eq!(percent_encode("already-safe_123~"), "already-safe_123~");
        assert_eq!(percent_encode(""), "");
    }

    #[test]
    fn test_percent_encode_path_segments() {
        assert_eq!(
            percent_encode_path_segments("library/alpine"),
            "library/alpine"
        );
        assert_eq!(
            percent_encode_path_segments("owner/repo name"),
            "owner/repo%20name"
        );
    }

    #[test]
    fn test_percent_encode_colon_pair() {
        assert_eq!(percent_encode_colon_pair("sha256:abcdef"), "sha256:abcdef");
        assert_eq!(
            percent_encode_colon_pair("tag with space"),
            "tag%20with%20space"
        );
    }

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("hello"), "hello");
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("foo+bar"), "foo bar");
        assert_eq!(percent_decode("hello%40world"), "hello@world");
        // Invalid percent encoding: %XX where XX is not valid hex passes through the %
        assert_eq!(percent_decode("invalid%XX"), "invalid%XX");
    }

    #[test]
    fn test_percent_roundtrip() {
        let test_cases = [
            "hello world",
            "foo@bar.com",
            "test%value",
            "unicode: ñ",
            "special: <>&\"",
        ];
        for s in test_cases {
            let encoded = percent_encode(s);
            let decoded = percent_decode(&encoded);
            assert_eq!(s, decoded);
        }
    }

    #[test]
    fn test_parse_form_urlencoded() {
        let pairs = parse_form_urlencoded("name=John%20Doe&age=30");
        assert_eq!(pairs.get("name"), Some(&"John Doe".to_string()));
        assert_eq!(pairs.get("age"), Some(&"30".to_string()));
    }

    #[test]
    fn test_parse_form_urlencoded_empty() {
        let pairs = parse_form_urlencoded("");
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_parse_form_urlencoded_no_value() {
        let pairs = parse_form_urlencoded("key");
        assert_eq!(pairs.get("key"), Some(&String::new()));
    }

    #[test]
    fn test_url_encode_pair() {
        assert_eq!(url_encode_pair("name", "John Doe"), "name=John%20Doe");
        assert_eq!(url_encode_pair("foo", "bar baz"), "foo=bar%20baz");
        assert_eq!(url_encode_pair("", ""), "=");
    }

    #[test]
    fn test_url_encode_pairs() {
        let params = vec![("name", "John"), ("age", "30")];
        assert_eq!(url_encode_pairs(&params), "name=John&age=30");
    }
}
