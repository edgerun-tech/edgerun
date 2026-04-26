//! TFTP client for kernel loading over PXE

#![no_std]

#[derive(Debug, Clone, Copy, Default)]
pub struct TftpConfig {
    pub server_ip: u32,
    pub filename: &'static str,
    pub block_size: usize,
    pub timeout: u32,
    pub retries: u8,
}

impl TftpConfig {
    pub fn new(server_ip: u32, filename: &'static str) -> Self {
        Self {
            server_ip,
            filename,
            block_size: 512,
            timeout: 5,
            retries: 5,
        }
    }
}

pub struct TftpClient {
    pub config: TftpConfig,
    block: u16,
    last_block: u16,
    total_size: usize,
    state: TftpState,
    retries: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum TftpState {
    #[default]
    Init,
    Downloading,
    Completed,
    Error,
}

const TFTP_PORT: u16 = 69;
const TFTP_BLOCK_SIZE: usize = 512;
const OP_RRQ: u16 = 1;
const OP_DATA: u16 = 3;
const OP_ACK: u16 = 4;
const OP_ERROR: u16 = 5;

impl TftpClient {
    pub fn new(config: TftpConfig) -> Self {
        Self {
            config,
            block: 0,
            last_block: 0,
            total_size: 0,
            state: TftpState::Init,
            retries: 0,
        }
    }

    pub fn request(&mut self, _buf: &mut [u8]) -> usize {
        0
    }

    pub fn ack(&mut self, _buf: &mut [u8], block: u16) -> usize {
        self.last_block = block;
        let _ = _buf;
        0
    }

    pub fn parse_data(&mut self, _buf: &[u8]) -> Option<usize> {
        None
    }

    pub fn state(&self) -> TftpState {
        self.state
    }

    pub fn progress(&self) -> (u16, usize) {
        (self.last_block, self.total_size)
    }
}