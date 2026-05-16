use alloc::vec;

use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::{
    CHANNEL_KIND_MEMORY, ChannelEndpoint, NODE_ROLE_ADMISSION, NODE_ROLE_NOTARY,
    NODE_ROLE_VERIFIER, NotaryWorkReport, WORK_TYPE_NOTARY_SEAL, WORK_WIRE_ABI_VERSION,
    WorkAdmission, WorkPacket, WorkReceipt, blake3_hash, empty_signature, node_identity_from_key,
    packet_hash, sign_notary_work_report, sign_work_admission, sign_work_receipt,
};

use crate::{
    EdgeEmissionClaim, EdgeEmissionEvidence, ExternalTxObservation, WALLET_CORE_ABI_VERSION,
    WalletAddress, WalletAmount, WalletAssetId, WalletChainFamily, WalletChainId, WalletLedger,
    WalletSettlementError, WalletSettlementEvent, account, edge_emission_claim_hash,
    external_tx_observation_hash, verify_edge_emission_evidence, wallet_asset_id_hash,
};

fn chain(network: &str) -> WalletChainId {
    WalletChainId {
        abi_version: WALLET_CORE_ABI_VERSION,
        family: WalletChainFamily::EdgeRun,
        network: network.into(),
        namespace: None,
    }
}

fn edge_asset() -> WalletAssetId {
    WalletAssetId {
        abi_version: WALLET_CORE_ABI_VERSION,
        symbol: "EDGE".into(),
        chain: chain("edgerun"),
        contract: None,
        decimals: Some(6),
    }
}

fn address(value: &str) -> WalletAddress {
    WalletAddress {
        abi_version: WALLET_CORE_ABI_VERSION,
        chain: chain("edgerun"),
        address: value.into(),
    }
}

fn signed_evidence() -> (
    EdgeEmissionClaim,
    WorkAdmission,
    WorkReceipt,
    NotaryWorkReport,
) {
    let admission_key = Ed25519SigningKey::from_bytes(&[1u8; 32]);
    let worker_key = Ed25519SigningKey::from_bytes(&[2u8; 32]);
    let notary_key = Ed25519SigningKey::from_bytes(&[3u8; 32]);

    let request_hash = blake3_hash(b"wallet-core-request");
    let policy_hash = blake3_hash(b"wallet-core-policy");
    let admission = sign_work_admission(
        &admission_key,
        WorkAdmission {
            abi_version: WORK_WIRE_ABI_VERSION,
            admission_id: blake3_hash(b"wallet-core-admission-id"),
            dao_id: [9u8; 32],
            user: [7u8; 32],
            admission_node: node_identity_from_key(&admission_key, NODE_ROLE_ADMISSION),
            request_hash,
            assigned_route_commitment: blake3_hash(b"wallet-core-route"),
            assigned_channel: ChannelEndpoint::new(
                blake3_hash(b"wallet-core-channel"),
                CHANNEL_KIND_MEMORY,
                b"memory".to_vec(),
                "memory".into(),
            ),
            assigned_relay_path: vec![blake3_hash(b"wallet-core-relay")],
            admitted_budget: 100,
            policy_hash,
            sequence: 1,
            valid_until_unix_ms: 10_000,
            signature: empty_signature(),
        },
    );
    let admission_hash = packet_hash(&WorkPacket::WorkAdmission(admission.clone())).unwrap();

    let receipt = sign_work_receipt(
        &worker_key,
        WorkReceipt {
            abi_version: WORK_WIRE_ABI_VERSION,
            receipt_id: blake3_hash(b"wallet-core-receipt-id"),
            request_hash,
            admission_hash,
            worker: node_identity_from_key(&worker_key, NODE_ROLE_VERIFIER),
            relay_node_id: blake3_hash(b"wallet-core-relay"),
            input_hash: blake3_hash(b"wallet-core-input"),
            output_hash: blake3_hash(b"wallet-core-output"),
            units_used: 5,
            total_claim: 25,
            sequence: 2,
            signature: empty_signature(),
        },
    );
    let receipt_hash = packet_hash(&WorkPacket::WorkReceipt(receipt.clone())).unwrap();

    let notary_report = sign_notary_work_report(
        &notary_key,
        NotaryWorkReport {
            abi_version: WORK_WIRE_ABI_VERSION,
            notary: node_identity_from_key(&notary_key, NODE_ROLE_NOTARY),
            requester: [7u8; 32],
            request_hash,
            admission_hash,
            work_type: WORK_TYPE_NOTARY_SEAL,
            request_message_id: blake3_hash(b"wallet-core-message"),
            request_payload_hash: blake3_hash(b"wallet-core-payload"),
            via_relay: blake3_hash(b"wallet-core-relay"),
            sequence: 3,
            aad_hash: blake3_hash(b"wallet-core-aad"),
            input_hash: blake3_hash(b"wallet-core-input"),
            output_hash: blake3_hash(b"wallet-core-output"),
            response_hash: blake3_hash(b"wallet-core-response"),
            completed_unix_ms: 9_000,
            signature: empty_signature(),
        },
    );
    let notary_report_hash = crate::notary_work_report_hash(&notary_report);

    let claim = EdgeEmissionClaim {
        abi_version: WALLET_CORE_ABI_VERSION,
        claim_id: blake3_hash(b"wallet-core-claim"),
        beneficiary: account([7u8; 32], 0),
        amount: WalletAmount::new(250, 6),
        admission_hash,
        receipt_hash,
        notary_report_hash,
        policy_hash,
    };

    (claim, admission, receipt, notary_report)
}

