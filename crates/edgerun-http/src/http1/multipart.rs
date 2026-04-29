//! Multipart form data parsing (RFC 2046)
//!
//! Supports parsing `multipart/form-data` requests as used in HTML forms
//! and file uploads.
//!
//! # Usage
//! ```rust
//! use edgerun_http::http1::multipart::parse_multipart;
//!
//! let boundary = "boundary";
//! let body = b"--boundary\r\nContent-Disposition: form-data; name=\"field\"\r\n\r\nvalue\r\n--boundary--\r\n";
//! let parts = parse_multipart(body, boundary).unwrap();
//! for part in parts {
//!     assert_eq!(part.name, "field");
//!     assert_eq!(part.text(), Some("value"));
//! }
//! ```

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

/// A parsed multipart form field
#[derive(Debug, Clone)]
pub struct MultipartField {
    /// The field name (from Content-Disposition `name` parameter)
    pub name: String,
    /// The filename (from Content-Disposition `filename` parameter), if present
    pub filename: Option<String>,
    /// Content-Type of the field, if present
    pub content_type: Option<String>,
    /// The field data
    pub data: Vec<u8>,
    /// Additional headers
    pub headers: BTreeMap<String, String>,
}

impl MultipartField {
    /// Get the data as a UTF-8 string
    pub fn text(&self) -> Option<&str> {
        core::str::from_utf8(&self.data).ok()
    }
}

/// Parse multipart/form-data body
///
/// `body` is the raw request body bytes
/// `boundary` is the boundary string from the Content-Type header
///   (e.g., "----WebKitFormBoundary7MA4YWxkTrZu0gW" — without the leading `--`)
pub fn parse_multipart(body: &[u8], boundary: &str) -> Result<Vec<MultipartField>, MultipartError> {
    let mut delimiter = Vec::with_capacity(boundary.len() + 2);
    delimiter.extend_from_slice(b"--");
    delimiter.extend_from_slice(boundary.as_bytes());
    let delimiter_bytes = delimiter.as_slice();

    let mut fields = Vec::new();
    let mut pos = 0;

    // Skip preamble (data before first boundary)
    pos = skip_to_boundary(body, delimiter_bytes, pos)?;
    pos += delimiter_bytes.len();

    // Skip trailing CRLF after boundary
    pos = skip_crlf(body, pos);

    loop {
        if pos >= body.len() {
            break;
        }

        // Check for end delimiter
        if is_end_boundary(body, delimiter_bytes, pos) {
            break;
        }

        // Parse headers
        let (headers, headers_end) = parse_headers(&body[pos..])?;
        pos += headers_end;

        // Extract Content-Disposition
        let content_disposition = headers
            .get("content-disposition")
            .ok_or(MultipartError::MissingContentDisposition)?;

        let name = extract_param(content_disposition, "name").ok_or(MultipartError::MissingName)?;
        let filename = extract_param(content_disposition, "filename");

        // Extract Content-Type
        let content_type = headers.get("content-type").cloned();

        // Read body data until next boundary
        let data_start = pos;

        // Look for \r\n--boundary or --boundary-- pattern
        let next_boundary = find_next_boundary(body, delimiter_bytes, pos)?;
        let data = body[data_start..next_boundary].to_vec();

        // Remove trailing CRLF from data if present
        let data = strip_trailing_crlf(&data);

        fields.push(MultipartField {
            name,
            filename,
            content_type,
            data: data.to_vec(),
            headers,
        });

        pos = next_boundary;

        // Skip the boundary delimiter (\r\n--boundary or \r\n--boundary--)
        if body[pos..].starts_with(b"\r\n") {
            pos += 2;
        }

        // Check for end delimiter
        if is_end_boundary(body, delimiter_bytes, pos) {
            break;
        }

        if body[pos..].starts_with(delimiter_bytes) {
            pos += delimiter_bytes.len();
            pos = skip_crlf(body, pos);
        }
    }

    Ok(fields)
}

