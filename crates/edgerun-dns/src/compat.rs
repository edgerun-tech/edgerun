//! edgerun-rt adapters for the async API shape used by edgerun-dns.

use alloc::string::ToString;
use alloc::sync::Arc;
use core::pin::Pin;
use core::task::{Context, Poll};

#[cfg(not(target_os = "none"))]
extern crate std as host_std;

use crate::std::io;
use crate::std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
#[cfg(not(target_os = "none"))]
use host_std::io::{Read, Write};
#[cfg(not(target_os = "none"))]
use host_std::net::UdpSocket as StdUdpSocket;
#[cfg(not(target_os = "none"))]
use host_std::net::{TcpListener as StdTcpListener, TcpStream as StdTcpStream};

pub use edgerun_rt::{sleep, spawn, timeout, Duration, Instant};

#[cfg(not(target_os = "none"))]
const HOST_UDP_IDLE_TICK: host_std::time::Duration = host_std::time::Duration::from_millis(500);

pub struct RwLock<T>(edgerun_rt::RwLock<T>);

impl<T> RwLock<T> {
    pub fn new(value: T) -> Self {
        Self(edgerun_rt::RwLock::new(value))
    }

    pub async fn read(&self) -> edgerun_rt::RwLockReadGuard<'_, T> {
        self.0.read()
    }

    pub async fn write(&self) -> edgerun_rt::RwLockWriteGuard<'_, T> {
        self.0.write()
    }
}

#[cfg(target_os = "none")]
pub struct AsyncUdpSocket(edgerun_rt::UdpSocket);

#[cfg(not(target_os = "none"))]
pub struct AsyncUdpSocket(StdUdpSocket);

#[cfg(not(target_os = "none"))]
fn map_host_io(error: host_std::io::Error) -> io::Error {
    let kind = match error.kind() {
        host_std::io::ErrorKind::InvalidInput => io::ErrorKind::InvalidInput,
        host_std::io::ErrorKind::InvalidData => io::ErrorKind::InvalidData,
        host_std::io::ErrorKind::UnexpectedEof => io::ErrorKind::UnexpectedEof,
        host_std::io::ErrorKind::WouldBlock => io::ErrorKind::WouldBlock,
        host_std::io::ErrorKind::TimedOut => io::ErrorKind::TimedOut,
        host_std::io::ErrorKind::WriteZero => io::ErrorKind::WriteZero,
        host_std::io::ErrorKind::ConnectionRefused => io::ErrorKind::ConnectionRefused,
        host_std::io::ErrorKind::NotFound => io::ErrorKind::NotFound,
        _ => io::ErrorKind::Other,
    };
    io::Error::new(kind, error)
}

impl AsyncUdpSocket {
    #[cfg(target_os = "none")]
    pub fn bind(addr: &str) -> io::Result<Self> {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let mut socket = edgerun_rt::UdpSocket::new();
        socket
            .bind(to_bare_addr(addr))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(Self(socket))
    }

    #[cfg(not(target_os = "none"))]
    pub fn bind(addr: &str) -> io::Result<Self> {
        let socket = StdUdpSocket::bind(addr).map_err(map_host_io)?;
        socket
            .set_read_timeout(Some(HOST_UDP_IDLE_TICK))
            .map_err(map_host_io)?;
        socket
            .set_write_timeout(Some(HOST_UDP_IDLE_TICK))
            .map_err(map_host_io)?;
        Ok(Self(socket))
    }

    #[cfg(target_os = "none")]
    pub fn set_read_timeout(&self, _timeout: Option<Duration>) -> io::Result<()> {
        Ok(())
    }

