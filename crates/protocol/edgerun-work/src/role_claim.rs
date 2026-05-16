use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::codec::{EdgeWire, WireCursor, WireWriter, blake3_hash};
use crate::preimage::{HashBuilder, PreimageBuilder};
use crate::protocol::*;
use crate::signing::{sign_ed25519, verify_signature};

const ROLE_WORK_CLAIM_DOMAIN: &[u8] = b"edgerun:v1:work:role-work-claim";
const ROLE_WORK_CLAIM_HASH_DOMAIN: &[u8] = b"edgerun:v1:work:role-work-claim-hash";
const VERIFICATION_REPORT_DOMAIN: &[u8] = b"edgerun:v1:work:verification-report";

pub const VERIFICATION_RESULT_REJECTED: u16 = 0;
pub const VERIFICATION_RESULT_ACCEPTED: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleWorkClaim {
    pub abi_version: u16,
    pub claim_id: Hash,
    pub worker: NodeIdentity,
    pub controlling_admission_node_id: NodeId,
    pub request_hash: Hash,
    pub admission_hash: Hash,
    pub policy_hash: Hash,
    pub work_kind: u16,
    pub input_root: Hash,
    pub output_root: Hash,
    pub sequence_start: u64,
    pub sequence_end: u64,
    pub units_used: u64,
    pub total_claim: u64,
    pub evidence_root: Hash,
    pub valid_until_unix_ms: u64,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationReport {
    pub abi_version: u16,
    pub verifier: NodeIdentity,
    pub verified_worker_node_id: NodeId,
    pub verified_worker_role: u16,
    pub claim_hash: Hash,
    pub evidence_root: Hash,
    pub verifier_policy_hash: Hash,
    pub result: u16,
    pub checked_at_unix_ms: u64,
    pub signature: WorkSignature,
}

pub fn role_work_claim_preimage(value: &RoleWorkClaim) -> Vec<u8> {
    PreimageBuilder::domain(ROLE_WORK_CLAIM_DOMAIN)
        .hash(&value.claim_id)
        .node(&value.worker)
        .node_id(&value.controlling_admission_node_id)
        .hash(&value.request_hash)
        .hash(&value.admission_hash)
        .hash(&value.policy_hash)
        .u16(value.work_kind)
        .hash(&value.input_root)
        .hash(&value.output_root)
        .u64(value.sequence_start)
        .u64(value.sequence_end)
        .u64(value.units_used)
        .u64(value.total_claim)
        .hash(&value.evidence_root)
        .u64(value.valid_until_unix_ms)
        .finish()
}

pub fn role_work_claim_hash(value: &RoleWorkClaim) -> Hash {
    HashBuilder::domain(ROLE_WORK_CLAIM_HASH_DOMAIN)
        .bytes(&role_work_claim_preimage(value))
        .finish()
}

pub fn role_work_claim_id(
    worker: &NodeIdentity,
    admission_hash: Hash,
    work_kind: u16,
    sequence_start: u64,
    sequence_end: u64,
    input_root: Hash,
    output_root: Hash,
    evidence_root: Hash,
) -> Hash {
    HashBuilder::domain(ROLE_WORK_CLAIM_HASH_DOMAIN)
        .node_id(&worker.node_id)
        .u16(worker.role)
        .hash(&worker.public_key)
        .hash(&admission_hash)
        .u16(work_kind)
        .u64(sequence_start)
        .u64(sequence_end)
        .hash(&input_root)
        .hash(&output_root)
        .hash(&evidence_root)
        .finish()
}

pub fn sign_role_work_claim(key: &Ed25519SigningKey, mut value: RoleWorkClaim) -> RoleWorkClaim {
    value.signature = sign_ed25519(key, &role_work_claim_preimage(&value));
    value
}

pub fn verify_role_work_claim(value: &RoleWorkClaim) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.sequence_start <= value.sequence_end
        && value.units_used > 0
        && verify_signature(
            &value.worker,
            &value.signature,
            &role_work_claim_preimage(value),
        )
}

pub fn verification_report_preimage(value: &VerificationReport) -> Vec<u8> {
    PreimageBuilder::domain(VERIFICATION_REPORT_DOMAIN)
        .node(&value.verifier)
        .node_id(&value.verified_worker_node_id)
        .u16(value.verified_worker_role)
        .hash(&value.claim_hash)
        .hash(&value.evidence_root)
        .hash(&value.verifier_policy_hash)
        .u16(value.result)
        .u64(value.checked_at_unix_ms)
        .finish()
}

