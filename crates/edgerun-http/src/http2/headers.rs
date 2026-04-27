//! HTTP/2 request/response header validation (RFC 9113 §8.1)
//!
//! This module implements stateless validation of decoded HTTP/2 header blocks.
//! It does not modify HPACK state or interact with streams — it only checks
//! that a decoded `[(Vec<u8>, Vec<u8>)]` slice conforms to RFC 9113.
//!
//! # Validation Rules
//!
//! For **request headers** (client → server):
//! - Required pseudo-headers present: `:method`, `:scheme`, `:path`
//! - No duplicate pseudo-headers
//! - No unknown pseudo-headers
//! - No response pseudo-headers (`:status`) in requests
//! - Pseudo-headers appear before regular headers
//! - No connection-specific headers (`Connection`, `Keep-Alive`, etc.)
//! - `TE: trailers` is allowed; other `TE` values are rejected
//! - `:path` MUST NOT be empty for non-CONNECT requests
//!
//! For **header name case** (RFC 9113 §8.2):
//! - All header field names MUST be lowercase
//!
//! # Unit Tests
//!
//! 29 tests covering valid requests, missing pseudo-headers, duplicates,
//! unknown/response pseudo-headers, ordering, connection-specific headers,
//! and case validation. Run with:
//! ```text
//! cargo test -p edgerun-http --lib -- headers::
//! ```
//!
//! # See Also
//!
//! - [H2SPEC_ANALYSIS.md](../H2SPEC_ANALYSIS.md) for the h2spec analysis process
//! - [RFC 9113 §8.1](https://www.rfc-editor.org/rfc/rfc9113.html#name-http-fields) — HTTP Fields
//! - [RFC 9113 §8.2](https://www.rfc-editor.org/rfc/rfc9113.html#name-http-field-validity) — HTTP Field Validity

#[cfg(target_os = "none")]
use crate::prelude::v1::*;

use crate::http2::ErrorCode;