/// Extract boundary from Content-Type header value
///
/// e.g., `multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW`
pub fn extract_boundary(content_type: &str) -> Option<String> {
    extract_param(content_type, "boundary")
}

/// Check if Content-Type is multipart/form-data
pub fn is_multipart(content_type: &str) -> bool {
    content_type
        .get(..19)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("multipart/form-data"))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum MultipartError {
    MissingBoundary,
    MissingContentDisposition,
    MissingName,
    UnexpectedEof,
    InvalidUtf8,
}

impl fmt::Display for MultipartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MultipartError::MissingBoundary => write!(f, "missing boundary"),
            MultipartError::MissingContentDisposition => write!(f, "missing Content-Disposition"),
            MultipartError::MissingName => write!(f, "missing name parameter"),
            MultipartError::UnexpectedEof => write!(f, "unexpected EOF"),
            MultipartError::InvalidUtf8 => write!(f, "invalid UTF-8"),
        }
    }
}

impl core::error::Error for MultipartError {}

fn skip_to_boundary(body: &[u8], delimiter: &[u8], start: usize) -> Result<usize, MultipartError> {
    for i in start..body.len().saturating_sub(delimiter.len() - 1) {
        if body[i..i + delimiter.len()] == *delimiter {
            return Ok(i);
        }
    }
    Err(MultipartError::UnexpectedEof)
}

fn find_next_boundary(
    body: &[u8],
    delimiter: &[u8],
    start: usize,
) -> Result<usize, MultipartError> {
    // Search for regular delimiter first: \r\n--boundary
    let crlf = b"\r\n";
    for i in start..body.len().saturating_sub(crlf.len() + delimiter.len() - 1) {
        if body[i..i + crlf.len()] == *crlf {
            let after_crlf = i + crlf.len();
            if body.len() >= after_crlf + delimiter.len()
                && body[after_crlf..after_crlf + delimiter.len()] == *delimiter
            {
                return Ok(i);
            }
        }
    }

    // Fallback: take rest of body
    Ok(body.len())
}

fn is_end_boundary(body: &[u8], delimiter: &[u8], pos: usize) -> bool {
    body[pos..].starts_with(delimiter)
        && body.get(pos + delimiter.len()..pos + delimiter.len() + 2) == Some(b"--")
}

fn skip_crlf(body: &[u8], start: usize) -> usize {
    let mut pos = start;
    if pos < body.len() && body[pos] == b'\r' {
        pos += 1;
    }
    if pos < body.len() && body[pos] == b'\n' {
        pos += 1;
    }
    pos
}

fn parse_headers(body: &[u8]) -> Result<(BTreeMap<String, String>, usize), MultipartError> {
    let mut headers = BTreeMap::new();
    let mut pos = 0;

    loop {
        if pos >= body.len() {
            return Err(MultipartError::UnexpectedEof);
        }

        // Check for blank line (end of headers)
        if pos + 1 < body.len() && body[pos] == b'\r' && body[pos + 1] == b'\n' {
            pos += 2;
            break;
        }

        // Find end of line
        let line_end = body[pos..]
            .windows(2)
            .position(|w| w == b"\r\n")
            .map(|i| pos + i)
            .unwrap_or(body.len());

        let line =
            core::str::from_utf8(&body[pos..line_end]).map_err(|_| MultipartError::InvalidUtf8)?;

        if let Some(colon) = line.find(':') {
            let name = ascii_lowercase(line[..colon].trim());
            let value = line[colon + 1..].trim().to_string();
            headers.insert(name, value);
        }

        pos = line_end + 2;
    }

    Ok((headers, pos))
}

fn ascii_lowercase(value: &str) -> String {
    value
        .bytes()
        .map(|b| b.to_ascii_lowercase() as char)
        .collect()
}

