//! URL parsing for edgerun (RFC 3986 subset).
//!
//! Supports: scheme, authority, path, query, fragment
//! Does not support: userinfo, complex relative URL resolution

#![no_std]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use edgerun_json::{FromJson, JsonValue, JsonValueError, ToJson};

/// A parsed URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    scheme: String,
    authority: Authority,
    path: String,
    query: Option<String>,
    fragment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authority {
    host: String,
    port: Option<u16>,
}

/// Error parsing a URL.
pub struct ParseError {
    msg: &'static str,
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl core::fmt::Debug for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ParseError: {}", self.msg)
    }
}

impl Url {
    /// Parse a URL string.
    pub fn parse(input: &str) -> Result<Url, ParseError> {
        parse_url(input)
    }

    /// Get the scheme (e.g., "https").
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    /// Get the host.
    pub fn host(&self) -> &str {
        &self.authority.host
    }

    /// Get the port, if specified.
    pub fn port(&self) -> Option<u16> {
        self.authority.port
    }

    /// Get the path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Set the URL scheme. Returns `Err(())` if the scheme is invalid.
    pub fn set_scheme(&mut self, scheme: &str) -> Result<(), ()> {
        if scheme.is_empty()
            || !scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
        {
            return Err(());
        }
        self.scheme = scheme.to_ascii_lowercase();
        Ok(())
    }

    /// Set the URL path.
    pub fn set_path(&mut self, path: &str) {
        self.path = path.to_string();
    }

    /// Append a query pair using RFC 3986 percent encoding.
    pub fn append_query_pair(&mut self, key: &str, value: &str) {
        fn is_unreserved(b: u8) -> bool {
            matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~')
        }

        fn encode(value: &str) -> String {
            const HEX: &[u8; 16] = b"0123456789ABCDEF";
            let mut out = String::with_capacity(value.len());
            for &b in value.as_bytes() {
                if is_unreserved(b) {
                    out.push(b as char);
                } else {
                    out.push('%');
                    out.push(HEX[(b >> 4) as usize] as char);
                    out.push(HEX[(b & 0x0f) as usize] as char);
                }
            }
            out
        }

        let pair = format!("{}={}", encode(key), encode(value));
        match &mut self.query {
            Some(query) if !query.is_empty() => {
                query.push('&');
                query.push_str(&pair);
            }
            Some(query) => query.push_str(&pair),
            None => self.query = Some(pair),
        }
    }

    /// Get the query string, if present.
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    /// Get the fragment, if present.
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    /// Convert back to string.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        s.push_str(&self.scheme);
        s.push_str("://");
        s.push_str(&self.authority.host);
        if let Some(port) = self.authority.port {
            use core::fmt::Write;
            let _ = write!(s, ":{port}");
        }
        if !self.path.is_empty() {
            if !self.path.starts_with('/') {
                s.push('/');
            }
            s.push_str(self.path.trim_start_matches('/'));
        }
        if let Some(ref q) = self.query {
            s.push('?');
            s.push_str(q);
        }
        if let Some(ref f) = self.fragment {
            s.push('#');
            s.push_str(f);
        }
        s
    }
}

impl core::fmt::Display for Url {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl ToJson for Url {
    fn to_json(&self) -> JsonValue {
        JsonValue::String(self.to_string())
    }
}

impl FromJson for Url {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let value = String::from_json(value)?;
        Url::parse(&value).map_err(|_| JsonValueError::WrongType("invalid URL".to_string()))
    }
}

fn parse_url(input: &str) -> Result<Url, ParseError> {
    let input = input.trim();

    // Must have scheme
    let (rest, scheme) = parse_scheme(input).ok_or(ParseError {
        msg: "missing scheme",
    })?;
    if rest.len() < 3 || &rest[0..2] != "//" {
        return Err(ParseError { msg: "missing //" });
    }
    let rest = &rest[2..];

    // Parse authority (host:port)
    let (rest, authority) = parse_authority(rest).ok_or(ParseError {
        msg: "invalid authority",
    })?;

    // Parse path
    let path = if rest.is_empty() || rest.starts_with('?') || rest.starts_with('#') {
        String::new()
    } else if rest.starts_with('/') {
        let end = rest.find(['?', '#']).unwrap_or(rest.len());
        rest[..end].to_string()
    } else {
        let end = rest.find(['?', '#']).unwrap_or(rest.len());
        rest[..end].to_string()
    };
    let rest = if path.is_empty() {
        rest
    } else {
        let end = rest.find(['?', '#']).unwrap_or(rest.len());
        &rest[end..]
    };

    // Parse query
    let query = if let Some(rest) = rest.strip_prefix('?') {
        let end = rest.find('#').unwrap_or(rest.len());
        Some(rest[..end].to_string())
    } else {
        None
    };
    let rest = if query.is_some() {
        if let Some(rest) = rest.strip_prefix('?') {
            let end = rest.find('#').unwrap_or(rest.len());
            &rest[end..]
        } else {
            rest
        }
    } else {
        rest
    };

    // Parse fragment
    let fragment = rest.strip_prefix('#').map(|rest| rest.to_string());

    Ok(Url {
        scheme,
        authority,
        path,
        query,
        fragment,
    })
}

fn parse_scheme(input: &str) -> Option<(&str, String)> {
    let colon = input.find(':')?;
    let scheme = input[..colon].to_lowercase();
    if scheme.is_empty()
        || !scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
    {
        return None;
    }
    Some((&input[colon + 1..], scheme))
}

fn parse_authority(input: &str) -> Option<(&str, Authority)> {
    let slash = input.find('/');
    let question = input.find('?');
    let hash = input.find('#');
    let end = [slash, question, hash]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(input.len());

    let authority_str = &input[..end];
    if authority_str.is_empty() {
        return None;
    }
    let rest = &input[end..];

    let (host, port) = if let Some(colon) = authority_str.rfind(':') {
        let port_str = &authority_str[colon + 1..];
        let host = &authority_str[..colon];
        if host.is_empty() {
            return None;
        }
        (host.to_string(), port_str.parse().ok())
    } else {
        (authority_str.to_string(), None)
    };

    Some((rest, Authority { host, port }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_https() {
        let url = Url::parse("https://example.com").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host(), "example.com");
    }

    #[test]
    fn test_parse_with_port() {
        let url = Url::parse("https://example.com:8443").unwrap();
        assert_eq!(url.port(), Some(8443));
    }

    #[test]
    fn test_roundtrip() {
        let original = "https://acme-v02.api.letsencrypt.org/directory";
        let url = Url::parse(original).unwrap();
        assert_eq!(url.to_string(), original);
    }
}
