//! HTTP methods (RFC 9110 Section 9)

#[cfg(target_os = "none")]
use crate::prelude::v1::*;

use crate::is_tchar;
use core::fmt;
use core::str::FromStr;

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

impl FromStr for Method {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err("HTTP method cannot be empty".to_string());
        }
        if !s.bytes().all(is_tchar) {
            return Err(format!("Invalid HTTP method: {s}"));
        }

        // Case-insensitive match for standard methods (no allocation)
        if s.eq_ignore_ascii_case("GET") {
            Ok(Method::GET)
        } else if s.eq_ignore_ascii_case("POST") {
            Ok(Method::POST)
        } else if s.eq_ignore_ascii_case("PUT") {
            Ok(Method::PUT)
        } else if s.eq_ignore_ascii_case("DELETE") {
            Ok(Method::DELETE)
        } else if s.eq_ignore_ascii_case("PATCH") {
            Ok(Method::PATCH)
        } else if s.eq_ignore_ascii_case("HEAD") {
            Ok(Method::HEAD)
        } else if s.eq_ignore_ascii_case("OPTIONS") {
            Ok(Method::OPTIONS)
        } else if s.eq_ignore_ascii_case("CONNECT") {
            Ok(Method::CONNECT)
        } else if s.eq_ignore_ascii_case("TRACE") {
            Ok(Method::TRACE)
        } else {
            Ok(Method::Extension(s.to_string()))
        }
    }
}
