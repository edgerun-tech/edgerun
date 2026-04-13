/// SMTP response code (3-digit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmtpResponseCode {
    pub digit1: u8,
    pub digit2: u8,
    pub digit3: u8,
}

impl SmtpResponseCode {
    pub const fn new(d1: u8, d2: u8, d3: u8) -> Self {
        Self { digit1: d1, digit2: d2, digit3: d3 }
    }

    pub const fn as_u16(&self) -> u16 {
        self.digit1 as u16 * 100 + self.digit2 as u16 * 10 + self.digit3 as u16
    }

    pub const fn is_success(&self) -> bool { self.digit1 == 2 }
    pub const fn is_continuation(&self) -> bool { self.digit1 == 3 }
    pub const fn is_transient_failure(&self) -> bool { self.digit1 == 4 }
    pub const fn is_permanent_failure(&self) -> bool { self.digit1 == 5 }
}

impl std::fmt::Display for SmtpResponseCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", self.digit1, self.digit2, self.digit3)
    }
}

impl SmtpResponseCode {
    pub const SERVICE_READY: Self = Self::new(2, 2, 0);
    pub const CLOSING: Self = Self::new(2, 2, 1);
    pub const OK: Self = Self::new(2, 5, 0);
    pub const HELP: Self = Self::new(2, 1, 1);
    pub const START_MAIL_INPUT: Self = Self::new(3, 5, 4);
    pub const AUTH_CONTINUE: Self = Self::new(3, 3, 4);
    pub const SERVICE_UNAVAILABLE: Self = Self::new(4, 2, 1);
    pub const INSUFFICIENT_STORAGE: Self = Self::new(4, 5, 3);
    pub const SYNTAX_ERROR: Self = Self::new(5, 5, 4);
    pub const COMMAND_NOT_RECOGNIZED: Self = Self::new(5, 5, 2);
    pub const COMMAND_NOT_IMPLEMENTED: Self = Self::new(5, 5, 1);
    pub const BAD_SEQUENCE: Self = Self::new(5, 0, 3);
    pub const MAILBOX_NOT_FOUND: Self = Self::new(5, 1, 1);
    pub const RECIPIENT_REJECTED: Self = Self::new(5, 1, 1);
    pub const AUTHENTICATION_FAILED: Self = Self::new(5, 7, 8);
    pub const TOO_MANY_RECIPIENTS: Self = Self::new(4, 5, 3);
    pub const LINE_TOO_LONG: Self = Self::new(5, 5, 4);
}

// ===========================================================================
// Enhanced Status Code (RFC 3463)
// ===========================================================================

/// Enhanced status code per RFC 3463 (`X.Y.Z` format).
/// `X` = class (2/4/5), `Y` = subject (0–7), `Z` = detail (0–9).
#[derive(Debug, Clone)]
pub struct EnhancedStatusCode {
    pub class: u8,
    pub subject: u8,
    pub detail: u8,
}

impl EnhancedStatusCode {
    pub const fn new(class: u8, subject: u8, detail: u8) -> Self {
        Self { class, subject, detail }
    }
}

impl std::fmt::Display for EnhancedStatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.class, self.subject, self.detail)
    }
}

impl EnhancedStatusCode {
    pub const OK: Self = Self::new(2, 5, 0);
    pub const MAIL_FROM_OK: Self = Self::new(2, 1, 0);
    pub const RCPT_TO_OK: Self = Self::new(2, 1, 5);
    pub const QUEUED: Self = Self::new(2, 0, 0);
    pub const HELP_TEXT: Self = Self::new(2, 0, 0);

    pub const ADDRESSING_MAILBOX: Self = Self::new(5, 1, 1);
    pub const MESSAGE_TOO_LARGE: Self = Self::new(5, 3, 4);
    pub const FEATURE_NOT_IMPLEMENTED: Self = Self::new(5, 5, 1);
    pub const SYNTAX_ERROR: Self = Self::new(5, 5, 4);
    pub const BAD_SEQUENCE: Self = Self::new(5, 0, 3);
    pub const AUTH_REQUIRED: Self = Self::new(5, 7, 1);
    pub const AUTH_MECHANISM_UNKNOWN: Self = Self::new(5, 7, 4);

