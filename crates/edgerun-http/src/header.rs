//! HTTP headers

use std::fmt;

/// HTTP header name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HeaderName(String);

impl HeaderName {
    /// Create a new header name
    pub fn new(name: String) -> Result<Self, String> {
        if name.is_empty() {
            return Err("Header name cannot be empty".to_string());
        }
        // RFC 9110 Section 5.6.2: token = 1*tchar
        // tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*"
        //       / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
        //       / DIGIT / ALPHA
        if !name.bytes().all(is_tchar) {
            return Err(format!("Invalid header name: {name}"));
        }
        Ok(HeaderName(name))
    }

    /// Get the header name as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Check if a byte is a valid HTTP token character (RFC 9110 Section 5.6.2).
pub fn is_tchar(b: u8) -> bool {
    match b {
        b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+'
        | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~' => true,
        b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => true,
        _ => false,
    }
}

impl fmt::Display for HeaderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for HeaderName {
    type Error = String;

    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        HeaderName::new(s.to_string())
    }
}

impl TryFrom<String> for HeaderName {
    type Error = String;

    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        HeaderName::new(s)
    }
}

/// HTTP header value
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HeaderValue(String);

impl HeaderValue {
    /// Create a new header value
    pub fn new(value: String) -> Result<Self, String> {
        // RFC 9110 Section 5.5: field values can contain visible ASCII
        // (0x21-0x7E), SP (0x20), and HTAB (0x09). No other control chars.
        if !value.bytes().all(|b| {
            b == 0x09            // HTAB
                || b == 0x20     // SP
                || (0x21..=0x7E).contains(&b)  // visible ASCII
        }) {
            return Err(format!("Invalid header value: {value}"));
        }
        Ok(HeaderValue(value))
    }

    /// Get the header value as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for HeaderValue {
    type Error = String;

    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        HeaderValue::new(s.to_string())
    }
}

impl TryFrom<String> for HeaderValue {
    type Error = String;

    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        HeaderValue::new(s)
    }
}

/// HTTP header map
#[derive(Debug, Clone, Default)]
pub struct HeaderMap {
    headers: Vec<(HeaderName, HeaderValue)>,
}

impl HeaderMap {
    /// Create a new empty header map
    pub fn new() -> Self {
        HeaderMap {
            headers: Vec::new(),
        }
    }

    /// Insert a header. Returns `Err` if the name or value is invalid.
    pub fn insert(&mut self, name: &str, value: &str) -> std::result::Result<(), String> {
        let name = HeaderName::new(name.to_string())?;
        let value = HeaderValue::new(value.to_string())?;
        self.headers.push((name, value));
        Ok(())
    }

    /// Get the first value for a header name (case-insensitive)
    pub fn get(&self, name: &str) -> Option<&HeaderValue> {
        self.headers
            .iter()
            .find(|(n, _)| n.as_str().eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
    }

    /// Get all values for a header name (case-insensitive)
    pub fn get_all(&self, name: &str) -> Vec<&HeaderValue> {
        self.headers
            .iter()
            .filter(|(n, _)| n.as_str().eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
            .collect()
    }

    /// Check if a header exists
    pub fn contains_key(&self, name: &str) -> bool {
        self.headers
            .iter()
            .any(|(n, _)| n.as_str().eq_ignore_ascii_case(name))
    }

    /// Get all headers
    pub fn iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.headers.iter().map(|(n, v)| (n, v))
    }

    /// Get the number of headers
    pub fn len(&self) -> usize {
        self.headers.len()
    }

    /// Check if the header map is empty
    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }

    /// Remove all headers
    pub fn clear(&mut self) {
        self.headers.clear();
    }

    /// Format headers for HTTP request
    pub fn to_http_string(&self) -> String {
        self.headers
            .iter()
            .map(|(name, value)| format!("{}: {}\r\n", name.as_str(), value.as_str()))
            .collect()
    }
}

impl fmt::Display for HeaderMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (name, value) in &self.headers {
            writeln!(f, "{}: {}", name, value)?;
        }
        Ok(())
    }
}
