#![cfg(feature = "std")]

use std::thread;
use std::time::Duration;

use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn public_key_from_seed(seed: u8) -> PublicKey {
    let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
    let mut out = [0u8; 32];
    out.copy_from_slice(key.verifying_key().as_bytes());
    out
}

fn signed_admission(
    admission: &SimNode,
    user: PublicKey,
    request_hash: Hash,
    route_hash: Hash,
    channel: ChannelEndpoint,
    budget: u64,
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
            assigned_route_commitment: route_hash,
            assigned_channel: channel,
            assigned_relay_path: vec![[0u8; 32]],
            admitted_budget: budget,
            policy_hash: [0u8; 32],
            sequence: 1,
            valid_until_unix_ms: u64::MAX,
            signature: empty_signature(),
        },
    )
}

fn signed_storage_receipt(
    worker: &SimNode,
    request_hash: Hash,
    admission_hash: Hash,
    shard: &ErasureShard,
    claim: u64,
    sequence: u64,
) -> WorkReceipt {
    sign_work_receipt(
        &worker.key,
        WorkReceipt {
            abi_version: WORK_WIRE_ABI_VERSION,
            receipt_id: receipt_id_for_claim(
                request_hash,
                admission_hash,
                worker.identity.node_id,
                shard.hash,
                shard.hash,
                sequence,
            ),
            request_hash,
            admission_hash,
            worker: worker.identity.clone(),
            relay_node_id: [0u8; 32],
            input_hash: shard.hash,
            output_hash: shard.hash,
            units_used: shard.bytes.len() as u64,
            total_claim: claim,
            sequence,
            signature: empty_signature(),
        },
    )
}

#[test]
fn tcp_runtime_stores_erasure_shards_and_settles_receipts() {
    let admission = SimNode::from_seed(1, NODE_ROLE_ADMISSION);
    let user = public_key_from_seed(2);
    let mut client = SimNode::from_seed(3, NODE_ROLE_MESSAGE);
    let mut storage = SimNode::from_seed(4, NODE_ROLE_STORAGE);
    let mut role = ObjectStoreRole::default();
    let mut runtime = TcpNodeRuntime::bind(storage.identity.node_id, "127.0.0.1:0")
        .expect("bind storage tcp runtime");

    let mut route = storage.bind_memory_route(storage.identity.node_id, vec![DEPARTMENT_STORAGE]);
    route.endpoint.kind = CHANNEL_KIND_TCP;
    route.endpoint.address = runtime.listen_addr().to_string().into_bytes();
    route.endpoint.label = "tcp-storage-loopback".into();
    let route_hash = runtime.add_route(route.clone()).expect("storage tcp route");

    let file = b"tcp runtime erasure storage lifecycle payload".to_vec();
    let assigned_nodes = [
        storage.identity.node_id,
        storage.identity.node_id,
        storage.identity.node_id,
    ];
    let (manifest, shards) = encode_xor_2_1(&file, assigned_nodes).expect("encode");
    verify_manifest(&manifest, &shards).expect("manifest");

    let admission_doc = signed_admission(
        &admission,
        user,
        manifest.job_id,
        route_hash,
        route.endpoint.clone(),
        30,
    );
    let admission_hash = work_admission_hash(&admission_doc).expect("admission hash");
    let mut ledger = SettlementLedger::new();
    ledger.deposit_user_credit(user, 100);
    let mut shard_store = MemoryShardStore::new();

    for shard in &shards {
        let packet = client.message_to(
            storage.identity.node_id,
            storage.identity.node_id,
            DEPARTMENT_STORAGE,
            WORK_TYPE_OBJECT_STORE,
            shard.bytes.clone(),
        );
        let sent = runtime
            .send_ordered(
                &mut client.order,
                client.identity.node_id,
                storage.identity.node_id,
                packet,
            )
            .expect("tcp ordered send");
        assert_eq!(sent.envelope.route_hash, route_hash);
    }

    let mut packets = Vec::new();
    for _ in 0..100 {
        packets.extend(runtime.drain_packets());
        if packets.len() == shards.len() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(packets.len(), shards.len());

    let mut receiver_order = ChannelOrderBook::new();
    for (index, packet) in packets.into_iter().enumerate() {
        let packet_hash = packet_bytes(&packet)
            .map(|bytes| blake3_hash(&bytes))
            .unwrap();
        let channel_id = route.endpoint.channel_id;
        let sequence = receiver_order.next_sequence(
            channel_id,
            client.identity.node_id,
            storage.identity.node_id,
        );
        let previous_message_hash = receiver_order.last_message_hash(
            channel_id,
            client.identity.node_id,
            storage.identity.node_id,
        );
        let ordered = OrderedChannelEnvelope {
            envelope: ChannelEnvelope {
                abi_version: WORK_WIRE_ABI_VERSION,
                channel_id,
                from: client.identity.node_id,
                to: storage.identity.node_id,
                route_hash,
                packet_hash,
                packet,
            },
            sequence,
            previous_message_hash,
        };
        receiver_order
            .accept(&ordered, route_hash)
            .expect("ordered receive");
        storage
            .accept_ordered(&ordered, route_hash)
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
        shard_store.put(shards[index].clone());

        let receipt = signed_storage_receipt(
            &storage,
            manifest.job_id,
            admission_hash,
            &shards[index],
            10,
            index as u64 + 1,
        );
        ledger
            .settle_receipt(&admission_doc, &receipt)
            .expect("settle shard receipt");
    }

    let retrieved = retrieve_shards(&shard_store, &manifest, &[0, 1, 2]);
    assert_eq!(
        reconstruct_xor_2_1(&manifest, &retrieved).expect("reconstruct"),
        file
    );
    assert_eq!(ledger.user_balance(&user), 70);
    assert_eq!(ledger.worker_balance(&storage.identity.node_id), 30);
}
