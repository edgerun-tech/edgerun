//! Transport-free IMAP session state.
//!
//! The core validates command sequencing and owns connection state. Storage,
//! authentication, literals, TLS handshakes, and socket I/O stay in adapters.

use crate::prelude::*;

use crate::imap::message::{ImapCommand, ImapResponse};
use crate::imap::parser;
use crate::imap::types::ImapState;

#[derive(Clone, Debug)]
pub struct ImapSessionConfig {
    pub domain_name: String,
    pub tls_configured: bool,
    pub starttls_available: bool,
}

impl Default for ImapSessionConfig {
    fn default() -> Self {
        Self {
            domain_name: "edgerun.mail".to_string(),
            tls_configured: false,
            starttls_available: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ImapPeerContext {
    pub tls_active: bool,
}

pub trait ImapSessionPolicy {
    fn authenticate(&self, _user: &str, _token: &str) -> Option<String> {
        None
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RejectAllImapPolicy;

impl ImapSessionPolicy for RejectAllImapPolicy {}

#[derive(Clone, Debug)]
pub enum ImapSessionAction {
    Continue,
    Logout,
    StartTls,
    Authenticate { mechanism: String },
    AdapterCommand { command: ImapCommand },
}

#[derive(Clone, Debug)]
pub struct ImapSessionStep {
    pub responses: Vec<ImapResponse>,
    pub action: ImapSessionAction,
}

impl ImapSessionStep {
    fn continue_with(response: ImapResponse) -> Self {
        Self {
            responses: vec![response],
            action: ImapSessionAction::Continue,
        }
    }

    fn action(action: ImapSessionAction, response: ImapResponse) -> Self {
        Self {
            responses: vec![response],
            action,
        }
    }

    fn adapter(command: ImapCommand) -> Self {
        Self {
            responses: Vec::new(),
            action: ImapSessionAction::AdapterCommand { command },
        }
    }
}

#[derive(Clone, Debug)]
pub struct ImapSessionCore {
    pub config: ImapSessionConfig,
    pub peer: ImapPeerContext,
    pub state: ImapState,
    pub authenticated_user: Option<String>,
    pub selected_mailbox: Option<String>,
}

impl ImapSessionCore {
    pub fn new(config: ImapSessionConfig, peer: ImapPeerContext) -> Self {
        Self {
            config,
            peer,
            state: ImapState::NotAuthenticated,
            authenticated_user: None,
            selected_mailbox: None,
        }
    }

    pub fn greeting(&self) -> ImapResponse {
        ImapResponse::Untagged(format!(
            "OK {} IMAP4rev1 Service Ready",
            self.config.domain_name
        ))
    }

    pub fn handle_line<P: ImapSessionPolicy>(&mut self, line: &str, policy: &P) -> ImapSessionStep {
        let (tag, command_name, args) = match parser::parse_command_line(line) {
            Ok(parsed) => parsed,
            Err(err) => {
                return ImapSessionStep::continue_with(ImapResponse::bad(
                    "*",
                    &format!("Parse error: {}", err),
                ));
            }
        };

        let command = match ImapCommand::parse(&tag, &command_name, &args, Some(line)) {
            Ok(command) => command,
            Err(err) => {
                return ImapSessionStep::continue_with(ImapResponse::bad(
                    &tag,
                    &format!("Parse error: {}", err),
                ));
            }
        };

        self.handle_command(&tag, command, policy)
    }

    pub fn handle_command<P: ImapSessionPolicy>(
        &mut self,
        tag: &str,
        command: ImapCommand,
        policy: &P,
    ) -> ImapSessionStep {
        match command {
            ImapCommand::Capability => ImapSessionStep {
                responses: vec![
                    ImapResponse::Untagged(format!("CAPABILITY {}", self.capabilities().join(" "))),
                    ImapResponse::ok(tag, "CAPABILITY completed"),
                ],
                action: ImapSessionAction::Continue,
            },
            ImapCommand::Noop => {
                ImapSessionStep::continue_with(ImapResponse::ok(tag, "NOOP completed"))
            }
            ImapCommand::Logout => {
                self.state = ImapState::Logout;
                ImapSessionStep {
                    responses: vec![
                        ImapResponse::Untagged("BYE IMAP4rev1 Server logging out".to_string()),
                        ImapResponse::ok(tag, "LOGOUT completed"),
                    ],
                    action: ImapSessionAction::Logout,
                }
            }
            ImapCommand::Starttls => self.handle_starttls(tag),
            ImapCommand::Authenticate { mechanism } => {
                if self.state != ImapState::NotAuthenticated {
                    return ImapSessionStep::continue_with(ImapResponse::bad(
                        tag,
                        "Already authenticated",
                    ));
                }
                ImapSessionStep::action(
                    ImapSessionAction::Authenticate { mechanism },
                    ImapResponse::continuation(""),
                )
            }
            ImapCommand::Select { ref mailbox } | ImapCommand::Examine { ref mailbox } => {
                if self.state != ImapState::Authenticated {
                    return ImapSessionStep::continue_with(ImapResponse::bad(
                        tag,
                        "SELECT requires authenticated state",
                    ));
                }
                self.state = ImapState::Selected;
                self.selected_mailbox = Some(mailbox.clone());
                ImapSessionStep::adapter(command)
            }
            ImapCommand::Close | ImapCommand::Unselect => {
                if self.state != ImapState::Selected {
                    return ImapSessionStep::continue_with(ImapResponse::bad(
                        tag,
                        "Command requires selected mailbox",
                    ));
                }
                self.state = ImapState::Authenticated;
                self.selected_mailbox = None;
                ImapSessionStep::adapter(command)
            }
            ImapCommand::Fetch { .. }
            | ImapCommand::Store { .. }
            | ImapCommand::Search { .. }
            | ImapCommand::Copy { .. }
            | ImapCommand::Expunge
            | ImapCommand::Check
            | ImapCommand::Idle
            | ImapCommand::Move { .. }
            | ImapCommand::UidMove { .. }
            | ImapCommand::Sort { .. }
            | ImapCommand::Thread { .. } => {
                if self.state != ImapState::Selected {
                    return ImapSessionStep::continue_with(ImapResponse::bad(
                        tag,
                        "Command requires selected mailbox",
                    ));
                }
                ImapSessionStep::adapter(command)
            }
            ImapCommand::Create { .. }
            | ImapCommand::Delete { .. }
            | ImapCommand::Rename { .. }
            | ImapCommand::Subscribe { .. }
            | ImapCommand::Unsubscribe { .. }
            | ImapCommand::List { .. }
            | ImapCommand::Lsub { .. }
            | ImapCommand::Status { .. }
            | ImapCommand::Append { .. }
            | ImapCommand::Namespace
            | ImapCommand::Quota { .. }
            | ImapCommand::SetQuota { .. }
            | ImapCommand::Enable { .. }
            | ImapCommand::Id { .. } => {
                if self.state == ImapState::NotAuthenticated {
                    return ImapSessionStep::continue_with(ImapResponse::bad(
                        tag,
                        "Command requires authenticated state",
                    ));
                }
                ImapSessionStep::adapter(command)
            }
            ImapCommand::Uid { command } => self.handle_command(tag, *command, policy),
            ImapCommand::Done => ImapSessionStep::continue_with(ImapResponse::bad(
                tag,
                "DONE only valid during IDLE",
            )),
        }
    }

    pub fn mark_tls_active(&mut self) {
        self.peer.tls_active = true;
    }

    fn handle_starttls(&mut self, tag: &str) -> ImapSessionStep {
        if self.state != ImapState::NotAuthenticated {
            return ImapSessionStep::continue_with(ImapResponse::bad(
                tag,
                "STARTTLS only allowed before authentication",
            ));
        }
        if self.peer.tls_active {
            return ImapSessionStep::continue_with(ImapResponse::bad(tag, "TLS already active"));
        }
        if !self.config.starttls_available {
            return ImapSessionStep::continue_with(ImapResponse::bad(
                tag,
                "STARTTLS not available",
            ));
        }
        ImapSessionStep::action(
            ImapSessionAction::StartTls,
            ImapResponse::ok(tag, "Begin TLS negotiation now"),
        )
    }

    fn capabilities(&self) -> Vec<&'static str> {
        let mut capabilities = vec![
            "IMAP4rev1",
            "UIDPLUS",
            "CHILDREN",
            "IDLE",
            "NAMESPACE",
            "QUOTA",
            "MOVE",
            "SASL-IR",
            "ENABLE",
        ];
        if self.config.starttls_available && !self.peer.tls_active {
            capabilities.push("STARTTLS");
        }
        if self.config.tls_configured && !self.peer.tls_active {
            capabilities.push("LOGINDISABLED");
        }
        capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OneUser;

    impl ImapSessionPolicy for OneUser {
        fn authenticate(&self, user: &str, token: &str) -> Option<String> {
            (user == "user" && token == "pass").then(|| user.to_string())
        }
    }

    #[test]
    fn imap_session_rejects_selected_commands_before_login() {
        let mut core =
            ImapSessionCore::new(ImapSessionConfig::default(), ImapPeerContext::default());
        let step = core.handle_line("A001 FETCH 1:* (FLAGS)", &OneUser);
        assert_eq!(core.state, ImapState::NotAuthenticated);
        assert!(matches!(step.responses[0], ImapResponse::Tagged { .. }));
    }

    #[test]
    fn imap_session_login_is_rejected_without_transport() {
        let mut core =
            ImapSessionCore::new(ImapSessionConfig::default(), ImapPeerContext::default());
        let step = core.handle_line("A001 LOGIN user token", &OneUser);
        assert_eq!(core.state, ImapState::NotAuthenticated);
        assert_eq!(core.authenticated_user.as_deref(), None);
        assert!(matches!(step.action, ImapSessionAction::Continue));
    }

    #[test]
    fn imap_session_select_returns_adapter_action() {
        let mut core =
            ImapSessionCore::new(ImapSessionConfig::default(), ImapPeerContext::default());
        core.state = ImapState::Authenticated;
        core.authenticated_user = Some("user".to_string());
        let step = core.handle_line("A002 SELECT INBOX", &OneUser);
        assert_eq!(core.state, ImapState::Selected);
        assert_eq!(core.selected_mailbox.as_deref(), Some("INBOX"));
        assert!(matches!(
            step.action,
            ImapSessionAction::AdapterCommand {
                command: ImapCommand::Select { .. }
            }
        ));
    }
}
