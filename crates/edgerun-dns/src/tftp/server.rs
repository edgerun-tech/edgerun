//! TFTP server — RFC 1350 with RFC 2347/2348 option negotiation.

use alloc::{boxed::Box, format, string::{String, ToString}, vec, vec::Vec};
use alloc::collections::BTreeMap as HashMap;
use crate::std::io;
use crate::std::net::{SocketAddr, UdpSocket};
use crate::std::time::Duration;

use super::message::{TftpError, TftpMessage, TftpOptions};

// ---------------------------------------------------------------------------
// FileProvider trait
// ---------------------------------------------------------------------------

/// Trait for a TFTP file backend.
///
/// Implement this to serve TFTP files from any source:
/// - Filesystem
/// - Encrypted blob store
/// - In-memory cache
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
    /// Bind address (default "0.0.0.0:69").
    pub bind_addr: String,
    /// Default block size if client doesn't negotiate (default 512).
    pub default_blksize: u16,
    /// Transfer timeout in seconds (default 5).
    pub timeout_secs: u8,
}

impl Default for TftpServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:69".to_string(),
            default_blksize: 512,
            timeout_secs: 5,
        }
    }
}

/// TFTP server — serves files via the TFTP protocol.
///
/// # Example
/// ```no_run
/// use edgerun_dns::tftp::server::{TftpServer, FileProvider};
///
/// // Implement FileProvider for your backend, then:
/// // let provider = MyProvider;
/// // let config = TftpServerConfig::default();
/// // let mut server = TftpServer::new(config, provider).unwrap();
/// // server.run();
/// ```
pub struct TftpServer {
    socket: UdpSocket,
    provider: Box<dyn FileProvider>,
    timeout: Duration,
    /// Active transfers, keyed by (client_addr, filename).
    transfers: HashMap<(SocketAddr, String), TftpTransfer>,
    /// Next ephemeral port for new transfers.
    next_port: u16,
}

impl TftpServer {
    /// Create a new TFTP server.
    pub fn new(
        config: TftpServerConfig,
        provider: impl FileProvider + 'static,
    ) -> Result<Self, io::Error> {
        let socket = UdpSocket::bind(&config.bind_addr)?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;

        let timeout = Duration::from_secs(config.timeout_secs as u64);

        Ok(Self {
            socket,
            provider: Box::new(provider),
            timeout,
            transfers: HashMap::new(),
            next_port: 10000,
        })
    }

