//! HTTP methods

use std::fmt;
use std::str::FromStr;

/// HTTP method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// GET
    GET,
    /// POST
    POST,
    /// PUT
    PUT,
    /// DELETE
    DELETE,
    /// PATCH
    PATCH,
    /// HEAD
    HEAD,
    /// OPTIONS
    OPTIONS,
    /// CONNECT
    CONNECT,
    /// TRACE
    TRACE,
}

impl Method {
    /// Check if this method has a request body
    pub fn has_body(&self) -> bool {
        matches!(
            self,
            Method::POST | Method::PUT | Method::PATCH | Method::CONNECT
        )
    }

    /// Check if this method expects a response body
    pub fn expects_response_body(&self) -> bool {
        !matches!(self, Method::HEAD)
    }

    /// Convert method to string
    pub fn as_str(&self) -> &'static str {
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
        }
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
            _ => Err(format!("Invalid HTTP method: {}", s)),
        }
    }
}
