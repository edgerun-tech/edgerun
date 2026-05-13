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
    input_hash: Hash,
    output_hash: Hash,
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
                input_hash,
                output_hash,
                sequence,
            ),
            request_hash,
            admission_hash,
            worker: worker.identity.clone(),
            relay_node_id: [0u8; 32],
            input_hash,
            output_hash,
            units_used: 1,
            total_claim: claim,
            sequence,
            signature: empty_signature(),
        },
    )
}

#[test]
fn storage_lifecycle_runs_over_generic_work_channel() {
    let admission = SimNode::from_seed(231, NODE_ROLE_ADMISSION);
    let user = public_key_from_seed(232);
    let mut client = SimNode::from_seed(233, NODE_ROLE_MESSAGE);
    let mut storage_nodes = [
        SimNode::from_seed(234, NODE_ROLE_STORAGE),
        SimNode::from_seed(235, NODE_ROLE_STORAGE),
        SimNode::from_seed(236, NODE_ROLE_STORAGE),
    ];
    let mut storage_roles = [
        ObjectStoreRole::default(),
        ObjectStoreRole::default(),
        ObjectStoreRole::default(),
    ];

    let mut channel = MemoryChannelEngine::new();
    for storage in &storage_nodes {
        let route = storage.bind_memory_route(storage.identity.node_id, vec![DEPARTMENT_STORAGE]);
        channel.add_route(route).expect("storage route");
    }

    let file = b"storage lifecycle over generic work channel".to_vec();
    let assigned_nodes = [
        storage_nodes[0].identity.node_id,
        storage_nodes[1].identity.node_id,
        storage_nodes[2].identity.node_id,
    ];
    let (manifest, shards) = encode_xor_2_1(&file, assigned_nodes).expect("encode");
    verify_manifest(&manifest, &shards).expect("manifest");

    let first_route_hash = channel
        .route_hash_for(&storage_nodes[0].identity.node_id)
        .expect("first storage route hash");
    let first_route = channel
        .route_for(&storage_nodes[0].identity.node_id)
        .expect("first storage route")
        .clone();
    let admission_doc = signed_admission(
        &admission,
        user,
        manifest.job_id,
        first_route_hash,
        first_route.endpoint,
        30,
    );
    let admission_hash = work_admission_hash(&admission_doc).expect("admission hash");

    let mut ledger = SettlementLedger::new();
    ledger.deposit_user_credit(user, 100);
    let mut shard_store = MemoryShardStore::new();

    for (index, shard) in shards.iter().enumerate() {
        let target = storage_nodes[index].identity.node_id;
        let packet = client.message_to(
            target,
            target,
            DEPARTMENT_STORAGE,
            WORK_TYPE_OBJECT_STORE,
            shard.bytes.clone(),
        );
        let ordered = channel
            .send_ordered(&mut client.order, client.identity.node_id, target, packet)
            .expect("ordered shard dispatch");
        let expected_route = channel.route_hash_for(&target).expect("target route hash");
        storage_nodes[index]
            .accept_ordered(&ordered, expected_route)
            .expect("storage accepts ordered shard");

        let context = RoleContext {
            now_unix_ms: 1,
            local_node: storage_nodes[index].identity.clone(),
            policy_hash: [0u8; 32],
        };
        let output = storage_roles[index].handle(
            &context,
            RoleInput {
                packet: ordered.envelope.packet,
                previous_hash: ordered.previous_message_hash,
                channel_hash: ordered.envelope.channel_id,
            },
        );
        assert_eq!(output.status, ROLE_STATUS_ACCEPTED);
        shard_store.put(shard.clone());

        let receipt = signed_storage_receipt(
            &storage_nodes[index],
            manifest.job_id,
            admission_hash,
            shard.hash,
            shard.hash,
            10,
            index as u64 + 1,
        );
        ledger
            .settle_receipt(&admission_doc, &receipt)
            .expect("storage receipt settles");
    }

    let retrieved = retrieve_shards(&shard_store, &manifest, &[0, 1, 2]);
    let reconstructed = reconstruct_xor_2_1(&manifest, &retrieved).expect("reconstruct");
    assert_eq!(reconstructed, file);
    assert_eq!(ledger.user_balance(&user), 70);
    for storage in &storage_nodes {
        assert_eq!(ledger.worker_balance(&storage.identity.node_id), 10);
    }
}
