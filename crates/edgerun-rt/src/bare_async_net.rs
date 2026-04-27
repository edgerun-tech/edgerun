//! Bare-target async network compatibility for crates migrating off hosted runtimes.

extern crate alloc;

use alloc::sync::Arc;
use core::future::Future;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::io::{AsyncRead, AsyncWrite, IoError, Result as IoResult};

fn unavailable() -> IoError {
    IoError::Other("bare async network operation is unavailable")
}

fn unspecified_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)
}

pub struct AsyncTcpStream;

impl AsyncTcpStream {
    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        Ok(unspecified_addr())
    }

    pub fn peer_addr(&self) -> IoResult<SocketAddr> {
        Ok(unspecified_addr())
    }

    pub fn split(self: Arc<Self>) -> (Arc<Self>, Arc<Self>) {
        (self.clone(), self)
    }
}

impl AsyncRead for AsyncTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        Poll::Ready(Err(IoError::UnexpectedEof))
    }
}

impl AsyncWrite for AsyncTcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<IoResult<usize>> {
        Poll::Ready(Err(IoError::WriteZero))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for Arc<AsyncTcpStream> {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        Poll::Ready(Err(IoError::UnexpectedEof))
    }
}

impl AsyncWrite for Arc<AsyncTcpStream> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<IoResult<usize>> {
        Poll::Ready(Err(IoError::WriteZero))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }
}

pub struct ConnectFuture;

impl ConnectFuture {
    pub fn new<A>(_addrs: A) -> Self {
        Self
    }
}

impl Future for ConnectFuture {
    type Output = IoResult<Arc<AsyncTcpStream>>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(Err(unavailable()))
    }
}

pub struct AsyncTcpListener;

impl AsyncTcpListener {
    pub fn bind<A>(_addr: A) -> IoResult<Self> {
        Ok(Self)
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        Ok(unspecified_addr())
    }

    pub async fn accept(&self) -> IoResult<(Arc<AsyncTcpStream>, SocketAddr)> {
        Err(unavailable())
    }
}

pub struct AsyncUdpSocket;

impl AsyncUdpSocket {
    pub fn bind<A>(_addr: A) -> IoResult<Self> {
        Ok(Self)
    }

    pub fn from_std<T>(_socket: T) -> IoResult<Self> {
        Ok(Self)
    }

    pub fn connect<A>(&self, _addr: A) -> IoResult<()> {
        Ok(())
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        Ok(unspecified_addr())
    }

    pub fn peer_addr(&self) -> IoResult<SocketAddr> {
        Ok(unspecified_addr())
    }

    pub fn set_broadcast(&self, _broadcast: bool) -> IoResult<()> {
        Ok(())
    }

    pub async fn send_to(&self, _buf: &[u8], _target: SocketAddr) -> IoResult<usize> {
        Err(unavailable())
    }

    pub async fn recv_from(&self, _buf: &mut [u8]) -> IoResult<(usize, SocketAddr)> {
        Err(unavailable())
    }

    pub fn send(&self, _buf: &[u8]) -> IoResult<usize> {
        Err(unavailable())
    }

    pub fn recv(&self, _buf: &mut [u8]) -> IoResult<usize> {
        Err(unavailable())
    }
}
