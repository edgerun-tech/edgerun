use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn public_key_from_seed(seed: u8) -> PublicKey {
    let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
    let mut out = [0u8; 32];
    out.copy_from_slice(key.verifying_key().as_bytes());
    out
}

fn admission_doc(admission: &SimNode, user: PublicKey, request_hash: Hash, budget: u64) -> WorkAdmission {
    let channel = ChannelEndpoint::new(
        blake3_hash(b"security-channel"),
        CHANNEL_KIND_MEMORY,
        Vec::new(),
        "security".into(),
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
            assigned_route_hash: blake3_hash(b"security-route"),
            assigned_channel: channel,
            admitted_budget: budget,
            policy_hash: [0u8; 32],
            sequence: 1,
            valid_until_unix_ms: u64::MAX,
            signature: empty_signature(),
        },
    )
}

fn receipt_doc(worker: &SimNode, request_hash: Hash, admission_hash: Hash, claim: u64) -> WorkReceipt {
    let input_hash = blake3_hash(b"security-input");
    let output_hash = blake3_hash(b"security-output");
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
                1,
            ),
            request_hash,
            admission_hash,
            worker: worker.identity.clone(),
            relay_node_id: [0u8; 32],
            input_hash,
            output_hash,
            units_used: 1,
            total_claim: claim,
            sequence: 1,
            signature: empty_signature(),
        },
    )
}

#[test]
fn receipt_rejects_forged_worker_node_id() {
    let admission = SimNode::from_seed(1, NODE_ROLE_ADMISSION);
    let worker = SimNode::from_seed(2, NODE_ROLE_STORAGE);
    let user = public_key_from_seed(3);
    let request_hash = blake3_hash(b"forged-worker-node-id");
    let admission_doc = admission_doc(&admission, user, request_hash, 10);
    let admission_hash = work_admission_hash(&admission_doc).expect("admission hash");
    let mut receipt = receipt_doc(&worker, request_hash, admission_hash, 10);

    receipt.worker.node_id = blake3_hash(b"attacker-payment-node");
    assert!(!verify_work_receipt(&receipt));

    let mut ledger = SettlementLedger::new();
    ledger.deposit_user_credit(user, 100);
    assert!(matches!(
        ledger.settle_receipt_unchecked_evidence(&admission_doc, &receipt),
        Err(SettlementError::InvalidReceipt)
    ));
}

#[test]
fn admission_rejects_forged_admission_node_id() {
    let admission = SimNode::from_seed(11, NODE_ROLE_ADMISSION);
    let user = public_key_from_seed(12);
    let request_hash = blake3_hash(b"forged-admission-node-id");
    let mut doc = admission_doc(&admission, user, request_hash, 10);
    doc.admission_node.node_id = blake3_hash(b"fake-admission-node");

    assert!(!verify_work_admission(&doc));
}

#[test]
fn relay_assignment_rejects_forged_assigned_by_node_id() {
    let admission = SimNode::from_seed(21, NODE_ROLE_ADMISSION);
    let relay = SimNode::from_seed(22, NODE_ROLE_RELAY);
    let assignment = sign_relay_assignment(
        &admission.key,
        RelayAssignment {
            abi_version: WORK_WIRE_ABI_VERSION,
            node_id: relay.identity.node_id,
            relay: RelayEndpoint {
                relay_node_id: relay.identity.node_id,
                host: "relay.example".into(),
                port: 1,
            },
            assigned_by: admission.identity.clone(),
            sequence: 1,
            valid_until_unix_ms: u64::MAX,
            signature: empty_signature(),
        },
    );
    let mut forged = assignment;
    forged.assigned_by.node_id = blake3_hash(b"fake-assigned-by-node");

    assert!(!verify_relay_assignment(&forged));
}

#[test]
fn route_signature_verifies_non_available_status_but_planner_filters_it() {
    let admission = SimNode::from_seed(31, NODE_ROLE_ADMISSION);
    let storage = SimNode::from_seed(32, NODE_ROLE_STORAGE);
    let mut route = storage.advertise_memory_route(storage.identity.node_id, vec![DEPARTMENT_STORAGE]);
    route.status = ROUTE_STATUS_DRAINING;
    route = sign_route_advertisement(&storage.key, route);

    assert!(verify_route_advertisement(&route));
    assert!(!verify_available_route_advertisement(&route));

    let routes = vec![route];
    let snapshot = sign_route_snapshot(
        &admission.key,
        RouteSnapshot {
            abi_version: WORK_WIRE_ABI_VERSION,
            issued_by: admission.identity.clone(),
            sequence: 1,
            route_root: route_root_hash(&routes),
            routes,
            signature: empty_signature(),
        },
    );
    let plan = VerifiedRoutePlan::from_snapshot(snapshot).expect("snapshot verifies");
    assert_eq!(plan.routes_for_department(DEPARTMENT_STORAGE).len(), 0);
}

#[test]
fn batch_settlement_preflight_keeps_ledger_unchanged_on_failure() {
    let admission = SimNode::from_seed(41, NODE_ROLE_ADMISSION);
    let worker = SimNode::from_seed(42, NODE_ROLE_STORAGE);
    let user = public_key_from_seed(43);
    let request_hash = blake3_hash(b"batch-atomicity");
    let doc = admission_doc(&admission, user, request_hash, 1);
    let admission_hash = work_admission_hash(&doc).expect("admission hash");
    let receipts = vec![
        receipt_doc(&worker, request_hash, admission_hash, 1),
        receipt_doc(&worker, request_hash, admission_hash, 1),
    ];

    let mut ledger = SettlementLedger::new();
    ledger.deposit_user_credit(user, 10);
    assert!(matches!(
        ledger.settle_receipt_batch_unchecked_evidence(&doc, &receipts),
        Err(BatchSettlementError::DuplicateInBatch)
    ));
    assert_eq!(ledger.user_balance(&user), 10);
    assert_eq!(ledger.worker_balance(&worker.identity.node_id), 0);
    assert_eq!(ledger.admission_spent(&admission_hash), 0);
}
