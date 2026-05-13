use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::channel::*;
use crate::codec::{blake3_hash, encode_work_packet_once};
use crate::protocol::{Hash, NodeId, WORK_WIRE_ABI_VERSION, WorkPacket};
use crate::route_auth::{route_advertisement_preimage, verify_route_advertisement};

#[cfg(feature = "std")]
fn current_unix_ms() -> u64 {
    crate::std_runtime::unix_ms()
}

#[cfg(not(feature = "std"))]
fn current_unix_ms() -> u64 {
    0
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryChannelError {
    RouteMissing,
    RouteInvalid,
    PacketHashFailed,
    InboxMissing,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryChannelEngine {
    routes: BTreeMap<NodeId, RouteAdvertisement>,
    inboxes: BTreeMap<NodeId, Vec<ChannelEnvelope>>,
}

impl MemoryChannelEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_route(&mut self, route: RouteAdvertisement) -> Result<Hash, MemoryChannelError> {
        if !verify_route_advertisement(&route) || !route_is_available(&route, current_unix_ms()) {
            return Err(MemoryChannelError::RouteInvalid);
        }
        let node_id = route.node.node_id;
        let hash = route_hash(&route);
        self.routes.insert(node_id, route);
        self.inboxes.entry(node_id).or_default();
        Ok(hash)
    }

    pub fn add_unchecked_route(&mut self, route: RouteAdvertisement) -> Hash {
        let node_id = route.node.node_id;
        let hash = route_hash(&route);
        self.routes.insert(node_id, route);
        self.inboxes.entry(node_id).or_default();
        hash
    }

    pub fn remove_route(&mut self, node_id: NodeId) -> Option<RouteAdvertisement> {
        self.inboxes.remove(&node_id);
        self.routes.remove(&node_id)
    }

    pub fn route_for(&self, node_id: &NodeId) -> Option<&RouteAdvertisement> {
        let route = self.routes.get(node_id)?;
        route_is_available(route, current_unix_ms()).then_some(route)
    }

    pub fn deliver(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, MemoryChannelError> {
        let now = current_unix_ms();
        if self
            .routes
            .get(&to)
            .is_some_and(|route| !route_is_available(route, now))
        {
            self.remove_route(to);
            return Err(MemoryChannelError::RouteMissing);
        }
        let route = self
            .routes
            .get(&to)
            .ok_or(MemoryChannelError::RouteMissing)?;
        let encoded =
            encode_work_packet_once(&packet).map_err(|_| MemoryChannelError::PacketHashFailed)?;
        let envelope = ChannelEnvelope {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id: route.endpoint.channel_id,
            from,
            to,
            route_hash: route_hash(route),
            packet_hash: encoded.hash,
            packet,
        };
        self.inboxes
            .get_mut(&to)
            .ok_or(MemoryChannelError::InboxMissing)?
            .push(envelope.clone());
        Ok(envelope)
    }

    pub fn drain_inbox(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        self.inboxes.entry(node_id).or_default().drain(..).collect()
    }

    pub fn route_count(&self) -> usize {
        self.routes.len()
    }
}

pub fn route_is_available(route: &RouteAdvertisement, now_unix_ms: u64) -> bool {
    route.status == ROUTE_STATUS_AVAILABLE && route.valid_until_unix_ms >= now_unix_ms
}

pub fn route_hash(route: &RouteAdvertisement) -> Hash {
    blake3_hash(&route_advertisement_preimage(route))
}
