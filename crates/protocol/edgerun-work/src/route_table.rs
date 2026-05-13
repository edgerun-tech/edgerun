use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::channel::{ChannelEnvelope, RouteBinding};
use crate::protocol::{Hash, NodeId};
use crate::route_binding::{route_hash, route_is_available, verify_live_route_binding};

type RouteMap = BTreeMap<NodeId, RouteBinding>;
type RouteInboxMap = BTreeMap<NodeId, Vec<ChannelEnvelope>>;

#[derive(Clone, Debug, Default)]
pub(crate) struct RouteState {
    routes: RouteMap,
    inboxes: RouteInboxMap,
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

    pub(crate) fn route_for_send(
        &mut self,
        node_id: &NodeId,
        now_unix_ms: u64,
    ) -> Option<&RouteBinding> {
        if self
            .routes
            .get(node_id)
            .is_some_and(|route| !route_is_available(route, now_unix_ms))
        {
            self.remove_route(*node_id);
            return None;
        }
        self.routes.get(node_id)
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
