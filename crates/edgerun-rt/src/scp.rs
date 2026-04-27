//! SCP client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub struct ScpClient;

impl ScpClient {
    pub fn new() -> Self {
        Self
    }

    pub fn connect(&self, _host: &[u8], _port: u16) -> ScpConnectFuture {
        ScpConnectFuture { done: false }
    }

    pub fn login(&self, _user: &[u8], _pass: &[u8]) -> ScpLoginFuture {
        ScpLoginFuture { done: false }
    }

    pub fn get(&self, _remote: &[u8], _local: &[u8]) -> ScpGetFuture {
        ScpGetFuture { done: false }
    }

    pub fn put(&self, _local: &[u8], _remote: &[u8]) -> ScpPutFuture {
        ScpPutFuture { done: false }
    }

    pub fn get_recursive(&self, _remote: &[u8], _local: &[u8]) -> ScpGetFuture {
        ScpGetFuture { done: false }
    }

    pub fn put_recursive(&self, _local: &[u8], _remote: &[u8]) -> ScpPutFuture {
        ScpPutFuture { done: false }
    }
}

impl Default for ScpClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ScpConnectFuture {
    done: bool,
}

impl Future for ScpConnectFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct ScpLoginFuture {
    done: bool,
}

impl Future for ScpLoginFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct ScpGetFuture {
    done: bool,
}

impl Future for ScpGetFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct ScpPutFuture {
    done: bool,
}

impl Future for ScpPutFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}