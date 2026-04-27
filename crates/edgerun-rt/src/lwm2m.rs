//! LwM2M client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub const LWM2M_MAX_OBJECTS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LwM2MOperation {
    Read,
    Write,
    Execute,
    Observe,
    Discover,
}

pub struct LwM2MObject {
    pub id: u16,
    pub instances: usize,
}

pub struct LwM2MResource {
    pub id: u16,
    pub data: Vec<u8>,
    pub type_id: u8,
}

pub struct LwM2MClient {
    endpoint: Vec<u8>,
    server: Vec<u8>,
    objects: Vec<LwM2MObject>,
}

impl LwM2MClient {
    pub fn new(endpoint: &[u8], server: &[u8]) -> Self {
        Self {
            endpoint: endpoint.to_vec(),
            server: server.to_vec(),
            objects: Vec::new(),
        }
    }

    pub fn register(&mut self) -> LwM2MRegisterFuture {
        LwM2MRegisterFuture { done: false }
    }

    pub fn read(&self, _obj_id: u16, _res_id: u16) -> LwM2MReadFuture {
        LwM2MReadFuture { done: false }
    }

    pub fn write(&mut self, _obj_id: u16, _res_id: u16, _data: &[u8]) -> LwM2MWriteFuture {
        LwM2MWriteFuture { done: false }
    }

    pub fn observe(&self, _obj_id: u16, _res_id: u16) -> LwM2MObserveFuture {
        LwM2MObserveFuture { done: false }
    }

    pub fn deregister(&mut self) -> LwM2MDeregisterFuture {
        LwM2MDeregisterFuture { done: false }
    }
}

pub struct LwM2MRegisterFuture {
    done: bool,
}

impl Future for LwM2MRegisterFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct LwM2MReadFuture {
    done: bool,
}

impl Future for LwM2MReadFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct LwM2MWriteFuture {
    done: bool,
}

impl Future for LwM2MWriteFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct LwM2MObserveFuture {
    done: bool,
}

impl Future for LwM2MObserveFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct LwM2MDeregisterFuture {
    done: bool,
}

impl Future for LwM2MDeregisterFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}