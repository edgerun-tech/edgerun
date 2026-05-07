//! Complete email server: SMTP (RFC 5321), IMAP (RFC 3501), LMTP (RFC 2033).
//!
//! ## Protocols
//! - **SMTP** — Server + client with STARTTLS, SMTPS, AUTH, DSN, BDAT, rate limiting,
//!   outbound relay (DNS MX lookup, persistent queue, exponential backoff, DSN bounce),
//!   and Maildir persistent mailbox store.
//! - **IMAP** — IMAP4rev1 server with Maildir backend (reads same mailboxes SMTP writes to).
//! - **LMTP** — Local Mail Transfer Protocol for per-recipient delivery.
//!
//! ## Shared storage
//! `smtp::server::MaildirStore` and `imap::MaildirImapStore` both operate on the
//! same Maildir directory tree, enabling:
//! ```text
//! SMTP (delivery) → {root}/{user}/new/ → IMAP (fetch)
//! ```
//!
//! ## Unified server framework
//! The `server` module provides a generic TCP listener with:
//! - Shared TLS upgrade (STARTTLS) via `AsyncTlsStream::client/server()`
//! - Per-connection spawn with idle timeout + command limits
//! - `MailProtocol` trait for protocol-specific command dispatch

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

pub(crate) mod prelude {
    pub use alloc::boxed::Box;
    pub use alloc::format;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec;
    pub use alloc::vec::Vec;
}

#[cfg(not(target_os = "none"))]
pub mod command_middleware;
#[cfg(feature = "dkim")]
pub(crate) mod dns_query;
pub mod imap;
pub mod lmtp;
#[cfg(not(target_os = "none"))]
pub mod middleware_impls;
pub mod node_address;
#[cfg(not(target_os = "none"))]
pub mod rt;
#[cfg(not(target_os = "none"))]
pub mod server;
pub mod smtp;
