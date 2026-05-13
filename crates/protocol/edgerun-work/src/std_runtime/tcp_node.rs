use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::channel::{ChannelEnvelope, RouteAdvertisement};
use crate::channel_order::{ChannelOrderBook, OrderedChannelEnvelope};
use crate::codec::{ArchivedWorkPacketFrame, encode_work_packet_once};
use crate::protocol::{Hash, NodeId, WorkPacket};
use crate::route_auth::route_hash;
use crate::route_table::{RouteMap, insert_live_route, live_route_hash_for, route_for_send};
use crate::std_runtime::framing::{read_work_packet_frame, unix_ms, write_encoded_work_packet};
use crate::std_runtime::threading::{drain_joined_threads, join_locked_optional_thread};
use crate::work_channel::{WorkChannel, WorkChannelError};

const ACCEPT_POLL_MS: u64 = 10;
const CONNECTION_READ_TIMEOUT_MS: u64 = 250;

#[derive(Debug)]
pub struct TcpNodeRuntime {
    node_id: NodeId,
    listen_addr: SocketAddr,
    routes: RouteMap,
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
            routes: RouteMap::new(),
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
        join_locked_optional_thread(
            &self.accept_thread,
            "tcp accept thread poisoned",
            "tcp accept thread panicked",
        )?;
        drain_joined_threads(
            &self.worker_threads,
            "tcp worker thread list poisoned",
            "tcp worker thread panicked",
        )
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
                let envelope = ChannelEnvelope::new(
                    channel_id,
                    from,
                    self.node_id,
                    route_hash,
                    packet_hash,
                    packet,
                );
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
        insert_live_route(&mut self.routes, route, unix_ms()).ok_or(WorkChannelError::RouteInvalid)
    }

    fn remove_route(&mut self, node_id: NodeId) -> Option<RouteAdvertisement> {
        self.routes.remove(&node_id)
    }

    fn route_hash_for(&self, node_id: &NodeId) -> Option<Hash> {
        live_route_hash_for(&self.routes, node_id, unix_ms())
    }

    fn send_unordered(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, WorkChannelError> {
        let route = route_for_send(&mut self.routes, &to, unix_ms())
            .ok_or(WorkChannelError::RouteMissing)?;
        let encoded =
            encode_work_packet_once(&packet).map_err(|_| WorkChannelError::PacketHashFailed)?;
        let addr = Self::route_addr(route).ok_or(WorkChannelError::RouteMissing)?;
        let mut stream = TcpStream::connect(addr).map_err(|_| WorkChannelError::DeliveryFailed)?;
        write_encoded_work_packet(&mut stream, encoded.as_bytes())
            .map_err(|_| WorkChannelError::DeliveryFailed)?;
        Ok(ChannelEnvelope::for_route(
            route,
            route_hash(route),
            from,
            to,
            encoded.hash,
            packet,
        ))
    }

    fn recv_all(&mut self, _node_id: NodeId) -> Vec<ChannelEnvelope> {
        Vec::new()
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
