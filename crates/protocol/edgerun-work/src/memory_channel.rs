use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::channel::*;
use crate::codec::{blake3_hash, packet_bytes};
use crate::protocol::{Hash, NodeId, WorkPacket, WORK_WIRE_ABI_VERSION};
use crate::route_auth::verify_route_advertisement;

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
        if !verify_route_advertisement(&route) {
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
        self.routes.get(node_id)
    }

    pub fn deliver(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, MemoryChannelError> {
        let route = self.routes.get(&to).ok_or(MemoryChannelError::RouteMissing)?;
        let packet_hash = packet_bytes(&packet)
            .map(|bytes| blake3_hash(&bytes))
            .map_err(|_| MemoryChannelError::PacketHashFailed)?;
        let envelope = ChannelEnvelope {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id: route.endpoint.channel_id,
            from,
            to,
            route_hash: route_hash(route),
            packet_hash,
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

pub fn route_hash(route: &RouteAdvertisement) -> Hash {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&route.node.node_id);
    bytes.extend_from_slice(&route.relay_node_id);
    bytes.extend_from_slice(&route.endpoint.channel_id);
    bytes.extend_from_slice(&route.sequence.to_be_bytes());
    bytes.extend_from_slice(&route.valid_until_unix_ms.to_be_bytes());
    blake3_hash(&bytes)
}
