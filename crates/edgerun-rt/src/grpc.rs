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
        Self {
            method: method.to_vec(),
            data: data.to_vec(),
        }
    }
}

pub struct GrpcResponse {
    pub data: Vec<u8>,
    pub status: i32,
}

impl GrpcResponse {
    pub fn new(status: i32) -> Self {
        Self {
            data: Vec::new(),
            status,
        }
    }
}

pub struct GrpcClient {
    calls: usize,
}

impl GrpcClient {
    pub fn new() -> Self {
        Self { calls: 0 }
    }

    pub fn call(&mut self, req: &GrpcRequest) -> GrpcCallFuture {
        self.calls += 1;
        let mut response = GrpcResponse::new(0);
        response.data = req.data.clone();
        GrpcCallFuture {
            result: Some(Ok(response)),
        }
    }

    pub fn stream(&mut self, method: &[u8]) -> GrpcStreamFuture {
        self.calls += 1;
        let result = if method.is_empty() {
            Err(())
        } else {
            let mut response = GrpcResponse::new(0);
            response.data = method.to_vec();
            Ok(response)
        };
        GrpcStreamFuture {
            result: Some(result),
        }
    }

    #[must_use]
    pub fn calls(&self) -> usize {
        self.calls
    }
}

impl Default for GrpcClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GrpcCallFuture {
    result: Option<Result<GrpcResponse, ()>>,
}

impl Future for GrpcCallFuture {
    type Output = Result<GrpcResponse, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct GrpcStreamFuture {
    result: Option<Result<GrpcResponse, ()>>,
}

impl Future for GrpcStreamFuture {
    type Output = Result<GrpcResponse, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct GrpcServer;

impl GrpcServer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GrpcServer {
    fn default() -> Self {
        Self::new()
    }
}
