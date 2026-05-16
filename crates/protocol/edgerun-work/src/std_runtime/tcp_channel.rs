use std::io;
use std::net::{SocketAddr, ToSocketAddrs};

use crate::channel::{ChannelEnvelope, RouteBinding};
use crate::codec::ArchivedWorkPacketFrame;
use crate::protocol::{Hash, NodeId, WorkPacket};
use crate::route_table::RouteState;
use crate::std_runtime::framing::unix_ms;
use crate::std_runtime::tcp_server::{
    TcpPacketServer, send_encoded_packet_to_addr, send_encoded_packet_to_route, send_packet_to_addr,
};
use crate::work_channel::{WorkChannel, WorkChannelError};

#[derive(Debug)]
pub struct TcpIncomingServer {
    server: TcpPacketServer,
}

impl TcpIncomingServer {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        Ok(Self {
            server: TcpPacketServer::bind(addr, "tcp incoming server inbox poisoned")?,
        })
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.server.listen_addr()
    }

    pub fn drain_packet_frames(&self) -> Vec<ArchivedWorkPacketFrame> {
        self.server.drain_packet_frames()
    }

    pub fn drain_packets(&self) -> Vec<WorkPacket> {
        self.server.drain_packets()
    }

    pub fn shutdown(&mut self) -> io::Result<()> {
        self.server.shutdown()
    }
}

#[derive(Clone, Debug, Default)]
pub struct TcpWorkChannel {
    routes: RouteState,
}

impl TcpWorkChannel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn serve_incoming<A: ToSocketAddrs>(addr: A) -> io::Result<TcpIncomingServer> {
        TcpIncomingServer::bind(addr)
    }

    pub fn send_packet_to_addr<A: ToSocketAddrs>(addr: A, packet: &WorkPacket) -> io::Result<()> {
        send_packet_to_addr(addr, packet)
    }

    pub fn send_encoded_packet_to_addr<A: ToSocketAddrs>(addr: A, bytes: &[u8]) -> io::Result<()> {
        send_encoded_packet_to_addr(addr, bytes)
    }
}

impl WorkChannel for TcpWorkChannel {
    fn add_route(&mut self, route: RouteBinding) -> Result<Hash, WorkChannelError> {
        let hash = self
            .routes
            .insert_live_route(route, unix_ms())
            .ok_or(WorkChannelError::RouteInvalid)?;
        Ok(hash)
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
        let envelope = encoded.envelope;
        self.routes.push_inbox(envelope.clone());
        Ok(envelope)
    }

    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        self.routes.drain_inbox(node_id)
    }
}
