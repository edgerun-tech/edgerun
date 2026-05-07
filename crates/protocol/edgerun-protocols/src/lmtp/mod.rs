//! LMTP protocol types and transport-free session state.
//!
//! This module owns LMTP command sequencing and response primitives. It does
//! not own local sockets, mailbox storage, delivery, DNS, or authentication.

pub mod session_core;
pub mod types;

pub use session_core::{
    AllowAllLmtpPolicy, LmtpCommand, LmtpSessionAction, LmtpSessionConfig, LmtpSessionCore,
    LmtpSessionPolicy, LmtpSessionStep,
};
pub use types::{LmtpResponse, LmtpResponseCode};
