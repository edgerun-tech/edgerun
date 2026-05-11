use std::collections::BTreeMap;
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::channel::{ChannelEnvelope, RouteAdvertisement};
use crate::codec::{encode_work_packet_once, ArchivedWorkPacketFrame};
use crate::memory_channel::{route_hash, route_is_available};
use crate::protocol::{Hash, NodeId, WorkPacket, WORK_WIRE_ABI_VERSION};
use crate::route_auth::verify_route_advertisement;
use crate::std_runtime::framing::{read_work_packet_frame, unix_ms, write_encoded_work_packet, write_work_packet};
use crate::work_channel::{WorkChannel, WorkChannelError};

const ACCEPT_POLL_MS: u64 = 10;

#[derive(Debug)]
pub struct TcpIncomingServer {
    listen_addr: SocketAddr,
    inbox: Arc<Mutex<Vec<ArchivedWorkPacketFrame>>>,
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl TcpIncomingServer {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let listen_addr = listener.local_addr()?;
        let inbox = Arc::new(Mutex::new(Vec::new()));
        let shutdown = Arc::new(AtomicBool::new(false));
        let inbox_for_thread = Arc::clone(&inbox);
        let shutdown_for_thread = Arc::clone(&shutdown);
        let thread = thread::spawn(move || {
            while !shutdown_for_thread.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _peer)) => {
                        if let Ok(frame) = read_work_packet_frame(&mut stream) {
                            if !shutdown_for_thread.load(Ordering::Acquire) {
                                inbox_for_thread
                                    .lock()
                                    .expect("tcp incoming server inbox poisoned")
                                    .push(frame);
                            }
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            listen_addr,
            inbox,
            shutdown,
            thread: Some(thread),
        })
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    pub fn drain_packet_frames(&self) -> Vec<ArchivedWorkPacketFrame> {
        self.inbox
            .lock()
            .expect("tcp incoming server inbox poisoned")
            .drain(..)
            .collect()
    }

    pub fn drain_packets(&self) -> Vec<WorkPacket> {
        self.drain_packet_frames()
            .into_iter()
            .filter_map(|frame| frame.into_packet().ok())
            .collect()
    }

    pub fn shutdown(&mut self) -> io::Result<()> {
        self.shutdown.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.listen_addr);
        if let Some(handle) = self.thread.take() {
            handle
                .join()
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "tcp incoming server thread panicked"))?;
        }
        Ok(())
    }
}

impl Drop for TcpIncomingServer {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[derive(Clone, Debug, Default)]
pub struct TcpWorkChannel {
    routes: BTreeMap<NodeId, RouteAdvertisement>,
    inboxes: BTreeMap<NodeId, Vec<ChannelEnvelope>>,
}

impl TcpWorkChannel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn serve_incoming<A: ToSocketAddrs>(addr: A) -> io::Result<TcpIncomingServer> {
        TcpIncomingServer::bind(addr)
    }

    pub fn send_packet_to_addr<A: ToSocketAddrs>(addr: A, packet: &WorkPacket) -> io::Result<()> {
        let mut stream = TcpStream::connect(addr)?;
        write_work_packet(&mut stream, packet)
    }

    pub fn send_encoded_packet_to_addr<A: ToSocketAddrs>(addr: A, bytes: &[u8]) -> io::Result<()> {
        let mut stream = TcpStream::connect(addr)?;
        write_encoded_work_packet(&mut stream, bytes)
    }

    fn route_addr(route: &RouteAdvertisement) -> Option<String> {
        Some(String::from_utf8_lossy(&route.endpoint.address).into_owned())
    }
}

impl WorkChannel for TcpWorkChannel {
    fn add_route(&mut self, route: RouteAdvertisement) -> Result<Hash, WorkChannelError> {
        if !verify_route_advertisement(&route) || !route_is_available(&route, unix_ms()) {
            return Err(WorkChannelError::RouteInvalid);
        }
        let node_id = route.node.node_id;
        let hash = route_hash(&route);
        self.routes.insert(node_id, route);
        self.inboxes.entry(node_id).or_default();
        Ok(hash)
    }

    fn remove_route(&mut self, node_id: NodeId) -> Option<RouteAdvertisement> {
        self.inboxes.remove(&node_id);
        self.routes.remove(&node_id)
    }

    fn route_hash_for(&self, node_id: &NodeId) -> Option<Hash> {
        let route = self.routes.get(node_id)?;
        route_is_available(route, unix_ms()).then(|| route_hash(route))
    }

    fn send_unordered(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, WorkChannelError> {
        let now = unix_ms();
        if self.routes.get(&to).is_some_and(|route| !route_is_available(route, now)) {
            self.routes.remove(&to);
            return Err(WorkChannelError::RouteMissing);
        }
        let route = self.routes.get(&to).ok_or(WorkChannelError::RouteMissing)?;
        let addr = Self::route_addr(route).ok_or(WorkChannelError::RouteMissing)?;
        let encoded = encode_work_packet_once(&packet).map_err(|_| WorkChannelError::PacketHashFailed)?;
        Self::send_encoded_packet_to_addr(addr, encoded.as_bytes()).map_err(|_| WorkChannelError::DeliveryFailed)?;
        let envelope = ChannelEnvelope {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id: route.endpoint.channel_id,
            from,
            to,
            route_hash: route_hash(route),
            packet_hash: encoded.hash,
            packet,
        };
        self.inboxes.entry(to).or_default().push(envelope.clone());
        Ok(envelope)
    }

    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        self.inboxes.entry(node_id).or_default().drain(..).collect()
    }
}
