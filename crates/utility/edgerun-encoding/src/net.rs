//! Host:port and URL parsing utilities.
//!
//! Provides common parsing for extracting host and port from strings
//! like `"192.168.1.1:8080"` or `"http://example.com:443/path"`.
//!
//! All operations are `no_std` compatible.

use alloc::string::{String, ToString};
use core::str::FromStr;

/// Parse a `host:port` string into `(host, port)`.
///
/// Supports:
/// - `"hostname:8080"` → `Some(("hostname", 8080))`
/// - `"192.168.1.1:443"` → `Some(("192.168.1.1", 443))`
/// - `"hostname"` → `Some(("hostname", default_port))`
/// - `""` or invalid port → `None`
///
/// # Examples
/// ```
/// use edgerun_encoding::net::parse_host_port;
/// assert_eq!(parse_host_port("localhost:8080"), Some(("localhost".into(), 8080)));
/// assert_eq!(parse_host_port("localhost"), Some(("localhost".into(), 0)));
/// assert_eq!(parse_host_port("localhost:99999"), None);
/// ```
pub fn parse_host_port(s: &str) -> Option<(String, u16)> {
    // Use rsplitn to handle IPv4 addresses with ports correctly
    if let Some(colon_pos) = s.rfind(':') {
        let host = &s[..colon_pos];
        let port_str = &s[colon_pos + 1..];
        if host.is_empty() || port_str.is_empty() {
            return None;
        }
        // Parse port
        let port = u16::from_str(port_str).ok()?;
        Some((host.to_string(), port))
    } else {
        // No port specified — return the full string as host
        if s.is_empty() {
            return None;
        }
        Some((s.to_string(), 0))
    }
}

/// Parse a `host:port` string with a default port fallback.
///
/// Like `parse_host_port` but uses `default_port` when no port is specified.
///
/// # Examples
/// ```
/// use edgerun_encoding::net::parse_host_port_with_default;
/// assert_eq!(
///     parse_host_port_with_default("localhost:8080", 443),
///     Some(("localhost".into(), 8080))
/// );
/// assert_eq!(
///     parse_host_port_with_default("localhost", 443),
///     Some(("localhost".into(), 443))
/// );
/// ```
pub fn parse_host_port_with_default(s: &str, default_port: u16) -> Option<(String, u16)> {
    if let Some(colon_pos) = s.rfind(':') {
        let host = &s[..colon_pos];
        let port_str = &s[colon_pos + 1..];
        if host.is_empty() || port_str.is_empty() {
            return None;
        }
        let port = u16::from_str(port_str).ok()?;
        Some((host.to_string(), port))
    } else {
        if s.is_empty() {
            return None;
        }
        Some((s.to_string(), default_port))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_host_port_basic() {
        let (host, port) = parse_host_port("localhost:8080").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_parse_host_port_ipv4() {
        let (host, port) = parse_host_port("192.168.1.1:443").unwrap();
        assert_eq!(host, "192.168.1.1");
        assert_eq!(port, 443);
    }

    #[test]
    fn test_parse_host_port_no_port() {
        let (host, port) = parse_host_port("localhost").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 0);
    }

    #[test]
    fn test_parse_host_port_invalid_port() {
        assert!(parse_host_port("localhost:99999").is_none());
    }

    #[test]
    fn test_parse_host_port_empty() {
        assert!(parse_host_port("").is_none());
    }

    #[test]
    fn test_parse_host_port_trailing_colon() {
        assert!(parse_host_port("localhost:").is_none());
    }

    #[test]
    fn test_parse_host_port_with_default() {
        let (host, port) = parse_host_port_with_default("localhost:8080", 443).unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_parse_host_port_with_default_no_port() {
        let (host, port) = parse_host_port_with_default("localhost", 443).unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 443);
    }

    #[test]
    fn test_parse_host_port_with_default_empty() {
        assert!(parse_host_port_with_default("", 443).is_none());
    }
}
