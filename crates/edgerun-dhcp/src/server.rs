//! DHCPv4 UDP adapter.

use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use core::net::Ipv4Addr;
use core::time::Duration;

use edgerun_rt::{CancellationToken, Mutex, SocketAddr, UdpSocket, sleep};

use super::message::{DHCP_SERVER_PORT, io};
use super::server_core::DhcpServerCore;

pub use super::server_core::DhcpServerConfig;

pub struct DhcpServer {
    socket: Arc<UdpSocket>,
    core: Mutex<DhcpServerCore>,
    interface: Option<String>,
}

impl DhcpServer {
    pub fn new(
        config: DhcpServerConfig,
        pool_start: Ipv4Addr,
        pool_end: Ipv4Addr,
    ) -> Result<Self, io::Error> {
        let mut socket = UdpSocket::new();
        socket
            .bind(SocketAddr::new(0, DHCP_SERVER_PORT))
            .map_err(map_udp_error)?;
        socket.set_nonblocking(true);

        Ok(Self {
            socket: Arc::new(socket),
            core: Mutex::new(DhcpServerCore::new(config, pool_start, pool_end)),
            interface: None,
        })
    }

    pub fn set_interface(&mut self, iface: String) {
        self.interface = Some(iface);
    }

    pub async fn run(&self, shutdown: CancellationToken) {
        while !shutdown.is_cancelled() {
            if self.tick().await.is_err() {
                sleep(Duration::from_millis(100)).await;
            }
        }
    }

    pub async fn tick(&self) -> Result<(), io::Error> {
        let mut buf = [0u8; 1500];
        let (n, _src) = self.socket.recv_from(&mut buf).map_err(map_udp_error)?;
        let datagram = match self.core.lock().handle_wire(&buf[..n]) {
            Ok(Some(datagram)) => datagram,
            Ok(None) => return Ok(()),
            Err(_) => return Ok(()),
        };
        if datagram.wire.is_empty() {
            return Ok(());
        }

        let addr = SocketAddr::new(u32::from_be_bytes(datagram.dest.octets()), datagram.port);
        self.socket
            .send_to(&datagram.wire, addr)
            .map(|_| ())
            .map_err(map_udp_error)
    }

    pub async fn stats(&self) -> String {
        let core = self.core.lock();
        format!(
            "active={}, available={}, pool_size={}",
            core.pool.active_count(),
            core.pool.available_count(),
            core.pool.pool_size()
        )
    }
}

fn map_udp_error(_: edgerun_rt::UdpError) -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "UDP operation not ready")
}
