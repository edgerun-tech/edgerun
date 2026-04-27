//! TFTP client for kernel loading over PXE

#![no_std]

extern crate alloc;

pub mod blob_provider;
pub mod compat;
pub mod message;
pub mod server;
pub mod std;

pub use compat::CancellationToken;
pub use server::TftpServer;

pub const TFTP_PORT: u16 = 69;
pub const TFTP_BLOCK_SIZE: usize = 512;
pub const TFTP_MAX_BLOCK: usize = 65535;

pub const OP_RRQ: u16 = 1;
pub const OP_WRQ: u16 = 2;
pub const OP_DATA: u16 = 3;
pub const OP_ACK: u16 = 4;
pub const OP_ERROR: u16 = 5;
pub const OP_OACK: u16 = 6;

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct TftpHeader {
    pub opcode: u16,
    pub block: u16,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct TftpError {
    pub opcode: u16,
    pub code: u16,
    pub msg: [u8; 128],
}

impl Default for TftpError {
    fn default() -> Self {
        Self {
            opcode: OP_ERROR,
            code: 0,
            msg: [0; 128],
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct TftpData {
    pub opcode: u16,
    pub block: u16,
    pub data: [u8; TFTP_BLOCK_SIZE],
}

impl Default for TftpData {
    fn default() -> Self {
        Self {
            opcode: OP_DATA,
            block: 0,
            data: [0; TFTP_BLOCK_SIZE],
        }
    }
}
