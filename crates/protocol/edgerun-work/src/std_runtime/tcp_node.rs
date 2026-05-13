use std::collections::BTreeMap;
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::channel::{ChannelEnvelope, RouteAdvertisement, ROUTE_STATUS_AVAILABLE};
use crate::channel_order::{ChannelOrderBook, OrderedChannelEnvelope};
use crate::codec::{encode_work_packet_once, ArchivedWorkPacketFrame};
use crate::memory_channel::route_hash;
use crate::protocol::{Hash, NodeId, WorkPacket, WORK_WIRE_ABI_VERSION};
use crate::route_auth::verify_route_advertisement;
use crate::std_runtime::framing::{read_work_packet_frame, unix_ms, write_encoded_work_packet};
use crate::work_channel::{WorkChannel, WorkChannelError};

const ACCEPT_POLL_MS: u64 = 10;
const CONNECTION_READ_TIMEOUT_MS: u64 = 250;

#[derive(Debug)]
pub struct TcpNodeRuntime {
    node_id: NodeId,
    listen_addr: SocketAddr,
    routes: BTreeMap<NodeId, RouteAdvertisement>,
    inbox: Arc<Mutex<Vec<ArchivedWorkPacketFrame>>>,
    shutdown: Arc<AtomicBool>,
    accept_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
    worker_threads: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl Clone for TcpNodeRuntime {
    fn clone(&self) -> Self {
        Self {
            node_id: self.node_id,
            listen_addr: self.listen_addr,
            routes: self.routes.clone(),
            inbox: Arc::clone(&self.inbox),
            shutdown: Arc::clone(&self.shutdown),
            accept_thread: Arc::clone(&self.accept_thread),
            worker_threads: Arc::clone(&self.worker_threads),
        }
    }
}

impl TcpNodeRuntime {
    pub fn bind<A: ToSocketAddrs>(node_id: NodeId, addr: A) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let listen_addr = listener.local_addr()?;
        let inbox = Arc::new(Mutex::new(Vec::new()));
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_threads = Arc::new(Mutex::new(Vec::new()));
        let inbox_for_thread = Arc::clone(&inbox);
        let shutdown_for_thread = Arc::clone(&shutdown);
        let workers_for_thread = Arc::clone(&worker_threads);
        let accept_thread = thread::spawn(move || {
            while !shutdown_for_thread.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _peer)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(
                            CONNECTION_READ_TIMEOUT_MS,
                        )));
                        let inbox = Arc::clone(&inbox_for_thread);
                        let shutdown = Arc::clone(&shutdown_for_thread);
                        let handle = thread::spawn(move || {
                            let _ = read_stream_into_inbox(stream, inbox, shutdown);
                        });
                        workers_for_thread
                            .lock()
                            .expect("tcp worker thread list poisoned")
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
            node_id,
            listen_addr,
            routes: BTreeMap::new(),
            inbox,
            shutdown,
            accept_thread: Arc::new(Mutex::new(Some(accept_thread))),
            worker_threads,
        })
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    pub fn shutdown(&self) -> io::Result<()> {
        self.shutdown.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.listen_addr);
        if let Some(handle) = self
            .accept_thread
            .lock()
            .expect("tcp accept thread poisoned")
            .take()
        {
            handle
                .join()
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "tcp accept thread panicked"))?;
        }
        let mut workers = self
            .worker_threads
            .lock()
            .expect("tcp worker thread list poisoned");
        let handles = workers.drain(..).collect::<Vec<_>>();
        drop(workers);
        for handle in handles {
            handle
                .join()
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "tcp worker thread panicked"))?;
        }
        Ok(())
    }

    pub fn drain_packet_frames(&self) -> Vec<ArchivedWorkPacketFrame> {
        self.inbox
            .lock()
            .expect("tcp node inbox poisoned")
            .drain(..)
            .collect()
    }

    pub fn drain_packets(&self) -> Vec<WorkPacket> {
        self.drain_packet_frames()
            .into_iter()
            .filter_map(|frame| frame.into_packet().ok())
            .collect()
    }

    pub fn drain_ordered(
        &self,
        from: NodeId,
        order: &mut ChannelOrderBook,
        route_hash: Hash,
        channel_id: Hash,
    ) -> Vec<OrderedChannelEnvelope> {
        self.drain_packet_frames()
            .into_iter()
            .filter_map(|frame| {
                let packet_hash = frame.hash;
                let packet = frame.into_packet().ok()?;
                let sequence = order.next_sequence(channel_id, from, self.node_id);
                let previous_message_hash = order.last_message_hash(channel_id, from, self.node_id);
                let envelope = ChannelEnvelope {
                    abi_version: WORK_WIRE_ABI_VERSION,
                    channel_id,
                    from,
                    to: self.node_id,
                    route_hash,
                    packet_hash,
                    packet,
                };
                let ordered = OrderedChannelEnvelope {
                    envelope,
                    sequence,
                    previous_message_hash,
                };
                order.accept(&ordered, route_hash).ok()?;
                Some(ordered)
            })
            .collect()
    }

    fn route_addr(route: &RouteAdvertisement) -> Option<String> {
        Some(String::from_utf8_lossy(&route.endpoint.address).into_owned())
    }

    fn route_available(route: &RouteAdvertisement, now_unix_ms: u64) -> bool {
        route.status == ROUTE_STATUS_AVAILABLE && route.valid_until_unix_ms >= now_unix_ms
    }
}

