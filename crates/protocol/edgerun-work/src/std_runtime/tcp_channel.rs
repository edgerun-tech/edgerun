use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::channel::{ChannelEnvelope, RouteAdvertisement};
use crate::codec::{ArchivedWorkPacketFrame, encode_work_packet_once};
use crate::protocol::{Hash, NodeId, WorkPacket};
use crate::route_auth::route_hash;
use crate::route_table::{
    RouteInboxMap, RouteMap, drain_inbox, insert_live_route_with_inbox, live_route_hash_for,
    remove_route_with_inbox, route_for_send,
};
use crate::std_runtime::framing::{
    read_work_packet_frame, unix_ms, write_encoded_work_packet, write_work_packet,
};
use crate::std_runtime::threading::join_optional_thread;
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
        join_optional_thread(&mut self.thread, "tcp incoming server thread panicked")
    }
}

impl Drop for TcpIncomingServer {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[derive(Clone, Debug, Default)]
pub struct TcpWorkChannel {
    routes: RouteMap,
    inboxes: RouteInboxMap,
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
        let hash =
            insert_live_route_with_inbox(&mut self.routes, &mut self.inboxes, route, unix_ms())
                .ok_or(WorkChannelError::RouteInvalid)?;
        Ok(hash)
    }

    fn remove_route(&mut self, node_id: NodeId) -> Option<RouteAdvertisement> {
        remove_route_with_inbox(&mut self.routes, &mut self.inboxes, node_id)
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
        let addr = Self::route_addr(route).ok_or(WorkChannelError::RouteMissing)?;
        let encoded =
            encode_work_packet_once(&packet).map_err(|_| WorkChannelError::PacketHashFailed)?;
        Self::send_encoded_packet_to_addr(addr, encoded.as_bytes())
            .map_err(|_| WorkChannelError::DeliveryFailed)?;
        let envelope =
            ChannelEnvelope::for_route(route, route_hash(route), from, to, encoded.hash, packet);
        self.inboxes.entry(to).or_default().push(envelope.clone());
        Ok(envelope)
    }

    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        drain_inbox(&mut self.inboxes, node_id)
    }
}
