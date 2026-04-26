//! IPFS client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub struct IpfsMultihash {
    pub algorithm: u8,
    pub digest: Vec<u8>,
}

impl IpfsMultihash {
    pub fn new(algorithm: u8, digest: &[u8]) -> Self {
        Self { algorithm, digest: digest.to_vec() }
    }
}

pub struct IpfsPin;

pub struct IpfsClient;

impl IpfsClient {
    pub fn new() -> Self {
        Self
    }

    pub fn add(&self, _data: &[u8]) -> IpfsAddFuture {
        IpfsAddFuture { done: false }
    }

    pub fn cat(&self, _hash: &IpfsMultihash) -> IpfsCatFuture {
        IpfsCatFuture { done: false }
    }

    pub fn pin(&self, _hash: &IpfsMultihash) -> IpfsPinFuture {
        IpfsPinFuture { done: false }
    }

    pub fn unpin(&self, _hash: &IpfsMultihash) -> IpfsUnpinFuture {
        IpfsUnpinFuture { done: false }
    }

    pub fn ls(&self, _hash: &IpfsMultihash) -> IpfsLsFuture {
        IpfsLsFuture { done: false }
    }

    pub fn refs(&self, _hash: &IpfsMultihash) -> IpfsRefsFuture {
        IpfsRefsFuture { done: false }
    }

    pub fn block_get(&self, _hash: &IpfsMultihash) -> IpfsBlockGetFuture {
        IpfsBlockGetFuture { done: false }
    }

    pub fn block_put(&self, _data: &[u8]) -> IpfsBlockPutFuture {
        IpfsBlockPutFuture { done: false }
    }

    pub fn dag_get(&self, _hash: &IpfsMultihash, _path: &[u8]) -> IpfsDagGetFuture {
        IpfsDagGetFuture { done: false }
    }

    pub fn dag_put(&self, _data: &[u8]) -> IpfsDagPutFuture {
        IpfsDagPutFuture { done: false }
    }

    pub fn publish(&self, _name: &[u8], _hash: &IpfsMultihash) -> IpfsPublishFuture {
        IpfsPublishFuture { done: false }
    }

    pub fn resolve(&self, _path: &[u8]) -> IpfsResolveFuture {
        IpfsResolveFuture { done: false }
    }
}

impl Default for IpfsClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct IpfsAddFuture {
    done: bool,
}

impl Future for IpfsAddFuture {
    type Output = Result<IpfsMultihash, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct IpfsCatFuture {
    done: bool,
}

impl Future for IpfsCatFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct IpfsPinFuture {
    done: bool,
}

impl Future for IpfsPinFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct IpfsUnpinFuture {
    done: bool,
}

impl Future for IpfsUnpinFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct IpfsLsFuture {
    done: bool,
}

impl Future for IpfsLsFuture {
    type Output = Result<Vec<IpfsMultihash>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct IpfsRefsFuture {
    done: bool,
}

impl Future for IpfsRefsFuture {
    type Output = Result<Vec<IpfsMultihash>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct IpfsBlockGetFuture {
    done: bool,
}

impl Future for IpfsBlockGetFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct IpfsBlockPutFuture {
    done: bool,
}

impl Future for IpfsBlockPutFuture {
    type Output = Result<IpfsMultihash, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct IpfsDagGetFuture {
    done: bool,
}

impl Future for IpfsDagGetFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct IpfsDagPutFuture {
    done: bool,
}

impl Future for IpfsDagPutFuture {
    type Output = Result<IpfsMultihash, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct IpfsPublishFuture {
    done: bool,
}

impl Future for IpfsPublishFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct IpfsResolveFuture {
    done: bool,
}

impl Future for IpfsResolveFuture {
    type Output = Result<IpfsMultihash, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}