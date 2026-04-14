pub mod client;
pub mod maildir_store;
pub mod message;
pub mod parser;
pub mod server;
pub mod types;

pub use client::ImapClient;
pub use maildir_store::MaildirImapStore;
pub use message::{ImapCommand, ImapResponse, ImapResult};
pub use server::{ImapServer, ImapServerConfig, MemoryStore, MailStore, base64_decode};
pub use types::{
    Envelope, FetchAttr, Flags, ImapState, Mailbox, MailboxStatus, Message, SearchKey,
};
