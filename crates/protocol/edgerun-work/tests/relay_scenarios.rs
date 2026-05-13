use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn public_key_from_seed(seed: u8) -> PublicKey {
    let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
    let mut out = [0u8; 32];
    out.copy_from_slice(key.verifying_key().as_bytes());
    out
}

fn signed_relay_admission(
    admission: &SimNode,
    user: PublicKey,
    request_hash: Hash,
    assigned_route_commitment: Hash,
    assigned_channel: ChannelEndpoint,
    admitted_budget: u64,
    policy_hash: Hash,
) -> WorkAdmission {
    sign_work_admission(
        &admission.key,
        WorkAdmission {
            abi_version: WORK_WIRE_ABI_VERSION,
            admission_id: blake3_hash(&request_hash),
            dao_id: admission.identity.public_key,
            user,
            admission_node: admission.identity.clone(),
            request_hash,
            assigned_route_commitment,
            assigned_channel,
            assigned_relay_path: vec![[0u8; 32]],
            admitted_budget,
            policy_hash,
            sequence: 1,
            valid_until_unix_ms: u64::MAX,
            signature: empty_signature(),
        },
    )
}

#[test]
fn relay_forwards_ordered_message_to_final_node_and_gets_paid_with_delivery_proof() {
    let admission = SimNode::from_seed(141, NODE_ROLE_ADMISSION);
    let user = public_key_from_seed(142);
    let mut sender = SimNode::from_seed(143, NODE_ROLE_MESSAGE);
    let mut receiver = SimNode::from_seed(144, NODE_ROLE_MESSAGE);
    let relay_node = SimNode::from_seed(145, NODE_ROLE_RELAY);
    let mut relay = RelayRole::from_seed(145, 3);

    let mut channel = MemoryChannelEngine::new();
    let relay_route =
        relay_node.bind_memory_route(relay_node.identity.node_id, vec![DEPARTMENT_RELAY]);
    let receiver_route =
        receiver.bind_memory_route(relay_node.identity.node_id, vec![DEPARTMENT_MESSAGE]);
    let relay_route_hash = channel.add_route(relay_route.clone()).expect("relay route");
    let receiver_route_hash = channel.add_route(receiver_route).expect("receiver route");
    let mut recipient_policy = open_recipient_message_policy(receiver.identity.clone(), 1, 1_000);
    recipient_policy.allowed_relays.push(relay.identity.node_id);
    recipient_policy = sign_recipient_message_policy(&receiver.key, recipient_policy);
    let policy_hash = recipient_message_policy_hash(&recipient_policy);

    let request_hash = blake3_hash(b"relay-paid-request");
    let admission_doc = signed_relay_admission(
        &admission,
        user,
        request_hash,
        relay_route_hash,
        relay_route.endpoint.clone(),
        10,
        policy_hash,
    );
    let admission_hash = work_admission_hash(&admission_doc).expect("admission hash");

    let packet = sender.message_to(
        receiver.identity.node_id,
        relay_node.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"relay me".to_vec(),
    );
    let to_relay = deliver_ordered(
        &mut channel,
        &mut sender.order,
        sender.identity.node_id,
        relay_node.identity.node_id,
        packet,
    )
    .expect("sender to relay ordered delivery");

    let result = relay
        .forward_ordered(&mut channel, &to_relay, request_hash, admission_hash)
        .expect("relay forwards");
    assert_eq!(result.delivered_to, receiver.identity.node_id);
    assert_eq!(result.destination_route_hash, receiver_route_hash);
    assert_eq!(result.transit_receipt.total_claim, 3);
    assert_eq!(result.transit_receipt.output_hash, result.transit_hash);
    assert_eq!(relay.last_transit_hash, result.transit_hash);
    assert_ne!(result.transit_hash, result.forwarded_packet_hash);

    let mut inbox = channel.drain_inbox(receiver.identity.node_id);
    assert_eq!(inbox.len(), 1);
    let forwarded = OrderedChannelEnvelope {
        envelope: inbox.remove(0),
        sequence: 1,
        previous_message_hash: [0u8; 32],
    };
    receiver
        .accept_ordered(&forwarded, receiver_route_hash)
        .expect("receiver accepts relay forwarded message");
    let recipient_proof = channel_proof_for_allowed_ordered_message(
        &receiver.key,
        &receiver.identity,
        relay.identity.node_id,
        &forwarded,
        &recipient_policy,
        999,
    )
    .expect("receiver signs policy-gated delivery proof");
    let payable_receipt =
        relay.finalized_delivery_receipt(&result, channel_proof_hash(&recipient_proof));

    let evidence = DeliverySettlementEvidence {
        admission: &admission_doc,
        receipt: &payable_receipt,
        relay_input: &to_relay,
        recipient_delivery: &forwarded,
        recipient: &receiver.identity,
        recipient_policy: &recipient_policy,
        recipient_proof: &recipient_proof,
        previous_transit_hash: [0u8; 32],
        now_unix_ms: 999,
    };

    let mut ledger = SettlementLedger::new();
    ledger.deposit_user_credit(user, 10);
    let settle = ledger
        .settle_delivery(&evidence)
        .expect("relay delivery receipt settles only with proof");
    assert_eq!(settle.amount, 3);
    assert_eq!(ledger.user_balance(&user), 7);
    assert_eq!(ledger.worker_balance(&relay.identity.node_id), 3);
}

