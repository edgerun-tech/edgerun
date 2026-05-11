use edgerun_work::*;

#[test]
fn memory_channel_rejects_tampered_route_advertisement() {
    let relay = SimNode::from_seed(51, NODE_ROLE_RELAY);
    let receiver = SimNode::from_seed(52, NODE_ROLE_MESSAGE);
    let mut channel = MemoryChannelEngine::new();

    let route = receiver.advertise_memory_route(
        relay.identity.node_id,
        vec![DEPARTMENT_MESSAGE],
    );
    channel.add_route(route.clone()).expect("valid signed route");

    let mut tampered = route;
    tampered.sequence = tampered.sequence.saturating_add(1);
    assert!(matches!(
        channel.add_route(tampered),
        Err(MemoryChannelError::RouteInvalid)
    ));
}
