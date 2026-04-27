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
        Self {
            code,
            payload: Vec::new(),
            token: Vec::new(),
        }
    }
}

pub struct CoapClient {
    observations: Vec<Vec<u8>>,
}

impl CoapClient {
    pub fn new() -> Self {
        Self {
            observations: Vec::new(),
        }
    }

    pub fn request(&self, req: &CoapRequest) -> CoapRequestFuture {
        let mut response = match req.code {
            CoapCode::Get => CoapResponse::new(69),
            CoapCode::Post => CoapResponse::new(65),
            CoapCode::Put => CoapResponse::new(68),
            CoapCode::Delete => CoapResponse::new(66),
        };
        response.payload = req.payload.clone();
        response.token = req.token.clone();
        CoapRequestFuture {
            result: Some(Ok(response)),
        }
    }

    pub fn observe(&mut self, path: &[u8]) -> CoapObserveFuture {
        let result = if path.is_empty() {
            Err(())
        } else {
            self.observations.push(path.to_vec());
            let mut response = CoapResponse::new(69);
            response.token = path.to_vec();
            Ok(response)
        };
        CoapObserveFuture {
            result: Some(result),
        }
    }

    #[must_use]
    pub fn observations(&self) -> &[Vec<u8>] {
        &self.observations
    }
}

impl Default for CoapClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CoapRequestFuture {
    result: Option<Result<CoapResponse, ()>>,
}

impl Future for CoapRequestFuture {
    type Output = Result<CoapResponse, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct CoapObserveFuture {
    result: Option<Result<CoapResponse, ()>>,
}

impl Future for CoapObserveFuture {
    type Output = Result<CoapResponse, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}
