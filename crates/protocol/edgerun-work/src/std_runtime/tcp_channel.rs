use std::collections::BTreeMap;
use std::io;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::thread;

use crate::channel::{ChannelEnvelope, RouteAdvertisement};
use crate::codec::{blake3_hash, packet_bytes};
use crate::memory_channel::route_hash;
use crate::protocol::{Hash, NodeId, WorkPacket, WORK_WIRE_ABI_VERSION};
use crate::route_auth::verify_route_advertisement;
use crate::std_runtime::framing::{read_work_packet, write_work_packet};
use crate::work_channel::{WorkChannel, WorkChannelError};

#[derive(Clone, Debug, Default)]
pub struct TcpWorkChannel {
    routes: BTreeMap<NodeId, RouteAdvertisement>,
    inboxes: BTreeMap<NodeId, Vec<ChannelEnvelope>>,
}

impl TcpWorkChannel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn serve_incoming<A: ToSocketAddrs + Send + 'static>(addr: A) -> io::Result<()> {
        let listener = TcpListener::bind(addr)?;
        thread::spawn(move || {
            for incoming in listener.incoming() {
                if let Ok(mut stream) = incoming {
                    let _ = read_work_packet(&mut stream);
                }
            }
        });
        Ok(())
    }

    pub fn send_packet_to_addr<A: ToSocketAddrs>(addr: A, packet: &WorkPacket) -> io::Result<()> {
        let mut stream = TcpStream::connect(addr)?;
        write_work_packet(&mut stream, packet)
    }

    fn route_addr(route: &RouteAdvertisement) -> Option<String> {
        String::from_utf8(route.endpoint.address.clone()).ok()
    }
}

impl WorkChannel for TcpWorkChannel {
    fn add_route(&mut self, route: RouteAdvertisement) -> Result<Hash, WorkChannelError> {
        if !verify_route_advertisement(&route) {
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
        self.routes.get(node_id).map(route_hash)
    }

    fn send_unordered(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, WorkChannelError> {
        let route = self.routes.get(&to).ok_or(WorkChannelError::RouteMissing)?;
        let addr = Self::route_addr(route).ok_or(WorkChannelError::RouteMissing)?;
        let packet_hash = packet_bytes(&packet)
            .map(|bytes| blake3_hash(&bytes))
            .map_err(|_| WorkChannelError::PacketHashFailed)?;
        Self::send_packet_to_addr(addr, &packet).map_err(|_| WorkChannelError::DeliveryFailed)?;
        let envelope = ChannelEnvelope {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id: route.endpoint.channel_id,
            from,
            to,
            route_hash: route_hash(route),
            packet_hash,
            packet,
        };
        self.inboxes.entry(to).or_default().push(envelope.clone());
        Ok(envelope)
    }

    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        self.inboxes.entry(node_id).or_default().drain(..).collect()
    }
}
