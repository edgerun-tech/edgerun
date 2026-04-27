//! gRPC support

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrpcMethodType {
    Unary,
    ServerStreaming,
    ClientStreaming,
    BidiStreaming,
}

pub struct GrpcMethod {
    pub name: Vec<u8>,
    pub path: Vec<u8>,
    pub mtype: GrpcMethodType,
}

pub struct GrpcRequest {
    pub method: Vec<u8>,
    pub data: Vec<u8>,
}

impl GrpcRequest {
    pub fn new(method: &[u8], data: &[u8]) -> Self {
        Self { method: method.to_vec(), data: data.to_vec() }
    }
}

pub struct GrpcResponse {
    pub data: Vec<u8>,
    pub status: i32,
}

impl GrpcResponse {
    pub fn new(status: i32) -> Self {
        Self { data: Vec::new(), status }
    }
}

pub struct GrpcClient;

impl GrpcClient {
    pub fn new() -> Self {
        Self
    }

    pub fn call(&self, _req: &GrpcRequest) -> GrpcCallFuture {
        GrpcCallFuture { done: false }
    }

    pub fn stream(&self, _method: &[u8]) -> GrpcStreamFuture {
        GrpcStreamFuture { done: false }
    }
}

impl Default for GrpcClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GrpcCallFuture {
    done: bool,
}

impl Future for GrpcCallFuture {
    type Output = Result<GrpcResponse, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct GrpcStreamFuture {
    done: bool,
}

impl Future for GrpcStreamFuture {
    type Output = Result<GrpcResponse, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct GrpcServer;

impl GrpcServer {
    pub fn new() -> Self {
        Self
    }
}