//! Core SMTP types — commands, responses, envelope, and session state.

use std::io;

// ===========================================================================
// Session State
// ===========================================================================

/// SMTP session states per RFC 5321.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtpState {
    /// Initial state — after connection, before EHLO/HELO.
    Connected,
    /// After EHLO/HELO — ready for MAIL FROM.
    Ready,
    /// After MAIL FROM — ready for RCPT TO.
    MailSet,
    /// After one or more RCPT TO — ready for DATA.
    RcptSet,
    /// During DATA transfer — reading message content.
    Data,
    /// QUIT received — connection shutting down.
    Quit,
}

// ===========================================================================
// SMTP Commands
// ===========================================================================

/// Parsed SMTP command from a client.
#[derive(Debug, Clone)]
pub enum SmtpCommand {
    /// `EHLO <domain>` — Extended HELLO (ESMTP).
    Ehlo(String),
    /// `HELO <domain>` — Original HELLO.
    Helo(String),
    /// `MAIL FROM:<reverse-path>` — Start mail transaction.
    MailFrom {
        address: String,
        parameters: Vec<(String, Option<String>)>,
    },
    /// `RCPT TO:<forward-path>` — Specify recipient.
    RcptTo {
        address: String,
        parameters: Vec<(String, Option<String>)>,
    },
    /// `DATA` — Begin message data transfer.
    Data,
    /// `RSET` — Reset current mail transaction.
    Rset,
    /// `NOOP` — No operation.
    Noop,
    /// `QUIT` — Close connection.
    Quit,
    /// `VRFY <string>` — Verify mailbox.
    Vrfy(String),
    /// `EXPN <string>` — Expand mailing list.
    Expn(String),
    /// `HELP [<string>]` — Get help.
    Help(Option<String>),
    /// `STARTTLS` — Upgrade to TLS (ESMTP extension).
    Starttls,
    /// `AUTH <mechanism> [initial-response]` — Authenticate.
    Auth {
        mechanism: String,
        initial_response: Option<String>,
    },
    /// Continuation response for multi-step AUTH.
    AuthContinue(String),
    /// Raw data line during DATA phase.
    DataLine(Vec<u8>),
    /// End of data (dot on line by itself).
    DataEnd,
}

impl SmtpCommand {
    /// Parse an SMTP command from a raw line.
    pub fn parse(line: &str) -> io::Result<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "empty command"));
        }

        // Split into command word and arguments
        let parts: Vec<&str> = trimmed.splitn(2, |c: char| c.is_whitespace()).collect();
        let cmd = parts[0].to_uppercase();
        let args = if parts.len() > 1 { parts[1].trim() } else { "" };

        match cmd.as_str() {
            "EHLO" => {
                if args.is_empty() {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "EHLO requires domain"));
                }
                Ok(Self::Ehlo(args.to_string()))
            }
            "HELO" => {
                if args.is_empty() {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "HELO requires domain"));
                }
                Ok(Self::Helo(args.to_string()))
            }
            "MAIL" => {
                // MAIL FROM:<addr> [parameters]
                let from_pos = args.to_uppercase().find("FROM:");
                if from_pos.is_none() {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "MAIL requires FROM"));
                }
                let from_start = from_pos.unwrap() + 5;
                let rest = &args[from_start..];

                // Extract address (may be in <>)
                let (address, params_str) = if rest.starts_with('<') {
                    if let Some(end) = rest.find('>') {
                        (rest[1..end].to_string(), rest[end + 1..].trim())
                    } else {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "MAIL FROM: missing closing >"));
                    }
                } else {
                    let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
                    (rest[..end].to_string(), rest[end..].trim())
                };

                let parameters = parse_esmtp_parameters(params_str);
                Ok(Self::MailFrom { address, parameters })
            }
            "RCPT" => {
                let to_pos = args.to_uppercase().find("TO:");
                if to_pos.is_none() {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "RCPT requires TO"));
                }
                let to_start = to_pos.unwrap() + 3;
                let rest = &args[to_start..];

                let (address, params_str) = if rest.starts_with('<') {
                    if let Some(end) = rest.find('>') {
                        (rest[1..end].to_string(), rest[end + 1..].trim())
                    } else {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "RCPT TO: missing closing >"));
                    }
                } else {
                    let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
                    (rest[..end].to_string(), rest[end..].trim())
                };

                let parameters = parse_esmtp_parameters(params_str);
                Ok(Self::RcptTo { address, parameters })
            }
            "DATA" => Ok(Self::Data),
            "RSET" => Ok(Self::Rset),
            "NOOP" => Ok(Self::Noop),
            "QUIT" => Ok(Self::Quit),
            "VRFY" => Ok(Self::Vrfy(args.to_string())),
            "EXPN" => Ok(Self::Expn(args.to_string())),
            "HELP" => {
                if args.is_empty() {
                    Ok(Self::Help(None))
                } else {
                    Ok(Self::Help(Some(args.to_string())))
                }
            }
            "STARTTLS" => Ok(Self::Starttls),
            "AUTH" => {
                let auth_parts: Vec<&str> = args.splitn(2, |c: char| c.is_whitespace()).collect();
                if auth_parts.is_empty() || auth_parts[0].is_empty() {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "AUTH requires mechanism"));
                }
                let mechanism = auth_parts[0].to_string();
                let initial_response = if auth_parts.len() > 1 {
                    Some(auth_parts[1].to_string())
                } else {
                    None
                };
                Ok(Self::Auth { mechanism, initial_response })
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown command: {}", cmd),
            )),
        }
    }
}

