use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

#[test]
fn ordered_message_can_be_executed_by_message_role_library() {
    let relay = SimNode::from_seed(31, NODE_ROLE_RELAY);
    let mut sender = SimNode::from_seed(32, NODE_ROLE_MESSAGE);
    let mut receiver = SimNode::from_seed(33, NODE_ROLE_MESSAGE);
    let mut role = MessageRole::default();

    let mut channel = MemoryChannelEngine::new();
    channel
        .add_route(
            receiver.advertise_memory_route(relay.identity.node_id, vec![DEPARTMENT_MESSAGE]),
        )
        .expect("receiver route");

    let packet = sender.message_to(
        receiver.identity.node_id,
        relay.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        seal_message_for_recipient_with_ephemeral(
            &sender.identity,
            &receiver.identity,
            b"hello role",
            [77u8; 32],
            [88u8; 12],
        )
        .expect("sealed message"),
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
    receiver
        .accept_ordered(&ordered, expected_route)
        .expect("receiver accepts order");

    let context = RoleContext {
        now_unix_ms: 1,
        local_node: receiver.identity.clone(),
        policy_hash: [0u8; 32],
    };
    let output = role.handle(
        &context,
        RoleInput {
            packet: ordered.envelope.packet,
            previous_hash: ordered.previous_message_hash,
            channel_hash: ordered.envelope.channel_id,
        },
    );

    assert_eq!(output.status, ROLE_STATUS_ACCEPTED);
    assert_eq!(role.delivered_len(), 1);
    assert_eq!(
        role.delivered()[0].payload_hash,
        chat_payload_hash(b"hello role")
    );
}

#[test]
fn ordered_storage_payload_can_be_executed_by_object_store_role_library() {
    let relay = SimNode::from_seed(41, NODE_ROLE_RELAY);
    let mut sender = SimNode::from_seed(42, NODE_ROLE_MESSAGE);
    let mut storage = SimNode::from_seed(43, NODE_ROLE_STORAGE);
    let mut role = ObjectStoreRole::default();

    let mut channel = MemoryChannelEngine::new();
    channel
        .add_route(storage.advertise_memory_route(relay.identity.node_id, vec![DEPARTMENT_STORAGE]))
        .expect("storage route");

    let payload = b"object bytes".to_vec();
    let expected_object_hash = blake3_hash(&payload);
    let packet = sender.message_to(
        storage.identity.node_id,
        relay.identity.node_id,
        DEPARTMENT_STORAGE,
        WORK_TYPE_OBJECT_STORE,
        payload.clone(),
    );
    let ordered = deliver_ordered(
        &mut channel,
        &mut sender.order,
        sender.identity.node_id,
        storage.identity.node_id,
        packet,
    )
    .expect("ordered delivery");

    let expected_route = expected_route_hash(&channel, &storage.identity.node_id).unwrap();
    storage
        .accept_ordered(&ordered, expected_route)
        .expect("storage accepts order");

    let context = RoleContext {
        now_unix_ms: 1,
        local_node: storage.identity.clone(),
        policy_hash: [0u8; 32],
    };
    let output = role.handle(
        &context,
        RoleInput {
            packet: ordered.envelope.packet,
            previous_hash: ordered.previous_message_hash,
            channel_hash: ordered.envelope.channel_id,
        },
    );

    assert_eq!(output.status, ROLE_STATUS_ACCEPTED);
    assert_eq!(role.object_count(), 1);
    assert_eq!(role.get(&expected_object_hash), Some(payload.as_slice()));
}

#[test]
fn role_execution_rejects_same_key_wrong_role_capability() {
    let key = Ed25519SigningKey::from_bytes(&[55u8; 32]);
    let storage_identity = node_identity_from_key(&key, NODE_ROLE_STORAGE);
    let compute_identity = node_identity_from_key(&key, NODE_ROLE_COMPUTE);
    assert_eq!(storage_identity.node_id, compute_identity.node_id);

    let mut sender = SimNode::from_seed(56, NODE_ROLE_MESSAGE);
    let packet = sender.message_to(
        storage_identity.node_id,
        storage_identity.node_id,
        DEPARTMENT_STORAGE,
        WORK_TYPE_OBJECT_STORE,
        b"object bytes".to_vec(),
    );
    let mut role = ObjectStoreRole::default();
    let output = role.handle(
        &RoleContext {
            now_unix_ms: 1,
            local_node: compute_identity,
            policy_hash: [0u8; 32],
        },
        RoleInput {
            packet,
            previous_hash: [0u8; 32],
            channel_hash: [0u8; 32],
        },
    );

    assert_eq!(output.status, ROLE_STATUS_IGNORED);
    assert_eq!(role.object_count(), 0);
}
