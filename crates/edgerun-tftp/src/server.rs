//! TFTP server — RFC 1350 with RFC 2347/2348 option negotiation.

use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::Duration;

use edgerun_rt::{AsyncUdpSocket, Mutex};

use super::message::{TftpError, TftpMessage, TftpOptions};

// ---------------------------------------------------------------------------
// FileProvider trait
// ---------------------------------------------------------------------------

/// Trait for a TFTP file backend.
pub trait FileProvider: Send + Sync {
    /// Get the total size of a file (for tsize option).
    fn file_size(&self, filename: &str) -> Option<u64>;
    /// Read a block of data from a file at the given offset.
    fn read_block(&self, filename: &str, offset: usize, max_size: usize) -> Option<Vec<u8>>;
}

// ---------------------------------------------------------------------------
// Transfer state machine
// ---------------------------------------------------------------------------

/// State of an active TFTP transfer.
struct TftpTransfer {
    filename: String,
    client_addr: SocketAddr,
    blksize: u16,
    total_size: u64,
    current_block: u16,
    offset: usize,
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

/// TFTP server — serves files via the TFTP protocol.
///
/// Now fully async using `edgerun_rt::AsyncUdpSocket`.
pub struct TftpServer {
    socket: Arc<AsyncUdpSocket>,
    provider: Box<dyn FileProvider>,
    timeout: Duration,
    /// Active transfers, keyed by (client_addr, filename).
    transfers: Mutex<HashMap<(SocketAddr, String), TftpTransfer>>,
    /// Next ephemeral port for new transfers.
    next_port: Mutex<u16>,
}

impl TftpServer {
    /// Create a new TFTP server.
    pub fn new(config: TftpServerConfig, provider: impl FileProvider + 'static) -> Result<Self, io::Error> {
        let std_socket = UdpSocket::bind(&config.bind_addr)?;
        let socket = Arc::new(AsyncUdpSocket::from_std(std_socket)?);

        let timeout = Duration::from_secs(config.timeout_secs as u64);

        Ok(Self {
            socket,
            provider: Box::new(provider),
            timeout,
            transfers: Mutex::new(HashMap::new()),
            next_port: Mutex::new(10000),
        })
    }

