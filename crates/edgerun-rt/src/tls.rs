//! TLS support for secure connections

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub const TLS_MAX_RECORD: usize = 16384;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlsState {
    Handshake,
    Established,
    Closing,
    Closed,
}

pub struct TlsSession {
    state: TlsState,
    version: [u8; 2],
    cipher_suite: usize,
}

impl TlsSession {
    pub fn new() -> Self {
        Self {
            state: TlsState::Handshake,
            version: [0x03, 0x03],
            cipher_suite: 0,
        }
    }

    pub fn handshake(&mut self) -> HandshakeFuture {
        HandshakeFuture { session: self }
    }

    pub fn encrypt(&self, _data: &[u8]) -> Vec<u8> {
        Vec::new()
    }

    pub fn decrypt(&self, _data: &[u8]) -> Vec<u8> {
        Vec::new()
    }

    pub fn close(&mut self) {
        self.state = TlsState::Closing;
    }

    pub fn is_established(&self) -> bool {
        self.state == TlsState::Established
    }
}

impl Default for TlsSession {
    fn default() -> Self {
        Self::new()
    }
}

pub struct HandshakeFuture<'a> {
    session: &'a mut TlsSession,
}

impl Future for HandshakeFuture<'_> {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        if self.session.is_established() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}

pub struct TlsStream<'a> {
    session: &'a mut TlsSession,
}

impl<'a> TlsStream<'a> {
    pub fn new(session: &'a mut TlsSession) -> Self {
        Self { session }
    }

    pub fn read(&self, _buf: &mut [u8]) -> TlsReadFuture<'_> {
        TlsReadFuture { stream: self }
    }

    pub fn write(&self, _buf: &[u8]) -> TlsWriteFuture<'_> {
        TlsWriteFuture { stream: self }
    }
}

pub struct TlsReadFuture<'a> {
    stream: &'a TlsStream<'a>,
}

impl Future for TlsReadFuture<'_> {
    type Output = Result<usize, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct TlsWriteFuture<'a> {
    stream: &'a TlsStream<'a>,
}

impl Future for TlsWriteFuture<'_> {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}