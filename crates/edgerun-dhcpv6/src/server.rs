//! DHCPv6 UDP adapter.

use crate::std::io;
use crate::std::net::{SocketAddr, UdpSocket};
use crate::std::prelude::v1::String;
use crate::std::time::Duration;

use super::lease::LeasePool;
use super::message::DHCPV6_SERVER_PORT;
use super::server_core::Dhcpv6ServerCore;

pub use super::server_core::{Dhcpv6Datagram, Dhcpv6ServerConfig};

/// DHCPv6 server — UDP adapter around `Dhcpv6ServerCore`.
pub struct Dhcpv6Server {
    socket: UdpSocket,
    core: Dhcpv6ServerCore,
}

impl Dhcpv6Server {
    /// Create a new DHCPv6 server.
    pub fn new(config: Dhcpv6ServerConfig, pool: LeasePool) -> Result<Self, io::Error> {
        let socket = UdpSocket::bind(("::", DHCPV6_SERVER_PORT))?;
        socket.set_broadcast(false)?;
        socket.set_read_timeout(Some(Duration::from_millis(200)))?;

        Ok(Self {
            socket,
            core: Dhcpv6ServerCore::new(config, pool),
        })
    }

    /// Run the server event loop.
    pub fn run(&mut self) -> Result<(), io::Error> {
        edgerun_log::info!(
            "edgerun-dhcpv6: server listening on UDP {}",
            DHCPV6_SERVER_PORT
        );

        loop {
            match self.tick() {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) if e.kind() == io::ErrorKind::TimedOut => continue,
                Err(e) => {
                    edgerun_log::warn!("edgerun-dhcpv6: server error: {}", e);
                    return Err(e);
                }
            }
        }
    }

    /// Process one incoming packet.
    pub fn tick(&mut self) -> Result<(), io::Error> {
        let mut buf = [0u8; 1500];
        let (n, src) = self.socket.recv_from(&mut buf)?;

        let datagram = match self.core.handle_wire(&buf[..n]) {
            Ok(Some(datagram)) => datagram,
            Ok(None) => return Ok(()),
            Err(e) => {
                edgerun_log::warn!("edgerun-dhcpv6: failed to parse from {}: {}", src, e);
                return Ok(());
            }
        };

        let dest = SocketAddr::new(src.ip(), datagram.port);
        if let Err(e) = self.socket.send_to(&datagram.wire, dest) {
            edgerun_log::warn!("edgerun-dhcpv6: failed to send to {}: {}", dest, e);
        }

        Ok(())
    }

    /// Get pool statistics.
    pub fn stats(&self) -> String {
        self.core.stats()
    }

    pub fn core(&self) -> &Dhcpv6ServerCore {
        &self.core
    }

    pub fn core_mut(&mut self) -> &mut Dhcpv6ServerCore {
        &mut self.core
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::std::net::Ipv6Addr;

    #[test]
    fn test_server_config_default() {
        let config = Dhcpv6ServerConfig::default();
        assert!(!config.server_duid.is_empty());
        assert!(!config.dns_servers.is_empty());
    }

    #[test]
    fn test_server_with_pool() {
        let mut pool = LeasePool::new();
        pool.add_addresses([Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)]);

        let config = Dhcpv6ServerConfig::default();
        assert!(!config.server_duid.is_empty());
        assert_eq!(pool.available_addresses.len(), 1);
    }
}
