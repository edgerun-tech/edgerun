//! UDP socket for bare-metal networking.
//!
//! These are low-level node/runtime primitives. Application code requests
//! protocol/resource access through `edgerun-node`; the node decides whether a
//! UDP binding becomes a native socket, mesh/browser route, or no resource.

use core::sync::atomic::{AtomicU16, Ordering};

use crate::rt::bare_async_net::{bare_udp_recv_from, bare_udp_send_to};

static NEXT_UDP_PORT: AtomicU16 = AtomicU16::new(49152);

#[derive(Debug, Clone, Copy, Default)]
pub struct SocketAddr(pub u32, pub u16);

impl SocketAddr {
    pub const fn new(ip: u32, port: u16) -> Self {
        Self(ip, port)
    }

    pub const fn from_array(addr: [u8; 4], port: u16) -> Self {
        Self(
            (addr[0] as u32) << 24
                | (addr[1] as u32) << 16
                | (addr[2] as u32) << 8
                | (addr[3] as u32),
            port,
        )
    }

    pub fn from_bytes4(ip: [u8; 4], port: u16) -> Self {
        Self(
            ((ip[0] as u32) << 24)
                | ((ip[1] as u32) << 16)
                | ((ip[2] as u32) << 8)
                | (ip[3] as u32),
            port,
        )
    }

    pub fn ip_bytes(&self) -> [u8; 4] {
        [
            (self.0 >> 24) as u8,
            (self.0 >> 16) as u8,
            (self.0 >> 8) as u8,
            self.0 as u8,
        ]
    }

    pub fn port(&self) -> u16 {
        self.1
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

pub struct UdpSocket {
    local: SocketAddr,
    remote: SocketAddr,
    bound: bool,
}

impl UdpSocket {
    pub fn new() -> Self {
        Self {
            local: SocketAddr::default(),
            remote: SocketAddr::default(),
            bound: false,
        }
    }

    pub fn bind(&mut self, addr: SocketAddr) -> Result<(), UdpError> {
        let port = if addr.port() == 0 {
            allocate_udp_port()
        } else {
            addr.port()
        };
        self.local = SocketAddr::new(addr.as_u32(), port);
        self.bound = true;
        Ok(())
    }

    pub fn connect(&mut self, addr: SocketAddr) -> Result<(), UdpError> {
        self.remote = addr;
        Ok(())
    }

    pub fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize, UdpError> {
        let local = self
            .local_addr()
            .unwrap_or_else(|| SocketAddr::new(0, allocate_udp_port()));
        bare_udp_send_to(
            local.ip_bytes(),
            local.port(),
            addr.ip_bytes(),
            addr.port(),
            buf,
        )
        .map_err(|_| UdpError)
    }

    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), UdpError> {
        let local = self.local_addr().ok_or(UdpError)?;
        let (len, src_ip, src_port) =
            bare_udp_recv_from(local.ip_bytes(), local.port(), buf).map_err(|_| UdpError)?;
        Ok((len, SocketAddr::from_array(src_ip, src_port)))
    }

    pub fn local_addr(&self) -> Option<SocketAddr> {
        if self.bound { Some(self.local) } else { None }
    }

    pub fn remote_addr(&self) -> Option<SocketAddr> {
        if self.remote.1 != 0 {
            Some(self.remote)
        } else {
            None
        }
    }

    pub fn set_nonblocking(&self, _nonblocking: bool) {}
}

fn allocate_udp_port() -> u16 {
    let port = NEXT_UDP_PORT.fetch_add(1, Ordering::AcqRel);
    if port < 49152 {
        NEXT_UDP_PORT.store(49153, Ordering::Release);
        49152
    } else {
        port
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpError;

impl core::fmt::Display for UdpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "UDP error")
    }
}
