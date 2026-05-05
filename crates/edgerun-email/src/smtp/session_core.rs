//! Transport-free SMTP session state.
//!
//! This module owns command sequencing and envelope mutation only. Socket I/O,
//! TLS handshakes, AUTH credential checks, recipient stores, and message
//! delivery remain adapter responsibilities.

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::smtp::protocol::ESMTP_EXTENSIONS;
use crate::smtp::types::command::{
    extract_dsn_envid, extract_dsn_notify, extract_dsn_orcpt, extract_dsn_ret,
};
use crate::smtp::types::{
    EnhancedStatusCode, MailEnvelope, ServerLimits, SmtpCommand, SmtpResponse, SmtpResponseCode,
    SmtpState,
};

#[derive(Clone, Debug)]
pub struct SmtpSessionConfig {
    pub domain: String,
    pub limits: ServerLimits,
    pub local_domains: Vec<String>,
    pub auth_mechanisms: Vec<String>,
    pub require_auth: bool,
    pub tls_configured: bool,
    pub starttls_available: bool,
    pub queue_available: bool,
}

impl Default for SmtpSessionConfig {
    fn default() -> Self {
        Self {
            domain: "edgerun.mail".to_string(),
            limits: ServerLimits::default(),
            local_domains: vec!["edgerun.mail".to_string()],
            auth_mechanisms: vec!["PLAIN".to_string(), "LOGIN".to_string()],
            require_auth: false,
            tls_configured: false,
            starttls_available: false,
            queue_available: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SmtpSessionAuth {
    pub authenticated: bool,
    pub identity: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct SmtpPeerContext {
    pub trusted_submitter: bool,
    pub tls_active: bool,
}

pub trait SmtpSessionPolicy {
    fn auth_required(&self) -> bool {
        false
    }

    fn validate_sender(&self, _address: &str) -> bool {
        true
    }

    fn validate_local_recipient(&self, _address: &str) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AllowAllSmtpPolicy;

impl SmtpSessionPolicy for AllowAllSmtpPolicy {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SmtpSessionAction {
    Continue,
    Quit,
    StartTls,
    ReadBdat { size: usize, last: bool },
    Deliver,
}

#[derive(Clone, Debug)]
pub struct SmtpSessionStep {
    pub responses: Vec<SmtpResponse>,
    pub action: SmtpSessionAction,
}

impl SmtpSessionStep {
    fn continue_with(response: SmtpResponse) -> Self {
        Self {
            responses: vec![response],
            action: SmtpSessionAction::Continue,
        }
    }

    fn action(action: SmtpSessionAction, response: SmtpResponse) -> Self {
        Self {
            responses: vec![response],
            action,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SmtpSessionCore {
    pub config: SmtpSessionConfig,
    pub peer: SmtpPeerContext,
    pub state: SmtpState,
    pub envelope: MailEnvelope,
    pub ehlo_domain: Option<String>,
    pub auth: SmtpSessionAuth,
    pub command_count: usize,
    pub pending_bdat_bytes: usize,
    pub pending_bdat_last: bool,
}

impl SmtpSessionCore {
    pub fn new(config: SmtpSessionConfig, peer: SmtpPeerContext) -> Self {
        Self {
            config,
            peer,
            state: SmtpState::Connected,
            envelope: MailEnvelope::new(String::new()),
            ehlo_domain: None,
            auth: SmtpSessionAuth::default(),
            command_count: 0,
            pending_bdat_bytes: 0,
            pending_bdat_last: false,
        }
    }

    pub fn greeting(&self) -> SmtpResponse {
        SmtpResponse::service_ready(&self.config.domain)
    }

    pub fn handle_line<P: SmtpSessionPolicy>(&mut self, line: &str, policy: &P) -> SmtpSessionStep {
        if line.len() > self.config.limits.max_line_length {
            return SmtpSessionStep::continue_with(SmtpResponse::line_too_long(
                line.len(),
                self.config.limits.max_line_length,
            ));
        }

        if self.state == SmtpState::Data {
            return self.handle_data_line(line);
        }

        if self.command_count >= self.config.limits.max_commands {
            return SmtpSessionStep::continue_with(SmtpResponse::bad_sequence("Too many commands"));
        }

        self.command_count += 1;
        match SmtpCommand::parse(line) {
            Ok(command) => self.handle_command(command, policy),
            Err(err) => {
                SmtpSessionStep::continue_with(SmtpResponse::syntax_error(&err.to_string()))
            }
        }
    }

    pub fn handle_command<P: SmtpSessionPolicy>(
        &mut self,
        command: SmtpCommand,
        policy: &P,
    ) -> SmtpSessionStep {
        match command {
            SmtpCommand::Ehlo(domain) => {
                self.ehlo_domain = Some(domain.clone());
                self.state = SmtpState::Ready;

                let mut lines = vec![alloc::format!("Hello {}", domain)];
                for ext in ESMTP_EXTENSIONS {
                    lines.push(ext.to_string());
                }

                let auth_transport_ok = self.peer.tls_active || !self.config.tls_configured;
                if !self.auth.authenticated
                    && auth_transport_ok
                    && !self.config.auth_mechanisms.is_empty()
                {
                    lines.push(alloc::format!(
                        "AUTH {}",
                        self.config.auth_mechanisms.join(" ")
                    ));
                }

                if self.config.starttls_available && !self.peer.tls_active {
                    lines.push("STARTTLS".to_string());
                }

                SmtpSessionStep::continue_with(SmtpResponse::multiline(SmtpResponseCode::OK, lines))
            }
            SmtpCommand::Helo(domain) => {
                self.ehlo_domain = Some(domain.clone());
                self.state = SmtpState::Ready;
                SmtpSessionStep::continue_with(SmtpResponse::ok(&alloc::format!(
                    "Hello {}", domain
                )))
            }
            SmtpCommand::MailFrom {
                address,
                parameters,
            } => self.handle_mail_from(address, parameters, policy),
            SmtpCommand::RcptTo {
                address,
                parameters,
            } => self.handle_rcpt_to(address, parameters, policy),
            SmtpCommand::Data => {
                if self.state != SmtpState::RcptSet {
                    return SmtpSessionStep::continue_with(SmtpResponse::bad_sequence(
                        "No valid recipients",
                    ));
                }
                self.state = SmtpState::Data;
                SmtpSessionStep::continue_with(SmtpResponse::start_mail_input())
            }
            SmtpCommand::Bdat { size, last } => self.handle_bdat(size, last),
            SmtpCommand::Rset => {
                self.reset_transaction();
                SmtpSessionStep::continue_with(SmtpResponse::ok("OK"))
            }
            SmtpCommand::Noop => SmtpSessionStep::continue_with(SmtpResponse::ok("OK")),
            SmtpCommand::Quit => {
                self.state = SmtpState::Quit;
                SmtpSessionStep::action(SmtpSessionAction::Quit, SmtpResponse::closing())
            }
            SmtpCommand::Starttls => {
                if self.peer.tls_active {
                    return SmtpSessionStep::continue_with(SmtpResponse::bad_sequence(
                        "TLS already active",
                    ));
                }
                if !self.config.starttls_available {
                    return SmtpSessionStep::continue_with(SmtpResponse::command_not_implemented(
                        "STARTTLS",
                    ));
                }
                SmtpSessionStep::action(
                    SmtpSessionAction::StartTls,
                    SmtpResponse::ok("Ready to start TLS"),
                )
            }
            SmtpCommand::Vrfy(_) => SmtpSessionStep::continue_with(SmtpResponse::vrfy_disabled()),
            SmtpCommand::Expn(_) => SmtpSessionStep::continue_with(SmtpResponse::expn_disabled()),
            SmtpCommand::Help(_) => {
                SmtpSessionStep::continue_with(SmtpResponse::help_text(&self.config.domain))
            }
            SmtpCommand::Turn => {
                SmtpSessionStep::continue_with(SmtpResponse::command_not_implemented("TURN"))
            }
            SmtpCommand::Etrn(domain) => SmtpSessionStep::continue_with(SmtpResponse::ok(
                &alloc::format!("No mail for {}", domain),
            )),
            SmtpCommand::Auth { .. } | SmtpCommand::AuthResponse(_) => {
                SmtpSessionStep::continue_with(SmtpResponse::command_not_implemented("AUTH"))
            }
        }
    }

    pub fn append_bdat(&mut self, chunk: &[u8]) -> SmtpSessionStep {
        self.envelope.data.extend_from_slice(chunk);
        self.pending_bdat_bytes = self.pending_bdat_bytes.saturating_sub(chunk.len());

        if self.message_too_large() {
            self.reset_transaction();
            return SmtpSessionStep::continue_with(SmtpResponse::message_too_large());
        }

        if self.pending_bdat_bytes == 0 && self.pending_bdat_last {
            self.state = SmtpState::Ready;
            return SmtpSessionStep {
                responses: Vec::new(),
                action: SmtpSessionAction::Deliver,
            };
        }

        SmtpSessionStep {
            responses: Vec::new(),
            action: SmtpSessionAction::Continue,
        }
    }

    fn handle_mail_from<P: SmtpSessionPolicy>(
        &mut self,
        address: String,
        parameters: Vec<(String, Option<String>)>,
        policy: &P,
    ) -> SmtpSessionStep {
        if self.state != SmtpState::Ready && self.state != SmtpState::MailSet {
            return SmtpSessionStep::continue_with(SmtpResponse::bad_sequence(
                "MAIL FROM not allowed in current state",
            ));
        }

        if (self.config.require_auth || policy.auth_required()) && !self.auth.authenticated {
            return SmtpSessionStep::continue_with(SmtpResponse::auth_required());
        }

        if !policy.validate_sender(&address) {
            return SmtpSessionStep::continue_with(SmtpResponse::mailbox_not_found(&address));
        }

        if self.config.limits.max_message_size > 0 {
            for (key, value) in &parameters {
                if key.eq_ignore_ascii_case("SIZE") {
                    if let Some(size) = value.as_ref().and_then(|value| value.parse::<usize>().ok())
                    {
                        if size > self.config.limits.max_message_size {
                            return SmtpSessionStep::continue_with(
                                SmtpResponse::message_too_large(),
                            );
                        }
                    }
                }
            }
        }

        for (key, value) in &parameters {
            if key.eq_ignore_ascii_case("BODY") {
                if let Some(body_type) = value {
                    let body_type = body_type.to_lowercase();
                    if body_type != "7bit" && body_type != "8bitmime" && body_type != "binarymime" {
                        return SmtpSessionStep::continue_with(SmtpResponse::syntax_error(
                            &alloc::format!("Unknown BODY type: {}", body_type),
                        ));
                    }
                }
            }
        }

        self.envelope = MailEnvelope::new(address);
        self.envelope.from_parameters = parameters;
        self.envelope.dsn_ret = extract_dsn_ret(&self.envelope.from_parameters).unwrap_or_default();
        self.envelope.dsn_envid = extract_dsn_envid(&self.envelope.from_parameters);
        self.envelope.authenticated_identity = self.auth.identity.clone();
        self.state = SmtpState::MailSet;

        SmtpSessionStep::continue_with(
            SmtpResponse::ok("Sender OK").with_enhanced(EnhancedStatusCode::MAIL_FROM_OK),
        )
    }

    fn handle_rcpt_to<P: SmtpSessionPolicy>(
        &mut self,
        address: String,
        parameters: Vec<(String, Option<String>)>,
        policy: &P,
    ) -> SmtpSessionStep {
        if self.state != SmtpState::MailSet && self.state != SmtpState::RcptSet {
            return SmtpSessionStep::continue_with(SmtpResponse::bad_sequence(
                "RCPT TO not allowed in current state",
            ));
        }

        if self.envelope.recipient_count() >= self.config.limits.max_recipients {
            return SmtpSessionStep::continue_with(SmtpResponse::too_many_recipients(
                self.envelope.recipient_count() + 1,
            ));
        }

        let domain = extract_domain_from_address(&address);
        let is_local = self
            .config
            .local_domains
            .iter()
            .any(|local_domain| local_domain.eq_ignore_ascii_case(&domain));

        if is_local && !policy.validate_local_recipient(&address) {
            return SmtpSessionStep::continue_with(SmtpResponse::mailbox_not_found(&address));
        }
        if !is_local && (!self.peer.trusted_submitter || !self.config.queue_available) {
            return SmtpSessionStep::continue_with(SmtpResponse::mailbox_not_found(&address));
        }

        let notify = extract_dsn_notify(&parameters).unwrap_or_default();
        let orcpt = extract_dsn_orcpt(&parameters);
        self.envelope
            .add_recipient(address, parameters, notify, orcpt);
        self.state = SmtpState::RcptSet;

        SmtpSessionStep::continue_with(
            SmtpResponse::ok("Recipient OK").with_enhanced(EnhancedStatusCode::RCPT_TO_OK),
        )
    }

    fn handle_data_line(&mut self, line: &str) -> SmtpSessionStep {
        if line == "." {
            self.state = SmtpState::Ready;
            self.command_count += 1;
            return SmtpSessionStep {
                responses: Vec::new(),
                action: SmtpSessionAction::Deliver,
            };
        }

        let data_line = line.strip_prefix("..").unwrap_or(line);
        self.envelope.data.extend_from_slice(data_line.as_bytes());
        self.envelope.data.extend_from_slice(b"\r\n");

        if self.message_too_large() {
            self.reset_transaction();
            return SmtpSessionStep::continue_with(SmtpResponse::message_too_large());
        }

        SmtpSessionStep {
            responses: Vec::new(),
            action: SmtpSessionAction::Continue,
        }
    }

    fn handle_bdat(&mut self, size: usize, last: bool) -> SmtpSessionStep {
        if self.state != SmtpState::RcptSet && self.state != SmtpState::Data {
            return SmtpSessionStep::continue_with(SmtpResponse::bad_sequence(
                "BDAT requires RCPT TO first",
            ));
        }

        let requested_size = self.envelope.data.len().checked_add(size);
        if requested_size.is_none()
            || (self.config.limits.max_message_size > 0
                && requested_size.unwrap_or(usize::MAX) > self.config.limits.max_message_size)
        {
            return SmtpSessionStep::continue_with(SmtpResponse::message_too_large());
        }

        self.state = SmtpState::Data;
        self.pending_bdat_bytes = size;
        self.pending_bdat_last = last;
        SmtpSessionStep::action(
            SmtpSessionAction::ReadBdat { size, last },
            SmtpResponse::ok("OK"),
        )
    }

    fn reset_transaction(&mut self) {
        self.envelope.reset();
        self.envelope.authenticated_identity = self.auth.identity.clone();
        self.pending_bdat_bytes = 0;
        self.pending_bdat_last = false;
        self.state = SmtpState::Ready;
    }

    fn message_too_large(&self) -> bool {
        self.config.limits.max_message_size > 0
            && self.envelope.data.len() > self.config.limits.max_message_size
    }
}

pub fn extract_domain_from_address(address: &str) -> String {
    let address = address.trim();
    let address = address.strip_prefix('<').unwrap_or(address);
    let address = address.strip_suffix('>').unwrap_or(address);
    if let Some(at_pos) = address.rfind('@') {
        address[at_pos + 1..].to_string()
    } else {
        address.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smtp_session_enforces_mail_sequence_without_transport() {
        let mut core = SmtpSessionCore::new(
            SmtpSessionConfig::default(),
            SmtpPeerContext {
                trusted_submitter: true,
                tls_active: false,
            },
        );
        let policy = AllowAllSmtpPolicy;

        assert_eq!(core.greeting().code, SmtpResponseCode::SERVICE_READY);
        let rcpt = core.handle_line("RCPT TO:<a@edgerun.mail>", &policy);
        assert_eq!(rcpt.responses[0].code, SmtpResponseCode::BAD_SEQUENCE);

        let ehlo = core.handle_line("EHLO node.local", &policy);
        assert_eq!(ehlo.responses[0].code, SmtpResponseCode::OK);
        assert_eq!(core.state, SmtpState::Ready);

        let mail = core.handle_line("MAIL FROM:<sender@edgerun.mail>", &policy);
        assert_eq!(mail.responses[0].code, SmtpResponseCode::OK);
        assert_eq!(core.state, SmtpState::MailSet);

        let rcpt = core.handle_line("RCPT TO:<a@edgerun.mail>", &policy);
        assert_eq!(rcpt.responses[0].code, SmtpResponseCode::OK);
        assert_eq!(core.state, SmtpState::RcptSet);
    }

    #[test]
    fn smtp_session_collects_data_until_delivery_boundary() {
        let mut core = SmtpSessionCore::new(
            SmtpSessionConfig::default(),
            SmtpPeerContext {
                trusted_submitter: true,
                tls_active: false,
            },
        );
        let policy = AllowAllSmtpPolicy;

        core.handle_line("EHLO node.local", &policy);
        core.handle_line("MAIL FROM:<sender@edgerun.mail>", &policy);
        core.handle_line("RCPT TO:<a@edgerun.mail>", &policy);
        let data = core.handle_line("DATA", &policy);
        assert_eq!(data.responses[0].code, SmtpResponseCode::START_MAIL_INPUT);

        let line = core.handle_line("Subject: hi", &policy);
        assert_eq!(line.action, SmtpSessionAction::Continue);
        let done = core.handle_line(".", &policy);
        assert_eq!(done.action, SmtpSessionAction::Deliver);
        assert_eq!(core.envelope.data, b"Subject: hi\r\n");
        assert_eq!(core.state, SmtpState::Ready);
    }
}
