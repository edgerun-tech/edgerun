//! LMTP-specific types.
//!
//! LMTP responses are identical to SMTP responses but with different
//! semantics: each recipient gets an individual response after DATA.

use std::fmt;

/// LMTP response code (same format as SMTP, RFC 5321).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LmtpResponseCode {
    pub digit1: u8, // category (2=success, 4=temp fail, 5=perm fail)
    pub digit2: u8, // subcategory
    pub digit3: u8, // specific detail
}

impl LmtpResponseCode {
    pub const fn new(d1: u8, d2: u8, d3: u8) -> Self {
        Self {
            digit1: d1,
            digit2: d2,
            digit3: d3,
        }
    }

    /// 250 OK — action completed
    pub const OK: Self = Self::new(2, 5, 0);
    /// 220 Service ready
    pub const SERVICE_READY: Self = Self::new(2, 2, 0);
    /// 452 Insufficient system storage
    pub const INSUFFICIENT_STORAGE: Self = Self::new(4, 5, 2);
    /// 550 Mailbox not found
    pub const MAILBOX_NOT_FOUND: Self = Self::new(5, 5, 0);
    /// 552 Message too large for system
    pub const MESSAGE_TOO_LARGE: Self = Self::new(5, 5, 2);

    pub fn is_success(&self) -> bool {
        self.digit1 == 2
    }

    pub fn as_u16(&self) -> u16 {
        self.digit1 as u16 * 100 + self.digit2 as u16 * 10 + self.digit3 as u16
    }
}

impl fmt::Display for LmtpResponseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.digit1, self.digit2, self.digit3)
    }
}

/// LMTP response (code + message).
#[derive(Debug, Clone)]
pub struct LmtpResponse {
    pub code: LmtpResponseCode,
    pub message: String,
}

impl LmtpResponse {
    pub fn new(code: LmtpResponseCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn ok(msg: &str) -> Self {
        Self::new(LmtpResponseCode::OK, msg)
    }

    /// Format as "NNN message\r\n"
    pub fn format(&self) -> String {
        format!("{} {}\r\n", self.code, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_code_values() {
        assert_eq!(LmtpResponseCode::OK.as_u16(), 250);
        assert_eq!(LmtpResponseCode::SERVICE_READY.as_u16(), 220);
        assert_eq!(LmtpResponseCode::INSUFFICIENT_STORAGE.as_u16(), 452);
        assert_eq!(LmtpResponseCode::MAILBOX_NOT_FOUND.as_u16(), 550);
        // MESSAGE_TOO_LARGE is 552 — matches RFC 5321
        assert_eq!(LmtpResponseCode::MESSAGE_TOO_LARGE.as_u16(), 552);
    }

    #[test]
    fn test_response_format() {
        let r = LmtpResponse::ok("Recipient accepted");
        assert_eq!(r.format(), "250 Recipient accepted\r\n");
    }

    #[test]
    fn test_is_success() {
        assert!(LmtpResponseCode::OK.is_success());
        assert!(!LmtpResponseCode::MAILBOX_NOT_FOUND.is_success());
    }
}