    /// Run the server event loop (blocking).
    pub fn run(&mut self) -> Result<(), io::Error> {
        eprintln!(
            "edgerun-tftp: server listening on {}",
            self.socket.local_addr().unwrap()
        );

        loop {
            match self.tick() {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) if e.kind() == io::ErrorKind::TimedOut => continue,
                Err(e) => {
                    eprintln!("edgerun-tftp: server error: {}", e);
                    return Err(e);
                }
            }
        }
    }

    /// Process one incoming packet. Call from your own event loop.
    pub fn tick(&mut self) -> Result<(), io::Error> {
        let mut buf = [0u8; 65536]; // Max UDP
        let (n, src) = self.socket.recv_from(&mut buf)?;

        let msg = match TftpMessage::from_wire(&buf[..n]) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("edgerun-tftp: parse error from {}: {}", src, e);
                return Ok(());
            }
        };

        match msg {
            TftpMessage::RRQ {
                filename,
                mode,
                options,
                ..
            } => {
                if mode.to_lowercase() != "octet" {
                    let err = TftpMessage::error(
                        TftpError::IllegalOperation,
                        "Only octet mode is supported",
                    );
                    let _ = self.socket.send_to(&err.to_wire(), src);
                    return Ok(());
                }
                self.handle_rrq(src, filename, options)?;
            }

            TftpMessage::WRQ { filename, .. } => {
                // TFTP writes not supported
                let err = TftpMessage::error(
                    TftpError::AccessViolation,
                    &format!("Write access denied: {}", filename),
                );
                let _ = self.socket.send_to(&err.to_wire(), src);
            }

            TftpMessage::ACK { block } => {
                self.handle_ack(src, block)?;
            }

            TftpMessage::DATA { .. } | TftpMessage::OACK { .. } => {
                // Client shouldn't send these unsolicited
                let err = TftpMessage::error(TftpError::IllegalOperation, "Unexpected message");
                let _ = self.socket.send_to(&err.to_wire(), src);
            }

            TftpMessage::ERROR { code, message } => {
                eprintln!("edgerun-tftp: ERROR from {}: {:?} - {}", src, code, message);
                // Clean up transfer
                self.transfers.retain(|(addr, _), _| *addr != src);
            }
        }

        Ok(())
    }

    // --- Transfer handling ---

    fn handle_rrq(
        &mut self,
        client_addr: SocketAddr,
        filename: String,
        client_options: TftpOptions,
    ) -> Result<(), io::Error> {
        eprintln!(
            "edgerun-tftp: RRQ '{}' from {} (blksize={})",
            filename, client_addr, client_options.blksize
        );

        // Check if file exists
        let total_size = match self.provider.file_size(&filename) {
            Some(size) => size,
            None => {
                eprintln!("edgerun-tftp: file not found: {}", filename);
                let err = TftpMessage::error(
                    TftpError::FileNotFound,
                    &format!("File not found: {}", filename),
                );
                let _ = self.socket.send_to(&err.to_wire(), client_addr);
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
        self.transfers.insert(
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
            let _ = self.socket.send_to(&wire, client_addr);
            eprintln!(
                "edgerun-tftp: OACK sent to {} (blksize={}, tsize={})",
                client_addr, negotiated.blksize, total_size
            );
        } else {
            // No options to negotiate, start sending data
            self.send_next_block(&client_addr)?;
        }

        Ok(())
    }

    fn handle_ack(&mut self, client_addr: SocketAddr, block: u16) -> Result<(), io::Error> {
        // Find the transfer for this client
        let key = self
            .transfers
            .keys()
            .find(|(addr, _)| *addr == client_addr)
            .cloned();

        if let Some(key) = key {
            let transfer = &self.transfers[&key];

            // Only process ACK for the current block
            if block == transfer.current_block || (block == 0 && transfer.current_block == 0) {
                // block 0 is ACK for OACK
                if transfer.offset >= transfer.total_size as usize {
                    // Transfer complete
                    eprintln!(
                        "edgerun-tftp: transfer complete '{}' to {} ({} bytes)",
                        transfer.filename, client_addr, transfer.offset
                    );
                    self.transfers.remove(&key);
                } else {
                    self.send_next_block(&client_addr)?;
                }
            }
        }

        Ok(())
    }

    fn send_next_block(&mut self, client_addr: &SocketAddr) -> Result<(), io::Error> {
        let key = self
            .transfers
            .keys()
            .find(|(addr, _)| addr == client_addr)
            .cloned();

        if let Some(key) = key {
            let transfer = self.transfers.get_mut(&key).unwrap();
            let blksize = transfer.blksize as usize;

            // Read the next block
            match self
                .provider
                .read_block(&transfer.filename, transfer.offset, blksize)
            {
                Some(data) => {
                    let is_last = data.len() < blksize;
                    transfer.current_block = transfer.current_block.wrapping_add(1);

                    let data_msg = TftpMessage::data(transfer.current_block, data);
                    let wire = data_msg.to_wire();
                    let _ = self.socket.send_to(&wire, *client_addr);

                    transfer.offset += data_msg_data_len(&data_msg);

                    if is_last {
                        eprintln!(
                            "edgerun-tftp: final block {} for '{}' to {}",
                            transfer.current_block, transfer.filename, client_addr
                        );
                    }
                }
                None => {
                    let err =
                        TftpMessage::error(TftpError::NotDefined, "Failed to read file block");
                    let _ = self.socket.send_to(&err.to_wire(), *client_addr);
                    self.transfers.remove(&key);
                }
            }
        }

        Ok(())
    }

    /// Get the number of active transfers.
    pub fn active_transfers(&self) -> usize {
        self.transfers.len()
    }
}

fn data_msg_data_len(msg: &TftpMessage) -> usize {
    match msg {
        TftpMessage::DATA { data, .. } => data.len(),
        _ => 0,
    }
}
