//! HTTP Range Requests support (RFC 7233)
//!
//! Handles `Range` and `Content-Range` headers for partial content delivery.
//! Supports byte-range requests for efficient resume and partial downloads.
//!
//! # Server-side
//! When a client sends `Range: bytes=0-1023`, the server should:
//! 1. Parse the range
//! 2. Validate it against the resource size
//! 3. Return `206 Partial Content` with `Content-Range` header
//!
//! # Client-side
//! When receiving a `206 Partial Content` response, the client can:
//! 1. Check `Content-Range` to know which bytes were received
//! 2. Request remaining ranges

#[cfg(target_os = "none")]
use crate::prelude::v1::*;

use std::fmt;
use std::ops::Range;

/// A parsed Range header value
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeSpecifier {
    /// Byte range: `bytes=start-end`
    /// - `bytes=0-1023` → Bytes(0..1024)
    /// - `bytes=500-` → Bytes(500..end)
    /// - `bytes=-500` → SuffixBytes(500) (last 500 bytes)
    Bytes(Vec<ByteRange>),
    /// Unsatisfiable range
    Unsatisfiable,
}

/// A single byte range
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ByteRange {
    /// Standard range: bytes from start to end (inclusive)
    Range { start: u64, end: Option<u64> },
    /// Suffix range: last N bytes
    Suffix(u64),
}

/// Parsed Content-Range header
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentRange {
    /// The unit (always "bytes" for HTTP)
    pub unit: String,
    /// The range of bytes being sent
    pub range: Range<u64>,
    /// The complete size of the resource
    pub complete_size: Option<u64>,
}

impl ContentRange {
    /// Format as Content-Range header value
    pub fn to_header_value(&self) -> String {
        match self.complete_size {
            Some(size) => format!(
                "bytes {}-{}/{}",
                self.range.start,
                self.range.end.saturating_sub(1),
                size
            ),
            None => format!(
                "bytes {}-{}/{}",
                self.range.start,
                self.range.end.saturating_sub(1),
                "*"
            ),
        }
    }

    /// Parse from Content-Range header value
    pub fn from_header_value(value: &str) -> Option<Self> {
        // Format: bytes start-end/complete-size or bytes start-end/*
        if !value.starts_with("bytes ") {
            return None;
        }

        let after_unit = &value[6..];
        let parts: Vec<&str> = after_unit.splitn(2, '/').collect();
        if parts.len() != 2 {
            return None;
        }

        let range_parts = parts[0].splitn(2, '-').collect::<Vec<_>>();
        if range_parts.len() != 2 {
            return None;
        }

        let start = range_parts[0].parse::<u64>().ok()?;
        let end = range_parts[1].parse::<u64>().ok()?;
        // end in header is inclusive, we use exclusive
        let end_exclusive = end + 1;

        let complete_size = if parts[1] == "*" {
            None
        } else {
            Some(parts[1].parse::<u64>().ok()?)
        };

        Some(ContentRange {
            unit: "bytes".to_string(),
            range: start..end_exclusive,
            complete_size,
        })
    }
}

impl fmt::Display for ContentRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_header_value())
    }
}

/// Parse Range header value
///
/// Supports:
/// - `bytes=0-1023`
/// - `bytes=500-`
/// - `bytes=-500`
/// - `bytes=0-100, 200-300` (multiple ranges)
pub fn parse_range_header(value: &str) -> Option<RangeSpecifier> {
    if !value.starts_with("bytes=") {
        return None;
    }

    let ranges_str = &value[6..];
    let mut ranges = Vec::new();

    for range_str in ranges_str.split(',') {
        let range_str = range_str.trim();
        if range_str.is_empty() {
            continue;
        }

        let byte_range = parse_single_byte_range(range_str)?;
        ranges.push(byte_range);
    }

    if ranges.is_empty() {
        return None;
    }

    Some(RangeSpecifier::Bytes(ranges))
}

