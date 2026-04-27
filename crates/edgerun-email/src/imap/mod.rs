#[cfg(not(target_os = "none"))]
pub mod client;
#[cfg(not(target_os = "none"))]
pub mod maildir_store;
pub mod message;
pub mod parser;
#[cfg(not(target_os = "none"))]
pub mod server;
pub mod types;

#[cfg(not(target_os = "none"))]
pub use client::ImapClient;
#[cfg(not(target_os = "none"))]
pub use maildir_store::MaildirImapStore;
pub use message::{ImapCommand, ImapResponse, ImapResult};
#[cfg(not(target_os = "none"))]
pub use server::{base64_decode, ImapServer, ImapServerConfig, MailStore, MemoryStore};
pub use types::{
    Envelope, FetchAttr, Flags, ImapState, Mailbox, MailboxStatus, Message, SearchKey,
};
