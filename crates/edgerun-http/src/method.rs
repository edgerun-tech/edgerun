//! HTTP methods (RFC 9110 Section 9)

use std::fmt;
use std::str::FromStr;

/// HTTP method
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Method {
    /// GET — Retrieve a resource
    GET,
    /// HEAD — Like GET but no response body
    HEAD,
    /// POST — Submit data to a resource
    POST,
    /// PUT — Replace a resource
    PUT,
    /// DELETE — Remove a resource
    DELETE,
    /// CONNECT — Tunnel to a server
    CONNECT,
    /// OPTIONS — Describe communication options
    OPTIONS,
    /// TRACE — Echo the received request
    TRACE,
    /// PATCH — Partially modify a resource
    PATCH,
    /// Extension method (WebDAV, CalDAV, custom, etc.)
    /// RFC 9110 Section 9.1 allows any valid token as a method.
    Extension(String),
}

impl Method {
    /// Check if this method has a request body
    pub fn has_body(&self) -> bool {
        match self {
            Method::POST | Method::PUT | Method::PATCH | Method::CONNECT => true,
            // Extension methods may have bodies; we conservatively allow it
            Method::Extension(_) => true,
            Method::GET | Method::HEAD | Method::DELETE | Method::OPTIONS | Method::TRACE => false,
        }
    }

    /// Check if this method expects a response body
    pub fn expects_response_body(&self) -> bool {
        !matches!(self, Method::HEAD)
    }

    /// Convert method to string
    pub fn as_str(&self) -> &str {
        match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
            Method::PATCH => "PATCH",
            Method::HEAD => "HEAD",
            Method::OPTIONS => "OPTIONS",
            Method::CONNECT => "CONNECT",
            Method::TRACE => "TRACE",
            Method::Extension(s) => s,
        }
    }

    /// Check if this is a standard (RFC-registered) method
    pub fn is_standard(&self) -> bool {
        !matches!(self, Method::Extension(_))
    }

    /// Check if this is an extension method (WebDAV, CalDAV, custom, etc.)
    pub fn is_extension(&self) -> bool {
        matches!(self, Method::Extension(_))
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Check if a byte is a valid HTTP token character (RFC 9110 Section 5.6.2).
fn is_tchar(b: u8) -> bool {
    matches!(b,
        b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+'
        | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
        | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z'
    )
}

impl FromStr for Method {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err("HTTP method cannot be empty".to_string());
        }
        if !s.bytes().all(is_tchar) {
            return Err(format!("Invalid HTTP method: {s}"));
        }

        // Case-insensitive match for standard methods
        match s.to_uppercase().as_str() {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "PATCH" => Ok(Method::PATCH),
            "HEAD" => Ok(Method::HEAD),
            "OPTIONS" => Ok(Method::OPTIONS),
            "CONNECT" => Ok(Method::CONNECT),
            "TRACE" => Ok(Method::TRACE),
            // RFC 9110 Section 9.1: extension methods are allowed
            other => Ok(Method::Extension(other.to_string())),
        }
    }
}
