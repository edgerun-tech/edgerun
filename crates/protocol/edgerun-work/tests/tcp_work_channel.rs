use std::io::Read;
use std::net::TcpListener;
use std::thread;
use edgerun_work::*;

#[test]
fn tcp_work_channel_uses_same_ordered_channel_trait() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
        }
    });

    let mut sender = SimNode::from_seed(241, NODE_ROLE_MESSAGE);
    let mut receiver = SimNode::from_seed(242, NODE_ROLE_MESSAGE);
    let mut channel = TcpWorkChannel::new();

    let mut route = receiver.advertise_memory_route(receiver.identity.node_id, vec![DEPARTMENT_MESSAGE]);
    route.endpoint.kind = CHANNEL_KIND_TCP;
    route.endpoint.address = format!("127.0.0.1:{}", addr.port()).into_bytes();
    route.endpoint.label = "tcp-test".into();
    route = sign_route_advertisement(&receiver.key, route);
    let route_hash = channel.add_route(route).expect("tcp route");

    let packet = sender.message_to(
        receiver.identity.node_id,
        receiver.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"tcp work channel trait path".to_vec(),
    );
    let ordered = channel
        .send_ordered(
            &mut sender.order,
            sender.identity.node_id,
            receiver.identity.node_id,
            packet,
        )
        .expect("ordered send over tcp adapter");
    assert_eq!(ordered.sequence, 1);
    assert_eq!(ordered.envelope.route_hash, route_hash);

    let mut inbox = channel.recv_all(receiver.identity.node_id);
    assert_eq!(inbox.len(), 1);
    let received = OrderedChannelEnvelope {
        envelope: inbox.remove(0),
        sequence: 1,
        previous_message_hash: [0u8; 32],
    };
    receiver
        .accept_ordered(&received, route_hash)
        .expect("receiver accepts tcp-adapter envelope");
}
