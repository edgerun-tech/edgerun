//! NFS client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NfsFileType {
    Regular,
    Directory,
    Symlink,
    Char,
    Block,
    Socket,
    Fifo,
}

pub struct NfsFileHandle {
    pub data: Vec<u8>,
}

pub struct NfsStat {
    pub ftype: NfsFileType,
    pub mode: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub atime: u32,
    pub mtime: u32,
}

pub struct NfsEntry {
    pub name: Vec<u8>,
    pub handle: NfsFileHandle,
    pub stat: NfsStat,
}

pub struct NfsClient;

impl NfsClient {
    pub fn new() -> Self {
        Self
    }

    pub fn mount(&self, _server: &[u8], _export: &[u8]) -> NfsMountFuture {
        NfsMountFuture { done: false }
    }

    pub fn lookup(&self, _handle: &NfsFileHandle, _name: &[u8]) -> NfsLookupFuture {
        NfsLookupFuture { done: false }
    }

    pub fn read(&self, _handle: &NfsFileHandle, _offset: u64, _count: usize) -> NfsReadFuture {
        NfsReadFuture { done: false }
    }

    pub fn write(&self, _handle: &NfsFileHandle, _offset: u64, _data: &[u8]) -> NfsWriteFuture {
        NfsWriteFuture { done: false }
    }

    pub fn readdir(&self, _handle: &NfsFileHandle, _cookie: u64) -> NfsReaddirFuture {
        NfsReaddirFuture { done: false }
    }

    pub fn mkdir(&self, _parent: &NfsFileHandle, _name: &[u8]) -> NfsMkdirFuture {
        NfsMkdirFuture { done: false }
    }

    pub fn rmdir(&self, _parent: &NfsFileHandle, _name: &[u8]) -> NfsRmdirFuture {
        NfsRmdirFuture { done: false }
    }

    pub fn create(&self, _parent: &NfsFileHandle, _name: &[u8]) -> NfsCreateFuture {
        NfsCreateFuture { done: false }
    }

    pub fn remove(&self, _parent: &NfsFileHandle, _name: &[u8]) -> NfsRemoveFuture {
        NfsRemoveFuture { done: false }
    }

    pub fn unmount(&self) -> NfsUnmountFuture {
        NfsUnmountFuture { done: false }
    }
}

impl Default for NfsClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct NfsMountFuture {
    done: bool,
}

impl Future for NfsMountFuture {
    type Output = Result<NfsFileHandle, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct NfsLookupFuture {
    done: bool,
}

impl Future for NfsLookupFuture {
    type Output = Result<NfsFileHandle, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct NfsReadFuture {
    done: bool,
}

impl Future for NfsReadFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct NfsWriteFuture {
    done: bool,
}

impl Future for NfsWriteFuture {
    type Output = Result<usize, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct NfsReaddirFuture {
    done: bool,
}

impl Future for NfsReaddirFuture {
    type Output = Result<Vec<NfsEntry>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct NfsMkdirFuture {
    done: bool,
}

impl Future for NfsMkdirFuture {
    type Output = Result<NfsFileHandle, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct NfsRmdirFuture {
    done: bool,
}

impl Future for NfsRmdirFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct NfsCreateFuture {
    done: bool,
}

impl Future for NfsCreateFuture {
    type Output = Result<NfsFileHandle, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct NfsRemoveFuture {
    done: bool,
}

impl Future for NfsRemoveFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct NfsUnmountFuture {
    done: bool,
}

impl Future for NfsUnmountFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}