fn parse_single_byte_range(range_str: &str) -> Option<ByteRange> {
    if let Some(dash_pos) = range_str.find('-') {
        let start_str = &range_str[..dash_pos];
        let end_str = &range_str[dash_pos + 1..];

        if start_str.is_empty() {
            // Suffix range: -500 means last 500 bytes
            let suffix_len = end_str.parse::<u64>().ok()?;
            Some(ByteRange::Suffix(suffix_len))
        } else if end_str.is_empty() {
            // Open-ended range: 500-
            let start = start_str.parse::<u64>().ok()?;
            Some(ByteRange::Range { start, end: None })
        } else {
            // Full range: 0-1023
            let start = start_str.parse::<u64>().ok()?;
            let end = end_str.parse::<u64>().ok()?;
            Some(ByteRange::Range {
                start,
                end: Some(end),
            })
        }
    } else {
        None
    }
}

/// Resolve a byte range against a resource size
///
/// Returns the actual start and end (exclusive) byte positions,
/// or None if the range is unsatisfiable.
pub fn resolve_byte_range(range: &ByteRange, resource_size: u64) -> Option<Range<u64>> {
    match range {
        ByteRange::Range { start, end: None } => {
            if *start >= resource_size {
                None // Unsatisfiable
            } else {
                Some(*start..resource_size)
            }
        }
        ByteRange::Range {
            start,
            end: Some(end),
        } => {
            if *start >= resource_size || *start > *end {
                None // Unsatisfiable
            } else {
                let actual_end = (*end + 1).min(resource_size);
                Some(*start..actual_end)
            }
        }
        ByteRange::Suffix(suffix_len) => {
            if *suffix_len == 0 {
                return None;
            }
            let actual_start = resource_size.saturating_sub(*suffix_len);
            Some(actual_start..resource_size)
        }
    }
}

/// Check if a RangeSpecifier is satisfiable for a given resource size
pub fn is_range_satisfiable(spec: &RangeSpecifier, resource_size: u64) -> bool {
    match spec {
        RangeSpecifier::Bytes(ranges) => ranges
            .iter()
            .all(|r| resolve_byte_range(r, resource_size).is_some()),
        RangeSpecifier::Unsatisfiable => false,
    }
}

/// Check if the request has a Range header
pub fn has_range_header(headers: &crate::HeaderMap) -> bool {
    headers.get("range").is_some()
}

/// Get the parsed RangeSpecifier from request headers
pub fn get_range(headers: &crate::HeaderMap) -> Option<RangeSpecifier> {
    headers
        .get("range")
        .and_then(|v| parse_range_header(v.as_str()))
}

/// Build a 206 Partial Content response
///
/// Takes the full data, the requested range, and returns:
/// - The sliced data to send
/// - The Content-Range header value
/// - The status code (206)
pub fn build_partial_response(
    full_data: &[u8],
    range_spec: &RangeSpecifier,
) -> Option<(Vec<u8>, String, u16)> {
    match range_spec {
        RangeSpecifier::Bytes(ranges) => {
            if ranges.is_empty() {
                return None;
            }

            // For now, only support single range (multipart not implemented)
            let range = &ranges[0];
            let resolved = resolve_byte_range(range, full_data.len() as u64)?;

            let slice = full_data[resolved.start as usize..resolved.end as usize].to_vec();
            let content_range = ContentRange {
                unit: "bytes".to_string(),
                range: resolved.clone(),
                complete_size: Some(full_data.len() as u64),
            };

            Some((slice, content_range.to_header_value(), 206))
        }
        RangeSpecifier::Unsatisfiable => None,
    }
}

