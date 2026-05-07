use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use core::time::Duration;

use edgerun_protocols::tftp::{TftpPeerId, TftpReadCore, TftpReadProvider};
use edgerun_rt::UdpSocket;

/// Trait for a TFTP file backend.
pub trait FileProvider: Send + Sync {
    fn read_file(&self, path: &str) -> edgerun_rt::io::Result<Vec<u8>> {
        let _ = path;
        Err(edgerun_rt::io::IoError::Other("read_file not implemented"))
    }

    fn file_size(&self, filename: &str) -> Option<u64> {
        self.read_file(filename).ok().map(|data| data.len() as u64)
    }

    fn read_block(&self, filename: &str, offset: usize, max_size: usize) -> Option<Vec<u8>> {
        let data = self.read_file(filename).ok()?;
        if offset > data.len() {
            return None;
        }
        let end = offset.saturating_add(max_size).min(data.len());
        Some(data[offset..end].to_vec())
    }
}

impl TftpReadProvider for Box<dyn FileProvider> {
    fn file_size(&self, filename: &str) -> Option<u64> {
        (**self).file_size(filename)
    }

    fn read_block(&self, filename: &str, offset: usize, max_size: usize) -> Option<Vec<u8>> {
        (**self).read_block(filename, offset, max_size)
    }
}

pub struct TftpServerConfig {
    pub bind_addr: String,
    pub default_blksize: u16,
    pub timeout_secs: u8,
}

impl Default for TftpServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "[::]:69".to_string(),
            default_blksize: 512,
            timeout_secs: 5,
        }
    }
}

pub struct TftpServer {
    socket: Arc<UdpSocket>,
    core: edgerun_rt::Mutex<TftpReadCore<Box<dyn FileProvider>>>,
    _timeout: Duration,
}

impl TftpServer {
    pub fn new(
        config: TftpServerConfig,
        provider: impl FileProvider + 'static,
    ) -> edgerun_rt::io::Result<Self> {
        let addr = parse_socket_addr(&config.bind_addr)?;
        let mut socket = UdpSocket::new();
        socket
            .bind(to_rt_addr(addr))
            .map_err(|_| edgerun_rt::io::IoError::Other("TFTP bind failed"))?;
        Ok(Self {
            socket: Arc::new(socket),
            core: edgerun_rt::Mutex::new(TftpReadCore::new(Box::new(provider))),
            _timeout: Duration::from_secs(config.timeout_secs as u64),
        })
    }

    pub async fn run(&self, shutdown: edgerun_rt::CancellationToken) {
        while !shutdown.is_cancelled() {
            if self.tick().await.is_err() {
                edgerun_rt::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    pub async fn tick(&self) -> edgerun_rt::io::Result<()> {
        let mut buf = [0u8; 65536];
        let (n, src) = self
            .socket
            .recv_from(&mut buf)
            .map_err(|_| edgerun_rt::io::IoError::Other("TFTP receive failed"))?;
        let peer = peer_id(src);
        let replies = match self.core.lock().handle_wire(peer, &buf[..n]) {
            Ok(replies) => replies,
            Err(_) => return Ok(()),
        };

        for reply in replies {
            let _ = self.socket.send_to(&reply.wire, src);
        }
        Ok(())
    }

    pub async fn active_transfers(&self) -> usize {
        self.core.lock().active_transfers()
    }
}

fn peer_id(addr: edgerun_rt::SocketAddr) -> TftpPeerId {
    TftpPeerId(format!("{}:{}", format_ip(addr.ip_bytes()), addr.port()).into_bytes())
}

impl FileProvider for Arc<dyn FileProvider> {
    fn read_file(&self, path: &str) -> edgerun_rt::io::Result<Vec<u8>> {
        (**self).read_file(path)
    }

    fn file_size(&self, filename: &str) -> Option<u64> {
        (**self).file_size(filename)
    }

    fn read_block(&self, filename: &str, offset: usize, max_size: usize) -> Option<Vec<u8>> {
        (**self).read_block(filename, offset, max_size)
    }
}

impl FileProvider for Box<dyn FileProvider> {
    fn read_file(&self, path: &str) -> edgerun_rt::io::Result<Vec<u8>> {
        (**self).read_file(path)
    }

    fn file_size(&self, filename: &str) -> Option<u64> {
        (**self).file_size(filename)
    }

    fn read_block(&self, filename: &str, offset: usize, max_size: usize) -> Option<Vec<u8>> {
        (**self).read_block(filename, offset, max_size)
    }
}

fn parse_socket_addr(addr: &str) -> edgerun_rt::io::Result<SocketAddr> {
    addr.parse()
        .map_err(|_| edgerun_rt::io::IoError::Other("invalid TFTP bind address"))
}

fn to_rt_addr(addr: SocketAddr) -> edgerun_rt::SocketAddr {
    match addr {
        SocketAddr::V4(addr) => {
            edgerun_rt::SocketAddr::from_bytes4(addr.ip().octets(), addr.port())
        }
        SocketAddr::V6(addr) => edgerun_rt::SocketAddr::new(0, addr.port()),
    }
}

fn format_ip(bytes: [u8; 4]) -> String {
    IpAddr::V4(Ipv4Addr::from(bytes)).to_string()
}
