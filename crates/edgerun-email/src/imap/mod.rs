#[cfg(not(target_os = "none"))]
pub mod client;
#[cfg(not(target_os = "none"))]
pub mod maildir_store;
pub mod message;
pub mod parser;
#[cfg(not(target_os = "none"))]
pub mod server;
pub mod session_core;
pub mod types;

#[cfg(not(target_os = "none"))]
pub use client::ImapClient;
#[cfg(not(target_os = "none"))]
pub use maildir_store::MaildirImapStore;
pub use edgerun_protocols::imap::message::{ImapCommand, ImapResponse, ImapResult};
#[cfg(not(target_os = "none"))]
pub use server::{base64_decode, ImapServer, ImapServerConfig, MailStore, MemoryStore};
pub use edgerun_protocols::imap::session_core::{
    ImapPeerContext, ImapSessionAction, ImapSessionConfig, ImapSessionCore, ImapSessionPolicy,
    ImapSessionStep, RejectAllImapPolicy,
};
pub use edgerun_protocols::imap::types::{
    Envelope, FetchAttr, Flags, ImapInternalDate, ImapState, Mailbox, MailboxStatus, Message,
    SearchKey,
};
