use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::channel::{ChannelEnvelope, RouteBinding};
use crate::protocol::{Hash, NodeId};
use crate::route_binding::{
    available_route_hash, route_hash, route_is_available, verify_live_route_binding,
};

pub(crate) type RouteMap = BTreeMap<NodeId, RouteBinding>;
pub(crate) type RouteInboxMap = BTreeMap<NodeId, Vec<ChannelEnvelope>>;

pub(crate) fn insert_route(routes: &mut RouteMap, route: RouteBinding) -> Hash {
    let node_id = route.node.node_id;
    let hash = route_hash(&route);
    routes.insert(node_id, route);
    hash
}

#[cfg(feature = "std")]
pub(crate) fn insert_live_route(
    routes: &mut RouteMap,
    route: RouteBinding,
    now_unix_ms: u64,
) -> Option<Hash> {
    verify_live_route_binding(&route, now_unix_ms).then(|| insert_route(routes, route))
}

pub(crate) fn insert_route_with_inbox(
    routes: &mut RouteMap,
    inboxes: &mut RouteInboxMap,
    route: RouteBinding,
) -> Hash {
    let node_id = route.node.node_id;
    let hash = insert_route(routes, route);
    inboxes.entry(node_id).or_default();
    hash
}

pub(crate) fn insert_live_route_with_inbox(
    routes: &mut RouteMap,
    inboxes: &mut RouteInboxMap,
    route: RouteBinding,
    now_unix_ms: u64,
) -> Option<Hash> {
    verify_live_route_binding(&route, now_unix_ms)
        .then(|| insert_route_with_inbox(routes, inboxes, route))
}

pub(crate) fn remove_route_with_inbox(
    routes: &mut RouteMap,
    inboxes: &mut RouteInboxMap,
    node_id: NodeId,
) -> Option<RouteBinding> {
    inboxes.remove(&node_id);
    routes.remove(&node_id)
}

pub(crate) fn drain_inbox(inboxes: &mut RouteInboxMap, node_id: NodeId) -> Vec<ChannelEnvelope> {
    inboxes.entry(node_id).or_default().drain(..).collect()
}

pub(crate) fn live_route_hash_for(
    routes: &RouteMap,
    node_id: &NodeId,
    now_unix_ms: u64,
) -> Option<Hash> {
    let route = routes.get(node_id)?;
    available_route_hash(route, now_unix_ms)
}

pub(crate) fn route_for_send<'a>(
    routes: &'a mut RouteMap,
    node_id: &NodeId,
    now_unix_ms: u64,
) -> Option<&'a RouteBinding> {
    if routes
        .get(node_id)
        .is_some_and(|route| !route_is_available(route, now_unix_ms))
    {
        routes.remove(node_id);
        return None;
    }
    routes.get(node_id)
}
