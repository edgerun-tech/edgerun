use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn store_request(bytes: &[u8]) -> ObjectStoreRequest {
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

fn retrieve_request(object: &ObjectStoreRequest) -> ObjectRetrieveRequest {
    ObjectRetrieveRequest {
        manifest_hash: object.manifest_hash,
        job_id: object.job_id,
        shard_index: object.shard_index,
        shard_hash: object.shard_hash,
    }
}

fn signed_storage_message(
    key: &Ed25519SigningKey,
    from: NodeId,
    to: NodeId,
    via_relay: NodeId,
    sequence: u64,
    payload: Vec<u8>,
) -> NetworkMessage {
    let payload_hash = blake3_hash(&payload);
    let message_id = HashBuilder::domain(b"edgerun:test:storage-message")
        .node_id(&from)
        .node_id(&to)
        .node_id(&via_relay)
        .u64(sequence)
        .hash(&payload_hash)
        .finish();
    sign_network_message(
        key,
        NetworkMessage {
            abi_version: WORK_WIRE_ABI_VERSION,
            message_id,
            prev_hash: [0u8; 32],
            from,
            to,
            via_relay,
            department: DEPARTMENT_STORAGE,
            work_type: WORK_TYPE_OBJECT_STORE,
            sequence,
            payload_hash,
            payload,
            signature: empty_signature(),
        },
    )
}

#[test]
fn admission_assigns_relay_and_relay_forwards_storage_work() {
    let admission_key = Ed25519SigningKey::from_bytes(&[40u8; 32]);
    let client_key = Ed25519SigningKey::from_bytes(&[41u8; 32]);
    let relay_key = Ed25519SigningKey::from_bytes(&[42u8; 32]);
    let storage_key = Ed25519SigningKey::from_bytes(&[43u8; 32]);

    let client = node_identity_from_key(&client_key, NODE_ROLE_MESSAGE);
    let relay_identity = node_identity_from_key(&relay_key, NODE_ROLE_RELAY);
    let storage_identity = node_identity_from_key(&storage_key, NODE_ROLE_STORAGE);

    let admission = InMemoryAdmissionController::new(
        admission_key,
        AdmissionConfig {
            dao_id: [1u8; 32],
            policy_hash: [2u8; 32],
            heartbeat_grace_secs: 30,
        },
    );
    let admission_listener = TcpListener::bind("127.0.0.1:0").expect("admission listener");
    let admission_addr = admission_listener.local_addr().expect("admission addr");
    let admission_thread = thread::spawn(move || {
        for _ in 0..2 {
            let (stream, _) = admission_listener.accept().expect("admission accept");
            admission.handle_connection(stream).expect("admission connection");
        }
    });

    let mut client_runtime = TcpNodeRuntime::bind(client.node_id, "127.0.0.1:0").expect("client tcp");
    let mut relay_runtime = TcpNodeRuntime::bind(relay_identity.node_id, "127.0.0.1:0").expect("relay tcp");
    let storage_runtime = TcpNodeRuntime::bind(storage_identity.node_id, "127.0.0.1:0").expect("storage tcp");

    let (_relay_client, relay_response) = WorkClient::connect(
        admission_addr,
        relay_key.clone(),
        NODE_ROLE_RELAY,
        relay_runtime.listen_addr().ip().to_string(),
        relay_runtime.listen_addr().port(),
    )
    .expect("relay registers with admission");
    assert!(matches!(relay_response, WorkPacket::RelayPeerList(_)));

    let (_storage_client, storage_response) = WorkClient::connect(
        admission_addr,
        storage_key.clone(),
        NODE_ROLE_STORAGE,
        storage_runtime.listen_addr().ip().to_string(),
        storage_runtime.listen_addr().port(),
    )
    .expect("storage asks admission for relay");
    let WorkPacket::RelayAssignment(assignment) = storage_response else {
        panic!("storage must receive relay assignment");
    };
    assert!(verify_relay_assignment(&assignment));
    assert_eq!(assignment.node_id, storage_identity.node_id);
    assert_eq!(assignment.relay.relay_node_id, relay_identity.node_id);

    let relay_route = RouteAdvertisementBuilder::new(
        &relay_key,
        NODE_ROLE_RELAY,
        tcp_endpoint("relay", relay_runtime.listen_addr().to_string()),
    )
    .departments(vec![DEPARTMENT_RELAY])
    .valid_until_unix_ms(unix_ms().saturating_add(60_000))
    .build(&relay_key);
    let storage_route = RouteAdvertisementBuilder::new(
        &storage_key,
        NODE_ROLE_STORAGE,
        tcp_endpoint("storage", storage_runtime.listen_addr().to_string()),
    )
    .relay_node_id(assignment.relay.relay_node_id)
    .departments(vec![DEPARTMENT_STORAGE, DEPARTMENT_RETRIEVAL])
    .valid_until_unix_ms(assignment.valid_until_unix_ms)
    .build(&storage_key);

    let relay_route_hash = client_runtime.add_route(relay_route.clone()).expect("client has relay route");
    relay_runtime.add_route(storage_route).expect("relay has assigned storage route");

    let object = store_request(b"stored through assigned relay");
    let retrieve = retrieve_request(&object);
    let payload = storage_payload_bytes(&StoragePayload::StoreRequest(object)).expect("payload bytes");
    let message = signed_storage_message(
        &client_key,
        client.node_id,
        storage_identity.node_id,
        relay_identity.node_id,
        1,
        payload,
    );
    client_runtime
        .send_unordered(client.node_id, relay_identity.node_id, WorkPacket::NetworkMessage(message))
        .expect("client sends storage work to relay");

    thread::sleep(Duration::from_millis(50));
    let mut order = ChannelOrderBook::new();
    let ordered = relay_runtime.drain_ordered(
        client.node_id,
        &mut order,
        relay_route_hash,
        relay_route.endpoint.channel_id,
    );
    assert_eq!(ordered.len(), 1);

    let mut relay_role = RelayRole::from_key(relay_key, 0);
    relay_role
        .forward_ordered_on(&mut relay_runtime, &ordered[0], [0u8; 32], [0u8; 32])
        .expect("relay forwards to storage");

    thread::sleep(Duration::from_millis(50));
    let mut storage_role = TypedObjectStoreRole::memory_with_capacity(4096);
    let outputs = storage_runtime
        .drain_packets()
        .into_iter()
        .map(|packet| {
            storage_role.handle(
                &RoleContext {
                    now_unix_ms: unix_ms(),
                    local_node: storage_identity.clone(),
                    policy_hash: [2u8; 32],
                },
                RoleInput {
                    packet,
                    previous_hash: [0u8; 32],
                    channel_hash: [0u8; 32],
                },
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].status, ROLE_STATUS_ACCEPTED);
    assert_eq!(storage_role.object_count(), 1);
    assert_eq!(
        storage_role.retrieve(&retrieve).expect("stored object").bytes,
        b"stored through assigned relay"
    );

    client_runtime.shutdown().expect("client shutdown");
    relay_runtime.shutdown().expect("relay shutdown");
    storage_runtime.shutdown().expect("storage shutdown");
    admission_thread.join().expect("admission thread");
}
