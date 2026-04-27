//! FTP client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FtpTransferType {
    Ascii,
    Binary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FtpState {
    Disconnected,
    Connected,
    Authenticated,
    Transferring,
}

pub struct FtpFile {
    pub name: Vec<u8>,
    pub size: u64,
    pub modified: u32,
}

pub struct FtpClient {
    state: FtpState,
}

impl FtpClient {
    pub fn new() -> Self {
        Self { state: FtpState::Disconnected }
    }

    pub fn connect(&self, _host: &[u8]) -> FtpConnectFuture {
        FtpConnectFuture { done: false }
    }

    pub fn login(&self, _user: &[u8], _pass: &[u8]) -> FtpLoginFuture {
        FtpLoginFuture { done: false }
    }

    pub fn get(&self, _remote: &[u8]) -> FtpGetFuture {
        FtpGetFuture { done: false }
    }

    pub fn put(&self, _local: &[u8], _remote: &[u8]) -> FtpPutFuture {
        FtpPutFuture { done: false }
    }

    pub fn list(&self, _path: &[u8]) -> FtpListFuture {
        FtpListFuture { done: false }
    }

    pub fn cd(&self, _path: &[u8]) -> FtpCdFuture {
        FtpCdFuture { done: false }
    }

    pub fn mkdir(&self, _path: &[u8]) -> FtpMkdirFuture {
        FtpMkdirFuture { done: false }
    }

    pub fn rmdir(&self, _path: &[u8]) -> FtpRmdirFuture {
        FtpRmdirFuture { done: false }
    }

    pub fn delete(&self, _path: &[u8]) -> FtpDeleteFuture {
        FtpDeleteFuture { done: false }
    }

    pub fn quit(&self) -> FtpQuitFuture {
        FtpQuitFuture { done: false }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.state, FtpState::Connected | FtpState::Authenticated)
    }
}

impl Default for FtpClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FtpConnectFuture {
    done: bool,
}

impl Future for FtpConnectFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct FtpLoginFuture {
    done: bool,
}

impl Future for FtpLoginFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct FtpGetFuture {
    done: bool,
}

impl Future for FtpGetFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct FtpPutFuture {
    done: bool,
}

impl Future for FtpPutFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct FtpListFuture {
    done: bool,
}

impl Future for FtpListFuture {
    type Output = Result<Vec<FtpFile>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct FtpCdFuture {
    done: bool,
}

impl Future for FtpCdFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct FtpMkdirFuture {
    done: bool,
}

impl Future for FtpMkdirFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct FtpRmdirFuture {
    done: bool,
}

impl Future for FtpRmdirFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct FtpDeleteFuture {
    done: bool,
}

impl Future for FtpDeleteFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct FtpQuitFuture {
    done: bool,
}

impl Future for FtpQuitFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}