impl Drop for TcpNodeRuntime {
    fn drop(&mut self) {
        if Arc::strong_count(&self.accept_thread) == 1 {
            let _ = self.shutdown();
        }
    }
}

impl WorkChannel for TcpNodeRuntime {
    fn add_route(&mut self, route: RouteAdvertisement) -> Result<Hash, WorkChannelError> {
        if !verify_route_advertisement(&route) || !Self::route_available(&route, unix_ms()) {
            return Err(WorkChannelError::RouteInvalid);
        }
        let node_id = route.node.node_id;
        let hash = route_hash(&route);
        self.routes.insert(node_id, route);
        Ok(hash)
    }

    fn remove_route(&mut self, node_id: NodeId) -> Option<RouteAdvertisement> {
        self.routes.remove(&node_id)
    }

    fn route_hash_for(&self, node_id: &NodeId) -> Option<Hash> {
        let route = self.routes.get(node_id)?;
        Self::route_available(route, unix_ms()).then(|| route_hash(route))
    }

    fn send_unordered(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, WorkChannelError> {
        let now = unix_ms();
        if self
            .routes
            .get(&to)
            .is_some_and(|route| !Self::route_available(route, now))
        {
            self.routes.remove(&to);
            return Err(WorkChannelError::RouteMissing);
        }
        let route = self.routes.get(&to).ok_or(WorkChannelError::RouteMissing)?;
        let encoded =
            encode_work_packet_once(&packet).map_err(|_| WorkChannelError::PacketHashFailed)?;
        let addr = Self::route_addr(route).ok_or(WorkChannelError::RouteMissing)?;
        let mut stream = TcpStream::connect(addr).map_err(|_| WorkChannelError::DeliveryFailed)?;
        write_encoded_work_packet(&mut stream, encoded.as_bytes())
            .map_err(|_| WorkChannelError::DeliveryFailed)?;
        Ok(ChannelEnvelope {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id: route.endpoint.channel_id,
            from,
            to,
            route_hash: route_hash(route),
            packet_hash: encoded.hash,
            packet,
        })
    }

    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        if node_id != self.node_id {
            return Vec::new();
        }
        self.drain_packet_frames()
            .into_iter()
            .filter_map(|frame| {
                let packet_hash = frame.hash;
                let packet = frame.into_packet().ok()?;
                Some(ChannelEnvelope {
                    abi_version: WORK_WIRE_ABI_VERSION,
                    channel_id: [0u8; 32],
                    from: [0u8; 32],
                    to: self.node_id,
                    route_hash: [0u8; 32],
                    packet_hash,
                    packet,
                })
            })
            .collect()
    }
}

fn read_stream_into_inbox(
    mut stream: TcpStream,
    inbox: Arc<Mutex<Vec<ArchivedWorkPacketFrame>>>,
    shutdown: Arc<AtomicBool>,
) -> io::Result<()> {
    if shutdown.load(Ordering::Acquire) {
        return Ok(());
    }
    let frame = read_work_packet_frame(&mut stream)?;
    if !shutdown.load(Ordering::Acquire) {
        inbox.lock().expect("tcp node inbox poisoned").push(frame);
    }
    Ok(())
}
