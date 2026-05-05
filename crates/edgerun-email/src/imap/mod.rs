#[cfg(not(target_os = "none"))]
pub mod client;
#[cfg(not(target_os = "none"))]
pub mod maildir_store;
#[cfg(not(target_os = "none"))]
pub mod message;
pub mod parser;
#[cfg(not(target_os = "none"))]
pub mod server;
#[cfg(not(target_os = "none"))]
pub mod session_core;
#[cfg(not(target_os = "none"))]
pub mod types;

#[cfg(not(target_os = "none"))]
pub use client::ImapClient;
#[cfg(not(target_os = "none"))]
pub use maildir_store::MaildirImapStore;
#[cfg(not(target_os = "none"))]
pub use message::{ImapCommand, ImapResponse, ImapResult};
#[cfg(not(target_os = "none"))]
pub use server::{base64_decode, ImapServer, ImapServerConfig, MailStore, MemoryStore};
#[cfg(not(target_os = "none"))]
pub use session_core::{
    ImapPeerContext, ImapSessionAction, ImapSessionConfig, ImapSessionCore, ImapSessionPolicy,
    ImapSessionStep, RejectAllImapPolicy,
};
#[cfg(not(target_os = "none"))]
pub use types::{
    Envelope, FetchAttr, Flags, ImapState, Mailbox, MailboxStatus, Message, SearchKey,
};
