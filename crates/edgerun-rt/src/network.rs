//! Network stack integration

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub const MAX_SOCKETS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SocketState {
    Closed,
    Listen,
    Established,
    Closing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SocketKind {
    Stream,
    Dgram,
    Raw,
}

use crate::udp::SocketAddr;

pub struct TcpListener;

impl TcpListener {
    pub fn new(_addr: [u8; 4], _port: u16) -> Option<Self> {
        Some(Self)
    }

    pub fn accept(&self) -> Option<TcpStream> {
        None
    }
}

pub struct TcpStream;

impl TcpStream {
    pub fn connect(_addr: [u8; 4], _port: u16) -> ConnectFuture {
        ConnectFuture
    }
}

pub struct ConnectFuture;

impl Future for ConnectFuture {
    type Output = Result<TcpStream, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct TcpRead;

impl Future for TcpRead {
    type Output = Result<usize, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct TcpWrite;

impl Future for TcpWrite {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}