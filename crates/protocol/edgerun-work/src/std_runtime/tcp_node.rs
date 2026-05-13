use std::io;
use std::net::{SocketAddr, ToSocketAddrs};

use crate::channel::{ChannelEnvelope, RouteBinding};
use crate::channel_order::{ChannelOrderBook, OrderedChannelEnvelope};
use crate::codec::ArchivedWorkPacketFrame;
use crate::protocol::{Hash, NodeId, WorkPacket};
use crate::route_table::{RouteMap, insert_live_route, live_route_hash_for, route_for_send};
use crate::std_runtime::framing::unix_ms;
use crate::std_runtime::tcp_server::{TcpPacketServer, send_unordered_to_route};
use crate::work_channel::{WorkChannel, WorkChannelError};

#[derive(Debug)]
pub struct TcpNodeRuntime {
    node_id: NodeId,
    server: TcpPacketServer,
    routes: RouteMap,
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
            routes: RouteMap::new(),
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
}

impl WorkChannel for TcpNodeRuntime {
    fn add_route(&mut self, route: RouteBinding) -> Result<Hash, WorkChannelError> {
        insert_live_route(&mut self.routes, route, unix_ms()).ok_or(WorkChannelError::RouteInvalid)
    }

    fn remove_route(&mut self, node_id: NodeId) -> Option<RouteBinding> {
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
        send_unordered_to_route(route, from, to, packet)
    }

    fn recv_all(&mut self, _node_id: NodeId) -> Vec<ChannelEnvelope> {
        Vec::new()
    }
}
