//! C-string (null-terminated string) utilities.
//!
//! Consolidated from identical implementations in:
//! - `edgerun-linux-cec/src/lib.rs`
//! - `edgerun-mgmt-bluetooth/src/lib.rs`
//!
//! Provides safe conversion between null-terminated byte slices and Rust strings.

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::str;

/// Decode a null-terminated C string to a Rust `String`.
///
/// Stops at the first null byte (`\0`). Returns an error if the bytes
/// before the null terminator are not valid UTF-8.
///
/// This consolidates the duplicate implementations in:
/// - `edgerun-linux-cec/src/lib.rs`
/// - `edgerun-mgmt-bluetooth/src/lib.rs`
///
/// # Examples
/// ```
/// use edgerun_encoding::cstring::decode_c_string;
/// assert_eq!(decode_c_string(b"hello\0world"), Ok("hello".to_string()));
/// assert_eq!(decode_c_string(b"hello\0"), Ok("hello".to_string()));
/// assert_eq!(decode_c_string(b"\0"), Ok("".to_string()));
/// ```
pub fn decode_c_string(bytes: &[u8]) -> Result<String, core::str::Utf8Error> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    str::from_utf8(&bytes[..end]).map(|s| s.to_string())
}

/// Decode a null-terminated C string, replacing invalid UTF-8.
pub fn decode_c_string_lossy(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// Decode a null-terminated C string, returning a string slice reference.
///
/// This is a zero-copy variant that returns a reference into the original buffer.
/// Useful when you don't need ownership of the string.
///
/// # Examples
/// ```
/// use edgerun_encoding::cstring::decode_c_string_ref;
/// let data = b"hello\0world";
/// assert_eq!(decode_c_string_ref(data), Ok("hello"));
/// ```
pub fn decode_c_string_ref(bytes: &[u8]) -> Result<&str, core::str::Utf8Error> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    str::from_utf8(&bytes[..end])
}

/// Encode a string to a null-terminated C string.
///
/// Appends a `\0` byte to the UTF-8 representation of the string.
///
/// # Examples
/// ```
/// use edgerun_encoding::cstring::encode_c_string;
/// assert_eq!(encode_c_string("hello"), vec![b'h', b'e', b'l', b'l', b'o', 0]);
/// assert_eq!(encode_c_string(""), vec![0]);
/// ```
pub fn encode_c_string(s: &str) -> Vec<u8> {
    let mut result = Vec::with_capacity(s.len() + 1);
    result.extend_from_slice(s.as_bytes());
    result.push(0);
    result
}

/// Decode multiple null-terminated C strings from a byte buffer.
///
/// Parses consecutive null-terminated strings until the buffer is exhausted.
/// Empty strings (consecutive null bytes) are included as empty strings.
///
/// # Examples
/// ```
/// use edgerun_encoding::cstring::decode_c_strings;
/// let data = b"foo\0bar\0baz\0";
/// let strings = decode_c_strings(data).unwrap();
/// assert_eq!(strings, vec!["foo", "bar", "baz"]);
/// ```
pub fn decode_c_strings(bytes: &[u8]) -> Result<Vec<String>, core::str::Utf8Error> {
    let mut result = Vec::new();
    let mut offset = 0;

    while offset < bytes.len() {
        let end = bytes[offset..]
            .iter()
            .position(|&b| b == 0)
            .map(|p| offset + p)
            .unwrap_or(bytes.len());

        result.push(str::from_utf8(&bytes[offset..end])?.to_string());
        offset = end + 1; // skip null byte
    }

    Ok(result)
}

/// Decode a null-terminated C string and trim whitespace.
///
/// This variant is for callers like `edgerun-linux-cec` that need trimming.
///
/// # Examples
/// ```
/// use edgerun_encoding::cstring::decode_c_string_trimmed;
/// assert_eq!(decode_c_string_trimmed(b"hello  \0world").unwrap(), "hello");
/// assert_eq!(decode_c_string_trimmed(b"  trimmed  \0").unwrap(), "trimmed");
/// ```
pub fn decode_c_string_trimmed(bytes: &[u8]) -> Result<String, core::str::Utf8Error> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let s = str::from_utf8(&bytes[..end])?;
    Ok(s.trim().to_string())
}

/// Decode a null-terminated C string, replacing invalid UTF-8 and trimming whitespace.
pub fn decode_c_string_lossy_trimmed(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    match String::from_utf8_lossy(&bytes[..end]) {
        Cow::Borrowed(s) => s.trim().to_string(),
        Cow::Owned(s) => s.trim().to_string(),
    }
}

/// Decode a PC/SC-style multi-string: NUL-separated strings terminated by an empty entry.
pub fn decode_c_multi_string_lossy_until_empty(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0usize;
    while start < bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|&b| b == 0)
            .map(|i| start + i)
            .unwrap_or(bytes.len());
        if end == start {
            break;
        }
        out.push(String::from_utf8_lossy(&bytes[start..end]).into_owned());
        start = end + 1;
    }
    out
}

/// Read a NUL-terminated string from `bytes[*cursor..]`, replacing invalid UTF-8.
///
/// Returns `None` when there is no terminator. On success, advances `cursor`
/// past the terminator and returns the decoded string.
pub fn read_c_string_lossy(bytes: &[u8], cursor: &mut usize) -> Option<String> {
    let end = bytes.get(*cursor..)?.iter().position(|&b| b == 0)?;
    let start = *cursor;
    *cursor += end + 1;
    Some(String::from_utf8_lossy(&bytes[start..start + end]).into_owned())
}