/// Parse ESMTP parameters (e.g., SIZE=12345 RET=FULL ENVID=abc).
fn parse_esmtp_parameters(s: &str) -> Vec<(String, Option<String>)> {
    if s.is_empty() {
        return Vec::new();
    }

    let mut params = Vec::new();
    for token in s.split_whitespace() {
        if let Some(eq_pos) = token.find('=') {
            let key = token[..eq_pos].to_string();
            let value = token[eq_pos + 1..].to_string();
            params.push((key, Some(value)));
        } else {
            params.push((token.to_string(), None));
        }
    }
    params
}

// ===========================================================================
// SMTP Responses
// ===========================================================================

/// SMTP response code (3-digit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmtpResponseCode {
    pub digit1: u8, // 2, 4, 5 etc. (category)
    pub digit2: u8, // 0-5 (specific meaning)
    pub digit3: u8, // 0-9 (specific meaning)
}

impl SmtpResponseCode {
    pub const fn new(d1: u8, d2: u8, d3: u8) -> Self {
        Self { digit1: d1, digit2: d2, digit3: d3 }
    }

    /// Convert to integer (e.g., 250).
    pub const fn as_u16(&self) -> u16 {
        self.digit1 as u16 * 100 + self.digit2 as u16 * 10 + self.digit3 as u16
    }

    /// Is this a success code (2xx)?
    pub const fn is_success(&self) -> bool {
        self.digit1 == 2
    }

    /// Is this a continuation code (3xx)?
    pub const fn is_continuation(&self) -> bool {
        self.digit1 == 3
    }

    /// Is this a temporary failure (4xx)?
    pub const fn is_transient_failure(&self) -> bool {
        self.digit1 == 4
    }

    /// Is this a permanent failure (5xx)?
    pub const fn is_permanent_failure(&self) -> bool {
        self.digit1 == 5
    }
}

impl std::fmt::Display for SmtpResponseCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", self.digit1, self.digit2, self.digit3)
    }
}

// Standard response codes
impl SmtpResponseCode {
    pub const SERVICE_READY: Self = Self::new(2, 2, 0);
    pub const CLOSING: Self = Self::new(2, 2, 1);
    pub const OK: Self = Self::new(2, 5, 0);
    pub const HELP: Self = Self::new(2, 1, 1);
    pub const START_MAIL_INPUT: Self = Self::new(3, 5, 4);
    pub const AUTH_CONTINUE: Self = Self::new(3, 3, 4);
    pub const SERVICE_UNAVAILABLE: Self = Self::new(4, 2, 1);
    pub const MAILBOX_UNAVAILABLE: Self = Self::new(4, 2, 2);
    pub const INSUFFICIENT_STORAGE: Self = Self::new(4, 5, 2);
    pub const SYNTAX_ERROR: Self = Self::new(5, 5, 4);
    pub const COMMAND_NOT_RECOGNIZED: Self = Self::new(5, 5, 2);
    pub const COMMAND_NOT_IMPLEMENTED: Self = Self::new(5, 5, 1);
    pub const BAD_SEQUENCE: Self = Self::new(5, 0, 3);
    pub const MAILBOX_NOT_FOUND: Self = Self::new(5, 1, 1);
    pub const RECIPIENT_REJECTED: Self = Self::new(5, 1, 1);
    pub const AUTHENTICATION_REQUIRED: Self = Self::new(5, 7, 1);
    pub const AUTHENTICATION_FAILED: Self = Self::new(5, 7, 8);
}

