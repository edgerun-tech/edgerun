//! SMTP protocol types and transport-free session state.
//!
//! This module owns command parsing, response formatting, envelope metadata,
//! header helpers, and SMTP state transitions. It does not own sockets, TLS,
//! DNS lookup, authentication backends, queues, or message storage.

pub mod protocol;
pub mod session_core;
pub mod types;

pub use session_core::{
    extract_domain_from_address, AllowAllSmtpPolicy, SmtpPeerContext, SmtpSessionAction,
    SmtpSessionAuth, SmtpSessionConfig, SmtpSessionCore, SmtpSessionPolicy, SmtpSessionStep,
};
pub use types::{
    get_date, get_from_address, get_header, get_subject, parse_headers, DsnNotify, DsnRet,
    EnhancedStatusCode, MailEnvelope, ServerLimits, SmtpCommand, SmtpResponse, SmtpResponseCode,
    SmtpState,
};
