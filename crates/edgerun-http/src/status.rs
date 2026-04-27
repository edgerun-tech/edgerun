//! HTTP status codes

use alloc::borrow::Cow;
use alloc::format;
use alloc::string::{String, ToString};
use core::fmt;

/// HTTP status code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusCode(u16);

impl StatusCode {
    /// Create a new status code
    pub fn new(code: u16) -> Result<Self, String> {
        if (100..600).contains(&code) {
            Ok(StatusCode(code))
        } else {
            Err(format!("Invalid status code: {}", code))
        }
    }

    /// Get the status code as a u16
    pub fn as_u16(&self) -> u16 {
        self.0
    }

    /// Get the status code as a string (e.g., "200").
    ///
    /// Returns a `String` for codes that don't have a static representation.
    pub fn as_str(&self) -> Cow<'static, str> {
        match self.0 {
            100 => "100".into(),
            101 => "101".into(),
            102 => "102".into(),
            103 => "103".into(),
            200 => "200".into(),
            201 => "201".into(),
            202 => "202".into(),
            203 => "203".into(),
            204 => "204".into(),
            205 => "205".into(),
            206 => "206".into(),
            207 => "207".into(),
            208 => "208".into(),
            226 => "226".into(),
            300 => "300".into(),
            301 => "301".into(),
            302 => "302".into(),
            303 => "303".into(),
            304 => "304".into(),
            305 => "305".into(),
            306 => "306".into(),
            307 => "307".into(),
            308 => "308".into(),
            400 => "400".into(),
            401 => "401".into(),
            402 => "402".into(),
            403 => "403".into(),
            404 => "404".into(),
            405 => "405".into(),
            406 => "406".into(),
            407 => "407".into(),
            408 => "408".into(),
            409 => "409".into(),
            410 => "410".into(),
            411 => "411".into(),
            412 => "412".into(),
            413 => "413".into(),
            414 => "414".into(),
            415 => "415".into(),
            416 => "416".into(),
            417 => "417".into(),
            418 => "418".into(),
            421 => "421".into(),
            422 => "422".into(),
            423 => "423".into(),
            424 => "424".into(),
            425 => "425".into(),
            426 => "426".into(),
            428 => "428".into(),
            429 => "429".into(),
            431 => "431".into(),
            451 => "451".into(),
            500 => "500".into(),
            501 => "501".into(),
            502 => "502".into(),
            503 => "503".into(),
            504 => "504".into(),
            505 => "505".into(),
            506 => "506".into(),
            507 => "507".into(),
            508 => "508".into(),
            510 => "510".into(),
            511 => "511".into(),
            other => Cow::Owned(other.to_string()),
        }
    }

    /// Get the reason phrase for this status code
    pub fn reason(&self) -> &'static str {
        match self.0 {
            100 => "Continue",
            101 => "Switching Protocols",
            102 => "Processing",
            103 => "Early Hints",
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            203 => "Non-Authoritative Information",
            204 => "No Content",
            205 => "Reset Content",
            206 => "Partial Content",
            207 => "Multi-Status",
            208 => "Already Reported",
            226 => "IM Used",
            300 => "Multiple Choices",
            301 => "Moved Permanently",
            302 => "Found",
            303 => "See Other",
            304 => "Not Modified",
            305 => "Use Proxy",
            307 => "Temporary Redirect",
            308 => "Permanent Redirect",
            400 => "Bad Request",
            401 => "Unauthorized",
            402 => "Payment Required",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            406 => "Not Acceptable",
            407 => "Proxy Authentication Required",
            408 => "Request Timeout",
            409 => "Conflict",
            410 => "Gone",
            411 => "Length Required",
            412 => "Precondition Failed",
            413 => "Payload Too Large",
            414 => "URI Too Long",
            415 => "Unsupported Media Type",
            416 => "Range Not Satisfiable",
            417 => "Expectation Failed",
            418 => "I'm a teapot",
            421 => "Misdirected Request",
            422 => "Unprocessable Entity",
            423 => "Locked",
            424 => "Failed Dependency",
            425 => "Too Early",
            426 => "Upgrade Required",
            428 => "Precondition Required",
            429 => "Too Many Requests",
            431 => "Request Header Fields Too Large",
            451 => "Unavailable For Legal Reasons",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            505 => "HTTP Version Not Supported",
            506 => "Variant Also Negotiates",
            507 => "Insufficient Storage",
            508 => "Loop Detected",
            510 => "Not Extended",
            511 => "Network Authentication Required",
            _ => "Unknown Status",
        }
    }

    /// Check if this is a successful status code (2xx)
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.0)
    }

    /// Check if this is a redirection status code (3xx)
    pub fn is_redirection(&self) -> bool {
        (300..400).contains(&self.0)
    }

    /// Check if this is a client error status code (4xx)
    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.0)
    }

    /// Check if this is a server error status code (5xx)
    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.0)
    }

    /// Check if this is an informational status code (1xx)
    pub fn is_informational(&self) -> bool {
        (100..200).contains(&self.0)
    }

    // Common status code constants

    /// 200 OK
    pub const OK: StatusCode = StatusCode(200);
    /// 201 Created
    pub const CREATED: StatusCode = StatusCode(201);
    /// 204 No Content
    pub const NO_CONTENT: StatusCode = StatusCode(204);
    /// 301 Moved Permanently
    pub const MOVED_PERMANENTLY: StatusCode = StatusCode(301);
    /// 302 Found
    pub const FOUND: StatusCode = StatusCode(302);
    /// 304 Not Modified
    pub const NOT_MODIFIED: StatusCode = StatusCode(304);
    /// 400 Bad Request
    pub const BAD_REQUEST: StatusCode = StatusCode(400);
    /// 401 Unauthorized
    pub const UNAUTHORIZED: StatusCode = StatusCode(401);
    /// 403 Forbidden
    pub const FORBIDDEN: StatusCode = StatusCode(403);
    /// 404 Not Found
    pub const NOT_FOUND: StatusCode = StatusCode(404);
    /// 500 Internal Server Error
    pub const INTERNAL_SERVER_ERROR: StatusCode = StatusCode(500);
    /// 502 Bad Gateway
    pub const BAD_GATEWAY: StatusCode = StatusCode(502);
    /// 503 Service Unavailable
    pub const SERVICE_UNAVAILABLE: StatusCode = StatusCode(503);
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0, self.reason())
    }
}
