//! LMTP server and types (RFC 2033).
//!
//! LMTP (Local Mail Transfer Protocol) is SMTP without queueing — each message
//! is delivered immediately with per-recipient responses.
//!
//! ## Server example
//! ```no_run
//! use edgerun_lmtp::server::{LmtpServer, LmtpServerConfig};
//! use edgerun_rt::CancellationToken;
//!
//! # async fn example() -> std::io::Result<()> {
//! let config = LmtpServerConfig::default();
//! let server = LmtpServer::with_memory_store(config)?;
//! let shutdown = CancellationToken::new();
//! server.run(shutdown).await
//! # }
//! ```

pub mod server;
pub mod types;

pub use server::{LmtpServer, LmtpServerConfig};
pub use types::{LmtpResponse, LmtpResponseCode};