#[test]
fn relay_transit_hashes_chain_across_forwarded_packets() {
    let mut sender = SimNode::from_seed(146, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(147, NODE_ROLE_MESSAGE);
    let relay_node = SimNode::from_seed(148, NODE_ROLE_RELAY);
    let mut relay = RelayRole::from_seed(148, 1);
    let mut channel = MemoryChannelEngine::new();
    channel
        .add_route(
            relay_node.bind_memory_route(relay_node.identity.node_id, vec![DEPARTMENT_RELAY]),
        )
        .expect("relay route");
    channel
        .add_route(
            receiver.bind_memory_route(relay_node.identity.node_id, vec![DEPARTMENT_MESSAGE]),
        )
        .expect("receiver route");

    let first = sender.message_to(
        receiver.identity.node_id,
        relay_node.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"first".to_vec(),
    );
    let first_to_relay = deliver_ordered(
        &mut channel,
        &mut sender.order,
        sender.identity.node_id,
        relay_node.identity.node_id,
        first,
    )
    .expect("first ordered");
    let first_result = relay
        .forward_ordered(&mut channel, &first_to_relay, [1u8; 32], [2u8; 32])
        .expect("first forwarded");

    let second = sender.message_to(
        receiver.identity.node_id,
        relay_node.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"second".to_vec(),
    );
    let second_to_relay = deliver_ordered(
        &mut channel,
        &mut sender.order,
        sender.identity.node_id,
        relay_node.identity.node_id,
        second,
    )
    .expect("second ordered");
    let second_result = relay
        .forward_ordered(&mut channel, &second_to_relay, [1u8; 32], [2u8; 32])
        .expect("second forwarded");

    assert_ne!(first_result.transit_hash, second_result.transit_hash);
    assert_eq!(relay.last_transit_hash, second_result.transit_hash);
    assert_eq!(
        first_result.transit_receipt.output_hash,
        first_result.transit_hash
    );
    assert_eq!(
        second_result.transit_receipt.output_hash,
        second_result.transit_hash
    );
    assert_eq!(first_result.transit_receipt.sequence, 1);
    assert_eq!(second_result.transit_receipt.sequence, 2);
}

#[test]
fn relay_rejects_message_for_wrong_relay() {
    let mut sender = SimNode::from_seed(151, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(152, NODE_ROLE_MESSAGE);
    let relay_a = SimNode::from_seed(153, NODE_ROLE_RELAY);
    let relay_b = SimNode::from_seed(154, NODE_ROLE_RELAY);
    let mut relay_role_b = RelayRole::from_seed(154, 1);

    let mut channel = MemoryChannelEngine::new();
    channel
        .add_route(relay_b.bind_memory_route(relay_b.identity.node_id, vec![DEPARTMENT_RELAY]))
        .expect("relay b route");
    channel
        .add_route(receiver.bind_memory_route(relay_b.identity.node_id, vec![DEPARTMENT_MESSAGE]))
        .expect("receiver route");

    let packet = sender.message_to(
        receiver.identity.node_id,
        relay_a.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"wrong relay".to_vec(),
    );
    let to_relay_b = deliver_ordered(
        &mut channel,
        &mut sender.order,
        sender.identity.node_id,
        relay_b.identity.node_id,
        packet,
    )
    .expect("delivered to relay b envelope");

    assert!(matches!(
        relay_role_b.forward_ordered(
            &mut channel,
            &to_relay_b,
            blake3_hash(b"request"),
            blake3_hash(b"admission")
        ),
        Err(RelayRoleError::WrongRelay)
    ));
}

#[test]
fn relay_rejects_when_destination_route_is_missing() {
    let mut sender = SimNode::from_seed(161, NODE_ROLE_MESSAGE);
    let receiver = SimNode::from_seed(162, NODE_ROLE_MESSAGE);
    let relay_node = SimNode::from_seed(163, NODE_ROLE_RELAY);
    let mut relay = RelayRole::from_seed(163, 1);

    let mut channel = MemoryChannelEngine::new();
    channel
        .add_route(
            relay_node.bind_memory_route(relay_node.identity.node_id, vec![DEPARTMENT_RELAY]),
        )
        .expect("relay route");

    let packet = sender.message_to(
        receiver.identity.node_id,
        relay_node.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"missing destination".to_vec(),
    );
    let to_relay = deliver_ordered(
        &mut channel,
        &mut sender.order,
        sender.identity.node_id,
        relay_node.identity.node_id,
        packet,
    )
    .expect("sender to relay ordered delivery");

    assert!(matches!(
        relay.forward_ordered(
            &mut channel,
            &to_relay,
            blake3_hash(b"request"),
            blake3_hash(b"admission")
        ),
        Err(RelayRoleError::DestinationRouteMissing)
    ));
}
