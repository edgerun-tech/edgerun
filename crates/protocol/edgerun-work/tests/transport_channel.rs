use edgerun_work::*;

#[derive(Default)]
struct FakeTransport {
    sent: Vec<(u16, Vec<u8>)>,
    recv: Vec<TransportPacketFrame>,
}

impl WorkPacketTransport for FakeTransport {
    fn send_packet_bytes(
        &mut self,
        route: &RouteBinding,
        packet_bytes: &[u8],
    ) -> Result<(), WorkTransportError> {
        self.sent.push((route.endpoint.kind, packet_bytes.to_vec()));
        Ok(())
    }

    fn recv_packet_frame(&mut self) -> Result<Option<TransportPacketFrame>, WorkTransportError> {
        Ok(self.recv.pop())
    }
}

fn route_for_endpoint(node: &SimNode, endpoint: ChannelEndpoint, _sequence: u64) -> RouteBinding {
    RouteBindingBuilder::new(&node.key, node.identity.role, endpoint)
        .departments(vec![DEPARTMENT_MESSAGE])
        .build()
}

#[test]
fn native_transport_channel_sends_quic_route_as_rkyv_bytes() {
    let mut sender = SimNode::from_seed(211, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(212, NODE_ROLE_MESSAGE);
    let mut channel = TransportWorkChannel::native(FakeTransport::default());
    let route = route_for_endpoint(&receiver, quic_endpoint("quic", b"127.0.0.1:4433"), 1);
    let route_hash = channel
        .add_route(route)
        .expect("native quic route accepted");
    let packet = sender.message_to(
        receiver.identity.node_id,
        receiver.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"hello over quic transport".to_vec(),
    );

    let envelope = channel
        .send_unordered(sender.identity.node_id, receiver.identity.node_id, packet)
        .expect("send over fake transport");

    assert_eq!(envelope.route_hash, route_hash);
    assert_eq!(channel.transport().sent.len(), 1);
    assert_eq!(channel.transport().sent[0].0, CHANNEL_KIND_QUIC);
    assert!(!channel.transport().sent[0].1.is_empty());
}

#[test]
fn browser_transport_channel_rejects_raw_quic_route_and_accepts_webtransport() {
    let receiver = SimNode::from_seed(213, NODE_ROLE_MESSAGE);
    let mut channel = TransportWorkChannel::browser(FakeTransport::default());
    let quic = route_for_endpoint(&receiver, quic_endpoint("quic", b"203.0.113.10:4433"), 1);
    let webtransport = route_for_endpoint(
        &receiver,
        webtransport_endpoint("webtransport", b"https://relay.edgerun.test/wt"),
        2,
    );

    assert_eq!(channel.add_route(quic), Err(WorkChannelError::RouteInvalid));
    assert!(channel.add_route(webtransport).is_ok());
    assert_eq!(channel.route_count(), 1);
}

#[test]
fn transport_channel_poll_recv_materializes_archived_frames_only_at_boundary() {
    let mut sender = SimNode::from_seed(214, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(215, NODE_ROLE_MESSAGE);
    let packet = sender.message_to(
        receiver.identity.node_id,
        receiver.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"incoming frame".to_vec(),
    );
    let encoded = encode_work_packet_once(&packet).expect("encode packet");
    let frame = archived_packet_frame_from_bytes(encoded.as_bytes()).expect("archived frame");
    let route_hash = [7u8; 32];
    let channel_id = [8u8; 32];
    let mut transport = FakeTransport::default();
    transport.recv.push(TransportPacketFrame {
        channel_id,
        from: sender.identity.node_id,
        to: receiver.identity.node_id,
        route_hash,
        frame,
    });
    let mut channel = TransportWorkChannel::browser(transport);

    assert_eq!(channel.poll_recv().expect("poll recv"), 1);
    let inbox = channel.recv_all(receiver.identity.node_id);

    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].channel_id, channel_id);
    assert_eq!(inbox[0].from, sender.identity.node_id);
    assert_eq!(inbox[0].to, receiver.identity.node_id);
    assert_eq!(inbox[0].route_hash, route_hash);
    assert_eq!(inbox[0].packet_hash, encoded.hash);
}
