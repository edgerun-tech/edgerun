use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn public_key_from_seed(seed: u8) -> PublicKey {
    let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
    let mut out = [0u8; 32];
    out.copy_from_slice(key.verifying_key().as_bytes());
    out
}

fn signed_batch_admission(
    admission: &SimNode,
    user: PublicKey,
    request_hash: Hash,
    budget: u64,
) -> WorkAdmission {
    let channel = ChannelEndpoint::new(
        blake3_hash(b"batch-memory-channel"),
        CHANNEL_KIND_MEMORY,
        Vec::new(),
        "batch-memory".into(),
    );
    sign_work_admission(
        &admission.key,
        WorkAdmission {
            abi_version: WORK_WIRE_ABI_VERSION,
            admission_id: blake3_hash(&request_hash),
            dao_id: admission.identity.public_key,
            user,
            admission_node: admission.identity.clone(),
            request_hash,
            assigned_route_hash: blake3_hash(b"batch-route"),
            assigned_channel: channel,
            admitted_budget: budget,
            policy_hash: [0u8; 32],
            sequence: 1,
            valid_until_unix_ms: u64::MAX,
            signature: empty_signature(),
        },
    )
}

fn signed_batch_receipt(
    worker: &SimNode,
    request_hash: Hash,
    admission_hash: Hash,
    index: u64,
    claim: u64,
) -> WorkReceipt {
    let input_hash = blake3_hash(&index.to_be_bytes());
    let output_hash = blake3_hash(&(index ^ 0xA5A5_A5A5).to_be_bytes());
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
                index,
            ),
            request_hash,
            admission_hash,
            worker: worker.identity.clone(),
            relay_node_id: [0u8; 32],
            input_hash,
            output_hash,
            units_used: 1,
            total_claim: claim,
            sequence: index,
            signature: empty_signature(),
        },
    )
}

#[test]
fn batch_settlement_handles_10k_receipts_with_deterministic_root() {
    const RECEIPT_COUNT: usize = 10_000;
    const WORKER_COUNT: usize = 64;

    let admission = SimNode::from_seed(201, NODE_ROLE_ADMISSION);
    let user = public_key_from_seed(202);
    let workers = (0..WORKER_COUNT)
        .map(|i| SimNode::from_seed(10 + i as u8, NODE_ROLE_STORAGE))
        .collect::<Vec<_>>();

    let request_hash = blake3_hash(b"batch-settlement-10k");
    let admission_doc = signed_batch_admission(&admission, user, request_hash, RECEIPT_COUNT as u64);
    let admission_hash = work_admission_hash(&admission_doc).expect("admission hash");

    let receipts = (0..RECEIPT_COUNT)
        .map(|i| {
            let worker = &workers[i % workers.len()];
            signed_batch_receipt(worker, request_hash, admission_hash, i as u64 + 1, 1)
        })
        .collect::<Vec<_>>();

    let batch = build_receipt_batch(&admission_doc, &receipts).expect("batch builds");
    let batch_again = build_receipt_batch(&admission_doc, &receipts).expect("batch builds again");
    assert_eq!(batch.batch_root, batch_again.batch_root);
    assert_eq!(batch.total_claim, RECEIPT_COUNT as u64);
    assert_eq!(batch.receipt_hashes.len(), RECEIPT_COUNT);
    assert_ne!(batch.batch_root, [0u8; 32]);

    let totals = worker_claim_totals(&receipts);
    assert_eq!(totals.len(), WORKER_COUNT);
    assert_eq!(totals.values().sum::<u64>(), RECEIPT_COUNT as u64);

    let mut ledger = SettlementLedger::new();
    ledger.deposit_user_credit(user, RECEIPT_COUNT as u64 + 1_000);
    let result = ledger
        .settle_receipt_batch(&admission_doc, &receipts)
        .expect("batch settlement");
    assert_eq!(result.batch_root, batch.batch_root);
    assert_eq!(result.receipt_count, RECEIPT_COUNT as u64);
    assert_eq!(result.total_claim, RECEIPT_COUNT as u64);
    assert_eq!(result.worker_count, WORKER_COUNT as u64);
    assert_eq!(ledger.user_balance(&user), 1_000);

    let worker_paid = workers
        .iter()
        .map(|worker| ledger.worker_balance(&worker.identity.node_id))
        .sum::<u64>();
    assert_eq!(worker_paid, RECEIPT_COUNT as u64);
}

#[test]
fn batch_settlement_rejects_duplicate_receipt_inside_batch() {
    let admission = SimNode::from_seed(211, NODE_ROLE_ADMISSION);
    let user = public_key_from_seed(212);
    let worker = SimNode::from_seed(213, NODE_ROLE_STORAGE);
    let request_hash = blake3_hash(b"duplicate-batch");
    let admission_doc = signed_batch_admission(&admission, user, request_hash, 10);
    let admission_hash = work_admission_hash(&admission_doc).expect("admission hash");

    let receipt = signed_batch_receipt(&worker, request_hash, admission_hash, 1, 1);
    let receipts = vec![receipt.clone(), receipt];
    assert!(matches!(
        build_receipt_batch(&admission_doc, &receipts),
        Err(BatchSettlementError::DuplicateInBatch)
    ));
}

#[test]
fn batch_settlement_rejects_over_budget_batch() {
    let admission = SimNode::from_seed(221, NODE_ROLE_ADMISSION);
    let user = public_key_from_seed(222);
    let worker = SimNode::from_seed(223, NODE_ROLE_STORAGE);
    let request_hash = blake3_hash(b"over-budget-batch");
    let admission_doc = signed_batch_admission(&admission, user, request_hash, 1);
    let admission_hash = work_admission_hash(&admission_doc).expect("admission hash");

    let receipts = vec![
        signed_batch_receipt(&worker, request_hash, admission_hash, 1, 1),
        signed_batch_receipt(&worker, request_hash, admission_hash, 2, 1),
    ];
    assert!(matches!(
        build_receipt_batch(&admission_doc, &receipts),
        Err(BatchSettlementError::Settlement(
            SettlementError::ClaimExceedsAdmissionBudget
        ))
    ));
}
