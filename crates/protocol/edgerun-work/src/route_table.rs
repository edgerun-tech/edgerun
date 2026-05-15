use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::channel::{ChannelEnvelope, RouteBinding};
use crate::codec::{encode_channel_envelope_for_route, EncodedChannelEnvelope};
use crate::protocol::{Hash, NodeId, WorkPacket};
use crate::route_binding::{route_hash, route_is_available, verify_live_route_binding};

type RouteMap = BTreeMap<NodeId, RouteBinding>;
type RouteInboxMap = BTreeMap<NodeId, Vec<ChannelEnvelope>>;

#[derive(Clone, Debug, Default)]
pub(crate) struct RouteState {
    routes: RouteMap,
    inboxes: RouteInboxMap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RouteStateSendError {
    RouteMissing,
    PacketHashFailed,
}

impl RouteState {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn insert_live_route(
        &mut self,
        route: RouteBinding,
        now_unix_ms: u64,
    ) -> Option<Hash> {
        verify_live_route_binding(&route, now_unix_ms).then(|| self.insert_unchecked_route(route))
    }

    pub(crate) fn insert_unchecked_route(&mut self, route: RouteBinding) -> Hash {
        let node_id = route.node.node_id;
        let hash = route_hash(&route);
        self.routes.insert(node_id, route);
        self.inboxes.entry(node_id).or_default();
        hash
    }

    pub(crate) fn remove_route(&mut self, node_id: NodeId) -> Option<RouteBinding> {
        self.inboxes.remove(&node_id);
        self.routes.remove(&node_id)
    }

    pub(crate) fn route_hash_for(&self, node_id: &NodeId, now_unix_ms: u64) -> Option<Hash> {
        self.route_for(node_id, now_unix_ms).map(route_hash)
    }

    pub(crate) fn route_for(&self, node_id: &NodeId, now_unix_ms: u64) -> Option<&RouteBinding> {
        let route = self.routes.get(node_id)?;
        route_is_available(route, now_unix_ms).then_some(route)
    }

    pub(crate) fn encode_for_send(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
        now_unix_ms: u64,
    ) -> Result<EncodedChannelEnvelope, RouteStateSendError> {
        self.encode_for_send_with_route(from, to, packet, now_unix_ms)
            .map(|(_, encoded)| encoded)
    }

    pub(crate) fn encode_for_send_with_route(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
        now_unix_ms: u64,
    ) -> Result<(RouteBinding, EncodedChannelEnvelope), RouteStateSendError> {
        if self
            .routes
            .get(&to)
            .is_some_and(|route| !route_is_available(route, now_unix_ms))
        {
            self.remove_route(to);
            return Err(RouteStateSendError::RouteMissing);
        }
        let route = self
            .routes
            .get(&to)
            .ok_or(RouteStateSendError::RouteMissing)?
            .clone();
        let encoded = encode_channel_envelope_for_route(&route, from, to, packet)
            .map_err(|_| RouteStateSendError::PacketHashFailed)?;
        Ok((route, encoded))
    }

    pub(crate) fn queue_send(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
        now_unix_ms: u64,
    ) -> Result<ChannelEnvelope, RouteStateSendError> {
        let envelope = self
            .encode_for_send(from, to, packet, now_unix_ms)?
            .envelope;
        self.push_inbox(envelope.clone());
        Ok(envelope)
    }

    pub(crate) fn push_inbox(&mut self, envelope: ChannelEnvelope) {
        self.inboxes.entry(envelope.to).or_default().push(envelope);
    }

    pub(crate) fn drain_inbox(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        self.inboxes.entry(node_id).or_default().drain(..).collect()
    }

    pub(crate) fn route_count(&self) -> usize {
        self.routes.len()
    }
}
