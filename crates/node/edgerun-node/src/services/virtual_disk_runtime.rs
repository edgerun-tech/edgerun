//! Node-owned virtual disk service bindings.
//!
//! Virtual disk protocol state lives in `edgerun-protocols` and stream/session
//! helpers live in `edgerun-virtual-disk`. This module is the host resource
//! owner: it binds native listeners, accepts sessions, and reports failures at
//! the node boundary.

extern crate std;

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::time::Duration;
use std::io;
use std::net::{TcpListener, TcpStream};
use std::thread;

use crate::rt::{self, CancellationToken};
use edgerun_protocols::block::{handle_request, BlockBackend, BlockError};
use edgerun_virtual_disk::{
    receive_request, send_response, serve_nbd_connection_multi, NbdExportEntry,
};

type SharedBlockBackend = Arc<dyn BlockBackend + Send + Sync>;

pub enum VirtualDiskBinding {
    BlockTcp {
        bind_addr: String,
        backend: SharedBlockBackend,
    },
    NbdTcp {
        bind_addr: String,
        exports: Vec<NbdExportEntry>,
    },
}

impl VirtualDiskBinding {
    pub fn block_tcp<B>(bind_addr: String, backend: Arc<B>) -> Self
    where
        B: BlockBackend + Send + Sync + 'static,
    {
        let backend: SharedBlockBackend = backend;
        Self::BlockTcp { bind_addr, backend }
    }

    pub fn nbd_tcp(bind_addr: String, exports: Vec<NbdExportEntry>) -> Self {
        Self::NbdTcp { bind_addr, exports }
    }
}

pub struct VirtualDiskRuntime {
    listener: TcpListener,
    mode: VirtualDiskMode,
}

enum VirtualDiskMode {
    Block { backend: SharedBlockBackend },
    Nbd { exports: Arc<Vec<NbdExportEntry>> },
}

impl VirtualDiskRuntime {
    pub fn bind(binding: VirtualDiskBinding) -> io::Result<Self> {
        let (bind_addr, mode) = match binding {
            VirtualDiskBinding::BlockTcp { bind_addr, backend } => {
                (bind_addr, VirtualDiskMode::Block { backend })
            }
            VirtualDiskBinding::NbdTcp { bind_addr, exports } => (
                bind_addr,
                VirtualDiskMode::Nbd {
                    exports: Arc::new(exports),
                },
            ),
        };
        let listener = TcpListener::bind(&bind_addr)?;
        listener.set_nonblocking(true)?;
        crate::node_info!("virtual disk listener bound on {}", bind_addr);
        Ok(Self { listener, mode })
    }

    pub fn local_addr(&self) -> io::Result<std::net::SocketAddr> {
        self.listener.local_addr()
    }

    pub async fn run(self, shutdown: CancellationToken) -> io::Result<()> {
        while !shutdown.is_cancelled() {
            match self.listener.accept() {
                Ok((stream, _peer)) => self.spawn_session(stream),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    rt::sleep(Duration::from_millis(100)).await;
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    fn spawn_session(&self, stream: TcpStream) {
        match &self.mode {
            VirtualDiskMode::Block { backend } => {
                let backend = Arc::clone(backend);
                thread::spawn(move || {
                    if let Err(error) = serve_block_stream(stream, backend) {
                        crate::node_warn!("virtual block session failed: {}", error);
                    }
                });
            }
            VirtualDiskMode::Nbd { exports } => {
                let exports = Arc::clone(exports);
                thread::spawn(move || {
                    let mut stream = stream;
                    if let Err(error) = serve_nbd_connection_multi(&mut stream, exports.as_slice())
                    {
                        crate::node_warn!("virtual NBD session failed: {}", error);
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use edgerun_protocols::block::BlockDeviceInfo;
    use edgerun_virtual_disk::{BlockClient, MemoryBlockBackend};

    fn test_info() -> BlockDeviceInfo {
        BlockDeviceInfo {
            block_size: 512,
            block_count: 8,
            readonly: false,
            supports_flush: true,
            supports_discard: true,
            supports_write_zeroes: true,
            model: "node-virtual-disk-test".into(),
            serial: "node-vdisk-001".into(),
        }
    }

    #[test]
    fn node_runtime_accepts_virtual_block_session() {
        let backend = Arc::new(MemoryBlockBackend::new(test_info()).unwrap());
        let runtime = VirtualDiskRuntime::bind(VirtualDiskBinding::block_tcp(
            "127.0.0.1:0".into(),
            Arc::clone(&backend),
        ))
        .unwrap();
        let addr = runtime.local_addr().unwrap();
        let server_backend = Arc::clone(&backend);
        let worker = thread::spawn(move || {
            let stream = loop {
                match runtime.listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::yield_now();
                    }
                    Err(error) => panic!("accept failed: {error}"),
                }
            };
            serve_block_stream(stream, server_backend).unwrap();
        });

        let stream = TcpStream::connect(addr).unwrap();
        let mut client = BlockClient::new(stream);
        client.handshake().unwrap();
        client.write_blocks(0, 1, vec![0x42; 512]).unwrap();
        assert_eq!(client.read_blocks(0, 1).unwrap(), vec![0x42; 512]);
        drop(client);
        worker.join().unwrap();

        let mut out = vec![0_u8; 512];
        backend.read_blocks(0, 1, &mut out).unwrap();
        assert_eq!(out, vec![0x42; 512]);
    }
}

fn serve_block_stream(
    mut stream: TcpStream,
    backend: SharedBlockBackend,
) -> Result<(), BlockError> {
    loop {
        match receive_request(&mut stream) {
            Ok(request) => {
                let response = handle_request(backend.as_ref(), request);
                send_response(&mut stream, &response)?;
            }
            Err(BlockError::ProtocolError(message)) if message == "unexpected EOF" => {
                return Ok(());
            }
            Err(error) => return Err(error),
        }
    }
}
