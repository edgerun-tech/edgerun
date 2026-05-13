use edgerun_work::*;

#[test]
fn recipient_signed_channel_proof_verifies_for_ordered_delivery() {
    let mut sender = SimNode::from_seed(151, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(152, NODE_ROLE_MESSAGE);
    let relay = RelayRole::from_seed(153, 1);
    let mut channel = MemoryChannelEngine::new();
    let route = receiver.bind_memory_route(relay.identity.node_id, vec![DEPARTMENT_MESSAGE]);
    let route_hash = channel.add_route(route).expect("receiver route");
    let packet = sender.message_to(
        receiver.identity.node_id,
        relay.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"prove recipient got this".to_vec(),
    );
    let envelope = channel
        .deliver(sender.identity.node_id, receiver.identity.node_id, packet)
        .expect("deliver to receiver");
    let ordered = OrderedChannelEnvelope {
        envelope,
        sequence: 1,
        previous_message_hash: [0u8; 32],
    };
    let mut receiver_order = ChannelOrderBook::new();
    let accepted_hash = receiver_order
        .accept(&ordered, route_hash)
        .expect("receiver accepts ordered delivery");

    let proof = channel_proof_for_ordered(
        &receiver.key,
        &receiver.identity,
        relay.identity.node_id,
        &ordered,
    )
    .expect("recipient signs channel proof");

    assert_eq!(proof.message_hash, accepted_hash);
    assert!(verify_channel_proof(&proof, &receiver.identity));
    verify_channel_proof_for_ordered(&proof, &receiver.identity, relay.identity.node_id, &ordered)
        .expect("proof matches ordered delivery");
}

#[test]
fn channel_proof_rejects_wrong_relay_recipient_or_message() {
    let mut sender = SimNode::from_seed(154, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(155, NODE_ROLE_MESSAGE);
    let wrong_receiver = SimNode::from_seed(156, NODE_ROLE_MESSAGE);
    let relay = RelayRole::from_seed(157, 1);
    let wrong_relay = RelayRole::from_seed(158, 1);
    let mut channel = MemoryChannelEngine::new();
    let route = receiver.bind_memory_route(relay.identity.node_id, vec![DEPARTMENT_MESSAGE]);
    let route_hash = channel.add_route(route).expect("receiver route");
    let packet = sender.message_to(
        receiver.identity.node_id,
        relay.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"tamper proof".to_vec(),
    );
    let envelope = channel
        .deliver(sender.identity.node_id, receiver.identity.node_id, packet)
        .expect("deliver to receiver");
    let ordered = OrderedChannelEnvelope {
        envelope,
        sequence: 1,
        previous_message_hash: [0u8; 32],
    };
    let mut receiver_order = ChannelOrderBook::new();
    receiver_order
        .accept(&ordered, route_hash)
        .expect("receiver accepts ordered delivery");
    let proof = channel_proof_for_ordered(
        &receiver.key,
        &receiver.identity,
        relay.identity.node_id,
        &ordered,
    )
    .expect("recipient signs channel proof");

    assert_eq!(
        verify_channel_proof_for_ordered(
            &proof,
            &receiver.identity,
            wrong_relay.identity.node_id,
            &ordered
        ),
        Err(ChannelProofError::WrongRelay)
    );
    assert_eq!(
        verify_channel_proof_for_ordered(
            &proof,
            &wrong_receiver.identity,
            relay.identity.node_id,
            &ordered
        ),
        Err(ChannelProofError::WrongRecipient)
    );

    let mut tampered = ordered.clone();
    tampered.sequence = 2;
    assert_eq!(
        verify_channel_proof_for_ordered(
            &proof,
            &receiver.identity,
            relay.identity.node_id,
            &tampered
        ),
        Err(ChannelProofError::WrongMessage)
    );
}

#[test]
fn channel_proof_rejects_signature_from_wrong_key() {
    let mut sender = SimNode::from_seed(159, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(160, NODE_ROLE_MESSAGE);
    let wrong_signer = SimNode::from_seed(161, NODE_ROLE_MESSAGE);
    let relay = RelayRole::from_seed(162, 1);
    let mut channel = MemoryChannelEngine::new();
    let route = receiver.bind_memory_route(relay.identity.node_id, vec![DEPARTMENT_MESSAGE]);
    let route_hash = channel.add_route(route).expect("receiver route");
    let packet = sender.message_to(
        receiver.identity.node_id,
        relay.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"wrong key proof".to_vec(),
    );
    let envelope = channel
        .deliver(sender.identity.node_id, receiver.identity.node_id, packet)
        .expect("deliver to receiver");
    let ordered = OrderedChannelEnvelope {
        envelope,
        sequence: 1,
        previous_message_hash: [0u8; 32],
    };
    let mut receiver_order = ChannelOrderBook::new();
    receiver_order
        .accept(&ordered, route_hash)
        .expect("receiver accepts ordered delivery");

    let proof = channel_proof_for_ordered(
        &wrong_signer.key,
        &receiver.identity,
        relay.identity.node_id,
        &ordered,
    )
    .expect("wrong signer can build malformed proof");

    assert!(!verify_channel_proof(&proof, &receiver.identity));
    assert_eq!(
        verify_channel_proof_for_ordered(
            &proof,
            &receiver.identity,
            relay.identity.node_id,
            &ordered
        ),
        Err(ChannelProofError::InvalidSignature)
    );
}
