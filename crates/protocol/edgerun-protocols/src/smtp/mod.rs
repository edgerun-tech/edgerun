//! SMTP protocol types and transport-free session state.
//!
//! This module owns command parsing, response formatting, envelope metadata,
//! header helpers, and SMTP state transitions. It does not own sockets, TLS,
//! DNS lookup, authentication backends, queues, or message storage.

pub mod auth;
pub mod dsn;
pub mod message_builder;
pub mod protocol;
pub mod session_core;
pub mod types;

pub use auth::{
    AuthCredentials, AuthParseError, credentials_from_login, decode_base64_raw, decode_login_field,
    decode_plain_response,
};
pub use dsn::{DeliveryStatus, DsnAction, DsnBounce};
pub use message_builder::{EmailBuilder, MimePart};
pub use protocol::dot_stuffed_data;
pub use session_core::{
    AllowAllSmtpPolicy, SmtpPeerContext, SmtpSessionAction, SmtpSessionAuth, SmtpSessionConfig,
    SmtpSessionCore, SmtpSessionPolicy, SmtpSessionStep, extract_domain_from_address,
};
pub use types::{
    DsnNotify, DsnRet, EnhancedStatusCode, MailEnvelope, ServerLimits, SmtpCommand, SmtpResponse,
    SmtpResponseCode, SmtpState, get_date, get_from_address, get_header, get_subject,
    parse_headers,
};
