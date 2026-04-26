//! HTTP server

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Unknown,
}

pub struct Request {
    pub method: Method,
    pub path: usize,
    pub body: Vec<u8>,
}

impl Request {
    pub fn new(method: Method, path: usize) -> Self {
        Self { method, path, body: Vec::new() }
    }
}

pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: u16) -> Self {
        Self { status, body: Vec::new() }
    }
    pub fn ok() -> Self { Self::new(200) }
    pub fn not_found() -> Self { Self::new(404) }
}

pub struct HttpServer;

impl HttpServer {
    pub fn new() -> Self { Self }
}

pub struct BindFuture;

impl Future for BindFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}