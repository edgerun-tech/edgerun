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
            assigned_route_hash: route_hash,
            assigned_channel: channel,
            admitted_budget: budget,
            policy_hash: [0u8; 32],
            sequence: 1,
            valid_until_unix_ms: u64::MAX,
            signature: empty_signature(),
        },
    )
}

fn signed_receipt(
    worker: &SimNode,
    request_hash: Hash,
    admission_hash: Hash,
    shard: &ErasureShard,
    claim: u64,
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
                1,
            ),
            request_hash,
            admission_hash,
            worker: worker.identity.clone(),
            relay_node_id: [0u8; 32],
            input_hash: shard.hash,
            output_hash: shard.hash,
            units_used: shard.bytes.len() as u64,
            total_claim: claim,
            sequence: 1,
            signature: empty_signature(),
        },
    )
}

#[test]
fn wasm_worker_node_processes_storage_work_over_ws_channel() {
    let admission = SimNode::from_seed(1, NODE_ROLE_ADMISSION);
    let user = public_key_from_seed(2);
    let mut client = SimNode::from_seed(3, NODE_ROLE_MESSAGE);
    let wasm_storage = SimNode::from_seed(4, NODE_ROLE_STORAGE);

    let mut worker = WasmWorkerNode::new(
        wasm_storage.identity.clone(),
        ObjectStoreRole::default(),
        [0u8; 32],
    );

    let mut route = wasm_storage.advertise_memory_route(
        wasm_storage.identity.node_id,
        vec![DEPARTMENT_STORAGE],
    );
    route.endpoint.kind = CHANNEL_KIND_WEBSOCKET;
    route.endpoint.address = b"ws://127.0.0.1:8080/edgerun".to_vec();
    route.endpoint.label = "browser-worker-ws".into();
    route = sign_route_advertisement(&wasm_storage.key, route);
    let route_hash = worker.add_route(route.clone()).expect("ws route");

    let file = b"browser worker wasm websocket storage shard".to_vec();
    let assigned_nodes = [
        wasm_storage.identity.node_id,
        wasm_storage.identity.node_id,
        wasm_storage.identity.node_id,
    ];
    let (manifest, shards) = encode_xor_2_1(&file, assigned_nodes).expect("encode");
    let shard = &shards[0];

    let admission_doc = signed_admission(
        &admission,
        user,
        manifest.job_id,
        route_hash,
        route.endpoint.clone(),
        10,
    );
    let admission_hash = work_admission_hash(&admission_doc).expect("admission hash");

    let packet = client.message_to(
        wasm_storage.identity.node_id,
        wasm_storage.identity.node_id,
        DEPARTMENT_STORAGE,
        WORK_TYPE_OBJECT_STORE,
        shard.bytes.clone(),
    );
    let ordered = worker
        .channel
        .send_ordered(
            &mut client.order,
            client.identity.node_id,
            wasm_storage.identity.node_id,
            packet,
        )
        .expect("ws ordered send");
    assert_eq!(ordered.envelope.route_hash, route_hash);

    let frame_bytes = channel_envelope_bytes(&ordered.envelope).expect("encode envelope frame");
    let mut ws_frames = worker.drain_outbound_ws_frames();
    assert_eq!(ws_frames.len(), 1);
    assert_eq!(ws_frames[0].route_hash, route_hash);
    assert_eq!(ws_frames[0].to, wasm_storage.identity.node_id);
    assert_eq!(ws_frames[0].envelope_bytes, frame_bytes);
    assert_eq!(ws_frames[0].packet_hash, ordered.envelope.packet_hash);
    assert_ne!(ws_frames[0].packet_bytes, ws_frames[0].envelope_bytes);

    let decoded = channel_envelope_from_bytes(&ws_frames.remove(0).envelope_bytes)
        .expect("decode envelope frame");
    assert_eq!(decoded.packet_hash, ordered.envelope.packet_hash);

    let output = worker
        .accept_inbound_bytes(&frame_bytes, 1)
        .expect("wasm worker accepts inbound byte frame");
    assert_eq!(output.status, ROLE_STATUS_ACCEPTED);
    assert_eq!(worker.role.object_count(), 1);

    let receipt = signed_receipt(&wasm_storage, manifest.job_id, admission_hash, shard, 10);
    let mut ledger = SettlementLedger::new();
    ledger.deposit_user_credit(user, 100);
    ledger
        .settle_receipt(&admission_doc, &receipt)
        .expect("wasm storage receipt settles");
    assert_eq!(ledger.user_balance(&user), 90);
    assert_eq!(ledger.worker_balance(&wasm_storage.identity.node_id), 10);
}