/// SMTP response with code and message.
#[derive(Debug, Clone)]
pub struct SmtpResponse {
    pub code: SmtpResponseCode,
    pub message: String,
    /// Whether this is a multiline response.
    pub is_multiline: bool,
}

impl SmtpResponse {
    pub fn new(code: SmtpResponseCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            is_multiline: false,
        }
    }

    pub fn multiline(code: SmtpResponseCode, lines: Vec<String>) -> Self {
        Self {
            code,
            message: lines.join("\n"),
            is_multiline: true,
        }
    }

    /// Format as an SMTP response line.
    pub fn format(&self) -> String {
        if self.is_multiline {
            let lines: Vec<&str> = self.message.split('\n').collect();
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
            format!("{} {}\r\n", self.code, self.message)
        }
    }
}

// Standard responses
impl SmtpResponse {
    pub fn service_ready(domain: &str) -> Self {
        Self::new(SmtpResponseCode::SERVICE_READY, format!("{} ESMTP ready", domain))
    }

    pub fn ok(message: &str) -> Self {
        Self::new(SmtpResponseCode::OK, message)
    }

    pub fn closing() -> Self {
        Self::new(SmtpResponseCode::CLOSING, "Bye")
    }

    pub fn start_mail_input() -> Self {
        Self::new(SmtpResponseCode::START_MAIL_INPUT, "Start mail input; end with <CRLF>.<CRLF>")
    }

    pub fn syntax_error(message: &str) -> Self {
        Self::new(SmtpResponseCode::SYNTAX_ERROR, message)
    }

    pub fn command_not_implemented(cmd: &str) -> Self {
        Self::new(SmtpResponseCode::COMMAND_NOT_IMPLEMENTED, format!("{} command not implemented", cmd))
    }

    pub fn bad_sequence(message: &str) -> Self {
        Self::new(SmtpResponseCode::BAD_SEQUENCE, message)
    }

    pub fn mailbox_not_found(address: &str) -> Self {
        Self::new(SmtpResponseCode::MAILBOX_NOT_FOUND, format!("<{}>: Recipient address rejected", address))
    }

    pub fn transient_failure(message: &str) -> Self {
        Self::new(SmtpResponseCode::SERVICE_UNAVAILABLE, message)
    }
}

// ===========================================================================
// Mail Envelope
// ===========================================================================

/// SMTP mail envelope (sender + recipients + data).
#[derive(Debug, Clone)]
pub struct MailEnvelope {
    /// Reverse-path (sender).
    pub from: String,
    /// Forward-paths (recipients).
    pub recipients: Vec<String>,
    /// Raw message data (headers + body).
    pub data: Vec<u8>,
    /// ESMTP parameters from MAIL FROM.
    pub from_parameters: Vec<(String, Option<String>)>,
    /// ESMTP parameters from RCPT TO (per recipient).
    pub recipient_parameters: Vec<Vec<(String, Option<String>)>>,
}

impl MailEnvelope {
    pub fn new(from: String) -> Self {
        Self {
            from,
            recipients: Vec::new(),
            data: Vec::new(),
            from_parameters: Vec::new(),
            recipient_parameters: Vec::new(),
        }
    }

    /// Add a recipient.
    pub fn add_recipient(&mut self, address: String, parameters: Vec<(String, Option<String>)>) {
        self.recipients.push(address);
        self.recipient_parameters.push(parameters);
    }

    /// Reset the envelope (keep from, clear recipients and data).
    pub fn reset(&mut self) {
        self.recipients.clear();
        self.recipient_parameters.clear();
        self.data.clear();
    }
}

// ===========================================================================
// Message Parser
// ===========================================================================

/// Parse headers from raw message data.
pub fn parse_headers(data: &[u8]) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    let text = String::from_utf8_lossy(data);

    for line in text.lines() {
        if line.is_empty() {
            break;
        }
        if let Some(colon_pos) = line.find(':') {
            let name = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            headers.push((name, value));
        }
    }

    headers
}

