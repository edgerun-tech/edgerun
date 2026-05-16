use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn public_key_from_seed(seed: u8) -> PublicKey {
    let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
    let mut out = [0u8; 32];
    out.copy_from_slice(key.verifying_key().as_bytes());
    out
}

fn deterministic_bytes(len: usize, seed: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut x = seed as u64 + 1;
    for _ in 0..len {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        out.push((x >> 32) as u8);
    }
    out
}

fn signed_admission(
    admission: &SimNode,
    user: PublicKey,
    request_hash: Hash,
    assigned_route_commitment: Hash,
    assigned_channel: ChannelEndpoint,
    admitted_budget: u64,
    sequence: u64,
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
            policy_hash: [0u8; 32],
            sequence,
            valid_until_unix_ms: u64::MAX,
            signature: empty_signature(),
        },
    )
}

fn signed_receipt(
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
fn scaled_storage_retrieval_and_relay_costs_match_settlement() {
    let admission = SimNode::from_seed(1, NODE_ROLE_ADMISSION);
    let user = public_key_from_seed(2);
    let relays = [
        SimNode::from_seed(3, NODE_ROLE_RELAY),
        SimNode::from_seed(4, NODE_ROLE_RELAY),
        SimNode::from_seed(5, NODE_ROLE_RELAY),
        SimNode::from_seed(6, NODE_ROLE_RELAY),
    ];
    let storage_nodes = [
        SimNode::from_seed(7, NODE_ROLE_STORAGE),
        SimNode::from_seed(8, NODE_ROLE_STORAGE),
        SimNode::from_seed(9, NODE_ROLE_STORAGE),
        SimNode::from_seed(10, NODE_ROLE_STORAGE),
        SimNode::from_seed(11, NODE_ROLE_STORAGE),
        SimNode::from_seed(12, NODE_ROLE_STORAGE),
        SimNode::from_seed(13, NODE_ROLE_STORAGE),
        SimNode::from_seed(14, NODE_ROLE_STORAGE),
    ];

    let mut routes = Vec::new();
    for relay in &relays {
        routes.push(relay.bind_memory_route(relay.identity.node_id, vec![DEPARTMENT_RELAY]));
    }
    for (index, storage) in storage_nodes.iter().enumerate() {
        let relay = &relays[index % relays.len()];
        routes.push(storage.bind_memory_route(relay.identity.node_id, vec![DEPARTMENT_STORAGE]));
    }
    let storage_routes = routes
        .iter()
        .filter(|route| route.departments.contains(&DEPARTMENT_STORAGE))
        .collect::<Vec<_>>();
    assert_eq!(storage_routes.len(), storage_nodes.len());

    let prices = UnitPriceTable {
        store_per_kib_epoch: 2,
        retrieve_per_kib: 3,
        relay_per_kib: 1,
        receipt_base: 1,
    };
    let file_sizes = [512usize, 1024, 4097, 65_536, 262_145, 1_048_576];
    let epochs = 30;

    let mut ledger = SettlementLedger::new();
    ledger.deposit_user_credit(user, 50_000_000);
    let initial_balance = ledger.user_balance(&user);
    let mut shard_store = MemoryShardStore::new();
    let mut expected_total = 0u64;
    let mut expected_storage_total = 0u64;
    let mut expected_retrieval_total = 0u64;
    let mut expected_relay_total = 0u64;

    for (file_index, file_size) in file_sizes.iter().copied().enumerate() {
        let assigned_nodes = [
            storage_routes[(file_index * 3) % storage_routes.len()]
                .node
                .node_id,
            storage_routes[(file_index * 3 + 1) % storage_routes.len()]
                .node
                .node_id,
            storage_routes[(file_index * 3 + 2) % storage_routes.len()]
                .node
                .node_id,
        ];
        let file = deterministic_bytes(file_size, file_index as u8 + 50);
        let (manifest, shards) =
            encode_xor_2_1(&file, assigned_nodes).expect("encode erasure file");
        verify_manifest(&manifest, &shards).expect("manifest verifies");
        assert_eq!(manifest.original_hash, blake3_hash(&file));

        let relay_nodes = relays
            .iter()
            .map(|relay| relay.identity.node_id)
            .collect::<Vec<_>>();
        let estimate =
            estimate_erasure_storage_cost(&manifest, &shards, &relay_nodes, epochs, prices);
        assert_eq!(estimate.original_bytes, file_size as u64);
        assert_eq!(
            estimate.stored_bytes,
            shards.iter().map(|s| s.bytes.len() as u64).sum::<u64>()
        );
        assert!(estimate.erasure_overhead_bps >= 15_000 || file_size <= 1);
        assert_eq!(
            estimate.total,
            estimate.storage_total + estimate.retrieval_total + estimate.relay_total
        );
        assert_ne!(estimate.estimate_hash, [0u8; 32]);

        let first_route = routes
            .iter()
            .find(|route| route.node.node_id == assigned_nodes[0])
            .expect("assigned route");
        let admission_doc = signed_admission(
            &admission,
            user,
            manifest.job_id,
            route_commitment(first_route),
            first_route.endpoint.clone(),
            estimate.total,
            1_000 + file_index as u64,
        );
        let admission_hash = work_admission_hash(&admission_doc).expect("admission hash");
        ledger
            .reserve_admission_budget(&admission_doc)
            .expect("reserve admission budget");

        for shard in &shards {
            shard_store.put(shard.clone());
        }

        for (index, share) in estimate.storage_shares.iter().enumerate() {
            let worker = storage_nodes
                .iter()
                .find(|node| node.identity.node_id == share.node_id)
                .expect("storage worker");
            let shard = &shards[index];
            let receipt = signed_receipt(
                worker,
                manifest.job_id,
                admission_hash,
                shard.hash,
                shard.hash,
                share.amount,
                index as u64 + 1,
            );
            ledger
                .settle_receipt(&admission_doc, &receipt)
                .expect("storage settlement");
        }

        let retrieved = retrieve_shards(&shard_store, &manifest, &[0, 1, 2]);
        assert_eq!(
            reconstruct_xor_2_1(&manifest, &retrieved).expect("reconstruct all"),
            file
        );

        for (index, share) in estimate.retrieval_shares.iter().enumerate() {
            let worker = storage_nodes
                .iter()
                .find(|node| node.identity.node_id == share.node_id)
                .expect("retrieval worker");
            let shard = &shards[index];
            let receipt = signed_receipt(
                worker,
                manifest.job_id,
                admission_hash,
                shard.hash,
                manifest.original_hash,
                share.amount,
                10_000 + index as u64,
            );
            ledger
                .settle_receipt(&admission_doc, &receipt)
                .expect("retrieval settlement");
        }

        for (index, share) in estimate.relay_shares.iter().enumerate() {
            let worker = relays
                .iter()
                .find(|node| node.identity.node_id == share.node_id)
                .expect("relay worker");
            let input_hash = blake3_hash(&[file_index as u8, index as u8, 0xA5]);
            let output_hash = blake3_hash(&[file_index as u8, index as u8, 0x5A]);
            let receipt = signed_receipt(
                worker,
                manifest.job_id,
                admission_hash,
                input_hash,
                output_hash,
                share.amount,
                20_000 + index as u64,
            );
            ledger
                .settle_receipt(&admission_doc, &receipt)
                .expect("relay settlement");
        }

        assert_eq!(ledger.admission_spent(&admission_hash), estimate.total);
        expected_total = expected_total.saturating_add(estimate.total);
        expected_storage_total = expected_storage_total.saturating_add(estimate.storage_total);
        expected_retrieval_total =
            expected_retrieval_total.saturating_add(estimate.retrieval_total);
        expected_relay_total = expected_relay_total.saturating_add(estimate.relay_total);
    }

    assert_eq!(ledger.user_balance(&user), initial_balance - expected_total);
    let storage_paid = storage_nodes
        .iter()
        .map(|node| ledger.worker_balance(&node.identity.node_id))
        .sum::<u64>();
    let relay_paid = relays
        .iter()
        .map(|node| ledger.worker_balance(&node.identity.node_id))
        .sum::<u64>();
    assert_eq!(
        storage_paid,
        expected_storage_total + expected_retrieval_total
    );
    assert_eq!(relay_paid, expected_relay_total);
    assert_eq!(storage_paid + relay_paid, expected_total);
}
