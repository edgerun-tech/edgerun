//! WebSocket support

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub const WS_MAX_FRAME: usize = 16384;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpCode {
    Continuation,
    Text,
    Binary,
    Close,
    Ping,
    Pong,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WsState {
    Connecting,
    Open,
    Closing,
    Closed,
}

pub struct WebSocket {
    state: WsState,
    ping_pending: bool,
}

impl WebSocket {
    pub fn new() -> Self {
        Self {
            state: WsState::Connecting,
            ping_pending: false,
        }
    }

    pub fn handshake(&mut self) -> WsHandshakeFuture {
        WsHandshakeFuture { ws: self }
    }

    pub fn send_text(&mut self, _msg: &str) -> WsSendFuture {
        WsSendFuture { done: false }
    }

    pub fn send_binary(&mut self, _data: &[u8]) -> WsSendFuture {
        WsSendFuture { done: false }
    }

    pub fn recv(&self) -> WsRecvFuture {
        WsRecvFuture { done: false }
    }

    pub fn close(&mut self) {
        self.state = WsState::Closing;
    }

    pub fn is_open(&self) -> bool {
        self.state == WsState::Open
    }
}

impl Default for WebSocket {
    fn default() -> Self {
        Self::new()
    }
}

pub struct WsHandshakeFuture<'a> {
    ws: &'a mut WebSocket,
}

impl Future for WsHandshakeFuture<'_> {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        if self.ws.is_open() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}

pub struct WsSendFuture {
    done: bool,
}

impl Future for WsSendFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct WsRecvFuture {
    done: bool,
}

impl Future for WsRecvFuture {
    type Output = Result<(OpCode, Vec<u8>), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct WebSocketServer;

impl WebSocketServer {
    pub fn new() -> Self {
        Self
    }
}