/// Read a NUL-terminated string from a fixed offset, replacing invalid UTF-8.
///
/// Returns `(string, next_offset)` where `next_offset` is after the terminator.
pub fn read_c_string_lossy_at(bytes: &[u8], offset: usize) -> Option<(String, usize)> {
    let mut cursor = offset;
    let value = read_c_string_lossy(bytes, &mut cursor)?;
    Some((value, cursor))
}

/// Parse consecutive NUL-terminated key/value string pairs.
///
/// Stops at a trailing NUL, end of input, or an incomplete pair.
pub fn decode_c_string_pairs_lossy(bytes: &[u8], offset: usize) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut cursor = offset;
    while cursor < bytes.len() {
        if bytes[cursor] == 0 {
            break;
        }
        let Some(name) = read_c_string_lossy(bytes, &mut cursor) else {
            break;
        };
        if cursor >= bytes.len() {
            break;
        }
        let Some(value) = read_c_string_lossy(bytes, &mut cursor) else {
            break;
        };
        pairs.push((name, value));
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_decode_c_string() {
        assert_eq!(decode_c_string(b"hello\0world"), Ok("hello".to_string()));
        assert_eq!(decode_c_string(b"hello\0"), Ok("hello".to_string()));
        assert_eq!(decode_c_string(b"\0"), Ok("".to_string()));
        assert_eq!(decode_c_string(b""), Ok("".to_string()));
    }

    #[test]
    fn test_decode_c_string_lossy() {
        assert_eq!(decode_c_string_lossy(b"hello\0world"), "hello");
        assert_eq!(decode_c_string_lossy(b"\xff\xfe\0"), "\u{fffd}\u{fffd}");
    }

    #[test]
    fn test_decode_c_string_ref() {
        let data = b"hello\0world";
        assert_eq!(decode_c_string_ref(data), Ok("hello"));

        let data = b"\0";
        assert_eq!(decode_c_string_ref(data), Ok(""));
    }

    #[test]
    fn test_decode_c_string_invalid_utf8() {
        assert!(decode_c_string(b"\xff\xfe\x00").is_err());
    }

    #[test]
    fn test_encode_c_string() {
        assert_eq!(
            encode_c_string("hello"),
            vec![b'h', b'e', b'l', b'l', b'o', 0]
        );
        assert_eq!(encode_c_string(""), vec![0]);
        assert_eq!(encode_c_string("a"), vec![b'a', 0]);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = "hello world";
        let encoded = encode_c_string(original);
        let decoded = decode_c_string(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_decode_c_strings() {
        let data = b"foo\0bar\0baz\0";
        let strings = decode_c_strings(data).unwrap();
        assert_eq!(strings, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn test_decode_c_strings_empty() {
        let data = b"\0\0\0";
        let strings = decode_c_strings(data).unwrap();
        assert_eq!(strings, vec!["", "", ""]);
    }

    #[test]
    fn test_decode_c_strings_single() {
        let data = b"single\0";
        let strings = decode_c_strings(data).unwrap();
        assert_eq!(strings, vec!["single"]);
    }

    #[test]
    fn test_decode_c_strings_invalid_utf8() {
        let data = b"foo\0\xff\xfe\0";
        assert!(decode_c_strings(data).is_err());
    }

    #[test]
    fn test_decode_c_string_trimmed() {
        assert_eq!(decode_c_string_trimmed(b"hello  \0world").unwrap(), "hello");
        assert_eq!(
            decode_c_string_trimmed(b"  trimmed  \0").unwrap(),
            "trimmed"
        );
        assert_eq!(decode_c_string_trimmed(b"no-trim\0").unwrap(), "no-trim");
        assert_eq!(decode_c_string_trimmed(b"\0").unwrap(), "");
    }

    #[test]
    fn test_decode_c_string_lossy_trimmed() {
        assert_eq!(decode_c_string_lossy_trimmed(b" hello  \0world"), "hello");
        assert_eq!(decode_c_string_lossy_trimmed(b"\xff \0"), "\u{fffd}");
    }

    #[test]
    fn test_decode_c_multi_string_lossy_until_empty() {
        assert_eq!(
            decode_c_multi_string_lossy_until_empty(b"Reader1\0Reader2\0\0"),
            vec!["Reader1".to_string(), "Reader2".to_string()]
        );
        assert_eq!(
            decode_c_multi_string_lossy_until_empty(b"\0"),
            Vec::<String>::new()
        );
        assert_eq!(
            decode_c_multi_string_lossy_until_empty(b"Reader1\0Reader2"),
            vec!["Reader1".to_string(), "Reader2".to_string()]
        );
    }

    #[test]
    fn test_read_c_string_lossy_at() {
        let data = b"xxhello\0world";
        let (value, next) = read_c_string_lossy_at(data, 2).unwrap();
        assert_eq!(value, "hello");
        assert_eq!(next, 8);
        assert_eq!(read_c_string_lossy_at(b"unterminated", 0), None);
    }

    #[test]
    fn test_decode_c_string_pairs_lossy() {
        let mut data = Vec::new();
        data.extend_from_slice(b"blksize");
        data.push(0);
        data.extend_from_slice(b"1456");
        data.push(0);
        data.extend_from_slice(b"tsize");
        data.push(0);
        data.extend_from_slice(b"0");
        data.push(0);
        let pairs = decode_c_string_pairs_lossy(&data, 0);
        assert_eq!(
            pairs,
            vec![
                ("blksize".to_string(), "1456".to_string()),
                ("tsize".to_string(), "0".to_string()),
            ]
        );
    }
}
