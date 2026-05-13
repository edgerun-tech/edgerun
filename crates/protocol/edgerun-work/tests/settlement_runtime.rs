#![cfg(feature = "std")]

use std::thread;

use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn public_key_from_seed(seed: u8) -> PublicKey {
    let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
    let mut out = [0u8; 32];
    out.copy_from_slice(key.verifying_key().as_bytes());
    out
}

fn admission_doc(
    admission: &SimNode,
    user: PublicKey,
    request_hash: Hash,
    budget: u64,
) -> WorkAdmission {
    let channel = ChannelEndpoint::new(
        blake3_hash(b"runtime-settlement-channel"),
        CHANNEL_KIND_MEMORY,
        Vec::new(),
        "runtime-settlement".into(),
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
            assigned_route_commitment: blake3_hash(b"runtime-settlement-route"),
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

fn receipt_doc(
    worker: &SimNode,
    request_hash: Hash,
    admission_hash: Hash,
    claim: u64,
    sequence: u64,
) -> WorkReceipt {
    let input_hash = blake3_hash(&sequence.to_be_bytes());
    let output_hash = blake3_hash(&(sequence ^ 0xDEAD_BEEF).to_be_bytes());
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
fn finalized_admission_pruning_removes_paid_receipt_tracking() {
    let admission = SimNode::from_seed(1, NODE_ROLE_ADMISSION);
    let worker = SimNode::from_seed(2, NODE_ROLE_STORAGE);
    let user = public_key_from_seed(3);
    let request_hash = blake3_hash(b"prune-admission");
    let doc = admission_doc(&admission, user, request_hash, 10);
    let admission_hash = work_admission_hash(&doc).expect("admission hash");
    let receipt = receipt_doc(&worker, request_hash, admission_hash, 10, 1);
    let receipt_hash = work_receipt_hash(&receipt).expect("receipt hash");

    let mut ledger = SettlementLedger::new();
    ledger.deposit_user_credit(user, 100);
    ledger.settle_receipt(&doc, &receipt).expect("settle");
    assert!(ledger.is_receipt_paid(&receipt_hash));
    assert_eq!(ledger.paid_receipt_count(), 1);
    assert_eq!(ledger.tracked_admission_count(), 1);

    let pruned = ledger.prune_finalized_admission(&admission_hash);
    assert!(pruned.removed_admission);
    assert_eq!(pruned.removed_receipts, 1);
    assert!(!ledger.is_receipt_paid(&receipt_hash));
    assert_eq!(ledger.paid_receipt_count(), 0);
    assert_eq!(ledger.tracked_admission_count(), 0);
}

#[test]
fn thread_safe_settlement_wrapper_serializes_concurrent_settlements() {
    let admission = SimNode::from_seed(11, NODE_ROLE_ADMISSION);
    let worker = SimNode::from_seed(12, NODE_ROLE_STORAGE);
    let user = public_key_from_seed(13);
    let request_hash = blake3_hash(b"thread-safe-settlement");
    let doc = admission_doc(&admission, user, request_hash, 100);
    let admission_hash = work_admission_hash(&doc).expect("admission hash");
    let ledger = ThreadSafeSettlementLedger::new();
    ledger.deposit_user_credit(user, 100);

    let mut handles = Vec::new();
    for i in 0..100u64 {
        let ledger = ledger.clone();
        let doc = doc.clone();
        let receipt = receipt_doc(&worker, request_hash, admission_hash, 1, i + 1);
        handles.push(thread::spawn(move || {
            ledger
                .settle_receipt(&doc, &receipt)
                .expect("settle concurrent")
        }));
    }
    for handle in handles {
        handle.join().expect("thread join");
    }

    assert_eq!(ledger.user_balance(&user), 0);
    assert_eq!(ledger.worker_balance(&worker.identity.node_id), 100);
}