/// Build 416 Range Not Satisfiable response headers
pub fn range_not_satisfiable_response(resource_size: u64) -> (u16, String) {
    let content_range = format!("bytes */{}", resource_size);
    (416, content_range)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_range_header_simple() {
        let spec = parse_range_header("bytes=0-1023").unwrap();
        match spec {
            RangeSpecifier::Bytes(ranges) => {
                assert_eq!(ranges.len(), 1);
                match &ranges[0] {
                    ByteRange::Range { start, end } => {
                        assert_eq!(*start, 0);
                        assert_eq!(*end, Some(1023));
                    }
                    _ => panic!("expected Range"),
                }
            }
            _ => panic!("expected Bytes"),
        }
    }

    #[test]
    fn test_parse_range_header_open_ended() {
        let spec = parse_range_header("bytes=500-").unwrap();
        match spec {
            RangeSpecifier::Bytes(ranges) => {
                assert_eq!(ranges.len(), 1);
                match &ranges[0] {
                    ByteRange::Range { start, end } => {
                        assert_eq!(*start, 500);
                        assert!(end.is_none());
                    }
                    _ => panic!("expected Range"),
                }
            }
            _ => panic!("expected Bytes"),
        }
    }

    #[test]
    fn test_parse_range_header_suffix() {
        let spec = parse_range_header("bytes=-500").unwrap();
        match spec {
            RangeSpecifier::Bytes(ranges) => {
                assert_eq!(ranges.len(), 1);
                match &ranges[0] {
                    ByteRange::Suffix(len) => assert_eq!(*len, 500),
                    _ => panic!("expected Suffix"),
                }
            }
            _ => panic!("expected Bytes"),
        }
    }

    #[test]
    fn test_parse_range_header_multiple() {
        let spec = parse_range_header("bytes=0-100, 200-300").unwrap();
        match spec {
            RangeSpecifier::Bytes(ranges) => {
                assert_eq!(ranges.len(), 2);
            }
            _ => panic!("expected Bytes"),
        }
    }

    #[test]
    fn test_resolve_byte_range() {
        let resource_size = 1000;

        // Standard range
        let range = ByteRange::Range {
            start: 100,
            end: Some(199),
        };
        let resolved = resolve_byte_range(&range, resource_size).unwrap();
        assert_eq!(resolved, 100..200);

        // Open-ended range
        let range = ByteRange::Range {
            start: 500,
            end: None,
        };
        let resolved = resolve_byte_range(&range, resource_size).unwrap();
        assert_eq!(resolved, 500..1000);

        // Suffix range
        let range = ByteRange::Suffix(100);
        let resolved = resolve_byte_range(&range, resource_size).unwrap();
        assert_eq!(resolved, 900..1000);
    }

    #[test]
    fn test_resolve_unsatisfiable_range() {
        let resource_size = 100;

        // Start beyond resource
        let range = ByteRange::Range {
            start: 200,
            end: Some(300),
        };
        assert!(resolve_byte_range(&range, resource_size).is_none());

        // Empty suffix
        let range = ByteRange::Suffix(0);
        assert!(resolve_byte_range(&range, resource_size).is_none());
    }

    #[test]
    fn test_content_range_parsing() {
        let cr = ContentRange::from_header_value("bytes 0-999/10000").unwrap();
        assert_eq!(cr.range, 0..1000);
        assert_eq!(cr.complete_size, Some(10000));
    }

    #[test]
    fn test_content_range_format() {
        let cr = ContentRange {
            unit: "bytes".to_string(),
            range: 0..1000,
            complete_size: Some(10000),
        };
        assert_eq!(cr.to_header_value(), "bytes 0-999/10000");
    }

    #[test]
    fn test_content_range_unknown_size() {
        let cr = ContentRange::from_header_value("bytes 0-999/*").unwrap();
        assert_eq!(cr.complete_size, None);
    }

    #[test]
    fn test_build_partial_response() {
        let data = b"Hello, World!";
        let spec = parse_range_header("bytes=0-4").unwrap();

        let (body, content_range, status) = build_partial_response(data, &spec).unwrap();
        assert_eq!(status, 206);
        assert_eq!(body, b"Hello");
        assert_eq!(content_range, "bytes 0-4/13");
    }

    #[test]
    fn test_range_not_satisfiable() {
        let (status, content_range) = range_not_satisfiable_response(1000);
        assert_eq!(status, 416);
        assert_eq!(content_range, "bytes */1000");
    }

    #[test]
    fn test_is_range_satisfiable() {
        let spec = parse_range_header("bytes=0-100").unwrap();
        assert!(is_range_satisfiable(&spec, 1000));
        // Range 0-100 fits in 101 bytes (0..=100 is 101 bytes)
        assert!(is_range_satisfiable(&spec, 101));
    }

    #[test]
    fn test_has_range_header() {
        let mut headers = crate::HeaderMap::new();
        headers.insert("range", "bytes=0-100").unwrap();
        assert!(has_range_header(&headers));

        let headers = crate::HeaderMap::new();
        assert!(!has_range_header(&headers));
    }
}