/// Result of header validation.
/// `Ok` means headers are valid.
/// `Err` contains the error code and a description.
pub type ValidationResult = std::result::Result<(), (u32, &'static str)>;

/// Validate HTTP/2 request headers (client → server direction).
///
/// Checks per RFC 9113 §8.1:
/// - Required pseudo-headers present (`:method`, `:scheme`, `:path`)
/// - No duplicate pseudo-headers
/// - No unknown pseudo-headers
/// - No response pseudo-headers (`:status`) in requests
/// - Pseudo-headers appear before regular headers
/// - No connection-specific headers
///
/// This function is **stateless** — it only validates the decoded header block.
/// It does not validate content-length vs body length, or other request-body
/// semantics.
pub fn validate_request_headers(headers: &[(Vec<u8>, Vec<u8>)]) -> ValidationResult {
    let mut method_count = 0u32;
    let mut scheme_count = 0u32;
    let mut path_count = 0u32;
    let mut saw_regular_header = false;

    for (name, value) in headers {
        let name_str = String::from_utf8_lossy(name);

        if name_str.starts_with(':') {
            // Pseudo-header field
            if saw_regular_header {
                return Err((
                    ErrorCode::PROTOCOL_ERROR.to_u32(),
                    "pseudo-header after regular header",
                ));
            }
            match name_str.as_ref() {
                ":method" => method_count += 1,
                ":scheme" => scheme_count += 1,
                ":path" => path_count += 1,
                ":authority" => {} // optional for CONNECT, harmless otherwise
                ":status" => {
                    return Err((
                        ErrorCode::PROTOCOL_ERROR.to_u32(),
                        "response pseudo-header in request",
                    ));
                }
                _ => {
                    return Err((ErrorCode::PROTOCOL_ERROR.to_u32(), "unknown pseudo-header"));
                }
            }
        } else {
            // Regular header — check for connection-specific headers
            let name_lower = name_str.to_lowercase();
            if is_connection_specific_header(&name_lower) {
                // Exception: "te" with value "trailers" is allowed
                if name_lower == "te" {
                    let value_str = String::from_utf8_lossy(value).to_lowercase();
                    if value_str != "trailers" {
                        return Err((
                            ErrorCode::PROTOCOL_ERROR.to_u32(),
                            "TE header with non-trailers value",
                        ));
                    }
                } else {
                    return Err((
                        ErrorCode::PROTOCOL_ERROR.to_u32(),
                        "connection-specific header present",
                    ));
                }
            }
            saw_regular_header = true;
        }
    }

    // Required pseudo-headers
    if method_count == 0 {
        return Err((ErrorCode::PROTOCOL_ERROR.to_u32(), "missing :method"));
    }
    if scheme_count == 0 {
        return Err((ErrorCode::PROTOCOL_ERROR.to_u32(), "missing :scheme"));
    }
    if path_count == 0 {
        return Err((ErrorCode::PROTOCOL_ERROR.to_u32(), "missing :path"));
    }

    // No duplicates
    if method_count > 1 {
        return Err((ErrorCode::PROTOCOL_ERROR.to_u32(), "duplicate :method"));
    }
    if scheme_count > 1 {
        return Err((ErrorCode::PROTOCOL_ERROR.to_u32(), "duplicate :scheme"));
    }
    if path_count > 1 {
        return Err((ErrorCode::PROTOCOL_ERROR.to_u32(), "duplicate :path"));
    }

    // :path MUST NOT be empty for non-CONNECT requests (RFC 9113 §8.3.1)
    for (name, value) in headers {
        let name_str = String::from_utf8_lossy(name);
        if name_str == ":path" && value.is_empty() {
            return Err((ErrorCode::PROTOCOL_ERROR.to_u32(), "empty :path"));
        }
    }

    Ok(())
}

/// Check if a header name is a connection-specific header that must not
/// appear in HTTP/2 (RFC 9113 §8.2.2).
pub fn is_connection_specific_header(name_lower: &str) -> bool {
    matches!(
        name_lower,
        "connection"
            | "keep-alive"
            | "proxy-connection"
            | "transfer-encoding"
            | "upgrade"
            | "http2-settings"
            | "te"
    )
}

/// Validate that all header field names are lowercase (RFC 9113 §8.2).
/// HTTP/2 requires all header names to be lowercase.
pub fn validate_header_name_case(headers: &[(Vec<u8>, Vec<u8>)]) -> ValidationResult {
    for (name, _) in headers {
        let name_str = String::from_utf8_lossy(name);
        // Pseudo-headers always start with ':' and are lowercase by definition
        if !name_str.starts_with(':') && name_str.to_lowercase() != name_str {
            return Err((
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                "header field name not lowercase",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ──

    fn h(name: &str, value: &str) -> (Vec<u8>, Vec<u8>) {
        (name.as_bytes().to_vec(), value.as_bytes().to_vec())
    }

    fn minimal_valid_request() -> Vec<(Vec<u8>, Vec<u8>)> {
        vec![h(":method", "GET"), h(":scheme", "https"), h(":path", "/")]
    }

    // ── Valid request headers ──

    #[test]
    fn test_valid_minimal_request() {
        assert!(validate_request_headers(&minimal_valid_request()).is_ok());
    }

    #[test]
    fn test_valid_request_with_authority() {
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":authority", "example.com"),
            h(":path", "/index.html"),
            h("accept", "text/html"),
        ];
        assert!(validate_request_headers(&headers).is_ok());
    }

    #[test]
    fn test_valid_post_request() {
        let headers = vec![
            h(":method", "POST"),
            h(":scheme", "https"),
            h(":path", "/api/data"),
            h("content-type", "application/json"),
            h("content-length", "123"),
        ];
        assert!(validate_request_headers(&headers).is_ok());
    }

    // ── Missing pseudo-headers ──

    #[test]
    fn test_missing_method() {
        let headers = vec![h(":scheme", "https"), h(":path", "/")];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "missing :method");
    }

    #[test]
    fn test_missing_scheme() {
        let headers = vec![h(":method", "GET"), h(":path", "/")];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "missing :scheme");
    }

    #[test]
    fn test_missing_path() {
        let headers = vec![h(":method", "GET"), h(":scheme", "https")];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "missing :path");
    }

    #[test]
    fn test_empty_path() {
        // h2spec test: :path with empty value is NOT valid for non-CONNECT requests
        let headers = vec![h(":method", "GET"), h(":scheme", "https"), h(":path", "")];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "empty :path");
    }

    // ── Duplicate pseudo-headers ──

    #[test]
    fn test_duplicate_method() {
        let headers = vec![
            h(":method", "GET"),
            h(":method", "POST"),
            h(":scheme", "https"),
            h(":path", "/"),
        ];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "duplicate :method");
    }

    #[test]
    fn test_duplicate_scheme() {
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":scheme", "http"),
            h(":path", "/"),
        ];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "duplicate :scheme");
    }

    #[test]
    fn test_duplicate_path() {
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/a"),
            h(":path", "/b"),
        ];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "duplicate :path");
    }

    // ── Unknown / response pseudo-headers ──

    #[test]
    fn test_unknown_pseudo_header() {
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/"),
            h(":unknown", "value"),
        ];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "unknown pseudo-header");
    }

    #[test]
    fn test_response_status_in_request() {
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/"),
            h(":status", "200"),
        ];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "response pseudo-header in request");
    }

    // ── Pseudo-header ordering ──

    #[test]
    fn test_pseudo_header_after_regular() {
        let headers = vec![
            h(":method", "GET"),
            h("accept", "text/html"),
            h(":scheme", "https"), // appears after regular header
            h(":path", "/"),
        ];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "pseudo-header after regular header");
    }

    #[test]
    fn test_pseudo_header_as_trailers() {
        // h2spec: pseudo-header appearing as trailer (after regular headers)
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/"),
            h("x-custom", "value"),
            h(":extra", "trailer"), // pseudo-header after regular header
        ];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "pseudo-header after regular header");
    }

    // ── Connection-specific headers ──

    #[test]
    fn test_connection_header_rejected() {
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/"),
            h("connection", "keep-alive"),
        ];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "connection-specific header present");
    }

    #[test]
    fn test_keep_alive_rejected() {
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/"),
            h("keep-alive", "timeout=5"),
        ];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "connection-specific header present");
    }

    #[test]
    fn test_transfer_encoding_rejected() {
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/"),
            h("transfer-encoding", "chunked"),
        ];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "connection-specific header present");
    }

    #[test]
    fn test_upgrade_rejected() {
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/"),
            h("upgrade", "h2c"),
        ];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "connection-specific header present");
    }

    #[test]
    fn test_http2_settings_rejected() {
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/"),
            h("http2-settings", "AAMAAABkAAQAAP__"),
        ];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "connection-specific header present");
    }

    #[test]
    fn test_te_with_trailers_allowed() {
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/"),
            h("te", "trailers"),
        ];
        assert!(validate_request_headers(&headers).is_ok());
    }

    #[test]
    fn test_te_with_other_value_rejected() {
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/"),
            h("te", "gzip"),
        ];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "TE header with non-trailers value");
    }

    // ── Header name case validation ──

    #[test]
    fn test_lowercase_header_names_valid() {
        let headers = vec![h("content-type", "text/html"), h("accept-encoding", "gzip")];
        assert!(validate_header_name_case(&headers).is_ok());
    }

    #[test]
    fn test_uppercase_header_name_rejected() {
        let headers = vec![h("Content-Type", "text/html")];
        let err = validate_header_name_case(&headers).unwrap_err();
        assert_eq!(err.1, "header field name not lowercase");
    }

    #[test]
    fn test_mixed_case_header_name_rejected() {
        let headers = vec![h("X-Custom-Header", "value")];
        let err = validate_header_name_case(&headers).unwrap_err();
        assert_eq!(err.1, "header field name not lowercase");
    }

    #[test]
    fn test_pseudo_header_case_ignored_in_name_check() {
        // Pseudo-headers are not checked for lowercase in the name case validator
        let headers = vec![h(":method", "GET"), h("Host", "example.com")];
        let err = validate_header_name_case(&headers).unwrap_err();
        assert_eq!(err.1, "header field name not lowercase");
    }

    // ── Connection-specific header list completeness ──

    #[test]
    fn test_all_connection_specific_headers_listed() {
        // RFC 9113 §8.2.2 lists these as connection-specific
        let connection_headers = [
            "connection",
            "keep-alive",
            "proxy-connection",
            "transfer-encoding",
            "upgrade",
            "http2-settings",
        ];
        for name in connection_headers {
            assert!(
                is_connection_specific_header(name),
                "is_connection_specific_header({name}) should be true"
            );
        }
    }

    // ── Edge cases ──

    #[test]
    fn test_empty_headers_list() {
        // No headers at all — missing pseudo-headers
        let err = validate_request_headers(&[]).unwrap_err();
        assert_eq!(err.1, "missing :method");
    }

    #[test]
    fn test_only_regular_headers() {
        let headers = vec![h("host", "example.com"), h("accept", "*/*")];
        let err = validate_request_headers(&headers).unwrap_err();
        assert_eq!(err.1, "missing :method");
    }

    #[test]
    fn test_multiple_regular_headers_ok() {
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/"),
            h("host", "example.com"),
            h("accept", "*/*"),
            h("user-agent", "test"),
            h("x-custom", "value"),
        ];
        assert!(validate_request_headers(&headers).is_ok());
    }
}
