use alloc::vec::Vec;

use crate::channel::*;
use crate::codec::encode_channel_envelope_for_route;
use crate::protocol::{Hash, NodeId, WorkPacket};
use crate::route_binding::{current_unix_ms, route_is_available};
use crate::route_table::{
    RouteInboxMap, RouteMap, drain_inbox, insert_live_route_with_inbox, insert_route_with_inbox,
    remove_route_with_inbox, route_for_send,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryChannelError {
    RouteMissing,
    RouteInvalid,
    PacketHashFailed,
    InboxMissing,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryChannelEngine {
    routes: RouteMap,
    inboxes: RouteInboxMap,
}

impl MemoryChannelEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_route(&mut self, route: RouteBinding) -> Result<Hash, MemoryChannelError> {
        let hash = insert_live_route_with_inbox(
            &mut self.routes,
            &mut self.inboxes,
            route,
            current_unix_ms(),
        )
        .ok_or(MemoryChannelError::RouteInvalid)?;
        Ok(hash)
    }

    pub fn add_unchecked_route(&mut self, route: RouteBinding) -> Hash {
        insert_route_with_inbox(&mut self.routes, &mut self.inboxes, route)
    }

    pub fn remove_route(&mut self, node_id: NodeId) -> Option<RouteBinding> {
        remove_route_with_inbox(&mut self.routes, &mut self.inboxes, node_id)
    }

    pub fn route_for(&self, node_id: &NodeId) -> Option<&RouteBinding> {
        let route = self.routes.get(node_id)?;
        route_is_available(route, current_unix_ms()).then_some(route)
    }

    pub fn deliver(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, MemoryChannelError> {
        let route = route_for_send(&mut self.routes, &to, current_unix_ms())
            .ok_or(MemoryChannelError::RouteMissing)?;
        let encoded = encode_channel_envelope_for_route(route, from, to, packet)
            .map_err(|_| MemoryChannelError::PacketHashFailed)?;
        let envelope = encoded.envelope;
        self.inboxes
            .get_mut(&to)
            .ok_or(MemoryChannelError::InboxMissing)?
            .push(envelope.clone());
        Ok(envelope)
    }

    pub fn drain_inbox(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        drain_inbox(&mut self.inboxes, node_id)
    }

    pub fn route_count(&self) -> usize {
        self.routes.len()
    }
}
