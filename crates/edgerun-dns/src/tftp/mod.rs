//! TFTP compatibility re-exports.
//!
//! TFTP is implemented by the dedicated `edgerun-tftp` crate. This module keeps
//! the historical `edgerun_dns::tftp` path available without carrying a second
//! protocol implementation.

pub use edgerun_tftp::blob_provider::{BlobEntry, BlobTftpProvider, DecryptFn, MemFileProvider};
pub use edgerun_tftp::message::{
    DEFAULT_BLKSIZE, DEFAULT_TIMEOUT, MAX_BLKSIZE, TFTP_PORT, TftpError, TftpMessage, TftpOpcode,
    TftpOptions,
};
pub use edgerun_tftp::server::{FileProvider, TftpServer, TftpServerConfig};
pub use edgerun_tftp::{blob_provider, message, server};
