use edgerun_work::*;

#[test]
fn memory_channel_accepts_ordered_messages_and_rejects_replay() {
    let relay = SimNode::from_seed(1, NODE_ROLE_RELAY);
    let mut sender = SimNode::from_seed(2, NODE_ROLE_MESSAGE);
    let mut receiver = SimNode::from_seed(3, NODE_ROLE_MESSAGE);

    let mut channel = MemoryChannelEngine::new();
    channel
        .add_route(relay.advertise_memory_route(relay.identity.node_id, vec![DEPARTMENT_RELAY]))
        .expect("relay route");
    channel
        .add_route(receiver.advertise_memory_route(
            relay.identity.node_id,
            vec![DEPARTMENT_MESSAGE],
        ))
        .expect("receiver route");

    let packet = sender.message_to(
        receiver.identity.node_id,
        relay.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"hello".to_vec(),
    );
    let ordered = deliver_ordered(
        &mut channel,
        &mut sender.order,
        sender.identity.node_id,
        receiver.identity.node_id,
        packet,
    )
    .expect("ordered delivery");

    let expected_route = expected_route_hash(&channel, &receiver.identity.node_id).unwrap();
    let first_hash = receiver
        .accept_ordered(&ordered, expected_route)
        .expect("receiver accepts first message");
    assert_ne!(first_hash, [0u8; 32]);

    let replay = receiver.accept_ordered(&ordered, expected_route);
    assert!(matches!(
        replay,
        Err(NodeSimError::Order(ChannelOrderError::SequenceOutOfOrder))
    ));
}

#[test]
fn memory_channel_rejects_out_of_order_and_wrong_previous_hash() {
    let relay = SimNode::from_seed(11, NODE_ROLE_RELAY);
    let mut sender = SimNode::from_seed(12, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(13, NODE_ROLE_MESSAGE);

    let mut channel = MemoryChannelEngine::new();
    channel
        .add_route(receiver.advertise_memory_route(
            relay.identity.node_id,
            vec![DEPARTMENT_MESSAGE],
        ))
        .expect("receiver route");

    let packet = sender.message_to(
        receiver.identity.node_id,
        relay.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"first".to_vec(),
    );
    let ordered = deliver_ordered(
        &mut channel,
        &mut sender.order,
        sender.identity.node_id,
        receiver.identity.node_id,
        packet,
    )
    .expect("ordered delivery");
    let expected_route = expected_route_hash(&channel, &receiver.identity.node_id).unwrap();

    let mut book = ChannelOrderBook::new();
    let mut bad_sequence = ordered.clone();
    bad_sequence.sequence = 2;
    assert!(matches!(
        book.accept(&bad_sequence, expected_route),
        Err(ChannelOrderError::SequenceOutOfOrder)
    ));

    let mut bad_previous = ordered.clone();
    bad_previous.previous_message_hash = [9u8; 32];
    assert!(matches!(
        book.accept(&bad_previous, expected_route),
        Err(ChannelOrderError::PreviousHashMismatch)
    ));
}

#[test]
fn memory_channel_rejects_tampered_packet_hash_and_wrong_route() {
    let relay = SimNode::from_seed(21, NODE_ROLE_RELAY);
    let mut sender = SimNode::from_seed(22, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(23, NODE_ROLE_MESSAGE);

    let mut channel = MemoryChannelEngine::new();
    channel
        .add_route(receiver.advertise_memory_route(
            relay.identity.node_id,
            vec![DEPARTMENT_MESSAGE],
        ))
        .expect("receiver route");

    let packet = sender.message_to(
        receiver.identity.node_id,
        relay.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"tamper-check".to_vec(),
    );
    let ordered = deliver_ordered(
        &mut channel,
        &mut sender.order,
        sender.identity.node_id,
        receiver.identity.node_id,
        packet,
    )
    .expect("ordered delivery");
    let expected_route = expected_route_hash(&channel, &receiver.identity.node_id).unwrap();

    let mut book = ChannelOrderBook::new();
    let mut bad_hash = ordered.clone();
    bad_hash.envelope.packet_hash = [7u8; 32];
    assert!(matches!(
        book.accept(&bad_hash, expected_route),
        Err(ChannelOrderError::PacketHashMismatch)
    ));

    let mut other_book = ChannelOrderBook::new();
    assert!(matches!(
        other_book.accept(&ordered, [8u8; 32]),
        Err(ChannelOrderError::RouteHashMismatch)
    ));
}
