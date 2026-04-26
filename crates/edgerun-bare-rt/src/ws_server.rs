//! WebSocket server

extern crate alloc;

use alloc::vec::Vec;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WsOpcode {
    Continuation,
    Text,
    Binary,
    Close,
    Ping,
    Pong,
}

pub struct WsFrame {
    pub opcode: WsOpcode,
    pub payload: Vec<u8>,
    pub fin: bool,
}

pub struct WsConnection {
    pub id: u32,
}

pub struct WsServer;

impl WsServer {
    pub fn new() -> Self { Self }
    pub fn bind(&self, _addr: &[u8], _port: u16) -> WsBindFuture { WsBindFuture { done: false } }
    pub fn accept(&self) -> WsAcceptFuture { WsAcceptFuture { done: false } }
    pub fn broadcast(&self, _msg: &[u8]) -> WsBroadcastFuture { WsBroadcastFuture { done: false } }
}
impl Default for WsServer { fn default() -> Self { Self::new() } }

pub struct WsBindFuture { done: bool }
impl Future for WsBindFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct WsAcceptFuture { done: bool }
impl Future for WsAcceptFuture { type Output = Result<WsConnection, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct WsBroadcastFuture { done: bool }
impl Future for WsBroadcastFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }