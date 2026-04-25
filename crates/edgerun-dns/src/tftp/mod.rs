//! TFTP server (RFC 1350) with RFC 2347/2348 option extensions.
//!
//! Supports block size negotiation, read requests, encrypted blob store.
//! Used by PXE boot clients after DHCP provides server/bootfile info.

pub mod blob_provider;
pub mod message;
pub mod server;

pub use blob_provider::BlobTftpProvider;
pub use message::{TftpError, TftpMessage, TftpOpcode, TftpOptions};
pub use server::TftpServer;
