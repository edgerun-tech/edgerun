//! TFTP UDP adapter.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::compat::{self, AsyncUdpSocket};
use crate::protocol::{TftpPeerId, TftpReadCore, TftpReadProvider};
use crate::std::io;
use crate::std::net::SocketAddr;
use crate::std::sync::Mutex;
use crate::std::time::Duration;

// ---------------------------------------------------------------------------
// FileProvider trait
// ---------------------------------------------------------------------------

/// Trait for a TFTP file backend.
pub trait FileProvider: Send + Sync {
    /// Read a complete file. Existing simple providers can implement only this method.
    fn read_file(&self, path: &str) -> io::Result<Vec<u8>> {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("read_file not implemented for {path}"),
        ))
    }

    /// Get the total size of a file (for tsize option).
    fn file_size(&self, filename: &str) -> Option<u64> {
        self.read_file(filename).ok().map(|data| data.len() as u64)
    }

    /// Read a block of data from a file at the given offset.
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

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

/// TFTP server configuration.
pub struct TftpServerConfig {
    /// Bind address (default "[::]:69" for IPv6 dual-stack, or "0.0.0.0:69" for IPv4-only).
    pub bind_addr: String,
    /// Default block size if client doesn't negotiate (default 512).
    pub default_blksize: u16,
    /// Transfer timeout in seconds (default 5).
    pub timeout_secs: u8,
}

impl Default for TftpServerConfig {
    fn default() -> Self {
        Self {
            // Use IPv6 dual-stack by default to support both IPv4 and IPv6 clients
            bind_addr: "[::]:69".to_string(),
            default_blksize: 512,
            timeout_secs: 5,
        }
    }
}

/// TFTP server — serves files via UDP using the transport-free protocol core.
pub struct TftpServer {
    socket: Arc<AsyncUdpSocket>,
    core: Mutex<TftpReadCore<Box<dyn FileProvider>>>,
    _timeout: Duration,
}

impl TftpServer {
    /// Create a new TFTP server.
    pub fn new(
        config: TftpServerConfig,
        provider: impl FileProvider + 'static,
    ) -> Result<Self, io::Error> {
        let socket = Arc::new(AsyncUdpSocket::bind(&config.bind_addr)?);
        let timeout = Duration::from_secs(config.timeout_secs as u64);

        Ok(Self {
            socket,
            core: Mutex::new(TftpReadCore::new(Box::new(provider))),
            _timeout: timeout,
        })
    }

    /// Run the server event loop until shutdown is requested.
    pub async fn run(&self, shutdown: compat::CancellationToken) {
        edgerun_log::info!(
            "edgerun-tftp: server listening on {}",
            self.socket.local_addr().unwrap()
        );

        while !shutdown.is_cancelled() {
            match self.tick().await {
                Ok(()) => {}
                Err(e) => {
                    let _ = &e;
                    edgerun_log::warn!("edgerun-tftp: server error: {}", e);
                    compat::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        edgerun_log::info!("edgerun-tftp: server shut down");
    }

    /// Process one incoming packet.
    pub async fn tick(&self) -> Result<(), io::Error> {
        let mut buf = [0u8; 65536]; // Max UDP
        let (n, src) = self.socket.recv_from(&mut buf).await?;
        let peer = peer_id(src);
        let replies = match self.core.lock().await.handle_wire(peer, &buf[..n]) {
            Ok(replies) => replies,
            Err(e) => {
                let _ = &e;
                edgerun_log::warn!("edgerun-tftp: parse error from {}: {}", src, e);
                return Ok(());
            }
        };

        for reply in replies {
            let _ = self.socket.send_to(&reply.wire, src).await;
        }
        Ok(())
    }

    /// Get the number of active transfers.
    pub async fn active_transfers(&self) -> usize {
        self.core.lock().await.active_transfers()
    }
}

fn peer_id(addr: SocketAddr) -> TftpPeerId {
    TftpPeerId(addr.to_string().into_bytes())
}

// Blanket impl: Arc<dyn FileProvider> is itself a FileProvider
impl FileProvider for Arc<dyn FileProvider> {
    fn read_file(&self, path: &str) -> io::Result<Vec<u8>> {
        (**self).read_file(path)
    }

    fn file_size(&self, filename: &str) -> Option<u64> {
        (**self).file_size(filename)
    }

    fn read_block(&self, filename: &str, offset: usize, max_size: usize) -> Option<Vec<u8>> {
        (**self).read_block(filename, offset, max_size)
    }
}

// Blanket impl: Box<dyn FileProvider> is itself a FileProvider
impl FileProvider for Box<dyn FileProvider> {
    fn read_file(&self, path: &str) -> io::Result<Vec<u8>> {
        (**self).read_file(path)
    }

    fn file_size(&self, filename: &str) -> Option<u64> {
        (**self).file_size(filename)
    }

    fn read_block(&self, filename: &str, offset: usize, max_size: usize) -> Option<Vec<u8>> {
        (**self).read_block(filename, offset, max_size)
    }
}
