/// Parse headers from raw message data.
pub fn parse_headers(data: &[u8]) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    let text = String::from_utf8_lossy(data);

    for line in text.lines() {
        if line.is_empty() {
            break;
        }
        if let Some(colon_pos) = line.find(':') {
            let name = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            headers.push((name, value));
        }
    }

    headers
}

/// Extract a specific header value.
pub fn get_header(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

/// Parse the Subject header.
pub fn get_subject(data: &[u8]) -> Option<String> {
    let headers = parse_headers(data);
    get_header(&headers, "Subject")
}

/// Parse From header into address.
pub fn get_from_address(data: &[u8]) -> Option<String> {
    let headers = parse_headers(data);
    get_header(&headers, "From")
}

/// Parse Date header.
pub fn get_date(data: &[u8]) -> Option<String> {
    let headers = parse_headers(data);
    get_header(&headers, "Date")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_headers() {
        let data =
            b"From: sender@example.com\r\nTo: recipient@example.com\r\nSubject: Test\r\n\r\nBody";
        let headers = parse_headers(data);
        assert_eq!(headers.len(), 3);
        assert_eq!(
            headers[0],
            ("From".to_string(), "sender@example.com".to_string())
        );
        assert_eq!(
            headers[1],
            ("To".to_string(), "recipient@example.com".to_string())
        );
        assert_eq!(headers[2], ("Subject".to_string(), "Test".to_string()));
    }

    #[test]
    fn test_get_header_helpers() {
        let data = b"From: sender@example.com\r\nSubject: Hello\r\n\r\nHi";
        assert_eq!(
            get_from_address(data),
            Some("sender@example.com".to_string())
        );
        assert_eq!(get_subject(data), Some("Hello".to_string()));
        assert_eq!(get_date(data), None);
    }
}
