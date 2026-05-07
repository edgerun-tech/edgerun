//! Transport-free LMTP session state.
//!
//! LMTP is close to SMTP, but it has its own greeting, `LHLO`, and
//! per-recipient delivery responses after DATA. This core owns command
//! sequencing and envelope mutation without owning sockets or storage.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::smtp::types::{
    DsnNotify, EnhancedStatusCode, MailEnvelope, ServerLimits, SmtpCommand, SmtpResponse,
    SmtpResponseCode, SmtpState,
};

#[derive(Clone, Debug)]
pub struct LmtpSessionConfig {
    pub domain: String,
    pub limits: ServerLimits,
}

impl Default for LmtpSessionConfig {
    fn default() -> Self {
        Self {
            domain: "edgerun.mail".to_string(),
            limits: ServerLimits::default(),
        }
    }
}

pub trait LmtpSessionPolicy {
    fn validate_recipient(&self, _address: &str) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AllowAllLmtpPolicy;

impl LmtpSessionPolicy for AllowAllLmtpPolicy {}

#[derive(Clone, Debug)]
pub enum LmtpCommand {
    Lhlo(String),
    Smtp(SmtpCommand),
}

impl LmtpCommand {
    pub fn parse(line: &str) -> Result<Self, alloc::string::String> {
        let trimmed = line.trim();
        let mut parts = trimmed.splitn(2, |c: char| c.is_whitespace());
        let command = parts.next().unwrap_or_default();
        let args = parts.next().unwrap_or_default().trim();

        if command.eq_ignore_ascii_case("LHLO") {
            if args.is_empty() {
                return Err("LHLO requires domain".to_string());
            }
            return Ok(Self::Lhlo(args.to_string()));
        }

        SmtpCommand::parse(trimmed)
            .map(Self::Smtp)
            .map_err(|err| err.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LmtpSessionAction {
    Continue,
    Quit,
    Deliver,
}

#[derive(Clone, Debug)]
pub struct LmtpSessionStep {
    pub responses: Vec<SmtpResponse>,
    pub action: LmtpSessionAction,
}

impl LmtpSessionStep {
    fn continue_with(response: SmtpResponse) -> Self {
        Self {
            responses: vec![response],
            action: LmtpSessionAction::Continue,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LmtpSessionCore {
    pub config: LmtpSessionConfig,
    pub state: SmtpState,
    pub envelope: MailEnvelope,
    pub lhlo_domain: Option<String>,
}

impl LmtpSessionCore {
    pub fn new(config: LmtpSessionConfig) -> Self {
        Self {
            config,
            state: SmtpState::Connected,
            envelope: MailEnvelope::new(String::new()),
            lhlo_domain: None,
        }
    }

    pub fn greeting(&self) -> SmtpResponse {
        SmtpResponse::new(
            SmtpResponseCode::SERVICE_READY,
            format!("{} LMTP ready", self.config.domain),
        )
    }

    pub fn handle_line<P: LmtpSessionPolicy>(&mut self, line: &str, policy: &P) -> LmtpSessionStep {
        if self.state == SmtpState::Data {
            return self.handle_data_line(line);
        }

        if line.len() > self.config.limits.max_line_length {
            return LmtpSessionStep::continue_with(SmtpResponse::line_too_long(
                line.len(),
                self.config.limits.max_line_length,
            ));
        }

        match LmtpCommand::parse(line) {
            Ok(command) => self.handle_command(command, policy),
            Err(err) => LmtpSessionStep::continue_with(SmtpResponse::syntax_error(&err)),
        }
    }

    pub fn handle_command<P: LmtpSessionPolicy>(
        &mut self,
        command: LmtpCommand,
        policy: &P,
    ) -> LmtpSessionStep {
        match command {
            LmtpCommand::Lhlo(domain) => self.handle_lhlo(domain),
            LmtpCommand::Smtp(SmtpCommand::Ehlo(domain))
            | LmtpCommand::Smtp(SmtpCommand::Helo(domain)) => self.handle_lhlo(domain),
            LmtpCommand::Smtp(SmtpCommand::MailFrom {
                address,
                parameters,
            }) => self.handle_mail_from(address, parameters),
            LmtpCommand::Smtp(SmtpCommand::RcptTo {
                address,
                parameters,
            }) => self.handle_rcpt_to(address, parameters, policy),
            LmtpCommand::Smtp(SmtpCommand::Data) => {
                if self.state != SmtpState::RcptSet {
                    return LmtpSessionStep::continue_with(SmtpResponse::bad_sequence(
                        "No valid recipients",
                    ));
                }
                self.state = SmtpState::Data;
                LmtpSessionStep::continue_with(SmtpResponse::start_mail_input())
            }
            LmtpCommand::Smtp(SmtpCommand::Noop) => {
                LmtpSessionStep::continue_with(SmtpResponse::ok("OK"))
            }
            LmtpCommand::Smtp(SmtpCommand::Quit) => {
                self.state = SmtpState::Quit;
                LmtpSessionStep {
                    responses: vec![SmtpResponse::closing()],
                    action: LmtpSessionAction::Quit,
                }
            }
            LmtpCommand::Smtp(_) => {
                LmtpSessionStep::continue_with(SmtpResponse::command_not_implemented("LMTP"))
            }
        }
    }

    pub fn recipient_delivery_responses(
        &self,
        delivered: impl IntoIterator<Item = bool>,
    ) -> Vec<SmtpResponse> {
        delivered
            .into_iter()
            .map(|ok| {
                if ok {
                    SmtpResponse::ok("OK: delivered").with_enhanced(EnhancedStatusCode::QUEUED)
                } else {
                    SmtpResponse::transient_failure("Delivery failed")
                }
            })
            .collect()
    }

    pub fn reset_transaction(&mut self) {
        self.envelope.reset();
        self.state = SmtpState::Ready;
    }

    fn handle_lhlo(&mut self, domain: String) -> LmtpSessionStep {
        self.lhlo_domain = Some(domain.clone());
        self.state = SmtpState::Ready;

        let lines = vec![
            format!("Hello {}", domain),
            format!("SIZE {}", self.config.limits.max_message_size),
            "8BITMIME".to_string(),
            "ENHANCEDSTATUSCODES".to_string(),
            "SMTPUTF8".to_string(),
        ];

        LmtpSessionStep::continue_with(SmtpResponse::multiline(SmtpResponseCode::OK, lines))
    }

    fn handle_mail_from(
        &mut self,
        address: String,
        parameters: Vec<(String, Option<String>)>,
    ) -> LmtpSessionStep {
        if self.state != SmtpState::Ready && self.state != SmtpState::MailSet {
            return LmtpSessionStep::continue_with(SmtpResponse::bad_sequence(
                "MAIL FROM not allowed in current state",
            ));
        }

        for (key, value) in &parameters {
            if key.eq_ignore_ascii_case("SIZE") {
                if let Some(size) = value.as_ref().and_then(|value| value.parse::<usize>().ok()) {
                    if self.config.limits.max_message_size > 0
                        && size > self.config.limits.max_message_size
                    {
                        return LmtpSessionStep::continue_with(SmtpResponse::message_too_large());
                    }
                }
            }
        }

        self.envelope = MailEnvelope::new(address);
        self.envelope.from_parameters = parameters;
        self.state = SmtpState::MailSet;
        LmtpSessionStep::continue_with(
            SmtpResponse::ok("Sender OK").with_enhanced(EnhancedStatusCode::MAIL_FROM_OK),
        )
    }

    fn handle_rcpt_to<P: LmtpSessionPolicy>(
        &mut self,
        address: String,
        parameters: Vec<(String, Option<String>)>,
        policy: &P,
    ) -> LmtpSessionStep {
        if self.state != SmtpState::MailSet && self.state != SmtpState::RcptSet {
            return LmtpSessionStep::continue_with(SmtpResponse::bad_sequence(
                "RCPT TO not allowed in current state",
            ));
        }
        if self.envelope.recipient_count() >= self.config.limits.max_recipients {
            return LmtpSessionStep::continue_with(SmtpResponse::too_many_recipients(
                self.envelope.recipient_count() + 1,
            ));
        }
        if !policy.validate_recipient(&address) {
            return LmtpSessionStep::continue_with(SmtpResponse::mailbox_not_found(&address));
        }

        self.envelope
            .add_recipient(address, parameters, DsnNotify::default(), None);
        self.state = SmtpState::RcptSet;
        LmtpSessionStep::continue_with(
            SmtpResponse::ok("Recipient OK").with_enhanced(EnhancedStatusCode::RCPT_TO_OK),
        )
    }

    fn handle_data_line(&mut self, line: &str) -> LmtpSessionStep {
        if line == "." {
            self.state = SmtpState::Ready;
            return LmtpSessionStep {
                responses: Vec::new(),
                action: LmtpSessionAction::Deliver,
            };
        }

        let data_line = line.strip_prefix("..").unwrap_or(line);
        self.envelope.data.extend_from_slice(data_line.as_bytes());
        self.envelope.data.extend_from_slice(b"\r\n");

        if self.config.limits.max_message_size > 0
            && self.envelope.data.len() > self.config.limits.max_message_size
        {
            self.reset_transaction();
            return LmtpSessionStep::continue_with(SmtpResponse::message_too_large());
        }

        LmtpSessionStep {
            responses: Vec::new(),
            action: LmtpSessionAction::Continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lmtp_session_accepts_lhlo_without_transport() {
        let mut core = LmtpSessionCore::new(LmtpSessionConfig::default());
        let step = core.handle_line("LHLO node.local", &AllowAllLmtpPolicy);
        assert_eq!(core.state, SmtpState::Ready);
        assert_eq!(core.lhlo_domain.as_deref(), Some("node.local"));
        assert_eq!(step.responses[0].code, SmtpResponseCode::OK);
    }

    #[test]
    fn lmtp_session_collects_message_until_delivery_boundary() {
        let mut core = LmtpSessionCore::new(LmtpSessionConfig::default());
        core.handle_line("LHLO node.local", &AllowAllLmtpPolicy);
        core.handle_line("MAIL FROM:<sender@edgerun.mail>", &AllowAllLmtpPolicy);
        core.handle_line("RCPT TO:<user@edgerun.mail>", &AllowAllLmtpPolicy);
        core.handle_line("DATA", &AllowAllLmtpPolicy);
        core.handle_line("Subject: hello", &AllowAllLmtpPolicy);
        let step = core.handle_line(".", &AllowAllLmtpPolicy);

        assert_eq!(step.action, LmtpSessionAction::Deliver);
        assert_eq!(core.envelope.data, b"Subject: hello\r\n");
    }
}
