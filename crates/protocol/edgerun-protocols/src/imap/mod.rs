//! IMAP protocol types, parser helpers, and transport-free session state.
//!
//! This module owns IMAP command parsing, response formatting, mailbox data
//! types, and session state transitions. It does not own sockets, TLS,
//! authentication stores, mailbox storage, or filesystem access.

pub mod message;
pub mod parser;
pub mod session_core;
pub mod types;

pub use message::{ImapCommand, ImapResponse, ImapResult};
pub use session_core::{
    ImapPeerContext, ImapSessionAction, ImapSessionConfig, ImapSessionCore, ImapSessionPolicy,
    ImapSessionStep, RejectAllImapPolicy,
};
pub use types::{
    Envelope, FetchAttr, Flags, ImapInternalDate, ImapState, Mailbox, MailboxStatus, Message,
    SearchKey,
};