pub fn verification_report_hash(value: &VerificationReport) -> Hash {
    blake3_hash(&verification_report_preimage(value))
}

pub fn sign_verification_report(
    key: &Ed25519SigningKey,
    mut value: VerificationReport,
) -> VerificationReport {
    value.signature = sign_ed25519(key, &verification_report_preimage(&value));
    value
}

pub fn verify_verification_report(value: &VerificationReport) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.verifier.role == NODE_ROLE_VERIFIER
        && matches!(
            value.result,
            VERIFICATION_RESULT_REJECTED | VERIFICATION_RESULT_ACCEPTED
        )
        && verify_signature(
            &value.verifier,
            &value.signature,
            &verification_report_preimage(value),
        )
}

impl EdgeWire for RoleWorkClaim {
    fn encode_wire(&self, out: &mut WireWriter) {
        self.abi_version.encode_wire(out);
        self.claim_id.encode_wire(out);
        self.worker.encode_wire(out);
        self.controlling_admission_node_id.encode_wire(out);
        self.request_hash.encode_wire(out);
        self.admission_hash.encode_wire(out);
        self.policy_hash.encode_wire(out);
        self.work_kind.encode_wire(out);
        self.input_root.encode_wire(out);
        self.output_root.encode_wire(out);
        self.sequence_start.encode_wire(out);
        self.sequence_end.encode_wire(out);
        self.units_used.encode_wire(out);
        self.total_claim.encode_wire(out);
        self.evidence_root.encode_wire(out);
        self.valid_until_unix_ms.encode_wire(out);
        self.signature.encode_wire(out);
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        Ok(Self {
            abi_version: <u16 as EdgeWire>::decode_wire(input)?,
            claim_id: <Hash as EdgeWire>::decode_wire(input)?,
            worker: <NodeIdentity as EdgeWire>::decode_wire(input)?,
            controlling_admission_node_id: <NodeId as EdgeWire>::decode_wire(input)?,
            request_hash: <Hash as EdgeWire>::decode_wire(input)?,
            admission_hash: <Hash as EdgeWire>::decode_wire(input)?,
            policy_hash: <Hash as EdgeWire>::decode_wire(input)?,
            work_kind: <u16 as EdgeWire>::decode_wire(input)?,
            input_root: <Hash as EdgeWire>::decode_wire(input)?,
            output_root: <Hash as EdgeWire>::decode_wire(input)?,
            sequence_start: <u64 as EdgeWire>::decode_wire(input)?,
            sequence_end: <u64 as EdgeWire>::decode_wire(input)?,
            units_used: <u64 as EdgeWire>::decode_wire(input)?,
            total_claim: <u64 as EdgeWire>::decode_wire(input)?,
            evidence_root: <Hash as EdgeWire>::decode_wire(input)?,
            valid_until_unix_ms: <u64 as EdgeWire>::decode_wire(input)?,
            signature: <WorkSignature as EdgeWire>::decode_wire(input)?,
        })
    }
}

