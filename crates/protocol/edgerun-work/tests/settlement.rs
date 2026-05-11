use edgerun_work::*;
use edgerun_work::channel::{ChannelEndpoint, CHANNEL_KIND_MEMORY};
use edgerun_crypto::Ed25519SigningKey;

fn signed_storage_admission(
    admission: &SimNode,
    user: PublicKey,
    request_hash: Hash,
    admitted_budget: u64,
) -> WorkAdmission {
    let channel = ChannelEndpoint::new(
        blake3_hash(b"settlement-memory-channel"),
        CHANNEL_KIND_MEMORY,
        Vec::new(),
        "memory".into(),
    );
    sign_work_admission(
        &admission.key,
        WorkAdmission {
            abi_version: WORK_WIRE_ABI_VERSION,
            admission_id: blake3_hash(&request_hash),
            dao_id: user,
            admission_node: admission.identity.clone(),
            request_hash,
            assigned_route_hash: blake3_hash(b"settlement-route"),
            assigned_channel: channel,
            admitted_budget,
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
    let receipt_id = receipt_id_for_claim(
        request_hash,
        admission_hash,
        worker.identity.node_id,
        shard.hash,
        shard.hash,
        sequence,
    );
    sign_work_receipt(
        &worker.key,
        WorkReceipt {
            abi_version: WORK_WIRE_ABI_VERSION,
            receipt_id,
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
fn erasure_storage_receipts_debit_user_and_pay_storage_nodes() {
    let admission = SimNode::from_seed(111, NODE_ROLE_ADMISSION);
    let user_key = Ed25519SigningKey::from_bytes(&[112u8; 32]);
    let mut user = [0u8; 32];
    user.copy_from_slice(user_key.verifying_key().as_bytes());
    let storage0 = SimNode::from_seed(113, NODE_ROLE_STORAGE);
    let storage1 = SimNode::from_seed(114, NODE_ROLE_STORAGE);
    let storage2 = SimNode::from_seed(115, NODE_ROLE_STORAGE);
    let nodes = [
        storage0.identity.node_id,
        storage1.identity.node_id,
        storage2.identity.node_id,
    ];

    let file = b"settle payments for three erasure storage shards".to_vec();
    let (manifest, shards) = encode_xor_2_1(&file, nodes).expect("encode");
    verify_manifest(&manifest, &shards).expect("manifest");

    let request_hash = manifest.job_id;
    let admission_doc = signed_storage_admission(&admission, user, request_hash, 30);
    let admission_hash = work_admission_hash(&admission_doc).expect("admission hash");

    let mut ledger = SettlementLedger::new();
    ledger.deposit_user_credit(user, 100);

    let workers = [&storage0, &storage1, &storage2];
    for (index, shard) in shards.iter().enumerate() {
        let receipt = signed_storage_receipt(
            workers[index],
            request_hash,
            admission_hash,
            shard,
            10,
            index as u64 + 1,
        );
        let result = ledger
            .settle_receipt(user, &admission_doc, &receipt)
            .expect("settle shard receipt");
        assert_eq!(result.amount, 10);
    }

    assert_eq!(ledger.user_balance(&user), 70);
    assert_eq!(ledger.admission_spent(&admission_hash), 30);
    assert_eq!(ledger.worker_balance(&storage0.identity.node_id), 10);
    assert_eq!(ledger.worker_balance(&storage1.identity.node_id), 10);
    assert_eq!(ledger.worker_balance(&storage2.identity.node_id), 10);
}

#[test]
fn settlement_rejects_duplicate_and_over_budget_receipts() {
    let admission = SimNode::from_seed(121, NODE_ROLE_ADMISSION);
    let user_key = Ed25519SigningKey::from_bytes(&[122u8; 32]);
    let mut user = [0u8; 32];
    user.copy_from_slice(user_key.verifying_key().as_bytes());
    let storage0 = SimNode::from_seed(123, NODE_ROLE_STORAGE);
    let storage1 = SimNode::from_seed(124, NODE_ROLE_STORAGE);
    let storage2 = SimNode::from_seed(125, NODE_ROLE_STORAGE);
    let nodes = [
        storage0.identity.node_id,
        storage1.identity.node_id,
        storage2.identity.node_id,
    ];

    let (manifest, shards) = encode_xor_2_1(b"budget test", nodes).expect("encode");
    let request_hash = manifest.job_id;
    let admission_doc = signed_storage_admission(&admission, user, request_hash, 15);
    let admission_hash = work_admission_hash(&admission_doc).expect("admission hash");

    let mut ledger = SettlementLedger::new();
    ledger.deposit_user_credit(user, 100);

    let receipt = signed_storage_receipt(&storage0, request_hash, admission_hash, &shards[0], 10, 1);
    ledger
        .settle_receipt(user, &admission_doc, &receipt)
        .expect("first settlement");

    assert!(matches!(
        ledger.settle_receipt(user, &admission_doc, &receipt),
        Err(SettlementError::DuplicateReceipt)
    ));

    let receipt2 = signed_storage_receipt(&storage1, request_hash, admission_hash, &shards[1], 10, 2);
    assert!(matches!(
        ledger.settle_receipt(user, &admission_doc, &receipt2),
        Err(SettlementError::ClaimExceedsAdmissionBudget)
    ));
}
