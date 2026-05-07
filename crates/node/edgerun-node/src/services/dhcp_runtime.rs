use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::net::Ipv4Addr;
use core::time::Duration;

#[cfg(not(target_os = "none"))]
use std::net::{Ipv4Addr as HostIpv4Addr, SocketAddrV4, UdpSocket as HostUdpSocket};

use edgerun_protocols::dhcp::{message::io, DhcpServerConfig, DhcpServerCore, DHCP_SERVER_PORT};
use crate::rt::{sleep, CancellationToken, Mutex, SocketAddr, UdpSocket};

pub struct DhcpServer {
    socket: Arc<UdpSocket>,
    #[cfg(not(target_os = "none"))]
    host_socket: Arc<HostUdpSocket>,
    core: Mutex<DhcpServerCore>,
    interface: Option<String>,
}

impl DhcpServer {
    pub fn new(
        config: DhcpServerConfig,
        pool_start: Ipv4Addr,
        pool_end: Ipv4Addr,
    ) -> Result<Self, io::Error> {
        Self::new_bound(
            config,
            pool_start,
            pool_end,
            SocketAddr::new(0, DHCP_SERVER_PORT),
        )
    }

    pub fn new_bound(
        config: DhcpServerConfig,
        pool_start: Ipv4Addr,
        pool_end: Ipv4Addr,
        bind_addr: SocketAddr,
    ) -> Result<Self, io::Error> {
        let mut socket = UdpSocket::new();
        socket.bind(bind_addr).map_err(map_udp_error)?;
        socket.set_nonblocking(true);

        #[cfg(not(target_os = "none"))]
        let host_socket = {
            let ip = bind_addr.ip_bytes();
            let addr = SocketAddrV4::new(
                HostIpv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
                bind_addr.port(),
            );
            let socket = HostUdpSocket::bind(addr)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            socket
                .set_nonblocking(true)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            Arc::new(socket)
        };

        Ok(Self {
            socket: Arc::new(socket),
            #[cfg(not(target_os = "none"))]
            host_socket,
            core: Mutex::new(DhcpServerCore::new(config, pool_start, pool_end)),
            interface: None,
        })
    }

    pub fn set_interface(&mut self, iface: String) {
        self.interface = Some(iface);
    }

    pub async fn run(&self, shutdown: CancellationToken) -> Result<(), io::Error> {
        while !shutdown.is_cancelled() {
            match self.tick().await {
                Ok(()) => {}
                Err(error) if is_not_ready(&error) => sleep(Duration::from_millis(100)).await,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub async fn tick(&self) -> Result<(), io::Error> {
        #[cfg(not(target_os = "none"))]
        {
            let mut buf = [0u8; 1500];
            let (n, _src) = self
                .host_socket
                .recv_from(&mut buf)
                .map_err(map_host_error)?;
            let datagram = match self.core.lock().handle_wire(&buf[..n]) {
                Ok(Some(datagram)) => datagram,
                Ok(None) | Err(_) => return Ok(()),
            };
            if datagram.wire.is_empty() {
                return Ok(());
            }

            let dest = SocketAddrV4::new(
                HostIpv4Addr::new(
                    datagram.dest.octets()[0],
                    datagram.dest.octets()[1],
                    datagram.dest.octets()[2],
                    datagram.dest.octets()[3],
                ),
                datagram.port,
            );
            self.host_socket
                .send_to(&datagram.wire, dest)
                .map(|_| ())
                .map_err(map_host_error)
        }

        #[cfg(target_os = "none")]
        {
            let mut buf = [0u8; 1500];
            let (n, _src) = self.socket.recv_from(&mut buf).map_err(map_udp_error)?;
            let datagram = match self.core.lock().handle_wire(&buf[..n]) {
                Ok(Some(datagram)) => datagram,
                Ok(None) | Err(_) => return Ok(()),
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

fn map_udp_error(_: crate::rt::UdpError) -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "UDP operation not ready")
}

fn is_not_ready(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

#[cfg(not(target_os = "none"))]
fn map_host_error(error: std::io::Error) -> io::Error {
    let kind = match error.kind() {
        std::io::ErrorKind::WouldBlock => io::ErrorKind::WouldBlock,
        std::io::ErrorKind::TimedOut => io::ErrorKind::TimedOut,
        std::io::ErrorKind::ConnectionRefused => io::ErrorKind::ConnectionRefused,
        _ => io::ErrorKind::Other,
    };
    io::Error::new(kind, error.to_string())
}
