//! Async SMTP server and client (RFC 5321) using `edgerun-rt`.
//!
//! ## Server example
//! ```no_run
//! use edgerun_email::rt::CancellationToken;
//! use edgerun_email::smtp::server::{SmtpServer, SmtpServerConfig};
//!
//! # async fn example() -> std::io::Result<()> {
//! let config = SmtpServerConfig::default();
//! let server = SmtpServer::with_memory_store(config)?;
//! let shutdown = CancellationToken::new();
//! server.run(shutdown).await
//! # }
//! ```
//!
//! ## Client example
//! ```no_run
//! use edgerun_email::smtp::client::{EmailBuilder, SmtpClient};
//!
//! # async fn example() -> std::io::Result<()> {
//! let mut client = SmtpClient::connect("mail.example.com:25").await?;
//! client.ehlo("my.domain.com").await?;
//! client.mail_from("sender@my.domain.com").await?;
//! client.rcpt_to("recipient@example.com").await?;
//!
//! let message = EmailBuilder::new()
//!     .from("sender@my.domain.com")
//!     .to("recipient@example.com")
//!     .subject("Hello")
//!     .body("Hello, World!")
//!     .build();
//!
//! client.data(&message).await?;
//! client.quit().await?;
//! # Ok(())
//! # }
//! ```

#[cfg(not(target_os = "none"))]
pub mod client;
pub mod protocol;
#[cfg(not(target_os = "none"))]
pub mod relay;
#[cfg(not(target_os = "none"))]
pub mod server;
pub mod session_core;
pub mod types;

#[cfg(not(target_os = "none"))]
pub use crate::server::ConnectionInterceptor;
#[cfg(not(target_os = "none"))]
pub use client::{EmailBuilder, MimePart, SmtpClient};
#[cfg(not(target_os = "none"))]
pub use relay::{DeliveryWorker, DeliveryWorkerConfig, MailIndex, OutboundRelay};
#[cfg(not(target_os = "none"))]
pub use server::handler::{AuthCredentials, AuthResult};
#[cfg(not(target_os = "none"))]
pub use server::{MailHandler, MemoryMailStore, SmtpServer, SmtpServerConfig};
pub use edgerun_protocols::smtp::session_core::{
    extract_domain_from_address, AllowAllSmtpPolicy, SmtpPeerContext, SmtpSessionAction,
    SmtpSessionAuth, SmtpSessionConfig, SmtpSessionCore, SmtpSessionPolicy, SmtpSessionStep,
};
pub use edgerun_protocols::smtp::types::{
    get_date, get_from_address, get_subject, parse_headers, DsnNotify, DsnRet, EnhancedStatusCode,
    MailEnvelope, ServerLimits, SmtpCommand, SmtpResponse, SmtpResponseCode, SmtpState,
};