    /// Run the server event loop until shutdown is requested.
    pub async fn run(&self, shutdown: edgerun_rt::CancellationToken) {
        edgerun_log::info!(
            "edgerun-tftp: server listening on {}",
            self.socket.local_addr().unwrap()
        );

        while !shutdown.is_cancelled() {
            match self.tick().await {
                Ok(()) => {}
                Err(e) => {
                    edgerun_log::warn!("edgerun-tftp: server error: {}", e);
                    edgerun_rt::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        edgerun_log::info!("edgerun-tftp: server shut down");
    }

    /// Process one incoming packet.
    pub async fn tick(&self) -> Result<(), io::Error> {
        let mut buf = [0u8; 65536]; // Max UDP
        let (n, src) = match self.socket.recv_from(&mut buf).await {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        let msg = match TftpMessage::from_wire(&buf[..n]) {
            Ok(m) => m,
            Err(e) => {
                edgerun_log::warn!("edgerun-tftp: parse error from {}: {}", src, e);
                return Ok(());
            }
        };

        match msg {
            TftpMessage::RRQ { filename, mode, options, .. } => {
                if mode.to_lowercase() != "octet" {
                    let err = TftpMessage::error(
                        TftpError::IllegalOperation,
                        "Only octet mode is supported",
                    );
                    let _ = self.socket.send_to(&err.to_wire(), src).await;
                    return Ok(());
                }
                self.handle_rrq(src, filename, options).await?;
            }

            TftpMessage::WRQ { filename, .. } => {
                let err = TftpMessage::error(
                    TftpError::AccessViolation,
                    &format!("Write access denied: {}", filename),
                );
                let _ = self.socket.send_to(&err.to_wire(), src).await;
            }

            TftpMessage::ACK { block } => {
                self.handle_ack(src, block).await?;
            }

            TftpMessage::DATA { .. } | TftpMessage::OACK { .. } => {
                let err = TftpMessage::error(
                    TftpError::IllegalOperation,
                    "Unexpected message",
                );
                let _ = self.socket.send_to(&err.to_wire(), src).await;
            }

            TftpMessage::ERROR { code, message } => {
                edgerun_log::warn!("edgerun-tftp: ERROR from {}: {:?} - {}", src, code, message);
                self.transfers.lock().await.retain(|(addr, _), _| *addr != src);
            }
        }

        Ok(())
    }

    // --- Transfer handling ---

    async fn handle_rrq(
        &self,
        client_addr: SocketAddr,
        filename: String,
        client_options: TftpOptions,
    ) -> Result<(), io::Error> {
        edgerun_log::info!(
            "edgerun-tftp: RRQ '{}' from {} (blksize={})",
            filename,
            client_addr,
            client_options.blksize
        );

        // Check if file exists
        let total_size = match self.provider.file_size(&filename) {
            Some(size) => size,
            None => {
                edgerun_log::warn!("edgerun-tftp: file not found: {}", filename);
                let err = TftpMessage::error(
                    TftpError::FileNotFound,
                    &format!("File not found: {}", filename),
                );
                let _ = self.socket.send_to(&err.to_wire(), client_addr).await;
                return Ok(());
            }
        };

        // Negotiate options
        let negotiated = TftpOptions {
            blksize: client_options.blksize,
            tsize: Some(total_size),
            timeout: client_options.timeout,
        };

        let _blksize = negotiated.blksize as usize;

        // Create transfer state
        let key = (client_addr, filename.clone());
        self.transfers.lock().await.insert(
            key,
            TftpTransfer {
                filename,
                client_addr,
                blksize: negotiated.blksize,
                total_size,
                current_block: 0,
                offset: 0,
            },
        );

        // If client requested options, send OACK first
        if client_options.blksize != super::message::DEFAULT_BLKSIZE
            || client_options.tsize == Some(0)
            || client_options.timeout != super::message::DEFAULT_TIMEOUT
        {
            let oack = TftpMessage::OACK {
                options: negotiated.clone(),
            };
            let wire = oack.to_wire();
            let _ = self.socket.send_to(&wire, client_addr).await;
            edgerun_log::info!(
                "edgerun-tftp: OACK sent to {} (blksize={}, tsize={})",
                client_addr, negotiated.blksize, total_size
            );
        } else {
            // No options to negotiate, start sending data
            self.send_next_block(&client_addr).await?;
        }

        Ok(())
    }

    async fn handle_ack(&self, client_addr: SocketAddr, block: u16) -> Result<(), io::Error> {
        let transfer_info = {
            let transfers = self.transfers.lock().await;
            transfers
                .iter()
                .find(|(k, _)| k.0 == client_addr)
                .map(|(key, t)| {
                    (key.clone(), t.current_block, t.offset, t.total_size, t.blksize)
                })
        };

        if let Some((key, current_block, offset, total_size, blksize)) = transfer_info {
            if block == current_block || (block == 0 && current_block == 0) {
                if offset >= total_size as usize {
                    edgerun_log::info!(
                        "edgerun-tftp: transfer complete to {} ({} bytes)",
                        client_addr,
                        offset
                    );
                    self.transfers.lock().await.remove(&key);
                } else {
                    self.send_next_block_data(&client_addr, blksize).await?;
                }
            }
        }

        Ok(())
    }

    async fn send_next_block(&self, client_addr: &SocketAddr) -> Result<(), io::Error> {
        // Find the transfer for this client and extract the info we need
        let transfer_info = {
            let transfers = self.transfers.lock().await;
            transfers
                .iter()
                .find(|(addr, _)| addr.0 == *client_addr)
                .map(|(key, t)| {
                    (key.clone(), t.filename.clone(), t.offset, t.blksize as usize, t.current_block)
                })
        };

        if let Some((key, filename, offset, blksize, current_block)) = transfer_info {
            // Read the next block
            match self.provider.read_block(&filename, offset, blksize) {
                Some(data) => {
                    let is_last = data.len() < blksize;
                    let new_block = current_block.wrapping_add(1);
                    let data_len = data.len();

                    let data_msg = TftpMessage::data(new_block, data);
                    let wire = data_msg.to_wire();
                    let _ = self.socket.send_to(&wire, *client_addr).await;

                    // Update transfer state
                    let mut transfers = self.transfers.lock().await;
                    if let Some(t) = transfers.get_mut(&key) {
                        t.current_block = new_block;
                        t.offset += data_len;
                    }

                    if is_last {
                        edgerun_log::info!(
                            "edgerun-tftp: final block {} to {}",
                            new_block,
                            client_addr
                        );
                    }
                }
                None => {
                    let err = TftpMessage::error(
                        TftpError::NotDefined,
                        "Failed to read file block",
                    );
                    let _ = self.socket.send_to(&err.to_wire(), *client_addr).await;
                    self.transfers.lock().await.remove(&key);
                }
            }
        }

        Ok(())
    }

    /// Internal: send next block given we already know the transfer details.
    async fn send_next_block_data(&self, client_addr: &SocketAddr, _blksize: u16) -> Result<(), io::Error> {
        self.send_next_block(client_addr).await
    }

    /// Get the number of active transfers.
    pub async fn active_transfers(&self) -> usize {
        self.transfers.lock().await.len()
    }
}

// Blanket impl: Arc<dyn FileProvider> is itself a FileProvider
impl FileProvider for std::sync::Arc<dyn FileProvider> {
    fn file_size(&self, filename: &str) -> Option<u64> {
        (**self).file_size(filename)
    }
    fn read_block(&self, filename: &str, offset: usize, max_size: usize) -> Option<Vec<u8>> {
        (**self).read_block(filename, offset, max_size)
    }
}

// Blanket impl: Box<dyn FileProvider> is itself a FileProvider
impl FileProvider for Box<dyn FileProvider> {
    fn file_size(&self, filename: &str) -> Option<u64> {
        (**self).file_size(filename)
    }
    fn read_block(&self, filename: &str, offset: usize, max_size: usize) -> Option<Vec<u8>> {
        (**self).read_block(filename, offset, max_size)
    }
}
