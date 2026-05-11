use std::collections::BTreeMap;
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::channel::{ChannelEnvelope, RouteAdvertisement};
use crate::channel_order::{ChannelOrderBook, OrderedChannelEnvelope};
use crate::codec::{blake3_hash, packet_bytes};
use crate::memory_channel::route_hash;
use crate::protocol::{Hash, NodeId, WorkPacket, WORK_WIRE_ABI_VERSION};
use crate::route_auth::verify_route_advertisement;
use crate::std_runtime::framing::{read_work_packet, write_work_packet};
use crate::work_channel::{OrderedWorkChannel, WorkChannel, WorkChannelError};

#[derive(Clone, Debug)]
pub struct TcpNodeRuntime {
    node_id: NodeId,
    listen_addr: SocketAddr,
    routes: BTreeMap<NodeId, RouteAdvertisement>,
    inbox: Arc<Mutex<Vec<WorkPacket>>>,
}

impl TcpNodeRuntime {
    pub fn bind<A: ToSocketAddrs>(node_id: NodeId, addr: A) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        let listen_addr = listener.local_addr()?;
        let inbox = Arc::new(Mutex::new(Vec::new()));
        let inbox_for_thread = Arc::clone(&inbox);
        thread::spawn(move || {
            for incoming in listener.incoming() {
                if let Ok(stream) = incoming {
                    let inbox = Arc::clone(&inbox_for_thread);
                    thread::spawn(move || {
                        let _ = read_stream_into_inbox(stream, inbox);
                    });
                }
            }
        });
        Ok(Self {
            node_id,
            listen_addr,
            routes: BTreeMap::new(),
            inbox,
        })
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    pub fn drain_packets(&self) -> Vec<WorkPacket> {
        self.inbox.lock().expect("tcp node inbox poisoned").drain(..).collect()
    }

    pub fn drain_ordered(
        &self,
        from: NodeId,
        order: &mut ChannelOrderBook,
        route_hash: Hash,
        channel_id: Hash,
    ) -> Vec<OrderedChannelEnvelope> {
        self.drain_packets()
            .into_iter()
            .filter_map(|packet| {
                let packet_hash = packet_bytes(&packet).ok().map(|bytes| blake3_hash(&bytes))?;
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
        String::from_utf8(route.endpoint.address.clone()).ok()
    }
}

impl WorkChannel for TcpNodeRuntime {
    fn add_route(&mut self, route: RouteAdvertisement) -> Result<Hash, WorkChannelError> {
        if !verify_route_advertisement(&route) {
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
        self.routes.get(node_id).map(route_hash)
    }

    fn send_unordered(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, WorkChannelError> {
        let route = self.routes.get(&to).ok_or(WorkChannelError::RouteMissing)?;
        let packet_hash = packet_bytes(&packet)
            .map(|bytes| blake3_hash(&bytes))
            .map_err(|_| WorkChannelError::PacketHashFailed)?;
        let addr = Self::route_addr(route).ok_or(WorkChannelError::RouteMissing)?;
        let mut stream = TcpStream::connect(addr).map_err(|_| WorkChannelError::DeliveryFailed)?;
        write_work_packet(&mut stream, &packet).map_err(|_| WorkChannelError::DeliveryFailed)?;
        Ok(ChannelEnvelope {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id: route.endpoint.channel_id,
            from,
            to,
            route_hash: route_hash(route),
            packet_hash,
            packet,
        })
    }

    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        if node_id != self.node_id {
            return Vec::new();
        }
        self.drain_packets()
            .into_iter()
            .filter_map(|packet| {
                let packet_hash = packet_bytes(&packet).ok().map(|bytes| blake3_hash(&bytes))?;
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

fn read_stream_into_inbox(mut stream: TcpStream, inbox: Arc<Mutex<Vec<WorkPacket>>>) -> io::Result<()> {
    let packet = read_work_packet(&mut stream)?;
    inbox.lock().expect("tcp node inbox poisoned").push(packet);
    Ok(())
}
