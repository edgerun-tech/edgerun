//! HTTP client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub struct HttpClient;

impl HttpClient {
    pub fn new() -> Self {
        Self
    }

    pub fn get(&self, _url: &str) -> HttpGetFuture {
        HttpGetFuture { done: false }
    }

    pub fn post(&self, _url: &str, _body: &[u8]) -> HttpPostFuture {
        HttpPostFuture { done: false }
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct HttpGetFuture {
    done: bool,
}

impl Future for HttpGetFuture {
    type Output = Result<HttpResponse, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct HttpPostFuture {
    done: bool,
}

impl Future for HttpPostFuture {
    type Output = Result<HttpResponse, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn new(status: u16) -> Self {
        Self { status, body: Vec::new() }
    }

    pub fn ok() -> Self {
        Self::new(200)
    }
}