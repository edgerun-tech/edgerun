#[cfg(not(target_os = "none"))]
pub mod client;
#[cfg(not(target_os = "none"))]
pub mod server;
pub mod types;

#[cfg(not(target_os = "none"))]
pub use client::LmtpClient;
#[cfg(not(target_os = "none"))]
pub use server::{LmtpServer, LmtpServerConfig};
pub use types::{LmtpResponse, LmtpResponseCode};
