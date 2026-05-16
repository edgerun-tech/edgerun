use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::net::Ipv4Addr;
use core::time::Duration;

use crate::network::{BareFrameTransport, HostSocketTransport, TransportAddress};
use crate::rt::{AsyncUdpSocket, CancellationToken, Mutex, SocketAddr, UdpSocket, sleep};
use edgerun_protocols::dhcp::{DHCP_SERVER_PORT, DhcpServerConfig, DhcpServerCore, message::io};

pub struct DhcpServer {
    socket: Arc<UdpSocket>,
    #[cfg(not(target_os = "none"))]
    host_socket: Arc<AsyncUdpSocket>,
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
        let socket = BareFrameTransport
            .bind_datagram_now(&TransportAddress::bare_datagram(rt_addr_endpoint(
                bind_addr,
            )))
            .map_err(|_| map_udp_error(crate::rt::UdpError))?;
        socket.set_nonblocking(true);

        #[cfg(not(target_os = "none"))]
        let host_socket = {
            let ip = bind_addr.ip_bytes();
            let endpoint = format!(
                "{}.{}.{}.{}:{}",
                ip[0],
                ip[1],
                ip[2],
                ip[3],
                bind_addr.port()
            );
            let socket = HostSocketTransport
                .bind_datagram_now(&TransportAddress::host_datagram(endpoint.into_bytes()))
                .map_err(map_transport_error)?;
            socket.set_broadcast(true).map_err(map_rt_io_error)?;
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
                .await
                .map_err(map_rt_io_error)?;
            let datagram = match self.core.lock().handle_wire(&buf[..n]) {
                Ok(Some(datagram)) => datagram,
                Ok(None) | Err(_) => return Ok(()),
            };
            if datagram.wire.is_empty() {
                return Ok(());
            }

            let dest = core::net::SocketAddr::from((datagram.dest.octets(), datagram.port));
            self.host_socket
                .send_to(&datagram.wire, dest)
                .await
                .map(|_| ())
                .map_err(map_rt_io_error)
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

fn rt_addr_endpoint(addr: SocketAddr) -> Vec<u8> {
    let ip = addr.ip_bytes();
    format!("{}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], addr.port()).into_bytes()
}

fn map_transport_error(error: crate::network::TransportError) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error.to_string())
}

fn map_rt_io_error(error: crate::rt::IoError) -> io::Error {
    let kind = match error {
        crate::rt::IoError::UnexpectedEof | crate::rt::IoError::WriteZero => io::ErrorKind::Other,
        crate::rt::IoError::Other(_) => io::ErrorKind::Other,
    };
    io::Error::new(kind, error.to_string())
}

fn is_not_ready(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}
