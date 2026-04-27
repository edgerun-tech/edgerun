//! SFTP client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SftpFileType {
    Regular,
    Directory,
    Symlink,
    Other,
}

pub struct SftpFileAttr {
    pub ftype: SftpFileType,
    pub size: u64,
    pub permissions: u32,
    pub atime: u32,
    pub mtime: u32,
}

pub struct SftpEntry {
    pub name: Vec<u8>,
    pub attr: SftpFileAttr,
}

pub struct SftpClient;

impl SftpClient {
    pub fn new() -> Self {
        Self
    }

    pub fn connect(&self, _host: &[u8], _port: u16) -> SftpConnectFuture {
        SftpConnectFuture { done: false }
    }

    pub fn login(&self, _user: &[u8], _pass: &[u8]) -> SftpLoginFuture {
        SftpLoginFuture { done: false }
    }

    pub fn open(&self, _path: &[u8], _read: bool) -> SftpOpenFuture {
        SftpOpenFuture { done: false }
    }

    pub fn read(&self, _handle: &[u8], _offset: u64, _len: usize) -> SftpReadFuture {
        SftpReadFuture { done: false }
    }

    pub fn write(&self, _handle: &[u8], _offset: u64, _data: &[u8]) -> SftpWriteFuture {
        SftpWriteFuture { done: false }
    }

    pub fn close(&self, _handle: &[u8]) -> SftpCloseFuture {
        SftpCloseFuture { done: false }
    }

    pub fn readdir(&self, _path: &[u8]) -> SftpReaddirFuture {
        SftpReaddirFuture { done: false }
    }

    pub fn mkdir(&self, _path: &[u8]) -> SftpMkdirFuture {
        SftpMkdirFuture { done: false }
    }

    pub fn rmdir(&self, _path: &[u8]) -> SftpRmdirFuture {
        SftpRmdirFuture { done: false }
    }

    pub fn remove(&self, _path: &[u8]) -> SftpRemoveFuture {
        SftpRemoveFuture { done: false }
    }

    pub fn rename(&self, _old: &[u8], _new: &[u8]) -> SftpRenameFuture {
        SftpRenameFuture { done: false }
    }

    pub fn stat(&self, _path: &[u8]) -> SftpStatFuture {
        SftpStatFuture { done: false }
    }

    pub fn lstat(&self, _path: &[u8]) -> SftpStatFuture {
        SftpStatFuture { done: false }
    }

    pub fn fstat(&self, _handle: &[u8]) -> SftpStatFuture {
        SftpStatFuture { done: false }
    }

    pub fn setstat(&self, _path: &[u8], _attr: &SftpFileAttr) -> SftpSetstatFuture {
        SftpSetstatFuture { done: false }
    }

    pub fn symlink(&self, _target: &[u8], _link: &[u8]) -> SftpSymlinkFuture {
        SftpSymlinkFuture { done: false }
    }

    pub fn readlink(&self, _path: &[u8]) -> SftpReadlinkFuture {
        SftpReadlinkFuture { done: false }
    }

    pub fn realpath(&self, _path: &[u8]) -> SftpRealpathFuture {
        SftpRealpathFuture { done: false }
    }

    pub fn quit(&self) -> SftpQuitFuture {
        SftpQuitFuture { done: false }
    }
}

impl Default for SftpClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SftpConnectFuture {
    done: bool,
}

impl Future for SftpConnectFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpLoginFuture {
    done: bool,
}

impl Future for SftpLoginFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpOpenFuture {
    done: bool,
}

impl Future for SftpOpenFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpReadFuture {
    done: bool,
}

impl Future for SftpReadFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpWriteFuture {
    done: bool,
}

impl Future for SftpWriteFuture {
    type Output = Result<usize, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpCloseFuture {
    done: bool,
}

impl Future for SftpCloseFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpReaddirFuture {
    done: bool,
}

impl Future for SftpReaddirFuture {
    type Output = Result<Vec<SftpEntry>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpMkdirFuture {
    done: bool,
}

impl Future for SftpMkdirFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpRmdirFuture {
    done: bool,
}

impl Future for SftpRmdirFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpRemoveFuture {
    done: bool,
}

impl Future for SftpRemoveFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpRenameFuture {
    done: bool,
}

impl Future for SftpRenameFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpStatFuture {
    done: bool,
}

impl Future for SftpStatFuture {
    type Output = Result<SftpFileAttr, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpSetstatFuture {
    done: bool,
}

impl Future for SftpSetstatFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpSymlinkFuture {
    done: bool,
}

impl Future for SftpSymlinkFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpReadlinkFuture {
    done: bool,
}

impl Future for SftpReadlinkFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpRealpathFuture {
    done: bool,
}

impl Future for SftpRealpathFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SftpQuitFuture {
    done: bool,
}

impl Future for SftpQuitFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}