//! HTTP status codes

use std::fmt;

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