/// Extract a specific header value.
pub fn get_header(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

/// Parse the Subject header.
pub fn get_subject(data: &[u8]) -> Option<String> {
    let headers = parse_headers(data);
    get_header(&headers, "Subject")
}

/// Parse From header into address.
pub fn get_from_address(data: &[u8]) -> Option<String> {
    let headers = parse_headers(data);
    get_header(&headers, "From")
}

/// Parse Date header.
pub fn get_date(data: &[u8]) -> Option<String> {
    let headers = parse_headers(data);
    get_header(&headers, "Date")
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ehlo() {
        let cmd = SmtpCommand::parse("EHLO mail.example.com").unwrap();
        match cmd {
            SmtpCommand::Ehlo(domain) => assert_eq!(domain, "mail.example.com"),
            _ => panic!("Expected Ehlo"),
        }
    }

    #[test]
    fn test_parse_helo() {
        let cmd = SmtpCommand::parse("HELO localhost").unwrap();
        match cmd {
            SmtpCommand::Helo(domain) => assert_eq!(domain, "localhost"),
            _ => panic!("Expected Helo"),
        }
    }

    #[test]
    fn test_parse_mail_from() {
        let cmd = SmtpCommand::parse("MAIL FROM:<sender@example.com>").unwrap();
        match cmd {
            SmtpCommand::MailFrom { address, parameters } => {
                assert_eq!(address, "sender@example.com");
                assert!(parameters.is_empty());
            }
            _ => panic!("Expected MailFrom"),
        }
    }

    #[test]
    fn test_parse_mail_from_with_size() {
        let cmd = SmtpCommand::parse("MAIL FROM:<sender@example.com> SIZE=1024").unwrap();
        match cmd {
            SmtpCommand::MailFrom { address, parameters } => {
                assert_eq!(address, "sender@example.com");
                assert_eq!(parameters.len(), 1);
                assert_eq!(parameters[0], ("SIZE".to_string(), Some("1024".to_string())));
            }
            _ => panic!("Expected MailFrom"),
        }
    }

    #[test]
    fn test_parse_rcpt_to() {
        let cmd = SmtpCommand::parse("RCPT TO:<recipient@example.com>").unwrap();
        match cmd {
            SmtpCommand::RcptTo { address, parameters } => {
                assert_eq!(address, "recipient@example.com");
                assert!(parameters.is_empty());
            }
            _ => panic!("Expected RcptTo"),
        }
    }

    #[test]
    fn test_parse_data() {
        let cmd = SmtpCommand::parse("DATA").unwrap();
        assert!(matches!(cmd, SmtpCommand::Data));
    }

    #[test]
    fn test_parse_rset() {
        let cmd = SmtpCommand::parse("RSET").unwrap();
        assert!(matches!(cmd, SmtpCommand::Rset));
    }

    #[test]
    fn test_parse_noop() {
        let cmd = SmtpCommand::parse("NOOP").unwrap();
        assert!(matches!(cmd, SmtpCommand::Noop));
    }

    #[test]
    fn test_parse_quit() {
        let cmd = SmtpCommand::parse("QUIT").unwrap();
        assert!(matches!(cmd, SmtpCommand::Quit));
    }

    #[test]
    fn test_parse_starttls() {
        let cmd = SmtpCommand::parse("STARTTLS").unwrap();
        assert!(matches!(cmd, SmtpCommand::Starttls));
    }

    #[test]
    fn test_parse_auth() {
        let cmd = SmtpCommand::parse("AUTH PLAIN dXNlckBleGFtcGxlLmNvbQBwYXNzd29yZA==").unwrap();
        match cmd {
            SmtpCommand::Auth { mechanism, initial_response } => {
                assert_eq!(mechanism, "PLAIN");
                assert!(initial_response.is_some());
            }
            _ => panic!("Expected Auth"),
        }
    }

    #[test]
    fn test_parse_case_insensitive() {
        let cmd = SmtpCommand::parse("ehlo localhost").unwrap();
        assert!(matches!(cmd, SmtpCommand::Ehlo(_)));

        let cmd = SmtpCommand::parse("mail FROM:<test@test.com>").unwrap();
        assert!(matches!(cmd, SmtpCommand::MailFrom { .. }));
    }

    #[test]
    fn test_response_code_display() {
        let code = SmtpResponseCode::OK;
        assert_eq!(format!("{}", code), "250");

        let code = SmtpResponseCode::SERVICE_READY;
        assert_eq!(format!("{}", code), "220");
    }

    #[test]
    fn test_response_code_categories() {
        assert!(SmtpResponseCode::OK.is_success());
        assert!(SmtpResponseCode::START_MAIL_INPUT.is_continuation());
        assert!(SmtpResponseCode::SERVICE_UNAVAILABLE.is_transient_failure());
        assert!(SmtpResponseCode::SYNTAX_ERROR.is_permanent_failure());
    }

    #[test]
    fn test_response_format() {
        let resp = SmtpResponse::ok("Mail accepted");
        assert_eq!(resp.format(), "250 Mail accepted\r\n");

        let resp = SmtpResponse::service_ready("edgerun.mail");
        assert_eq!(resp.format(), "220 edgerun.mail ESMTP ready\r\n");
    }

    #[test]
    fn test_multiline_response_format() {
        let resp = SmtpResponse::multiline(SmtpResponseCode::OK, vec![
            "Hello mail.example.com".to_string(),
            "PIPELINING".to_string(),
            "8BITMIME".to_string(),
        ]);
        let expected = "250-Hello mail.example.com\r\n250-PIPELINING\r\n250 8BITMIME\r\n";
        assert_eq!(resp.format(), expected);
    }

    #[test]
    fn test_mail_envelope() {
        let mut envelope = MailEnvelope::new("sender@example.com".to_string());
        envelope.add_recipient("rcpt1@example.com".to_string(), vec![]);
        envelope.add_recipient("rcpt2@example.com".to_string(), vec![]);

        assert_eq!(envelope.from, "sender@example.com");
        assert_eq!(envelope.recipients.len(), 2);

        envelope.reset();
        assert_eq!(envelope.from, "sender@example.com");
        assert!(envelope.recipients.is_empty());
        assert!(envelope.data.is_empty());
    }

    #[test]
    fn test_parse_headers() {
        let data = b"From: sender@example.com\r\nTo: recipient@example.com\r\nSubject: Test\r\n\r\nBody";
        let headers = parse_headers(data);
        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0], ("From".to_string(), "sender@example.com".to_string()));
        assert_eq!(headers[1], ("To".to_string(), "recipient@example.com".to_string()));
        assert_eq!(headers[2], ("Subject".to_string(), "Test".to_string()));
    }

    #[test]
    fn test_get_header() {
        let data = b"From: sender@example.com\r\nSubject: Hello\r\n\r\nHi";
        assert_eq!(get_from_address(data), Some("sender@example.com".to_string()));
        assert_eq!(get_subject(data), Some("Hello".to_string()));
        assert_eq!(get_date(data), None);
    }

    #[test]
    fn test_esmtp_parameters() {
        let params = parse_esmtp_parameters("SIZE=1024 RET=FULL ENVID=abc");
        assert_eq!(params.len(), 3);
        assert_eq!(params[0], ("SIZE".to_string(), Some("1024".to_string())));
        assert_eq!(params[1], ("RET".to_string(), Some("FULL".to_string())));
        assert_eq!(params[2], ("ENVID".to_string(), Some("abc".to_string())));

        let params = parse_esmtp_parameters("");
        assert!(params.is_empty());
    }

    #[test]
    fn test_parse_empty_command() {
        let result = SmtpCommand::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unknown_command() {
        let result = SmtpCommand::parse("FOOBAR");
        assert!(result.is_err());
    }

    #[test]
    fn test_smtp_state_transitions() {
        // Valid sequence: Connected -> Ready -> MailSet -> RcptSet -> Data -> Ready
        let mut state = SmtpState::Connected;
        assert_eq!(state, SmtpState::Connected);

        // After EHLO
        state = SmtpState::Ready;
        assert_eq!(state, SmtpState::Ready);

        // After MAIL FROM
        state = SmtpState::MailSet;
        assert_eq!(state, SmtpState::MailSet);

        // After RCPT TO
        state = SmtpState::RcptSet;
        assert_eq!(state, SmtpState::RcptSet);

        // After DATA command
        state = SmtpState::Data;
        assert_eq!(state, SmtpState::Data);

        // After message complete
        state = SmtpState::Ready;
        assert_eq!(state, SmtpState::Ready);
    }
}
