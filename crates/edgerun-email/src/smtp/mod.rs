//! Async SMTP server and client (RFC 5321) using `edgerun-rt`.
//!
//! ## Server example
//! ```no_run
//! use edgerun_smtp::server::{SmtpServer, SmtpServerConfig};
//! use edgerun_rt::CancellationToken;
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
//! use edgerun_smtp::client::{SmtpClient, EmailBuilder};
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

pub mod client;
pub mod protocol;
pub mod relay;
pub mod server;
pub mod types;

pub use client::{EmailBuilder, MimePart, SmtpClient};
pub use server::{MailHandler, MemoryMailStore, SmtpServer, SmtpServerConfig};
pub use server::handler::{AuthCredentials, AuthResult};
pub use relay::{DeliveryWorker, DeliveryWorkerConfig, MailIndex, OutboundRelay};
pub use types::{
    DsnNotify, DsnRet, EnhancedStatusCode, MailEnvelope, ServerLimits, SmtpCommand,
    SmtpResponse, SmtpResponseCode, SmtpState,
    get_subject, get_from_address, get_date, parse_headers,
};