fn extract_param(header_value: &str, param_name: &str) -> Option<String> {
    header_value.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        if !name.trim().eq_ignore_ascii_case(param_name) {
            return None;
        }

        let value = value.trim();
        if let Some(quoted) = value.strip_prefix('"') {
            return quoted.find('"').map(|end| quoted[..end].to_string());
        }

        let end = value.find(char::is_whitespace).unwrap_or(value.len());
        Some(value[..end].to_string())
    })
}

fn strip_trailing_crlf(data: &[u8]) -> &[u8] {
    if data.len() >= 2 && data[data.len() - 2..] == [b'\r', b'\n'] {
        &data[..data.len() - 2]
    } else {
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_boundary() {
        let ct = "multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW";
        let boundary = extract_boundary(ct);
        assert_eq!(
            boundary.as_deref(),
            Some("----WebKitFormBoundary7MA4YWxkTrZu0gW")
        );
    }

    #[test]
    fn test_extract_boundary_quoted() {
        let ct = r#"multipart/form-data; boundary="my-boundary""#;
        let boundary = extract_boundary(ct);
        assert_eq!(boundary.as_deref(), Some("my-boundary"));
    }

    #[test]
    fn test_is_multipart() {
        assert!(is_multipart("multipart/form-data; boundary=xyz"));
        assert!(is_multipart("MULTIPART/FORM-DATA; boundary=xyz"));
        assert!(!is_multipart("application/x-www-form-urlencoded"));
        assert!(!is_multipart("multipart/mixed; boundary=xyz"));
    }

    #[test]
    fn test_parse_simple_multipart() {
        let boundary = "myboundary";
        let body = b"--myboundary\r\nContent-Disposition: form-data; name=\"field1\"\r\n\r\nvalue1\r\n--myboundary--\r\n";

        let fields = parse_multipart(body, boundary).unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "field1");
        assert_eq!(fields[0].filename, None);
        assert_eq!(fields[0].text(), Some("value1"));
    }

    #[test]
    fn test_parse_multipart_with_filename() {
        let boundary = "myboundary";
        let body = b"--myboundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\nContent-Type: text/plain\r\n\r\nfile contents\r\n--myboundary--\r\n";

        let fields = parse_multipart(body, boundary).unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "file");
        assert_eq!(fields[0].filename.as_deref(), Some("test.txt"));
        assert_eq!(fields[0].content_type.as_deref(), Some("text/plain"));
        assert_eq!(fields[0].text(), Some("file contents"));
    }

    #[test]
    fn test_parse_multipart_multiple_fields() {
        let boundary = "myboundary";
        let body = b"--myboundary\r\nContent-Disposition: form-data; name=\"field1\"\r\n\r\nvalue1\r\n--myboundary\r\nContent-Disposition: form-data; name=\"field2\"\r\n\r\nvalue2\r\n--myboundary--\r\n";

        let fields = parse_multipart(body, boundary).unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "field1");
        assert_eq!(fields[0].text(), Some("value1"));
        assert_eq!(fields[1].name, "field2");
        assert_eq!(fields[1].text(), Some("value2"));
    }

    #[test]
    fn test_extract_param_quoted() {
        let header = r#"form-data; name="myfield"; filename="myfile.txt""#;
        assert_eq!(extract_param(header, "name"), Some("myfield".to_string()));
        assert_eq!(
            extract_param(header, "filename"),
            Some("myfile.txt".to_string())
        );
    }

    #[test]
    fn test_extract_param_unquoted() {
        let header = "multipart/form-data; boundary=myboundary";
        assert_eq!(
            extract_param(header, "boundary"),
            Some("myboundary".to_string())
        );
    }

    #[test]
    fn test_extract_param_case_insensitive_name() {
        let header = r#"form-data; Name="myfield"; FILENAME="myfile.txt""#;
        assert_eq!(extract_param(header, "name"), Some("myfield".to_string()));
        assert_eq!(
            extract_param(header, "filename"),
            Some("myfile.txt".to_string())
        );
    }
}
