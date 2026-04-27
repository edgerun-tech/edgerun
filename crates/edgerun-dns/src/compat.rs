//! edgerun-bare-rt adapters for the async API shape used by edgerun-dns.

use alloc::string::ToString;
use alloc::sync::Arc;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::std::io;
use crate::std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

pub use edgerun_bare_rt::{sleep, spawn, timeout, Duration, Instant};

pub struct RwLock<T>(edgerun_bare_rt::RwLock<T>);

impl<T> RwLock<T> {
    pub fn new(value: T) -> Self {
        Self(edgerun_bare_rt::RwLock::new(value))
    }

    pub async fn read(&self) -> edgerun_bare_rt::RwLockReadGuard<'_, T> {
        self.0.read()
    }

    pub async fn write(&self) -> edgerun_bare_rt::RwLockWriteGuard<'_, T> {
        self.0.write()
    }
}

pub struct AsyncUdpSocket(edgerun_bare_rt::UdpSocket);

impl AsyncUdpSocket {
    pub fn bind(addr: &str) -> io::Result<Self> {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let mut socket = edgerun_bare_rt::UdpSocket::new();
        socket
            .bind(to_bare_addr(addr))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(Self(socket))
    }

    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.0.local_addr().map(from_bare_addr)
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.0
            .recv_from(buf)
            .map(|(n, addr)| (n, from_bare_addr(addr)))
            .map_err(|e| io::Error::new(io::ErrorKind::WouldBlock, e.to_string()))
    }

    pub async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.0
            .send_to(buf, to_bare_addr(addr))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    pub fn poll_recv_from(
        &mut self,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<(usize, SocketAddr)>> {
        Poll::Ready(
            self.0
                .recv_from(buf)
                .map(|(n, addr)| (n, from_bare_addr(addr)))
                .map_err(|e| io::Error::new(io::ErrorKind::WouldBlock, e.to_string())),
        )
    }

    pub fn poll_send_to(
        &mut self,
        _cx: &mut Context<'_>,
        buf: &[u8],
        addr: SocketAddr,
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(
            self.0
                .send_to(buf, to_bare_addr(addr))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string())),
        )
    }
}

pub struct AsyncTcpListener(edgerun_bare_rt::TcpListener);

impl AsyncTcpListener {
    pub fn bind(addr: &str) -> io::Result<Self> {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let mut listener = edgerun_bare_rt::TcpListener::new();
        listener
            .bind(to_bare_addr(addr))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        listener
            .listen(128)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(Self(listener))
    }

    pub async fn accept(&self) -> io::Result<(Arc<AsyncTcpStream>, SocketAddr)> {
        let this = self as *const Self as *mut Self;
        let socket = unsafe { &mut (*this).0 }
            .accept()
            .map_err(|e| io::Error::new(io::ErrorKind::WouldBlock, e.to_string()))?;
        let peer = socket
            .remote_addr()
            .map(from_bare_addr)
            .unwrap_or_else(|| SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)));
        Ok((Arc::new(AsyncTcpStream(socket)), peer))
    }

    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.0.local_addr().map(from_bare_addr)
    }
}

pub struct AsyncTcpStream(edgerun_bare_rt::TcpSocket);

pub trait AsyncRead {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>>;
}

pub trait AsyncWrite {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>>;
}

impl AsyncRead for AsyncTcpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(
            self.0
                .recv(buf)
                .map_err(|e| io::Error::new(io::ErrorKind::WouldBlock, e.to_string())),
        )
    }
}

impl AsyncWrite for AsyncTcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(
            self.0
                .send(
                    buf,
                    &edgerun_bare_rt::IpStack::new(),
                    edgerun_bare_rt::IpAddr::new(0, 0, 0, 0),
                    [0; 6],
                )
                .map_err(|e| io::Error::new(io::ErrorKind::WriteZero, e.to_string())),
        )
    }
}

pub fn to_bare_addr(addr: SocketAddr) -> edgerun_bare_rt::SocketAddr {
    match addr {
        SocketAddr::V4(addr) => {
            edgerun_bare_rt::SocketAddr::from_bytes4(addr.ip().octets(), addr.port())
        }
        SocketAddr::V6(addr) => edgerun_bare_rt::SocketAddr::new(0, addr.port()),
    }
}

pub fn from_bare_addr(addr: edgerun_bare_rt::SocketAddr) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::from(addr.ip_bytes())), addr.port())
}
