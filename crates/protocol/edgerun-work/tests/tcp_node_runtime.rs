use std::thread;
use std::time::Duration;

use edgerun_work::*;

#[test]
fn tcp_node_runtime_receives_framed_packet_over_loopback() {
    let mut sender = SimNode::from_seed(251, NODE_ROLE_MESSAGE);
    let mut receiver = SimNode::from_seed(252, NODE_ROLE_MESSAGE);
    let mut runtime = TcpNodeRuntime::bind(receiver.identity.node_id, "127.0.0.1:0")
        .expect("bind tcp runtime");

    let mut route = receiver.advertise_memory_route(receiver.identity.node_id, vec![DEPARTMENT_MESSAGE]);
    route.endpoint.kind = CHANNEL_KIND_TCP;
    route.endpoint.address = runtime.listen_addr().to_string().into_bytes();
    route.endpoint.label = "tcp-loopback".into();
    route = sign_route_advertisement(&receiver.key, route);
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

    let mut packets = Vec::new();
    for _ in 0..50 {
        packets = runtime.drain_packets();
        if !packets.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(packets.len(), 1);

    let packet_hash = packet_bytes(&packets[0]).map(|bytes| blake3_hash(&bytes)).unwrap();
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
}
