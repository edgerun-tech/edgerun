use edgerun_work::*;

fn route_for_endpoint(node: &SimNode, endpoint: ChannelEndpoint, sequence: u64) -> RouteAdvertisement {
    RouteAdvertisementBuilder::new(&node.key, node.identity.role, endpoint)
        .departments(vec![DEPARTMENT_MESSAGE])
        .sequence(sequence)
        .build(&node.key)
}

fn verified_plan(routes: Vec<RouteAdvertisement>) -> VerifiedRoutePlan {
    let issuer = SimNode::from_seed(201, NODE_ROLE_ADMISSION);
    let route_root = route_root_hash(&routes);
    let snapshot = sign_route_snapshot(
        &issuer.key,
        RouteSnapshot {
            abi_version: WORK_WIRE_ABI_VERSION,
            issued_by: issuer.identity,
            sequence: 1,
            routes,
            route_root,
            signature: empty_signature(),
        },
    );
    VerifiedRoutePlan::from_snapshot(snapshot).expect("valid route plan")
}

#[test]
fn native_policy_prefers_quic_before_tcp_websocket_and_webtransport() {
    let node = SimNode::from_seed(202, NODE_ROLE_MESSAGE);
    let ws = route_for_endpoint(
        &node,
        websocket_endpoint("ws", b"wss://relay.edgerun.test/ws"),
        1,
    );
    let webtransport = route_for_endpoint(
        &node,
        webtransport_endpoint("webtransport", b"https://relay.edgerun.test/wt"),
        2,
    );
    let tcp = route_for_endpoint(&node, tcp_endpoint("tcp", b"127.0.0.1:9100"), 3);
    let quic = route_for_endpoint(&node, quic_endpoint("quic", b"127.0.0.1:4433"), 4);
    let plan = verified_plan(vec![ws, webtransport, tcp, quic]);

    let selected = plan
        .preferred_route_for_node(node.identity.node_id, &RouteSelectionPolicy::native())
        .expect("native route selected");

    assert_eq!(selected.endpoint.kind, CHANNEL_KIND_QUIC);
}

#[test]
fn browser_policy_prefers_webtransport_and_rejects_raw_quic() {
    let node = SimNode::from_seed(203, NODE_ROLE_MESSAGE);
    let quic = route_for_endpoint(&node, quic_endpoint("quic", b"203.0.113.1:4433"), 1);
    let ws = route_for_endpoint(
        &node,
        websocket_endpoint("ws", b"wss://relay.edgerun.test/ws"),
        2,
    );
    let webtransport = route_for_endpoint(
        &node,
        webtransport_endpoint("webtransport", b"https://relay.edgerun.test/wt"),
        3,
    );
    let plan = verified_plan(vec![quic.clone(), ws, webtransport]);
    let browser = RouteSelectionPolicy::browser();

    assert!(!browser.allows(&quic));
    let selected = plan
        .preferred_route_for_node(node.identity.node_id, &browser)
        .expect("browser route selected");

    assert_eq!(selected.endpoint.kind, CHANNEL_KIND_WEBTRANSPORT);
}

#[test]
fn browser_policy_falls_back_to_websocket_when_webtransport_missing() {
    let node = SimNode::from_seed(204, NODE_ROLE_MESSAGE);
    let quic = route_for_endpoint(&node, quic_endpoint("quic", b"203.0.113.2:4433"), 1);
    let ws = route_for_endpoint(
        &node,
        websocket_endpoint("ws", b"wss://relay.edgerun.test/ws"),
        2,
    );
    let plan = verified_plan(vec![quic, ws]);

    let selected = plan
        .preferred_route_for_node(node.identity.node_id, &RouteSelectionPolicy::browser())
        .expect("browser fallback route selected");

    assert_eq!(selected.endpoint.kind, CHANNEL_KIND_WEBSOCKET);
}

#[test]
fn department_routes_are_sorted_by_policy_preference() {
    let node = SimNode::from_seed(205, NODE_ROLE_MESSAGE);
    let ws = route_for_endpoint(
        &node,
        websocket_endpoint("ws", b"wss://relay.edgerun.test/ws"),
        1,
    );
    let quic = route_for_endpoint(&node, quic_endpoint("quic", b"203.0.113.3:4433"), 2);
    let webtransport = route_for_endpoint(
        &node,
        webtransport_endpoint("webtransport", b"https://relay.edgerun.test/wt"),
        3,
    );
    let plan = verified_plan(vec![ws, quic, webtransport]);

    let native = plan.routes_for_department_by_policy(DEPARTMENT_MESSAGE, &RouteSelectionPolicy::native());
    assert_eq!(native.iter().map(|route| route.endpoint.kind).collect::<Vec<_>>(), vec![
        CHANNEL_KIND_QUIC,
        CHANNEL_KIND_WEBSOCKET,
        CHANNEL_KIND_WEBTRANSPORT,
    ]);

    let browser = plan.routes_for_department_by_policy(DEPARTMENT_MESSAGE, &RouteSelectionPolicy::browser());
    assert_eq!(browser.iter().map(|route| route.endpoint.kind).collect::<Vec<_>>(), vec![
        CHANNEL_KIND_WEBTRANSPORT,
        CHANNEL_KIND_WEBSOCKET,
    ]);
}

#[test]
fn endpoint_channel_id_is_domain_separated_by_transport_kind() {
    let address = b"relay.edgerun.test:443";
    let quic = quic_endpoint("quic", address);
    let webtransport = webtransport_endpoint("webtransport", address);
    let websocket = websocket_endpoint("websocket", address);

    assert_ne!(quic.channel_id, webtransport.channel_id);
    assert_ne!(quic.channel_id, websocket.channel_id);
    assert_ne!(webtransport.channel_id, websocket.channel_id);
}