    #[cfg(not(target_os = "none"))]
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.0.set_read_timeout(timeout).map_err(map_host_io)
    }

    #[cfg(target_os = "none")]
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.0.local_addr().map(from_bare_addr)
    }

    #[cfg(not(target_os = "none"))]
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.0.local_addr().ok().map(to_compat_addr)
    }

    #[cfg(target_os = "none")]
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.0
            .recv_from(buf)
            .map(|(n, addr)| (n, from_bare_addr(addr)))
            .map_err(|e| io::Error::new(io::ErrorKind::WouldBlock, e.to_string()))
    }

    #[cfg(not(target_os = "none"))]
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        match self.0.recv_from(buf) {
            Ok((size, addr)) => Ok((size, to_compat_addr(addr))),
            Err(error) => Err(map_host_io(error)),
        }
    }

    #[cfg(target_os = "none")]
    pub async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.0
            .send_to(buf, to_bare_addr(addr))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    #[cfg(not(target_os = "none"))]
    pub async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        match self.0.send_to(buf, addr) {
            Ok(value) => Ok(value),
            Err(error) => Err(map_host_io(error)),
        }
    }

    pub fn poll_recv_from(
        &mut self,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<(usize, SocketAddr)>> {
        #[cfg(target_os = "none")]
        return Poll::Ready(
            self.0
                .recv_from(buf)
                .map(|(n, addr)| (n, from_bare_addr(addr)))
                .map_err(|e| io::Error::new(io::ErrorKind::WouldBlock, e.to_string())),
        );
        #[cfg(not(target_os = "none"))]
        {
            match self.0.recv_from(buf) {
                Ok((size, addr)) => Poll::Ready(Ok((size, to_compat_addr(addr)))),
                Err(error) => Poll::Ready(Err(map_host_io(error))),
            }
        }
    }

    pub fn poll_send_to(
        &mut self,
        _cx: &mut Context<'_>,
        buf: &[u8],
        addr: SocketAddr,
    ) -> Poll<io::Result<usize>> {
        #[cfg(target_os = "none")]
        return Poll::Ready(
            self.0
                .send_to(buf, to_bare_addr(addr))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string())),
        );
        #[cfg(not(target_os = "none"))]
        {
            match self.0.send_to(buf, addr) {
                Ok(value) => Poll::Ready(Ok(value)),
                Err(error) => Poll::Ready(Err(map_host_io(error))),
            }
        }
    }
}

#[cfg(target_os = "none")]
pub struct AsyncTcpListener(edgerun_rt::TcpListener);

#[cfg(not(target_os = "none"))]
pub struct AsyncTcpListener(StdTcpListener);

impl AsyncTcpListener {
    #[cfg(target_os = "none")]
    pub fn bind(addr: &str) -> io::Result<Self> {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let mut listener = edgerun_rt::TcpListener::new();
        listener
            .bind(to_bare_addr(addr))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        listener
            .listen(128)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(Self(listener))
    }

    #[cfg(not(target_os = "none"))]
    pub fn bind(addr: &str) -> io::Result<Self> {
        let listener = StdTcpListener::bind(addr).map_err(map_host_io)?;
        Ok(Self(listener))
    }

    #[cfg(target_os = "none")]
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

    #[cfg(not(target_os = "none"))]
    pub async fn accept(&self) -> io::Result<(Arc<AsyncTcpStream>, SocketAddr)> {
        match self.0.accept() {
            Ok((stream, peer)) => Ok((Arc::new(AsyncTcpStream(stream)), to_compat_addr(peer))),
            Err(error) => Err(map_host_io(error)),
        }
    }

    #[cfg(target_os = "none")]
    pub fn set_nonblocking(&self, _nonblocking: bool) -> io::Result<()> {
        Ok(())
    }