#[test]
fn edge_emission_requires_matching_admitted_notarized_evidence() {
    let (claim, admission, receipt, notary_report) = signed_evidence();

    let claim_hash = verify_edge_emission_evidence(&EdgeEmissionEvidence {
        claim: &claim,
        admission: &admission,
        receipt: &receipt,
        notary_report: &notary_report,
    })
    .expect("matching evidence verifies");

    assert_eq!(claim_hash, edge_emission_claim_hash(&claim));

    let mut wrong_policy = claim.clone();
    wrong_policy.policy_hash = blake3_hash(b"wrong-policy");
    assert_eq!(
        verify_edge_emission_evidence(&EdgeEmissionEvidence {
            claim: &wrong_policy,
            admission: &admission,
            receipt: &receipt,
            notary_report: &notary_report,
        }),
        Err(WalletSettlementError::ProofMismatch)
    );
}

#[test]
fn wallet_ledger_replay_is_deterministic_and_rejects_duplicate_finalization() {
    let (claim, _, _, _) = signed_evidence();
    let events = vec![WalletSettlementEvent::EdgeEmissionFinalized(claim.clone())];

    let ledger_a = WalletLedger::replay(&events).expect("first replay");
    let ledger_b = WalletLedger::replay(&events).expect("second replay");
    assert_eq!(
        ledger_a.balance_by_asset_hash(&claim.beneficiary, claim.policy_hash, claim.amount.scale),
        ledger_b.balance_by_asset_hash(&claim.beneficiary, claim.policy_hash, claim.amount.scale)
    );

    let duplicate = vec![
        WalletSettlementEvent::EdgeEmissionFinalized(claim.clone()),
        WalletSettlementEvent::EdgeEmissionFinalized(claim),
    ];
    assert_eq!(
        WalletLedger::replay(&duplicate).unwrap_err(),
        WalletSettlementError::DuplicateClaim
    );
}

#[test]
fn wallet_ledger_rejects_duplicate_external_observations() {
    let observation = ExternalTxObservation {
        abi_version: WALLET_CORE_ABI_VERSION,
        chain: chain("bitcoin"),
        tx_id: "tx-1".into(),
        block_ref: "block-1".into(),
        confirmations: 6,
        asset: edge_asset(),
        amount: WalletAmount::new(100, 6),
        from_hint: Some("sender".into()),
        to: address("recipient"),
        observed_at_ms: 42,
        evidence_hash: blake3_hash(b"notary-evidence"),
    };
    let observation_hash = external_tx_observation_hash(&observation);

    let mut ledger = WalletLedger::new();
    ledger
        .apply(&WalletSettlementEvent::ExternalTxObserved(
            observation.clone(),
        ))
        .expect("first observation");
    assert!(ledger.has_external_observation(&observation_hash));
    assert_eq!(
        ledger.apply(&WalletSettlementEvent::ExternalTxObserved(observation)),
        Err(WalletSettlementError::DuplicateObservation)
    );
}

#[test]
fn asset_hash_is_stable_for_equal_records() {
    assert_eq!(
        wallet_asset_id_hash(&edge_asset()),
        wallet_asset_id_hash(&edge_asset())
    );
}