    pub const TRANSIENT_MESSAGE_TOO_LARGE: Self = Self::new(4, 3, 4);
}

// ===========================================================================
// SmtpResponse
// ===========================================================================

/// SMTP response with code, optional enhanced status, and message.
#[derive(Debug, Clone)]
pub struct SmtpResponse {
    pub code: SmtpResponseCode,
    pub enhanced_status: Option<EnhancedStatusCode>,
    pub message: String,
    pub is_multiline: bool,
}

impl SmtpResponse {
    pub fn new(code: SmtpResponseCode, message: impl Into<String>) -> Self {
        Self {
            code,
            enhanced_status: None,
            message: message.into(),
            is_multiline: false,
        }
    }

    pub fn with_enhanced(mut self, esc: EnhancedStatusCode) -> Self {
        self.enhanced_status = Some(esc);
        self
    }

    pub fn multiline(code: SmtpResponseCode, lines: Vec<String>) -> Self {
        Self {
            code,
            enhanced_status: None,
            message: lines.join("\n"),
            is_multiline: true,
        }
    }

    /// Format per RFC 3463 enhanced status codes.
    /// With enhanced: `250 2.1.0 OK`  — without: `250 OK`
    pub fn format(&self) -> String {
        let msg = if let Some(ref esc) = self.enhanced_status {
            format!("{} {}", esc, self.message)
        } else {
            self.message.clone()
        };

        if self.is_multiline {
            let lines: Vec<&str> = msg.split('\n').collect();
            let mut result = String::new();
            for (i, line) in lines.iter().enumerate() {
                if i == lines.len() - 1 {
                    result.push_str(&format!("{} {}\r\n", self.code, line));
                } else {
                    result.push_str(&format!("{}-{}\r\n", self.code, line));
                }
            }
            result
        } else {
            format!("{} {}\r\n", self.code, msg)
        }
    }
}

// ── Convenience constructors ────────────────────────────────────────────────

impl SmtpResponse {
    pub fn service_ready(domain: &str) -> Self {
        Self::new(SmtpResponseCode::SERVICE_READY, format!("{} ESMTP ready", domain))
    }

    pub fn ok(message: &str) -> Self {
        Self::new(SmtpResponseCode::OK, message).with_enhanced(EnhancedStatusCode::OK)
    }

    pub fn closing() -> Self {
        Self::new(SmtpResponseCode::CLOSING, "Bye").with_enhanced(EnhancedStatusCode::OK)
    }

    pub fn start_mail_input() -> Self {
        Self::new(SmtpResponseCode::START_MAIL_INPUT, "Start mail input; end with <CRLF>.<CRLF>")
    }

    pub fn syntax_error(message: &str) -> Self {
        Self::new(SmtpResponseCode::SYNTAX_ERROR, message)
            .with_enhanced(EnhancedStatusCode::SYNTAX_ERROR)
    }

    pub fn command_not_implemented(cmd: &str) -> Self {
        Self::new(SmtpResponseCode::COMMAND_NOT_IMPLEMENTED, format!("{} command not implemented", cmd))
            .with_enhanced(EnhancedStatusCode::FEATURE_NOT_IMPLEMENTED)
    }

    pub fn bad_sequence(message: &str) -> Self {
        Self::new(SmtpResponseCode::BAD_SEQUENCE, message)
            .with_enhanced(EnhancedStatusCode::BAD_SEQUENCE)
    }

    pub fn mailbox_not_found(address: &str) -> Self {
        Self::new(SmtpResponseCode::MAILBOX_NOT_FOUND, format!("<{}>: Recipient address rejected", address))
            .with_enhanced(EnhancedStatusCode::ADDRESSING_MAILBOX)
    }

    pub fn transient_failure(message: &str) -> Self {
        Self::new(SmtpResponseCode::SERVICE_UNAVAILABLE, message)
    }

    pub fn too_many_recipients(count: usize) -> Self {
        Self::new(SmtpResponseCode::TOO_MANY_RECIPIENTS,
            format!("Too many recipients (max 100, got {})", count))
            .with_enhanced(EnhancedStatusCode::MESSAGE_TOO_LARGE)
    }

