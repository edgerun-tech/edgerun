//! Multipart form data parsing (RFC 2046)
//!
//! Supports parsing `multipart/form-data` requests as used in HTML forms
//! and file uploads.
//!
//! # Usage
//! ```ignore
//! use edgerun_http::http1::multipart::MultipartParser;
//!
//! let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
//! let body = b"--boundary\r\nContent-Disposition: form-data; name=\"field\"\r\n\r\nvalue\r\n--boundary--\r\n";
//! let parts = MultipartParser::parse(body, boundary).unwrap();
//! for part in parts {
//!     println!("{}: {}", part.name, String::from_utf8_lossy(&part.data));
//! }
//! ```

use std::collections::HashMap;

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
    pub headers: HashMap<String, String>,
}

impl MultipartField {
    /// Get the data as a UTF-8 string
    pub fn text(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }
}

/// Parse multipart/form-data body
///
/// `body` is the raw request body bytes
/// `boundary` is the boundary string from the Content-Type header
///   (e.g., "----WebKitFormBoundary7MA4YWxkTrZu0gW" — without the leading `--`)
pub fn parse_multipart(body: &[u8], boundary: &str) -> Result<Vec<MultipartField>, MultipartError> {
    let delimiter = format!("--{}", boundary);
    let delimiter_bytes = delimiter.as_bytes();
    let end_delimiter = format!("--{}--", boundary);
    let end_delimiter_bytes = end_delimiter.as_bytes();

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
        if body[pos..].starts_with(end_delimiter_bytes) {
            break;
        }

        // Parse headers
        let (headers, headers_end) = parse_headers(&body[pos..])?;
        pos += headers_end;

        // Extract Content-Disposition
        let content_disposition = headers.get("content-disposition")
            .ok_or_else(|| MultipartError::MissingContentDisposition)?;

        let name = extract_param(content_disposition, "name")
            .ok_or_else(|| MultipartError::MissingName)?;
        let filename = extract_param(content_disposition, "filename");

        // Extract Content-Type
        let content_type = headers.get("content-type").cloned();

        // Read body data until next boundary
        let data_start = pos;

        // Look for \r\n--boundary or --boundary-- pattern
        let next_boundary = find_next_boundary(body, delimiter_bytes, end_delimiter_bytes, pos)?;
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
        if body[pos..].starts_with(end_delimiter_bytes) {
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
    content_type.to_lowercase().starts_with("multipart/form-data")
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

impl std::fmt::Display for MultipartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MultipartError::MissingBoundary => write!(f, "missing boundary"),
            MultipartError::MissingContentDisposition => write!(f, "missing Content-Disposition"),
            MultipartError::MissingName => write!(f, "missing name parameter"),
            MultipartError::UnexpectedEof => write!(f, "unexpected EOF"),
            MultipartError::InvalidUtf8 => write!(f, "invalid UTF-8"),
        }
    }
}

impl std::error::Error for MultipartError {}

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
    end_delimiter: &[u8],
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

    // Then check for end delimiter: \r\n--boundary--
    let crlf_end = b"\r\n";
    for i in start..body.len().saturating_sub(crlf_end.len() + end_delimiter.len() - 1) {
        if body[i..i + crlf_end.len()] == *crlf_end {
            let after_crlf = i + crlf_end.len();
            if body.len() >= after_crlf + end_delimiter.len()
                && body[after_crlf..after_crlf + end_delimiter.len()] == *end_delimiter
            {
                return Ok(i);
            }
        }
    }

    // Fallback: take rest of body
    Ok(body.len())
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

fn parse_headers(body: &[u8]) -> Result<(HashMap<String, String>, usize), MultipartError> {
    let mut headers = HashMap::new();
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

        let line = std::str::from_utf8(&body[pos..line_end])
            .map_err(|_| MultipartError::InvalidUtf8)?;

        if let Some(colon) = line.find(':') {
            let name = line[..colon].trim().to_lowercase();
            let value = line[colon + 1..].trim().to_string();
            headers.insert(name, value);
        }

        pos = line_end + 2;
    }

    Ok((headers, pos))
}

fn extract_param(header_value: &str, param_name: &str) -> Option<String> {
    // Try quoted value first: name="value"
    let pattern = format!("{}=\"", param_name);
    if let Some(start) = header_value.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = header_value[value_start..].find('"') {
            return Some(header_value[value_start..value_start + end].to_string());
        }
    }

    // Try unquoted value: name=value
    let pattern = format!("{}=", param_name);
    if let Some(start) = header_value.find(&pattern) {
        let value_start = start + pattern.len();
        let value_end = header_value[value_start..]
            .find(|c: char| c.is_whitespace() || c == ';')
            .map(|i| value_start + i)
            .unwrap_or(header_value.len());
        return Some(header_value[value_start..value_end].to_string());
    }

    None
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
        assert_eq!(boundary.as_deref(), Some("----WebKitFormBoundary7MA4YWxkTrZu0gW"));
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
        assert_eq!(extract_param(header, "filename"), Some("myfile.txt".to_string()));
    }

    #[test]
    fn test_extract_param_unquoted() {
        let header = "multipart/form-data; boundary=myboundary";
        assert_eq!(extract_param(header, "boundary"), Some("myboundary".to_string()));
    }
}
