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
        .add_route(receiver.bind_memory_route(relay.identity.node_id, vec![DEPARTMENT_MESSAGE]))
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
        .add_route(storage.bind_memory_route(relay.identity.node_id, vec![DEPARTMENT_STORAGE]))
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

#[test]
fn ordered_capability_message_uses_generic_capability_role() {
    let relay = SimNode::from_seed(61, NODE_ROLE_RELAY);
    let mut sender = SimNode::from_seed(62, NODE_ROLE_MESSAGE);
    let mut capability = SimNode::from_seed(63, NODE_ROLE_CAPABILITY);
    let mut role = CapabilityRole::default();

    let mut channel = MemoryChannelEngine::new();
    channel
        .add_route(capability.bind_memory_capability(relay.identity.node_id))
        .expect("capability route");

    let envelope = capability_envelope(
        [1u8; 32],
        [2u8; 32],
        blake3_hash(b"camera-0"),
        sender.identity.node_id,
        capability.identity.node_id,
        CAPABILITY_PACKET_INVOKE,
        CAPABILITY_OPERATION_STREAM_CHUNK,
        CAPABILITY_CONTENT_VIDEO,
        1,
        123,
        b"capability invocation payload".to_vec(),
    );
    let payload = encode_capability_envelope(&envelope).expect("capability envelope encodes");
    let packet = sender.message_to(
        capability.identity.node_id,
        relay.identity.node_id,
        DEPARTMENT_CAPABILITY,
        WORK_TYPE_CAPABILITY_INVOKE,
        payload,
    );
    let ordered = deliver_ordered(
        &mut channel,
        &mut sender.order,
        sender.identity.node_id,
        capability.identity.node_id,
        packet,
    )
    .expect("ordered capability delivery");

    let expected_route = expected_route_hash(&channel, &capability.identity.node_id).unwrap();
    capability
        .accept_ordered(&ordered, expected_route)
        .expect("capability accepts order");

    let output = role.handle(
        &RoleContext {
            now_unix_ms: 1,
            local_node: capability.identity.clone(),
            policy_hash: [0u8; 32],
        },
        RoleInput {
            packet: ordered.envelope.packet,
            previous_hash: ordered.previous_message_hash,
            channel_hash: ordered.envelope.channel_id,
        },
    );

    assert_eq!(output.status, ROLE_STATUS_ACCEPTED);
    assert_eq!(role.accepted_len(), 1);
    assert_eq!(
        role.accepted_messages()[0].work_type,
        WORK_TYPE_CAPABILITY_INVOKE
    );
    assert_eq!(role.accepted_envelopes()[0], envelope);
}

#[test]
fn trust_container_capability_is_the_only_chat_decryptor() {
    let relay = SimNode::from_seed(71, NODE_ROLE_RELAY);
    let sender = SimNode::from_seed(72, NODE_ROLE_MESSAGE);
    let mut chat_app = SimNode::from_seed(73, NODE_ROLE_CAPABILITY);
    let trust_key = Ed25519SigningKey::from_bytes(&[74u8; 32]);
    let wrong_trust_key = Ed25519SigningKey::from_bytes(&[75u8; 32]);
    let trust_identity = node_identity_from_key(&trust_key, NODE_ROLE_CAPABILITY);
    let plaintext = b"plaintext belongs to the portable trust container";
    let sealed = seal_message_for_recipient_with_ephemeral(
        &sender.identity,
        &trust_identity,
        plaintext,
        [76u8; 32],
        [77u8; 12],
    )
    .expect("message sealed to trust container");

    assert!(
        unseal_message_from_recipient_payload(&chat_app.key, &chat_app.identity, &sealed).is_err()
    );

    let envelope = trust_container_message_decrypt_request(
        [1u8; 32],
        [2u8; 32],
        chat_app.identity.node_id,
        trust_identity.node_id,
        1,
        123,
        sealed.clone(),
    );
    assert!(is_trust_container_message_decrypt_request(&envelope));
    let payload = encode_capability_envelope(&envelope).expect("capability envelope encodes");
    let packet = chat_app.message_to(
        trust_identity.node_id,
        relay.identity.node_id,
        DEPARTMENT_CAPABILITY,
        WORK_TYPE_CAPABILITY_INVOKE,
        payload,
    );

    let mut wrong_role = TrustContainerRole::new(wrong_trust_key);
    let wrong_output = wrong_role.handle(
        &RoleContext {
            now_unix_ms: 1,
            local_node: trust_identity.clone(),
            policy_hash: [0u8; 32],
        },
        RoleInput {
            packet: packet.clone(),
            previous_hash: [0u8; 32],
            channel_hash: [0u8; 32],
        },
    );
    assert_eq!(wrong_output.status, ROLE_STATUS_REJECTED);

    let mut trust_role = TrustContainerRole::new(trust_key);
    let output = trust_role.handle(
        &RoleContext {
            now_unix_ms: 1,
            local_node: trust_identity,
            policy_hash: [0u8; 32],
        },
        RoleInput {
            packet,
            previous_hash: [0u8; 32],
            channel_hash: [0u8; 32],
        },
    );

    assert_eq!(output.status, ROLE_STATUS_ACCEPTED);
    assert_eq!(output.bytes, plaintext);
    assert_eq!(trust_role.decrypted_count(), 1);
}
