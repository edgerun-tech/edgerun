#![cfg(feature = "std")]

use std::net::TcpListener;

use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn make_store(bytes: &[u8]) -> ObjectStoreRequest {
    let job_id = [31u8; 32];
    let shard_index = 0;
    ObjectStoreRequest {
        manifest_hash: [32u8; 32],
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
    work_type: u16,
    payload: Vec<u8>,
    sequence: u64,
) -> NetworkMessage {
    let payload_hash = blake3_hash(&payload);
    let message_id = HashBuilder::domain(b"edgerun:test:native-storage-daemon")
        .node_id(&from)
        .node_id(&to)
        .u64(sequence)
        .hash(&payload_hash)
        .finish();
    sign_network_message_payload(
        key,
        message_id,
        [0u8; 32],
        from,
        to,
        to,
        if work_type == WORK_TYPE_OBJECT_RETRIEVE {
            DEPARTMENT_RETRIEVAL
        } else {
            DEPARTMENT_STORAGE
        },
        work_type,
        sequence,
        payload,
    )
}

#[test]
fn object_storage_service_handles_objects_without_tcp() {
    let storage_key = Ed25519SigningKey::from_bytes(&[89u8; 32]);
    let client_key = Ed25519SigningKey::from_bytes(&[90u8; 32]);
    let client = node_identity_from_key(&client_key, NODE_ROLE_MESSAGE);
    let mut service = ObjectStorageService::memory(storage_key.clone(), 4096);
    let storage = node_identity_from_key(&storage_key, NODE_ROLE_STORAGE);

    assert_eq!(service.identity(), &storage);
    let route = service.route_advertisement(
        memory_endpoint("browser-local-object-store", &storage.node_id),
        storage.node_id,
        u64::MAX,
    );
    assert!(verify_route_advertisement(&route));
    assert_eq!(route.endpoint.kind, CHANNEL_KIND_MEMORY);

    let object = make_store(b"transport independent object storage");
    let store_payload = storage_payload_bytes(&StoragePayload::StoreRequest(object.clone()))
        .expect("store payload");
    let store_message = signed_storage_message(
        &client_key,
        client.node_id,
        storage.node_id,
        WORK_TYPE_OBJECT_STORE,
        store_payload,
        1,
    );
    let stored = service.handle_packet(WorkPacket::NetworkMessage(store_message), 1_000);
    assert_eq!(stored.status, ROLE_STATUS_ACCEPTED);
    assert_eq!(service.object_count(), 1);

    let retrieve_payload =
        storage_payload_bytes(&StoragePayload::RetrieveRequest(retrieve_request(&object)))
            .expect("retrieve payload");
    let retrieve_message = signed_storage_message(
        &client_key,
        client.node_id,
        storage.node_id,
        WORK_TYPE_OBJECT_RETRIEVE,
        retrieve_payload,
        2,
    );
    let retrieved = service.handle_packet(WorkPacket::NetworkMessage(retrieve_message), 1_001);
    assert_eq!(retrieved.status, ROLE_STATUS_ACCEPTED);
    let StoragePayload::RetrieveResponse(response) =
        storage_payload_from_bytes(&retrieved.bytes).expect("retrieve response")
    else {
        panic!("expected retrieve response");
    };
    assert_eq!(response.bytes, object.bytes);
}

#[test]
fn native_storage_daemon_binds_object_store_to_storage_identity() {
    let storage_key = Ed25519SigningKey::from_bytes(&[91u8; 32]);
    let client_key = Ed25519SigningKey::from_bytes(&[92u8; 32]);
    let client = node_identity_from_key(&client_key, NODE_ROLE_MESSAGE);
    let mut daemon = TcpObjectStorageDaemon::bind_memory(storage_key.clone(), "127.0.0.1:0", 4096)
        .expect("bind daemon");
    let storage = node_identity_from_key(&storage_key, NODE_ROLE_STORAGE);

    assert_eq!(daemon.identity(), &storage);
    let route = daemon.route_advertisement(storage.node_id, unix_ms().saturating_add(60_000));
    assert!(verify_route_advertisement(&route));
    assert_eq!(route.node, storage);
    assert_eq!(
        route.departments,
        vec![DEPARTMENT_STORAGE, DEPARTMENT_RETRIEVAL]
    );

    let object = make_store(b"native identity-bound object storage");
    let store_payload = storage_payload_bytes(&StoragePayload::StoreRequest(object.clone()))
        .expect("store payload");
    let store_message = signed_storage_message(
        &client_key,
        client.node_id,
        storage.node_id,
        WORK_TYPE_OBJECT_STORE,
        store_payload,
        1,
    );
    let stored = send_storage_daemon_request(
        daemon.listen_addr(),
        &WorkPacket::NetworkMessage(store_message),
    )
    .expect("store request");
    assert_eq!(stored.status, ROLE_STATUS_ACCEPTED);
    let Some(WorkPacket::Ack(ack)) = stored.packet else {
        panic!("expected store ack");
    };
    assert!(ack.ok);
    assert_eq!(daemon.object_count(), 1);

    let retrieve_payload =
        storage_payload_bytes(&StoragePayload::RetrieveRequest(retrieve_request(&object)))
            .expect("retrieve payload");
    let retrieve_message = signed_storage_message(
        &client_key,
        client.node_id,
        storage.node_id,
        WORK_TYPE_OBJECT_RETRIEVE,
        retrieve_payload,
        2,
    );
    let retrieved = send_storage_daemon_request(
        daemon.listen_addr(),
        &WorkPacket::NetworkMessage(retrieve_message),
    )
    .expect("retrieve request");
    assert_eq!(retrieved.status, ROLE_STATUS_ACCEPTED);
    let StoragePayload::RetrieveResponse(response) =
        storage_payload_from_bytes(&retrieved.bytes).expect("retrieve response")
    else {
        panic!("expected retrieve response");
    };
    assert_eq!(response.bytes, object.bytes);
    assert!(verify_retrieve_response(&response));

    daemon.shutdown().expect("shutdown daemon");
}

#[test]
fn service_response_rejects_oversized_byte_body() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let client = std::thread::spawn(move || std::net::TcpStream::connect(addr).expect("connect"));
    let (mut server, _) = listener.accept().expect("accept");
    let response = WorkServiceResponse {
        status: ROLE_STATUS_ACCEPTED,
        packet: None,
        bytes: vec![0u8; MAX_WORK_FRAME_LEN + 1],
    };

    let error = write_work_service_response(&mut server, &response).expect_err("oversized body");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    drop(server);
    let _ = client.join().expect("client thread");
}
