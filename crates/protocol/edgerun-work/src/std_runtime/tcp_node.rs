use std::io;
use std::net::{SocketAddr, ToSocketAddrs};

use crate::channel::{ChannelEnvelope, RouteBinding};
use crate::channel_order::{ChannelOrderBook, OrderedChannelEnvelope};
use crate::codec::ArchivedWorkPacketFrame;
use crate::protocol::{Hash, NodeId, WorkPacket};
use crate::route_table::RouteState;
use crate::std_runtime::framing::unix_ms;
use crate::std_runtime::tcp_server::{send_encoded_packet_to_route, TcpPacketServer};
use crate::work_channel::{WorkChannel, WorkChannelError};

#[derive(Debug)]
pub struct TcpNodeRuntime {
    node_id: NodeId,
    server: TcpPacketServer,
    routes: RouteState,
}

impl Clone for TcpNodeRuntime {
    fn clone(&self) -> Self {
        Self {
            node_id: self.node_id,
            server: self.server.clone(),
            routes: self.routes.clone(),
        }
    }
}

impl TcpNodeRuntime {
    pub fn bind<A: ToSocketAddrs>(node_id: NodeId, addr: A) -> io::Result<Self> {
        Ok(Self {
            node_id,
            server: TcpPacketServer::bind(addr, "tcp node inbox poisoned")?,
            routes: RouteState::new(),
        })
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.server.listen_addr()
    }

    pub fn shutdown(&self) -> io::Result<()> {
        self.server.shutdown()
    }

    pub fn drain_packet_frames(&self) -> Vec<ArchivedWorkPacketFrame> {
        self.server.drain_packet_frames()
    }

    pub fn drain_packets(&self) -> Vec<WorkPacket> {
        self.server.drain_packets()
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
                let envelope = ChannelEnvelope::new(
                    channel_id,
                    from,
                    self.node_id,
                    route_hash,
                    packet_hash,
                    packet,
                );
                order.accept_envelope(envelope, route_hash).ok()
            })
            .collect()
    }
}

impl WorkChannel for TcpNodeRuntime {
    fn add_route(&mut self, route: RouteBinding) -> Result<Hash, WorkChannelError> {
        self.routes
            .insert_live_route(route, unix_ms())
            .ok_or(WorkChannelError::RouteInvalid)
    }

    fn remove_route(&mut self, node_id: NodeId) -> Option<RouteBinding> {
        self.routes.remove_route(node_id)
    }

    fn route_hash_for(&self, node_id: &NodeId) -> Option<Hash> {
        self.routes.route_hash_for(node_id, unix_ms())
    }

    fn send_unordered(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, WorkChannelError> {
        let (route, encoded) = self
            .routes
            .encode_for_send_with_route(from, to, packet, unix_ms())
            .map_err(WorkChannelError::from)?;
        send_encoded_packet_to_route(&route, encoded.packet.as_bytes())
            .map_err(|_| WorkChannelError::DeliveryFailed)?;
        Ok(encoded.envelope)
    }

    fn recv_all(&mut self, _node_id: NodeId) -> Vec<ChannelEnvelope> {
        Vec::new()
    }
}
