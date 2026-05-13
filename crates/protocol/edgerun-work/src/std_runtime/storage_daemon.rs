use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use edgerun_crypto::Ed25519SigningKey;

use crate::channel::RouteAdvertisement;
use crate::object_storage_service::{ObjectStorageResponse, ObjectStorageService};
use crate::protocol::{Hash, NodeId, NodeIdentity, WorkPacket};
use crate::roles::ROLE_STATUS_ACCEPTED;
use crate::route_builder::tcp_endpoint;
use crate::std_runtime::framing::{read_work_packet, unix_ms, write_encoded_work_packet};
use crate::storage_adapter::{InMemoryObjectStorage, ObjectStorageAdapter};

const ACCEPT_POLL_MS: u64 = 10;
const STORAGE_RESPONSE_KIND_NONE: u8 = 0;
const STORAGE_RESPONSE_KIND_PACKET: u8 = 1;
const STORAGE_RESPONSE_KIND_BYTES: u8 = 2;

pub type StorageDaemonResponse = ObjectStorageResponse;

pub struct TcpObjectStorageDaemon<S: ObjectStorageAdapter + Send + 'static> {
    identity: NodeIdentity,
    listen_addr: SocketAddr,
    service: Arc<Mutex<ObjectStorageService<S>>>,
    shutdown: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
    worker_threads: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

pub type ObjectStorageDaemon<S> = TcpObjectStorageDaemon<S>;

impl TcpObjectStorageDaemon<InMemoryObjectStorage> {
    pub fn bind_memory<A: ToSocketAddrs>(
        key: Ed25519SigningKey,
        addr: A,
        capacity_bytes: u64,
    ) -> io::Result<Self> {
        Self::bind_with_storage(key, addr, InMemoryObjectStorage::new(capacity_bytes))
    }
}

#[cfg(feature = "std")]
impl TcpObjectStorageDaemon<crate::storage_adapter::FileObjectStorage> {
    pub fn bind_object_store_dir<A: ToSocketAddrs>(
        key: Ed25519SigningKey,
        addr: A,
        root: impl Into<std::path::PathBuf>,
        capacity_bytes: u64,
    ) -> io::Result<Self> {
        let storage = crate::storage_adapter::FileObjectStorage::open(root, capacity_bytes)?;
        Self::bind_with_storage(key, addr, storage)
    }
}

impl<S: ObjectStorageAdapter + Send + 'static> TcpObjectStorageDaemon<S> {
    pub fn bind_with_storage<A: ToSocketAddrs>(
        key: Ed25519SigningKey,
        addr: A,
        storage: S,
    ) -> io::Result<Self> {
        Self::bind_with_storage_and_policy(key, addr, storage, [0u8; 32])
    }

    pub fn bind_with_storage_and_policy<A: ToSocketAddrs>(
        key: Ed25519SigningKey,
        addr: A,
        storage: S,
        policy_hash: Hash,
    ) -> io::Result<Self> {
        let service = ObjectStorageService::with_storage_and_policy(key, storage, policy_hash);
        Self::bind_service(addr, service)
    }

    pub fn bind_service<A: ToSocketAddrs>(
        addr: A,
        service: ObjectStorageService<S>,
    ) -> io::Result<Self> {
        let identity = service.identity().clone();
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let listen_addr = listener.local_addr()?;
        let service = Arc::new(Mutex::new(service));
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_threads = Arc::new(Mutex::new(Vec::new()));
        let service_for_thread = Arc::clone(&service);
        let shutdown_for_thread = Arc::clone(&shutdown);
        let workers_for_thread = Arc::clone(&worker_threads);
        let accept_thread = thread::spawn(move || {
            while !shutdown_for_thread.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _peer)) => {
                        let service = Arc::clone(&service_for_thread);
                        let shutdown = Arc::clone(&shutdown_for_thread);
                        let handle = thread::spawn(move || {
                            let _ = handle_storage_connection(stream, service, shutdown);
                        });
                        workers_for_thread
                            .lock()
                            .expect("object storage daemon worker list poisoned")
                            .push(handle);
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            identity,
            listen_addr,
            service,
            shutdown,
            accept_thread: Some(accept_thread),
            worker_threads,
        })
    }

    pub fn identity(&self) -> &NodeIdentity {
        &self.identity
    }

    pub fn node_id(&self) -> NodeId {
        self.identity.node_id
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    pub fn policy_hash(&self) -> Hash {
        self.service
            .lock()
            .expect("object storage daemon service poisoned")
            .policy_hash()
    }

    pub fn object_count(&self) -> usize {
        self.service
            .lock()
            .expect("object storage daemon service poisoned")
            .object_count()
    }

    pub fn used_bytes(&self) -> u64 {
        self.service
            .lock()
            .expect("object storage daemon service poisoned")
            .used_bytes()
    }

    pub fn route_advertisement(
        &self,
        relay_node_id: NodeId,
        valid_until_unix_ms: u64,
    ) -> RouteAdvertisement {
        self.service
            .lock()
            .expect("object storage daemon service poisoned")
            .route_advertisement(
                tcp_endpoint("object-storage", self.listen_addr.to_string()),
                relay_node_id,
                valid_until_unix_ms,
            )
    }

    pub fn shutdown(&mut self) -> io::Result<()> {
        self.shutdown.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.listen_addr);
        if let Some(handle) = self.accept_thread.take() {
            handle.join().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::Other,
                    "object storage daemon accept thread panicked",
                )
            })?;
        }
        let mut workers = self
            .worker_threads
            .lock()
            .expect("object storage daemon worker list poisoned");
        let handles = workers.drain(..).collect::<Vec<_>>();
        drop(workers);
        for handle in handles {
            handle.join().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::Other,
                    "object storage daemon worker thread panicked",
                )
            })?;
        }
        Ok(())
    }
}