    pub fn line_too_long(len: usize, max: usize) -> Self {
        Self::new(SmtpResponseCode::LINE_TOO_LONG,
            format!("Line too long ({} > {} chars)", len, max))
            .with_enhanced(EnhancedStatusCode::SYNTAX_ERROR)
    }

    pub fn auth_required() -> Self {
        Self::new(SmtpResponseCode::AUTHENTICATION_FAILED, "Authentication required")
            .with_enhanced(EnhancedStatusCode::AUTH_REQUIRED)
    }

    pub fn auth_mechanism_unknown(mechanism: &str) -> Self {
        Self::new(SmtpResponseCode::AUTHENTICATION_FAILED, format!("AUTH {} not supported", mechanism))
            .with_enhanced(EnhancedStatusCode::AUTH_MECHANISM_UNKNOWN)
    }

    pub fn auth_success(identity: &str) -> Self {
        Self::new(SmtpResponseCode::OK, format!("Authentication successful ({})", identity))
            .with_enhanced(EnhancedStatusCode::AUTH_SUCCESS)
    }

    pub fn auth_continue(challenge: &str) -> Self {
        Self::new(SmtpResponseCode::AUTH_CONTINUE, challenge)
    }

    pub fn message_too_large() -> Self {
        Self::new(SmtpResponseCode::INSUFFICIENT_STORAGE, "Message too large")
            .with_enhanced(EnhancedStatusCode::TRANSIENT_MESSAGE_TOO_LARGE)
    }

    pub fn vrfy_disabled() -> Self {
        Self::new(SmtpResponseCode::COMMAND_NOT_IMPLEMENTED, "VRFY command disabled for security reasons")
            .with_enhanced(EnhancedStatusCode::FEATURE_NOT_IMPLEMENTED)
    }

    pub fn expn_disabled() -> Self {
        Self::new(SmtpResponseCode::COMMAND_NOT_IMPLEMENTED, "EXPN command disabled for security reasons")
            .with_enhanced(EnhancedStatusCode::FEATURE_NOT_IMPLEMENTED)
    }

    pub fn help_text(domain: &str) -> Self {
        let text = format!(
            "Supported commands: EHLO HELO MAIL RCPT DATA RSET NOOP QUIT VRFY EXPN HELP STARTTLS AUTH\n\
             {}\n\
             For more info see https://tools.ietf.org/html/rfc5321", domain);
        Self::new(SmtpResponseCode::HELP, text)
            .with_enhanced(EnhancedStatusCode::HELP_TEXT)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_status_format() {
        let resp = SmtpResponse::ok("OK");
        assert!(resp.enhanced_status.is_some());
        assert_eq!(resp.format(), "250 2.5.0 OK\r\n");
    }

    #[test]
    fn test_syntax_error_format() {
        let resp = SmtpResponse::syntax_error("bad command");
        assert_eq!(resp.format(), "554 5.5.4 bad command\r\n");
    }

    #[test]
    fn test_multiline_response() {
        let resp = SmtpResponse::multiline(SmtpResponseCode::OK, vec![
            "Hello mail.example.com".to_string(),
            "PIPELINING".to_string(),
        ]);
        assert_eq!(resp.format(), "250-Hello mail.example.com\r\n250 PIPELINING\r\n");
    }

    #[test]
    fn test_response_code_categories() {
        assert!(SmtpResponseCode::OK.is_success());
        assert!(SmtpResponseCode::START_MAIL_INPUT.is_continuation());
        assert!(SmtpResponseCode::SERVICE_UNAVAILABLE.is_transient_failure());
        assert!(SmtpResponseCode::SYNTAX_ERROR.is_permanent_failure());
    }

    #[test]
    fn test_response_code_display() {
        assert_eq!(format!("{}", SmtpResponseCode::OK), "250");
        assert_eq!(format!("{}", SmtpResponseCode::SERVICE_READY), "220");
    }

    #[test]
    fn test_specialized_responses() {
        assert!(SmtpResponse::too_many_recipients(101).format().contains("101"));
        assert!(SmtpResponse::line_too_long(1200, 998).format().contains("1200"));
        assert!(SmtpResponse::vrfy_disabled().format().contains("VRFY"));
        assert!(SmtpResponse::expn_disabled().format().contains("EXPN"));
        assert!(SmtpResponse::help_text("test.host").format().contains("EHLO"));
    }
}