    #[cfg(not(target_os = "none"))]
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking).map_err(map_host_io)
    }

    #[cfg(not(target_os = "none"))]
    pub fn local_addr_result(&self) -> io::Result<SocketAddr> {
        self.0.local_addr().map(to_compat_addr).map_err(map_host_io)
    }

    #[cfg(target_os = "none")]
    pub fn local_addr_result(&self) -> io::Result<SocketAddr> {
        self.local_addr()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "listener not bound"))
    }

    #[cfg(not(target_os = "none"))]
    pub fn unblock_accept(&self) {
        if let Ok(addr) = self.0.local_addr() {
            let _ = StdTcpStream::connect(addr);
        }
    }

    #[cfg(target_os = "none")]
    pub fn unblock_accept(&self) {}

    pub async fn accept_until_shutdown(
        &self,
        shutdown: &crate::compat::RwLock<bool>,
    ) -> io::Result<Option<(Arc<AsyncTcpStream>, SocketAddr)>> {
        loop {
            if *shutdown.read().await {
                return Ok(None);
            }

            let accepted = self.accept().await?;
            if *shutdown.read().await {
                return Ok(None);
            }
            return Ok(Some(accepted));
        }
    }

    #[cfg(target_os = "none")]
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.0.local_addr().map(from_bare_addr)
    }

    #[cfg(not(target_os = "none"))]
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.0.local_addr().ok().map(to_compat_addr)
    }
}

#[cfg(target_os = "none")]
pub struct AsyncTcpStream(edgerun_rt::TcpSocket);

#[cfg(not(target_os = "none"))]
pub struct AsyncTcpStream(StdTcpStream);

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
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        #[cfg(target_os = "none")]
        return Poll::Ready(
            self.0
                .recv(buf)
                .map_err(|e| io::Error::new(io::ErrorKind::WouldBlock, e.to_string())),
        );
        #[cfg(not(target_os = "none"))]
        {
            match self.0.read(buf) {
                Ok(value) => Poll::Ready(Ok(value)),
                Err(error) if error.kind() == host_std::io::ErrorKind::WouldBlock => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
                Err(error) => Poll::Ready(Err(map_host_io(error))),
            }
        }
    }
}

impl AsyncWrite for AsyncTcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        #[cfg(target_os = "none")]
        return Poll::Ready(
            self.0
                .send(
                    buf,
                    &edgerun_rt::IpStack::new(),
                    edgerun_rt::IpAddr::new(0, 0, 0, 0),
                    [0; 6],
                )
                .map_err(|e| io::Error::new(io::ErrorKind::WriteZero, e.to_string())),
        );
        #[cfg(not(target_os = "none"))]
        {
            match self.0.write(buf) {
                Ok(value) => Poll::Ready(Ok(value)),
                Err(error) if error.kind() == host_std::io::ErrorKind::WouldBlock => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
                Err(error) => Poll::Ready(Err(map_host_io(error))),
            }
        }
    }
}

#[cfg(not(target_os = "none"))]
fn to_compat_addr(addr: host_std::net::SocketAddr) -> SocketAddr {
    match addr {
        host_std::net::SocketAddr::V4(addr) => {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::from(addr.ip().octets())), addr.port())
        }
        host_std::net::SocketAddr::V6(addr) => SocketAddr::new(IpAddr::V6(*addr.ip()), addr.port()),
    }
}

pub fn to_bare_addr(addr: SocketAddr) -> edgerun_rt::SocketAddr {
    match addr {
        SocketAddr::V4(addr) => {
            edgerun_rt::SocketAddr::from_bytes4(addr.ip().octets(), addr.port())
        }
        SocketAddr::V6(addr) => edgerun_rt::SocketAddr::new(0, addr.port()),
    }
}

pub fn from_bare_addr(addr: edgerun_rt::SocketAddr) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::from(addr.ip_bytes())), addr.port())
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;

    #[test]
    fn udp_recv_idle_waits_for_host_timeout() {
        let socket = AsyncUdpSocket::bind("127.0.0.1:0").unwrap();
        let started = host_std::time::Instant::now();
        let result = edgerun_rt::block_on(async {
            let mut buf = [0u8; 64];
            socket.recv_from(&mut buf).await
        });

        let kind = result.unwrap_err().kind();
        assert!(
            matches!(kind, io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock),
            "unexpected idle UDP recv error kind: {kind:?}"
        );
        assert!(
            started.elapsed() >= host_std::time::Duration::from_millis(400),
            "idle UDP recv returned before the host socket timeout"
        );
    }
}
