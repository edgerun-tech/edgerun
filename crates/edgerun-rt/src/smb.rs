//! SMB client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmbFileMode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

pub struct SmbFile {
    pub name: Vec<u8>,
    pub size: u64,
    pub created: u32,
    pub modified: u32,
}

pub struct SmbShare {
    pub name: Vec<u8>,
    pub comment: Vec<u8>,
}

pub struct SmbClient;

impl SmbClient {
    pub fn new() -> Self {
        Self
    }

    pub fn connect(&self, _host: &[u8]) -> SmbConnectFuture {
        SmbConnectFuture { done: false }
    }

    pub fn login(&self, _user: &[u8], _pass: &[u8], _domain: &[u8]) -> SmbLoginFuture {
        SmbLoginFuture { done: false }
    }

    pub fn shares(&self) -> SmbSharesFuture {
        SmbSharesFuture { done: false }
    }

    pub fn open(&self, _share: &[u8], _path: &[u8], _mode: SmbFileMode) -> SmbOpenFuture {
        SmbOpenFuture { done: false }
    }

    pub fn read(&self, _fh: usize, _offset: u64, _len: usize) -> SmbReadFuture {
        SmbReadFuture { done: false }
    }

    pub fn write(&self, _fh: usize, _offset: u64, _data: &[u8]) -> SmbWriteFuture {
        SmbWriteFuture { done: false }
    }

    pub fn mkdir(&self, _share: &[u8], _path: &[u8]) -> SmbMkdirFuture {
        SmbMkdirFuture { done: false }
    }

    pub fn rmdir(&self, _share: &[u8], _path: &[u8]) -> SmbRmdirFuture {
        SmbRmdirFuture { done: false }
    }

    pub fn unlink(&self, _share: &[u8], _path: &[u8]) -> SmbUnlinkFuture {
        SmbUnlinkFuture { done: false }
    }

    pub fn close(&self, _fh: usize) -> SmbCloseFuture {
        SmbCloseFuture { done: false }
    }

    pub fn disconnect(&self) -> SmbDisconnectFuture {
        SmbDisconnectFuture { done: false }
    }
}

impl Default for SmbClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SmbConnectFuture {
    done: bool,
}

impl Future for SmbConnectFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SmbLoginFuture {
    done: bool,
}

impl Future for SmbLoginFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SmbSharesFuture {
    done: bool,
}

impl Future for SmbSharesFuture {
    type Output = Result<Vec<SmbShare>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SmbOpenFuture {
    done: bool,
}

impl Future for SmbOpenFuture {
    type Output = Result<usize, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SmbReadFuture {
    done: bool,
}

impl Future for SmbReadFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SmbWriteFuture {
    done: bool,
}

impl Future for SmbWriteFuture {
    type Output = Result<usize, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SmbMkdirFuture {
    done: bool,
}

impl Future for SmbMkdirFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SmbRmdirFuture {
    done: bool,
}

impl Future for SmbRmdirFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SmbUnlinkFuture {
    done: bool,
}

impl Future for SmbUnlinkFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SmbCloseFuture {
    done: bool,
}

impl Future for SmbCloseFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SmbDisconnectFuture {
    done: bool,
}

impl Future for SmbDisconnectFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}