#![cfg(feature = "std")]

use std::thread;
use std::time::Duration;

use edgerun_work::*;

fn memory_route(node: &SimNode, valid_until_unix_ms: u64) -> RouteBinding {
    let mut route = node.bind_memory_route(node.identity.node_id, vec![DEPARTMENT_MESSAGE]);
    route.valid_until_unix_ms = valid_until_unix_ms;
    route
}

#[test]
fn memory_channel_rejects_expired_route_on_add() {
    let node = SimNode::from_seed(61, NODE_ROLE_MESSAGE);
    let route = memory_route(&node, 0);
    let mut channel = MemoryChannelEngine::new();

    assert_eq!(
        channel.add_route(route),
        Err(MemoryChannelError::RouteInvalid)
    );
    assert_eq!(channel.route_count(), 0);
}

#[test]
fn memory_channel_hides_and_evicts_expired_unchecked_route() {
    let mut sender = SimNode::from_seed(63, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(64, NODE_ROLE_MESSAGE);
    let mut channel = MemoryChannelEngine::new();
    let route = memory_route(&receiver, unix_ms().saturating_add(1));

    channel.add_unchecked_route(route);
    thread::sleep(Duration::from_millis(20));

    assert!(channel.route_for(&receiver.identity.node_id).is_none());

    let packet = sender.message_to(
        receiver.identity.node_id,
        receiver.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"expired memory route should not deliver".to_vec(),
    );
    assert_eq!(
        channel.deliver(sender.identity.node_id, receiver.identity.node_id, packet),
        Err(MemoryChannelError::RouteMissing)
    );
    assert_eq!(channel.route_count(), 0);
}

#[test]
fn memory_channel_delivers_available_route() {
    let mut sender = SimNode::from_seed(65, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(66, NODE_ROLE_MESSAGE);
    let mut channel = MemoryChannelEngine::new();
    let route = memory_route(&receiver, u64::MAX);
    let route_hash = channel.add_route(route).expect("add available route");

    let packet = sender.message_to(
        receiver.identity.node_id,
        receiver.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"available memory route should deliver".to_vec(),
    );
    let envelope = channel
        .deliver(sender.identity.node_id, receiver.identity.node_id, packet)
        .expect("deliver over available memory route");

    assert_eq!(envelope.route_hash, route_hash);
    assert_eq!(channel.drain_inbox(receiver.identity.node_id).len(), 1);
}

#[test]
fn route_hash_changes_when_route_policy_fields_change() {
    let node = SimNode::from_seed(67, NODE_ROLE_MESSAGE);
    let base = memory_route(&node, u64::MAX);

    let mut changed_roles = base.clone();
    changed_roles.roles.push(NODE_ROLE_STORAGE);

    let mut changed_departments = base.clone();
    changed_departments.departments.push(DEPARTMENT_STORAGE);

    assert_ne!(route_hash(&base), route_hash(&changed_roles));
    assert_ne!(route_hash(&base), route_hash(&changed_departments));
}

#[test]
fn route_hash_changes_when_route_endpoint_or_chain_changes() {
    let node = SimNode::from_seed(68, NODE_ROLE_MESSAGE);
    let base = memory_route(&node, u64::MAX);

    let mut changed_endpoint = base.clone();
    changed_endpoint.endpoint.address = b"127.0.0.1:9000".to_vec();

    assert_ne!(route_hash(&base), route_hash(&changed_endpoint));
}
