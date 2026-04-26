//! TFTP client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TftpMode {
    Netascii,
    Octet,
    Mail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TftpOpcode {
    ReadRequest,
    WriteRequest,
    Data,
    Ack,
    Error,
    Oack,
}

pub struct TftpPacket {
    pub opcode: TftpOpcode,
    pub block: u16,
    pub data: Vec<u8>,
}

pub struct TftpProgress {
    pub total: u64,
    pub transferred: u64,
}

pub struct TftpClient;

impl TftpClient {
    pub fn new() -> Self {
        Self
    }

    pub fn get(&self, _server: &[u8], _file: &[u8]) -> TftpGetFuture {
        TftpGetFuture { done: false }
    }

    pub fn put(&self, _server: &[u8], _file: &[u8], _data: &[u8]) -> TftpPutFuture {
        TftpPutFuture { done: false }
    }

    pub fn progress(&self) -> Option<TftpProgress> {
        None
    }
}

impl Default for TftpClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TftpGetFuture {
    done: bool,
}

impl Future for TftpGetFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct TftpPutFuture {
    done: bool,
}

impl Future for TftpPutFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}