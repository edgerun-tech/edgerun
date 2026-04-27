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
    outbound: Vec<(OpCode, Vec<u8>)>,
    inbound: Vec<(OpCode, Vec<u8>)>,
}

impl WebSocket {
    pub fn new() -> Self {
        Self {
            state: WsState::Connecting,
            ping_pending: false,
            outbound: Vec::new(),
            inbound: Vec::new(),
        }
    }

    pub fn handshake(&mut self) -> WsHandshakeFuture<'_> {
        WsHandshakeFuture { ws: self }
    }

    pub fn send_text(&mut self, msg: &str) -> WsSendFuture {
        self.send_frame(OpCode::Text, msg.as_bytes())
    }

    pub fn send_binary(&mut self, data: &[u8]) -> WsSendFuture {
        self.send_frame(OpCode::Binary, data)
    }

    pub fn recv(&mut self) -> WsRecvFuture {
        let result = if self.is_open() {
            self.inbound.pop().ok_or(())
        } else {
            Err(())
        };
        WsRecvFuture {
            result: Some(result),
        }
    }

    pub fn close(&mut self) {
        self.state = WsState::Closing;
    }

    pub fn is_open(&self) -> bool {
        self.state == WsState::Open
    }

    pub fn queue_inbound(&mut self, opcode: OpCode, data: &[u8]) {
        self.inbound.push((opcode, data.to_vec()));
    }

    #[must_use]
    pub fn outbound(&self) -> &[(OpCode, Vec<u8>)] {
        &self.outbound
    }

    fn send_frame(&mut self, opcode: OpCode, data: &[u8]) -> WsSendFuture {
        let result = if self.is_open() && data.len() <= WS_MAX_FRAME {
            self.outbound.push((opcode, data.to_vec()));
            if opcode == OpCode::Ping {
                self.ping_pending = true;
            }
            Ok(())
        } else {
            Err(())
        };
        WsSendFuture {
            result: Some(result),
        }
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
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        self.ws.state = WsState::Open;
        Poll::Ready(Ok(()))
    }
}

pub struct WsSendFuture {
    result: Option<Result<(), ()>>,
}

impl Future for WsSendFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct WsRecvFuture {
    result: Option<Result<(OpCode, Vec<u8>), ()>>,
}

impl Future for WsRecvFuture {
    type Output = Result<(OpCode, Vec<u8>), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct WebSocketServer;

impl WebSocketServer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebSocketServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    #[test]
    fn websocket_handshake_send_and_recv_complete() {
        let mut ws = WebSocket::new();

        crate::block_on(Box::pin(ws.handshake())).unwrap();
        crate::block_on(Box::pin(ws.send_text("hello"))).unwrap();
        ws.queue_inbound(OpCode::Text, b"world");
        let frame = crate::block_on(Box::pin(ws.recv())).unwrap();

        assert!(ws.is_open());
        assert_eq!(ws.outbound()[0].1, b"hello");
        assert_eq!(frame, (OpCode::Text, b"world".to_vec()));
    }
}
