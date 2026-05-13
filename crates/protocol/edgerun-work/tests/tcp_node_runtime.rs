#![cfg(feature = "std")]

use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use edgerun_work::*;

fn signed_tcp_route(node: &SimNode, address: String, valid_until_unix_ms: u64) -> RouteBinding {
    let mut route = node.bind_memory_route(node.identity.node_id, vec![DEPARTMENT_MESSAGE]);
    route.endpoint.kind = CHANNEL_KIND_TCP;
    route.endpoint.address = address.into_bytes();
    route.endpoint.label = "tcp-test".into();
    route.valid_until_unix_ms = valid_until_unix_ms;
    route
}

#[test]
fn tcp_node_runtime_receives_framed_packet_over_loopback() {
    let mut sender = SimNode::from_seed(251, NODE_ROLE_MESSAGE);
    let mut receiver = SimNode::from_seed(252, NODE_ROLE_MESSAGE);
    let mut runtime =
        TcpNodeRuntime::bind(receiver.identity.node_id, "127.0.0.1:0").expect("bind tcp runtime");

    let route = signed_tcp_route(&receiver, runtime.listen_addr().to_string(), u64::MAX);
    let route_hash = runtime.add_route(route.clone()).expect("add route");

    let packet = sender.message_to(
        receiver.identity.node_id,
        receiver.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"hello loopback tcp runtime".to_vec(),
    );
    let sent = runtime
        .send_ordered(
            &mut sender.order,
            sender.identity.node_id,
            receiver.identity.node_id,
            packet,
        )
        .expect("send ordered to tcp runtime");
    assert_eq!(sent.envelope.route_hash, route_hash);
    assert!(runtime.recv_all(receiver.identity.node_id).is_empty());

    let mut packets = Vec::new();
    for _ in 0..50 {
        packets = runtime.drain_packets();
        if !packets.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(packets.len(), 1);

    let packet_hash = packet_bytes(&packets[0])
        .map(|bytes| blake3_hash(&bytes))
        .unwrap();
    let ordered = OrderedChannelEnvelope {
        envelope: ChannelEnvelope {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id: route.endpoint.channel_id,
            from: sender.identity.node_id,
            to: receiver.identity.node_id,
            route_hash,
            packet_hash,
            packet: packets.remove(0),
        },
        sequence: 1,
        previous_message_hash: [0u8; 32],
    };
    receiver
        .accept_ordered(&ordered, route_hash)
        .expect("receiver accepts tcp runtime packet");
    runtime.shutdown().expect("shutdown tcp runtime");
}

#[test]
fn tcp_node_runtime_reports_delivery_failed_for_dead_route() {
    let mut sender = SimNode::from_seed(253, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(254, NODE_ROLE_MESSAGE);
    let dead_listener = TcpListener::bind("127.0.0.1:0").expect("reserve dead route");
    let dead_addr = dead_listener.local_addr().expect("dead route bindingdr");
    drop(dead_listener);

    let mut runtime = TcpNodeRuntime::bind(sender.identity.node_id, "127.0.0.1:0")
        .expect("bind sender tcp runtime");
    let route = signed_tcp_route(&receiver, dead_addr.to_string(), u64::MAX);
    runtime.add_route(route).expect("add dead tcp route");

    let packet = sender.message_to(
        receiver.identity.node_id,
        receiver.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"delivery should fail".to_vec(),
    );
    let err = runtime
        .send_ordered(
            &mut sender.order,
            sender.identity.node_id,
            receiver.identity.node_id,
            packet,
        )
        .expect_err("dead tcp route must fail delivery");
    assert_eq!(err, WorkChannelError::DeliveryFailed);
    runtime.shutdown().expect("shutdown sender runtime");
}

#[test]
fn tcp_node_runtime_shutdown_stops_accepting_connections() {
    let runtime = TcpNodeRuntime::bind([1u8; 32], "127.0.0.1:0").expect("bind tcp runtime");
    let addr = runtime.listen_addr();
    drop(TcpStream::connect(addr).expect("runtime accepts before shutdown"));

    runtime.shutdown().expect("shutdown tcp runtime");

    assert!(TcpStream::connect(addr).is_err());
}

#[test]
fn tcp_node_runtime_removes_expired_route_during_selection() {
    let mut sender = SimNode::from_seed(255, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(250, NODE_ROLE_MESSAGE);
    let mut runtime =
        TcpNodeRuntime::bind(sender.identity.node_id, "127.0.0.1:0").expect("bind tcp runtime");

    let route = signed_tcp_route(
        &receiver,
        runtime.listen_addr().to_string(),
        unix_ms().saturating_add(50),
    );
    runtime.add_route(route).expect("add short-lived route");
    thread::sleep(Duration::from_millis(100));

    assert_eq!(runtime.route_hash_for(&receiver.identity.node_id), None);

    let packet = sender.message_to(
        receiver.identity.node_id,
        receiver.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"expired route should not be selected".to_vec(),
    );
    let err = runtime
        .send_ordered(
            &mut sender.order,
            sender.identity.node_id,
            receiver.identity.node_id,
            packet,
        )
        .expect_err("expired route must not be selected");
    assert_eq!(err, WorkChannelError::RouteMissing);
    runtime.shutdown().expect("shutdown tcp runtime");
}
