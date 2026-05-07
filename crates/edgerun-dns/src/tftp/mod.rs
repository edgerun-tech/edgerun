//! TFTP protocol compatibility re-exports.

pub use edgerun_protocols::tftp::message::{
    TftpError, TftpMessage, TftpOpcode, TftpOptions, DEFAULT_BLKSIZE, DEFAULT_TIMEOUT, MAX_BLKSIZE,
    TFTP_PORT,
};
pub use edgerun_protocols::tftp::{message, protocol};
