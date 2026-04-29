//! TFTP compatibility re-exports.
//!
//! TFTP is implemented by the dedicated `edgerun-tftp` crate. This module keeps
//! the historical `edgerun_dns::tftp` path available without carrying a second
//! protocol implementation.

pub use edgerun_tftp::blob_provider::{BlobEntry, BlobTftpProvider, DecryptFn, MemFileProvider};
pub use edgerun_tftp::message::{
    TftpError, TftpMessage, TftpOpcode, TftpOptions, DEFAULT_BLKSIZE, DEFAULT_TIMEOUT, MAX_BLKSIZE,
    TFTP_PORT,
};
pub use edgerun_tftp::server::{FileProvider, TftpServer, TftpServerConfig};
pub use edgerun_tftp::{blob_provider, message, server};
