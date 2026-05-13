use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use edgerun_crypto::Ed25519SigningKey;

use crate::channel::RouteBinding;
use crate::object_storage_service::ObjectStorageService;
use crate::protocol::{Hash, NodeId, NodeIdentity, WorkPacket};
use crate::roles::WorkServiceResponse;
use crate::route_builder::tcp_endpoint;
use crate::std_runtime::framing::{
    read_work_packet, read_work_service_response, send_work_service_request, unix_ms,
    write_work_service_response,
};
use crate::std_runtime::threading::{drain_joined_threads, join_optional_thread};
use crate::storage_adapter::{InMemoryObjectStorage, ObjectStorageAdapter};

const ACCEPT_POLL_MS: u64 = 10;

pub type StorageDaemonResponse = WorkServiceResponse;

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

    pub fn route_binding(&self, relay_node_id: NodeId, valid_until_unix_ms: u64) -> RouteBinding {
        self.service
            .lock()
            .expect("object storage daemon service poisoned")
            .route_binding(
                tcp_endpoint("object-storage", self.listen_addr.to_string()),
                relay_node_id,
                valid_until_unix_ms,
            )
    }

    pub fn shutdown(&mut self) -> io::Result<()> {
        self.shutdown.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.listen_addr);
        join_optional_thread(
            &mut self.accept_thread,
            "object storage daemon accept thread panicked",
        )?;
        drain_joined_threads(
            &self.worker_threads,
            "object storage daemon worker list poisoned",
            "object storage daemon worker thread panicked",
        )
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
    send_work_service_request(addr, packet)
}

pub fn read_storage_daemon_response(stream: &mut TcpStream) -> io::Result<StorageDaemonResponse> {
    read_work_service_response(stream)
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
    write_work_service_response(&mut stream, &output)
}