impl<S: ObjectStorageAdapter + Send + 'static> Drop for TcpObjectStorageDaemon<S> {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

pub fn send_storage_daemon_request<A: ToSocketAddrs>(
    addr: A,
    packet: &WorkPacket,
) -> io::Result<StorageDaemonResponse> {
    let mut stream = TcpStream::connect(addr)?;
    crate::std_runtime::framing::write_work_packet(&mut stream, packet)?;
    read_storage_daemon_response(&mut stream)
}

pub fn read_storage_daemon_response(stream: &mut TcpStream) -> io::Result<StorageDaemonResponse> {
    let mut header = [0u8; 7];
    stream.read_exact(&mut header)?;
    let status = u16::from_be_bytes([header[0], header[1]]);
    let kind = header[2];
    let len = u32::from_be_bytes([header[3], header[4], header[5], header[6]]) as usize;
    let mut body = vec![0u8; len];
    if len > 0 {
        stream.read_exact(&mut body)?;
    }
    match kind {
        STORAGE_RESPONSE_KIND_NONE => Ok(StorageDaemonResponse {
            status,
            packet: None,
            bytes: body,
        }),
        STORAGE_RESPONSE_KIND_PACKET => {
            let packet = crate::codec::packet_from_bytes(&body)
                .map_err(|_| io::Error::new(ErrorKind::InvalidData, "invalid response packet"))?;
            Ok(StorageDaemonResponse {
                status,
                packet: Some(packet),
                bytes: Vec::new(),
            })
        }
        STORAGE_RESPONSE_KIND_BYTES => Ok(StorageDaemonResponse {
            status,
            packet: None,
            bytes: body,
        }),
        _ => Err(io::Error::new(
            ErrorKind::InvalidData,
            "invalid storage response kind",
        )),
    }
}

fn handle_storage_connection<S: ObjectStorageAdapter>(
    mut stream: TcpStream,
    service: Arc<Mutex<ObjectStorageService<S>>>,
    shutdown: Arc<AtomicBool>,
) -> io::Result<()> {
    if shutdown.load(Ordering::Acquire) {
        return Ok(());
    }
    let packet = read_work_packet(&mut stream)?;
    let output = service
        .lock()
        .expect("object storage daemon service poisoned")
        .handle_packet(packet, unix_ms());
    write_storage_daemon_response(&mut stream, &output)
}

fn write_storage_daemon_response(
    stream: &mut TcpStream,
    output: &ObjectStorageResponse,
) -> io::Result<()> {
    let (kind, body) = if let Some(packet) = &output.packet {
        let bytes = crate::codec::packet_bytes(packet)
            .map_err(|_| io::Error::new(ErrorKind::InvalidData, "invalid response packet"))?;
        (STORAGE_RESPONSE_KIND_PACKET, bytes)
    } else if !output.bytes.is_empty() {
        (STORAGE_RESPONSE_KIND_BYTES, output.bytes.clone())
    } else {
        (STORAGE_RESPONSE_KIND_NONE, Vec::new())
    };
    if output.status != ROLE_STATUS_ACCEPTED && kind == STORAGE_RESPONSE_KIND_NONE {
        stream.write_all(&output.status.to_be_bytes())?;
        stream.write_all(&[STORAGE_RESPONSE_KIND_BYTES])?;
        stream.write_all(&(output.bytes.len() as u32).to_be_bytes())?;
        stream.write_all(&output.bytes)?;
        return stream.flush();
    }
    stream.write_all(&output.status.to_be_bytes())?;
    stream.write_all(&[kind])?;
    if kind == STORAGE_RESPONSE_KIND_PACKET {
        write_encoded_work_packet(stream, &body)
    } else {
        stream.write_all(&(body.len() as u32).to_be_bytes())?;
        stream.write_all(&body)?;
        stream.flush()
    }
}