impl EdgeWire for VerificationReport {
    fn encode_wire(&self, out: &mut WireWriter) {
        self.abi_version.encode_wire(out);
        self.verifier.encode_wire(out);
        self.verified_worker_node_id.encode_wire(out);
        self.verified_worker_role.encode_wire(out);
        self.claim_hash.encode_wire(out);
        self.evidence_root.encode_wire(out);
        self.verifier_policy_hash.encode_wire(out);
        self.result.encode_wire(out);
        self.checked_at_unix_ms.encode_wire(out);
        self.signature.encode_wire(out);
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        Ok(Self {
            abi_version: <u16 as EdgeWire>::decode_wire(input)?,
            verifier: <NodeIdentity as EdgeWire>::decode_wire(input)?,
            verified_worker_node_id: <NodeId as EdgeWire>::decode_wire(input)?,
            verified_worker_role: <u16 as EdgeWire>::decode_wire(input)?,
            claim_hash: <Hash as EdgeWire>::decode_wire(input)?,
            evidence_root: <Hash as EdgeWire>::decode_wire(input)?,
            verifier_policy_hash: <Hash as EdgeWire>::decode_wire(input)?,
            result: <u16 as EdgeWire>::decode_wire(input)?,
            checked_at_unix_ms: <u64 as EdgeWire>::decode_wire(input)?,
            signature: <WorkSignature as EdgeWire>::decode_wire(input)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::{CHANNEL_KIND_MEMORY, ChannelEndpoint};
    use crate::codec::{wire_bytes, wire_from_bytes};
    use crate::notary_role::{
        NotaryWorkReport, notary_aad_hash, notary_sealed_hash, sign_notary_work_report,
        verify_notary_work_report,
    };
    use crate::settlement::{SettlementError, SettlementLedger, work_admission_hash};
    use crate::signing::{empty_signature, sign_work_admission, sign_work_receipt};
    use crate::{SimNode, receipt_id_for_claim};
    use alloc::vec;

    fn signed_admission(
        admission: &SimNode,
        user: PublicKey,
        request_hash: Hash,
        policy_hash: Hash,
        budget: u64,
    ) -> WorkAdmission {
        sign_work_admission(
            &admission.key,
            WorkAdmission {
                abi_version: WORK_WIRE_ABI_VERSION,
                admission_id: blake3_hash(b"role-claim-admission"),
                dao_id: admission.identity.public_key,
                user,
                admission_node: admission.identity.clone(),
                request_hash,
                assigned_route_commitment: blake3_hash(b"role-claim-route"),
                assigned_channel: ChannelEndpoint::new(
                    blake3_hash(b"role-claim-channel"),
                    CHANNEL_KIND_MEMORY,
                    Vec::new(),
                    "memory".into(),
                ),
                assigned_relay_path: vec![blake3_hash(b"role-claim-relay")],
                admitted_budget: budget,
                policy_hash,
                sequence: 1,
                valid_until_unix_ms: 10_000,
                signature: empty_signature(),
            },
        )
    }

    fn signed_claim(
        worker: &SimNode,
        admission: &WorkAdmission,
        admission_hash: Hash,
        work_kind: u16,
        sequence_start: u64,
        sequence_end: u64,
        total_claim: u64,
    ) -> RoleWorkClaim {
        let input_root = blake3_hash(b"claim-input");
        let output_root = blake3_hash(b"claim-output");
        let evidence_root = blake3_hash(b"claim-evidence");
        let claim_id = role_work_claim_id(
            &worker.identity,
            admission_hash,
            work_kind,
            sequence_start,
            sequence_end,
            input_root,
            output_root,
            evidence_root,
        );
        sign_role_work_claim(
            &worker.key,
            RoleWorkClaim {
                abi_version: WORK_WIRE_ABI_VERSION,
                claim_id,
                worker: worker.identity.clone(),
                controlling_admission_node_id: admission.admission_node.node_id,
                request_hash: admission.request_hash,
                admission_hash,
                policy_hash: admission.policy_hash,
                work_kind,
                input_root,
                output_root,
                sequence_start,
                sequence_end,
                units_used: sequence_end - sequence_start + 1,
                total_claim,
                evidence_root,
                valid_until_unix_ms: 10_000,
                signature: empty_signature(),
            },
        )
    }

    fn receipt_for_claim(worker: &SimNode, claim: &RoleWorkClaim) -> WorkReceipt {
        let receipt_id = receipt_id_for_claim(
            claim.request_hash,
            claim.admission_hash,
            worker.identity.node_id,
            claim.input_root,
            claim.output_root,
            claim.sequence_end,
        );
        sign_work_receipt(
            &worker.key,
            WorkReceipt {
                abi_version: WORK_WIRE_ABI_VERSION,
                receipt_id,
                request_hash: claim.request_hash,
                admission_hash: claim.admission_hash,
                worker: worker.identity.clone(),
                relay_node_id: [0u8; 32],
                input_hash: claim.input_root,
                output_hash: claim.output_root,
                units_used: claim.units_used,
                total_claim: claim.total_claim,
                sequence: claim.sequence_end,
                signature: empty_signature(),
            },
        )
    }

    #[test]
    fn role_claim_settlement_binds_receipt_admission_and_worker_claim() {
        let admission_node = SimNode::from_seed(1, NODE_ROLE_ADMISSION);
        let worker = SimNode::from_seed(2, NODE_ROLE_STORAGE);
        let user = [3u8; 32];
        let admission = signed_admission(
            &admission_node,
            user,
            blake3_hash(b"request"),
            blake3_hash(b"policy"),
            100,
        );
        let admission_hash = work_admission_hash(&admission).expect("admission hash");
        let claim = signed_claim(
            &worker,
            &admission,
            admission_hash,
            WORK_TYPE_OBJECT_RETRIEVE,
            1,
            8,
            40,
        );
        let receipt = receipt_for_claim(&worker, &claim);

        let mut ledger = SettlementLedger::new();
        ledger.deposit_user_credit(user, 100);
        ledger
            .reserve_admission_budget(&admission)
            .expect("reserve admission budget");
        let result = ledger
            .settle_role_claim(&admission, &receipt, &claim, 5_000)
            .expect("settle role claim");

        assert_eq!(result.amount, 40);
        assert_eq!(ledger.worker_balance(&worker.identity.node_id), 40);
        assert!(ledger.is_role_claim_paid(&role_work_claim_hash(&claim)));

        let bytes = wire_bytes(&claim).expect("claim bytes");
        let decoded =
            wire_from_bytes::<RoleWorkClaim, RoleWorkClaim>(&bytes).expect("claim roundtrip");
        assert_eq!(decoded, claim);
    }

    #[test]
    fn role_claim_settlement_rejects_overlapping_sequence_ranges() {
        let admission_node = SimNode::from_seed(11, NODE_ROLE_ADMISSION);
        let worker = SimNode::from_seed(12, NODE_ROLE_STORAGE);
        let user = [13u8; 32];
        let admission = signed_admission(
            &admission_node,
            user,
            blake3_hash(b"range-request"),
            blake3_hash(b"range-policy"),
            100,
        );
        let admission_hash = work_admission_hash(&admission).expect("admission hash");
        let claim = signed_claim(
            &worker,
            &admission,
            admission_hash,
            WORK_TYPE_OBJECT_RETRIEVE,
            1,
            8,
            20,
        );
        let receipt = receipt_for_claim(&worker, &claim);
        let overlapping = signed_claim(
            &worker,
            &admission,
            admission_hash,
            WORK_TYPE_OBJECT_RETRIEVE,
            8,
            12,
            20,
        );
        let overlapping_receipt = receipt_for_claim(&worker, &overlapping);

        let mut ledger = SettlementLedger::new();
        ledger.deposit_user_credit(user, 100);
        ledger
            .reserve_admission_budget(&admission)
            .expect("reserve admission budget");
        ledger
            .settle_role_claim(&admission, &receipt, &claim, 5_000)
            .expect("first claim");

        assert!(matches!(
            ledger.settle_role_claim(&admission, &overlapping_receipt, &overlapping, 5_000),
            Err(SettlementError::ClaimRangeOverlap)
        ));
    }

    #[test]
    fn role_claim_settlement_rejects_receipt_mismatch_and_expiry() {
        let admission_node = SimNode::from_seed(21, NODE_ROLE_ADMISSION);
        let worker = SimNode::from_seed(22, NODE_ROLE_COMPUTE);
        let user = [23u8; 32];
        let admission = signed_admission(
            &admission_node,
            user,
            blake3_hash(b"compute-request"),
            blake3_hash(b"compute-policy"),
            100,
        );
        let admission_hash = work_admission_hash(&admission).expect("admission hash");
        let claim = signed_claim(
            &worker,
            &admission,
            admission_hash,
            WORK_TYPE_COMPUTE_RUN,
            1,
            1,
            10,
        );
        let mut receipt = receipt_for_claim(&worker, &claim);
        receipt.output_hash = blake3_hash(b"wrong-output");
        receipt = sign_work_receipt(&worker.key, receipt);

        let mut ledger = SettlementLedger::new();
        ledger.deposit_user_credit(user, 100);
        ledger
            .reserve_admission_budget(&admission)
            .expect("reserve admission budget");

        assert!(matches!(
            ledger.settle_role_claim(&admission, &receipt, &claim, 5_000),
            Err(SettlementError::RoleClaimReceiptMismatch)
        ));

        let receipt = receipt_for_claim(&worker, &claim);
        assert!(matches!(
            ledger.settle_role_claim(&admission, &receipt, &claim, 10_001),
            Err(SettlementError::RoleClaimExpired)
        ));
    }

    #[test]
    fn role_claim_rejects_cross_domain_admission_substitution() {
        let admission_a = SimNode::from_seed(51, NODE_ROLE_ADMISSION);
        let admission_b = SimNode::from_seed(52, NODE_ROLE_ADMISSION);
        let worker = SimNode::from_seed(53, NODE_ROLE_STORAGE);
        let user = [54u8; 32];
        let request_hash = blake3_hash(b"domain-request");
        let policy_hash = blake3_hash(b"domain-policy");
        let admitted_a = signed_admission(&admission_a, user, request_hash, policy_hash, 100);
        let admitted_b = signed_admission(&admission_b, user, request_hash, policy_hash, 100);
        let admission_hash_a = work_admission_hash(&admitted_a).expect("admission a hash");
        let claim = signed_claim(
            &worker,
            &admitted_a,
            admission_hash_a,
            WORK_TYPE_OBJECT_RETRIEVE,
            1,
            4,
            10,
        );
        let receipt = receipt_for_claim(&worker, &claim);

        let mut ledger = SettlementLedger::new();
        ledger.deposit_user_credit(user, 100);
        ledger
            .reserve_admission_budget(&admitted_b)
            .expect("reserve substituted admission budget");

        assert!(matches!(
            ledger.settle_role_claim(&admitted_b, &receipt, &claim, 1_000),
            Err(SettlementError::ReceiptAdmissionMismatch)
                | Err(SettlementError::RoleClaimAdmissionMismatch)
        ));
    }

    #[test]
    fn role_claim_rejects_wrong_controlling_admission_node() {
        let admission = SimNode::from_seed(61, NODE_ROLE_ADMISSION);
        let other_admission = SimNode::from_seed(62, NODE_ROLE_ADMISSION);
        let worker = SimNode::from_seed(63, NODE_ROLE_RELAY);
        let user = [64u8; 32];
        let admitted = signed_admission(
            &admission,
            user,
            blake3_hash(b"relay-control-request"),
            blake3_hash(b"relay-control-policy"),
            100,
        );
        let admission_hash = work_admission_hash(&admitted).expect("admission hash");
        let mut claim = signed_claim(
            &worker,
            &admitted,
            admission_hash,
            WORK_TYPE_MESSAGE_DELIVER,
            1,
            4,
            10,
        );
        claim.controlling_admission_node_id = other_admission.identity.node_id;
        claim = sign_role_work_claim(&worker.key, claim);
        let receipt = receipt_for_claim(&worker, &claim);

        let mut ledger = SettlementLedger::new();
        ledger.deposit_user_credit(user, 100);
        ledger
            .reserve_admission_budget(&admitted)
            .expect("reserve admission budget");

        assert!(matches!(
            ledger.settle_role_claim(&admitted, &receipt, &claim, 1_000),
            Err(SettlementError::RoleClaimAdmissionMismatch)
        ));
    }

    #[test]
    fn role_claim_range_overlap_is_scoped_to_admission_and_work_kind() {
        let admission_node = SimNode::from_seed(71, NODE_ROLE_ADMISSION);
        let worker = SimNode::from_seed(72, NODE_ROLE_STORAGE);
        let user = [73u8; 32];
        let admitted_a = signed_admission(
            &admission_node,
            user,
            blake3_hash(b"range-scope-a"),
            blake3_hash(b"range-scope-policy"),
            100,
        );
        let admitted_b = signed_admission(
            &admission_node,
            user,
            blake3_hash(b"range-scope-b"),
            blake3_hash(b"range-scope-policy"),
            100,
        );
        let hash_a = work_admission_hash(&admitted_a).expect("hash a");
        let hash_b = work_admission_hash(&admitted_b).expect("hash b");
        let claim_a = signed_claim(
            &worker,
            &admitted_a,
            hash_a,
            WORK_TYPE_OBJECT_RETRIEVE,
            1,
            5,
            10,
        );
        let receipt_a = receipt_for_claim(&worker, &claim_a);
        let different_kind = signed_claim(
            &worker,
            &admitted_a,
            hash_a,
            WORK_TYPE_OBJECT_STORE,
            3,
            5,
            10,
        );
        let different_kind_receipt = receipt_for_claim(&worker, &different_kind);
        let different_admission = signed_claim(
            &worker,
            &admitted_b,
            hash_b,
            WORK_TYPE_OBJECT_RETRIEVE,
            3,
            5,
            10,
        );
        let different_admission_receipt = receipt_for_claim(&worker, &different_admission);

        let mut ledger = SettlementLedger::new();
        ledger.deposit_user_credit(user, 200);
        ledger
            .reserve_admission_budget(&admitted_a)
            .expect("reserve admission a budget");
        ledger
            .reserve_admission_budget(&admitted_b)
            .expect("reserve admission b budget");
        ledger
            .settle_role_claim(&admitted_a, &receipt_a, &claim_a, 1_000)
            .expect("first claim");
        ledger
            .settle_role_claim(&admitted_a, &different_kind_receipt, &different_kind, 1_000)
            .expect("same range with different work kind");
        ledger
            .settle_role_claim(
                &admitted_b,
                &different_admission_receipt,
                &different_admission,
                1_000,
            )
            .expect("same range under different admission");
    }

    #[test]
    fn role_claim_rejects_zero_units_backwards_range_and_budget_abuse() {
        let admission_node = SimNode::from_seed(81, NODE_ROLE_ADMISSION);
        let worker = SimNode::from_seed(82, NODE_ROLE_COMPUTE);
        let user = [83u8; 32];
        let admission = signed_admission(
            &admission_node,
            user,
            blake3_hash(b"dos-economics-request"),
            blake3_hash(b"dos-economics-policy"),
            15,
        );
        let admission_hash = work_admission_hash(&admission).expect("admission hash");
        let mut zero_units = signed_claim(
            &worker,
            &admission,
            admission_hash,
            WORK_TYPE_COMPUTE_RUN,
            1,
            1,
            5,
        );
        zero_units.units_used = 0;
        zero_units = sign_role_work_claim(&worker.key, zero_units);
        let zero_units_receipt = receipt_for_claim(&worker, &zero_units);
        let mut backwards = signed_claim(
            &worker,
            &admission,
            admission_hash,
            WORK_TYPE_COMPUTE_RUN,
            1,
            1,
            5,
        );
        backwards.sequence_start = 4;
        backwards.sequence_end = 1;
        backwards.units_used = 1;
        backwards.claim_id = role_work_claim_id(
            &worker.identity,
            admission_hash,
            backwards.work_kind,
            backwards.sequence_start,
            backwards.sequence_end,
            backwards.input_root,
            backwards.output_root,
            backwards.evidence_root,
        );
        backwards = sign_role_work_claim(&worker.key, backwards);
        let backwards_receipt = receipt_for_claim(&worker, &backwards);

        let mut ledger = SettlementLedger::new();
        ledger.deposit_user_credit(user, 100);
        ledger
            .reserve_admission_budget(&admission)
            .expect("reserve admission budget");
        assert!(matches!(
            ledger.settle_role_claim(&admission, &zero_units_receipt, &zero_units, 1_000),
            Err(SettlementError::InvalidRoleClaim)
        ));
        assert!(matches!(
            ledger.settle_role_claim(&admission, &backwards_receipt, &backwards, 1_000),
            Err(SettlementError::InvalidRoleClaim)
        ));

        let ok = signed_claim(
            &worker,
            &admission,
            admission_hash,
            WORK_TYPE_COMPUTE_RUN,
            1,
            1,
            10,
        );
        let ok_receipt = receipt_for_claim(&worker, &ok);
        let over_budget = signed_claim(
            &worker,
            &admission,
            admission_hash,
            WORK_TYPE_COMPUTE_RUN,
            2,
            2,
            10,
        );
        let over_budget_receipt = receipt_for_claim(&worker, &over_budget);
        ledger
            .settle_role_claim(&admission, &ok_receipt, &ok, 1_000)
            .expect("within budget");
        assert!(matches!(
            ledger.settle_role_claim(&admission, &over_budget_receipt, &over_budget, 1_000),
            Err(SettlementError::ClaimExceedsAdmissionBudget)
        ));
    }

    #[test]
    fn colluding_verifier_report_does_not_settle_invalid_claim() {
        let admission_node = SimNode::from_seed(91, NODE_ROLE_ADMISSION);
        let worker = SimNode::from_seed(92, NODE_ROLE_STORAGE);
        let verifier = SimNode::from_seed(93, NODE_ROLE_VERIFIER);
        let user = [94u8; 32];
        let admission = signed_admission(
            &admission_node,
            user,
            blake3_hash(b"collusion-request"),
            blake3_hash(b"collusion-policy"),
            100,
        );
        let admission_hash = work_admission_hash(&admission).expect("admission hash");
        let claim = signed_claim(
            &worker,
            &admission,
            admission_hash,
            WORK_TYPE_OBJECT_RETRIEVE,
            1,
            1,
            10,
        );
        let mut receipt = receipt_for_claim(&worker, &claim);
        receipt.input_hash = blake3_hash(b"forged-input");
        receipt = sign_work_receipt(&worker.key, receipt);
        let colluding_report = sign_verification_report(
            &verifier.key,
            VerificationReport {
                abi_version: WORK_WIRE_ABI_VERSION,
                verifier: verifier.identity.clone(),
                verified_worker_node_id: worker.identity.node_id,
                verified_worker_role: worker.identity.role,
                claim_hash: role_work_claim_hash(&claim),
                evidence_root: claim.evidence_root,
                verifier_policy_hash: admission.policy_hash,
                result: VERIFICATION_RESULT_ACCEPTED,
                checked_at_unix_ms: 1_000,
                signature: empty_signature(),
            },
        );
        assert!(verify_verification_report(&colluding_report));

        let mut ledger = SettlementLedger::new();
        ledger.deposit_user_credit(user, 100);
        ledger
            .reserve_admission_budget(&admission)
            .expect("reserve admission budget");
        assert!(matches!(
            ledger.settle_role_claim(&admission, &receipt, &claim, 1_000),
            Err(SettlementError::RoleClaimReceiptMismatch)
        ));
        assert_eq!(ledger.worker_balance(&worker.identity.node_id), 0);
    }

    #[test]
    fn verifier_and_notary_reports_reject_wrong_role_signers() {
        let storage = SimNode::from_seed(101, NODE_ROLE_STORAGE);
        let notary = SimNode::from_seed(102, NODE_ROLE_NOTARY);
        let worker = SimNode::from_seed(103, NODE_ROLE_COMPUTE);
        let verifier_report = sign_verification_report(
            &storage.key,
            VerificationReport {
                abi_version: WORK_WIRE_ABI_VERSION,
                verifier: storage.identity.clone(),
                verified_worker_node_id: worker.identity.node_id,
                verified_worker_role: worker.identity.role,
                claim_hash: blake3_hash(b"wrong-role-claim"),
                evidence_root: blake3_hash(b"wrong-role-evidence"),
                verifier_policy_hash: blake3_hash(b"wrong-role-policy"),
                result: VERIFICATION_RESULT_ACCEPTED,
                checked_at_unix_ms: 1,
                signature: empty_signature(),
            },
        );
        assert!(!verify_verification_report(&verifier_report));

        let mut notary_report = sign_notary_work_report(
            &notary.key,
            NotaryWorkReport {
                abi_version: WORK_WIRE_ABI_VERSION,
                notary: notary.identity.clone(),
                requester: worker.identity.node_id,
                request_hash: blake3_hash(b"notary-wrong-role-request"),
                admission_hash: blake3_hash(b"notary-wrong-role-admission"),
                work_type: WORK_TYPE_OBJECT_RETRIEVE,
                request_message_id: blake3_hash(b"notary-wrong-role-message"),
                request_payload_hash: blake3_hash(b"notary-wrong-role-payload"),
                via_relay: [0u8; 32],
                sequence: 1,
                aad_hash: notary_aad_hash(b"aad"),
                input_hash: blake3_hash(b"input"),
                output_hash: blake3_hash(b"output"),
                response_hash: blake3_hash(b"response"),
                completed_unix_ms: 1,
                signature: empty_signature(),
            },
        );
        assert!(!verify_notary_work_report(&notary_report));
        notary_report.work_type = WORK_TYPE_NOTARY_SEAL;
        assert!(!verify_notary_work_report(&notary_report));
    }

    #[test]
    fn public_claim_and_reports_do_not_embed_private_payload_bytes() {
        let admission_node = SimNode::from_seed(111, NODE_ROLE_ADMISSION);
        let worker = SimNode::from_seed(112, NODE_ROLE_STORAGE);
        let verifier = SimNode::from_seed(113, NODE_ROLE_VERIFIER);
        let notary = SimNode::from_seed(114, NODE_ROLE_NOTARY);
        let user = [115u8; 32];
        let secret = b"private payload that should not appear in public proof";
        let admission = signed_admission(
            &admission_node,
            user,
            blake3_hash(secret),
            blake3_hash(b"privacy-policy"),
            100,
        );
        let admission_hash = work_admission_hash(&admission).expect("admission hash");
        let mut claim = signed_claim(
            &worker,
            &admission,
            admission_hash,
            WORK_TYPE_OBJECT_RETRIEVE,
            1,
            1,
            10,
        );
        claim.input_root = blake3_hash(secret);
        claim.output_root = blake3_hash(secret);
        claim.evidence_root = blake3_hash(secret);
        claim.claim_id = role_work_claim_id(
            &worker.identity,
            admission_hash,
            claim.work_kind,
            claim.sequence_start,
            claim.sequence_end,
            claim.input_root,
            claim.output_root,
            claim.evidence_root,
        );
        claim = sign_role_work_claim(&worker.key, claim);
        let verification_report = sign_verification_report(
            &verifier.key,
            VerificationReport {
                abi_version: WORK_WIRE_ABI_VERSION,
                verifier: verifier.identity.clone(),
                verified_worker_node_id: worker.identity.node_id,
                verified_worker_role: worker.identity.role,
                claim_hash: role_work_claim_hash(&claim),
                evidence_root: claim.evidence_root,
                verifier_policy_hash: admission.policy_hash,
                result: VERIFICATION_RESULT_ACCEPTED,
                checked_at_unix_ms: 1,
                signature: empty_signature(),
            },
        );
        let notary_report = sign_notary_work_report(
            &notary.key,
            NotaryWorkReport {
                abi_version: WORK_WIRE_ABI_VERSION,
                notary: notary.identity.clone(),
                requester: worker.identity.node_id,
                request_hash: admission.request_hash,
                admission_hash,
                work_type: WORK_TYPE_NOTARY_SEAL,
                request_message_id: blake3_hash(b"privacy-message"),
                request_payload_hash: blake3_hash(secret),
                via_relay: [0u8; 32],
                sequence: 1,
                aad_hash: notary_aad_hash(b"aad"),
                input_hash: blake3_hash(secret),
                output_hash: notary_sealed_hash(b"sealed"),
                response_hash: blake3_hash(b"response"),
                completed_unix_ms: 1,
                signature: empty_signature(),
            },
        );

        for public_bytes in [
            wire_bytes(&claim).expect("claim bytes"),
            wire_bytes(&verification_report).expect("verification report bytes"),
            wire_bytes(&notary_report).expect("notary report bytes"),
        ] {
            assert!(
                !public_bytes
                    .windows(secret.len())
                    .any(|window| window == secret)
            );
        }
    }

    #[test]
    fn verification_report_is_signed_by_verifier_role() {
        let worker = SimNode::from_seed(31, NODE_ROLE_STORAGE);
        let verifier = SimNode::from_seed(32, NODE_ROLE_VERIFIER);
        let report = sign_verification_report(
            &verifier.key,
            VerificationReport {
                abi_version: WORK_WIRE_ABI_VERSION,
                verifier: verifier.identity.clone(),
                verified_worker_node_id: worker.identity.node_id,
                verified_worker_role: worker.identity.role,
                claim_hash: blake3_hash(b"claim-hash"),
                evidence_root: blake3_hash(b"evidence-root"),
                verifier_policy_hash: blake3_hash(b"verifier-policy"),
                result: VERIFICATION_RESULT_ACCEPTED,
                checked_at_unix_ms: 123,
                signature: empty_signature(),
            },
        );
        assert!(verify_verification_report(&report));
        let bytes = wire_bytes(&report).expect("report bytes");
        let decoded = wire_from_bytes::<VerificationReport, VerificationReport>(&bytes)
            .expect("report roundtrip");
        assert_eq!(decoded, report);

        let mut tampered = report.clone();
        tampered.result = VERIFICATION_RESULT_REJECTED;
        assert!(!verify_verification_report(&tampered));
    }

    #[test]
    fn notary_work_report_signs_seal_or_unseal_output_claim() {
        let notary = SimNode::from_seed(41, NODE_ROLE_NOTARY);
        let report = sign_notary_work_report(
            &notary.key,
            NotaryWorkReport {
                abi_version: WORK_WIRE_ABI_VERSION,
                notary: notary.identity.clone(),
                requester: [42u8; 32],
                request_hash: blake3_hash(b"notary-request"),
                admission_hash: blake3_hash(b"notary-admission"),
                work_type: WORK_TYPE_NOTARY_SEAL,
                request_message_id: blake3_hash(b"message"),
                request_payload_hash: blake3_hash(b"payload"),
                via_relay: [43u8; 32],
                sequence: 7,
                aad_hash: notary_aad_hash(b"aad"),
                input_hash: blake3_hash(b"plain"),
                output_hash: notary_sealed_hash(b"sealed"),
                response_hash: blake3_hash(b"response"),
                completed_unix_ms: 456,
                signature: empty_signature(),
            },
        );
        assert!(verify_notary_work_report(&report));
        let bytes = wire_bytes(&report).expect("notary report bytes");
        let decoded = wire_from_bytes::<NotaryWorkReport, NotaryWorkReport>(&bytes)
            .expect("notary report roundtrip");
        assert_eq!(decoded, report);

        let mut tampered = report.clone();
        tampered.output_hash = blake3_hash(b"different-sealed");
        assert!(!verify_notary_work_report(&tampered));
    }
}
