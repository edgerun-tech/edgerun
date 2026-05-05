//! edgerun-rt adapters for the async TFTP server API.

use alloc::string::ToString;

use crate::std::io;
use crate::std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub use edgerun_rt::{CancellationToken, Duration, sleep};

pub struct AsyncUdpSocket(edgerun_rt::UdpSocket);

impl AsyncUdpSocket {
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
