//! TFTP message codec and read-transfer state machine.

pub mod message;
pub mod protocol;

pub use message::{
    DEFAULT_BLKSIZE, DEFAULT_TIMEOUT, MAX_BLKSIZE, TFTP_PORT, TftpError, TftpMessage, TftpOpcode,
    TftpOptions,
};
pub use protocol::{TftpDatagram, TftpPeerId, TftpReadCore, TftpReadProvider};

pub const TFTP_BLOCK_SIZE: usize = 512;
pub const TFTP_MAX_BLOCK: usize = 65535;

pub const OP_RRQ: u16 = 1;
pub const OP_WRQ: u16 = 2;
pub const OP_DATA: u16 = 3;
pub const OP_ACK: u16 = 4;
pub const OP_ERROR: u16 = 5;
pub const OP_OACK: u16 = 6;
