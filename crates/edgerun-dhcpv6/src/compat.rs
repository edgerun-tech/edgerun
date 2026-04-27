//! edgerun-bare-rt adapters for DHCPv6 networking.

use crate::std::net::{IpAddr, Ipv4Addr, SocketAddr};

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
