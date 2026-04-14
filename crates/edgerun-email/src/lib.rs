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

pub mod imap;
pub mod lmtp;
pub mod smtp;
