//! Async IMAP server and client (RFC 3501) using `edgerun-rt`.
//!
//! ## Server example
//! ```no_run
//! use edgerun_imap::server::{ImapServer, ImapServerConfig};
//! use edgerun_rt::CancellationToken;
//!
//! # async fn example() -> std::io::Result<()> {
//! let config = ImapServerConfig::default();
//! let server = ImapServer::new(config)?;
//! let shutdown = CancellationToken::new();
//! server.run(shutdown).await
//! # }
//! ```
//!
//! ## Client example
//! ```no_run
//! use edgerun_imap::client::ImapClient;
//!
//! # async fn example() -> std::io::Result<()> {
//! let mut client = ImapClient::connect("mail.example.com:143").await?;
//! client.login("user@example.com", "password").await?;
//! let mailboxes = client.list("", "*").await?;
//! for mb in &mailboxes {
//!     println!("Mailbox: {}", mb.name);
//! }
//! client.logout().await?;
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod maildir_store;
pub mod message;
pub mod parser;
pub mod server;
pub mod types;

pub use maildir_store::MaildirImapStore;
pub use message::{ImapCommand, ImapResponse, ImapResult};
pub use server::{ImapServer, ImapServerConfig, MemoryStore, MailStore, base64_decode};
pub use types::{
    Envelope, FetchAttr, Flags, ImapState, Mailbox, MailboxStatus, Message, SearchKey,
};
