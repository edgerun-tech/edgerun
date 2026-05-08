#[cfg(not(target_os = "none"))]
pub mod client;
#[cfg(not(target_os = "none"))]
pub mod server;
pub mod session_core;
pub mod types;

#[cfg(not(target_os = "none"))]
pub use client::LmtpClient;
pub use edgerun_protocols::lmtp::session_core::{
    AllowAllLmtpPolicy, LmtpCommand, LmtpSessionAction, LmtpSessionConfig, LmtpSessionCore,
    LmtpSessionPolicy, LmtpSessionStep,
};
pub use edgerun_protocols::lmtp::types::{LmtpResponse, LmtpResponseCode};
#[cfg(not(target_os = "none"))]
pub use server::{LmtpServer, LmtpServerConfig};
