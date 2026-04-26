//! CoAP client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoapCode {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoapType {
    Confirmable,
    NonConfirmable,
    Ack,
    Reset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoapOption {
    IfMatch,
    UriHost,
    UriPath,
    ContentFormat,
    MaxAge,
    UriQuery,
    Accept,
    LocationPath,
}

pub struct CoapRequest {
    pub code: CoapCode,
    pub path: Vec<u8>,
    pub payload: Vec<u8>,
    pub token: Vec<u8>,
}

impl CoapRequest {
    pub fn new(code: CoapCode, path: &[u8]) -> Self {
        Self {
            code,
            path: path.to_vec(),
            payload: Vec::new(),
            token: Vec::new(),
        }
    }

    pub fn payload(&mut self, data: &[u8]) {
        self.payload = data.to_vec();
    }
}

pub struct CoapResponse {
    pub code: u8,
    pub payload: Vec<u8>,
    pub token: Vec<u8>,
}

impl CoapResponse {
    pub fn new(code: u8) -> Self {
        Self { code, payload: Vec::new(), token: Vec::new() }
    }
}

pub struct CoapClient;

impl CoapClient {
    pub fn new() -> Self {
        Self
    }

    pub fn request(&self, _req: &CoapRequest) -> CoapRequestFuture {
        CoapRequestFuture { done: false }
    }

    pub fn observe(&self, _path: &[u8]) -> CoapObserveFuture {
        CoapObserveFuture { done: false }
    }
}

impl Default for CoapClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CoapRequestFuture {
    done: bool,
}

impl Future for CoapRequestFuture {
    type Output = Result<CoapResponse, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct CoapObserveFuture {
    done: bool,
}

impl Future for CoapObserveFuture {
    type Output = Result<CoapResponse, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}