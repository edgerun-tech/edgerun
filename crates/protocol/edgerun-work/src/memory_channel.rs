use alloc::vec::Vec;

use crate::channel::*;
use crate::codec::encode_channel_envelope_for_route;
use crate::protocol::{Hash, NodeId, WorkPacket};
use crate::route_binding::current_unix_ms;
use crate::route_table::RouteState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryChannelError {
    RouteMissing,
    RouteInvalid,
    PacketHashFailed,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryChannelEngine {
    routes: RouteState,
}

impl MemoryChannelEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_route(&mut self, route: RouteBinding) -> Result<Hash, MemoryChannelError> {
        let hash = self
            .routes
            .insert_live_route(route, current_unix_ms())
            .ok_or(MemoryChannelError::RouteInvalid)?;
        Ok(hash)
    }

    pub fn add_unchecked_route(&mut self, route: RouteBinding) -> Hash {
        self.routes.insert_unchecked_route(route)
    }

    pub fn remove_route(&mut self, node_id: NodeId) -> Option<RouteBinding> {
        self.routes.remove_route(node_id)
    }

    pub fn route_for(&self, node_id: &NodeId) -> Option<&RouteBinding> {
        self.routes.route_for(node_id, current_unix_ms())
    }

    pub fn deliver(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, MemoryChannelError> {
        let route = self
            .routes
            .route_for_send(&to, current_unix_ms())
            .ok_or(MemoryChannelError::RouteMissing)?;
        let encoded = encode_channel_envelope_for_route(route, from, to, packet)
            .map_err(|_| MemoryChannelError::PacketHashFailed)?;
        let envelope = encoded.envelope;
        self.routes.push_inbox(envelope.clone());
        Ok(envelope)
    }

    pub fn drain_inbox(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        self.routes.drain_inbox(node_id)
    }

    pub fn route_count(&self) -> usize {
        self.routes.route_count()
    }
}
