#![cfg(feature = "std")]

use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn make_store(bytes: &[u8]) -> ObjectStoreRequest {
    let job_id = [9u8; 32];
    let shard_index = 0;
    ObjectStoreRequest {
        manifest_hash: [4u8; 32],
        job_id,
        shard_index,
        shard_hash: typed_shard_hash(job_id, shard_index, false, bytes),
        original_len: bytes.len() as u64,
        bytes: bytes.to_vec(),
    }
}

fn make_message(
    key: &Ed25519SigningKey,
    from: NodeId,
    to: NodeId,
    via_relay: NodeId,
    payload: Vec<u8>,
) -> NetworkMessage {
    let payload_hash = blake3_hash(&payload);
    let message_id = HashBuilder::domain(b"edgerun:test:storage-message")
        .node_id(&from)
        .node_id(&to)
        .node_id(&via_relay)
        .hash(&payload_hash)
        .finish();
    sign_network_message_payload(
        key,
        message_id,
        [0u8; 32],
        from,
        to,
        via_relay,
        DEPARTMENT_STORAGE,
        WORK_TYPE_OBJECT_STORE,
        1,
        payload,
    )
}

#[test]
fn admission_assigns_relay_and_relay_forwards_storage_work() {
    let admission_key = Ed25519SigningKey::from_bytes(&[40u8; 32]);
    let client_key = Ed25519SigningKey::from_bytes(&[41u8; 32]);
    let relay_key = Ed25519SigningKey::from_bytes(&[42u8; 32]);
    let storage_key = Ed25519SigningKey::from_bytes(&[43u8; 32]);

    let client = node_identity_from_key(&client_key, NODE_ROLE_MESSAGE);
    let relay_id = node_identity_from_key(&relay_key, NODE_ROLE_RELAY);
    let storage_id = node_identity_from_key(&storage_key, NODE_ROLE_STORAGE);

    let admission = InMemoryAdmissionController::new(
        admission_key,
        AdmissionConfig {
            dao_id: [1u8; 32],
            policy_hash: [2u8; 32],
            heartbeat_grace_secs: 30,
        },
    );
    let listener = TcpListener::bind("127.0.0.1:0").expect("admission listener");
    let admission_addr = listener.local_addr().expect("admission addr");
    let admission_server = admission.clone();
    let admission_thread = thread::spawn(move || {
        let mut handlers = Vec::new();
        for _ in 0..2 {
            let (stream, _) = listener.accept().expect("admission accept");
            let controller = admission_server.clone();
            handlers.push(thread::spawn(move || {
                controller
                    .handle_connection(stream)
                    .expect("admission connection");
            }));
        }
        handlers
    });

    let mut client_tcp = TcpNodeRuntime::bind(client.node_id, "127.0.0.1:0").expect("client tcp");
    let mut relay_tcp = TcpNodeRuntime::bind(relay_id.node_id, "127.0.0.1:0").expect("relay tcp");
    let storage_tcp = TcpNodeRuntime::bind(storage_id.node_id, "127.0.0.1:0").expect("storage tcp");

    let (relay_admission, relay_response) = WorkClient::connect_with_relay_endpoint(
        admission_addr,
        relay_key.clone(),
        NODE_ROLE_RELAY,
        Some(tcp_endpoint("relay", relay_tcp.listen_addr().to_string())),
    )
    .expect("relay registration");
    assert!(matches!(relay_response, WorkPacket::RelayAssignment(_)));

    let (storage_admission, storage_response) =
        WorkClient::connect(admission_addr, storage_key.clone(), NODE_ROLE_STORAGE)
            .expect("storage relay assignment");
    let WorkPacket::RelayAssignment(assignment) = storage_response else {
        panic!("expected relay assignment");
    };
    assert!(verify_relay_assignment(&assignment));
    assert_eq!(assignment.node_id, storage_id.node_id);
    assert_eq!(assignment.relay.relay_node_id, relay_id.node_id);

    let relay_route = RouteBindingBuilder::new(
        &relay_key,
        NODE_ROLE_RELAY,
        tcp_endpoint("relay", relay_tcp.listen_addr().to_string()),
    )
    .departments(vec![DEPARTMENT_RELAY])
    .valid_until_unix_ms(unix_ms().saturating_add(60_000))
    .build();
    let storage_route = RouteBindingBuilder::new(
        &storage_key,
        NODE_ROLE_STORAGE,
        tcp_endpoint("storage", storage_tcp.listen_addr().to_string()),
    )
    .relay_node_id(assignment.relay.relay_node_id)
    .departments(vec![DEPARTMENT_STORAGE, DEPARTMENT_RETRIEVAL])
    .valid_until_unix_ms(assignment.valid_until_unix_ms)
    .build();

    let relay_route_hash = client_tcp
        .add_route(relay_route.clone())
        .expect("client route");
    relay_tcp.add_route(storage_route).expect("storage route");

    let object = make_store(b"stored through assigned relay");
    let retrieve = ObjectRetrieveRequest {
        manifest_hash: object.manifest_hash,
        job_id: object.job_id,
        shard_index: object.shard_index,
        shard_hash: object.shard_hash,
    };
    let payload = storage_payload_bytes(&StoragePayload::StoreRequest(object)).expect("payload");
    let message = make_message(
        &client_key,
        client.node_id,
        storage_id.node_id,
        relay_id.node_id,
        payload,
    );
    client_tcp
        .send_unordered(
            client.node_id,
            relay_id.node_id,
            WorkPacket::NetworkMessage(message),
        )
        .expect("send to relay");

    thread::sleep(Duration::from_millis(50));
    let mut order = ChannelOrderBook::new();
    let ordered = relay_tcp.drain_ordered(
        client.node_id,
        &mut order,
        relay_route_hash,
        relay_route.endpoint.channel_id,
    );
    assert_eq!(ordered.len(), 1);

    RelayRole::from_key(relay_key, 0)
        .forward_ordered_on(&mut relay_tcp, &ordered[0], [0u8; 32], [0u8; 32])
        .expect("relay forward");

    thread::sleep(Duration::from_millis(50));
    let mut storage_role = TypedObjectStoreRole::memory_with_capacity(4096);
    let packets = storage_tcp.drain_packets();
    assert_eq!(packets.len(), 1);
    let output = storage_role.handle(
        &RoleContext {
            now_unix_ms: unix_ms(),
            local_node: storage_id,
            policy_hash: [2u8; 32],
        },
        RoleInput {
            packet: packets.into_iter().next().expect("packet"),
            previous_hash: [0u8; 32],
            channel_hash: [0u8; 32],
        },
    );
    assert_eq!(output.status, ROLE_STATUS_ACCEPTED);
    assert_eq!(
        storage_role.retrieve(&retrieve).expect("stored").bytes,
        b"stored through assigned relay"
    );

    drop(storage_admission);
    drop(relay_admission);
    client_tcp.shutdown().expect("client shutdown");
    relay_tcp.shutdown().expect("relay shutdown");
    storage_tcp.shutdown().expect("storage shutdown");
    for handler in admission_thread.join().expect("admission thread") {
        handler.join().expect("admission handler");
    }
}
