use alloc::collections::{BTreeMap, BTreeSet};

use crate::batch_settlement::{BatchSettlementError, BatchSettlementResult};
use crate::channel::ChannelProof;
use crate::channel_order::OrderedChannelEnvelope;
use crate::codec::{EdgeWire, WireCursor, WireWriter, blake3_hash, packet_bytes};
use crate::delivery_proof::{channel_proof_hash, verify_channel_proof_for_ordered_with_policy};
use crate::preimage::HashBuilder;
use crate::protocol::*;
use crate::recipient_policy::{
    RecipientMessagePolicy, recipient_message_policy_allows, recipient_message_policy_hash,
};
use crate::relay_role::ordered_message_input_hash;
use crate::role_claim::{RoleWorkClaim, role_work_claim_hash, verify_role_work_claim};
use crate::signing::{verify_work_admission, verify_work_receipt};
use crate::transit_proof::{
    PacketTransitHashInput, RELAY_PROOF_CUSTODY_KIND_NOTARIZED, RELAY_PROOF_CUSTODY_KIND_PACKED,
    RELAY_PROOF_CUSTODY_KIND_STORED, RelayProofCustodyAck, RelayTransitBundle,
    RelayTransitBundleContext, packet_transit_hash, relay_delivery_output_hash,
    relay_transit_bundle_route_commitment, verify_relay_proof_custody_ack_for_bundle,
    verify_relay_proof_custody_chain, verify_relay_transit_bundle,
};

const RECEIPT_ID_DOMAIN: &[u8] = b"edgerun:v1:work:receipt-id";
const RELAY_TRANSIT_CUSTODY_REQUIREMENT_DOMAIN: &[u8] =
    b"edgerun:v1:work:relay-transit-custody-requirement";
const SETTLEMENT_RESULT_DOMAIN: &[u8] = b"edgerun:v1:work:settlement-result";
const RELAY_TRANSIT_CUSTODY_CHECK_RESULT_DOMAIN: &[u8] =
    b"edgerun:v1:work:relay-transit-custody-check-result";
const RELAY_TRANSIT_CUSTODY_SETTLEMENT_RESULT_DOMAIN: &[u8] =
    b"edgerun:v1:work:relay-transit-custody-settlement-result";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettlementError {
    InvalidAdmission,
    InvalidReceipt,
    UnknownUser,
    InsufficientBalance,
    DuplicateAdmission,
    AdmissionBudgetNotReserved,
    DuplicateReceipt,
    ClaimExceedsAdmissionBudget,
    ReceiptAdmissionMismatch,
    PacketHashFailed,
    WrongWorkerRole,
    WrongRelay,
    WrongRecipient,
    InvalidRecipientProof,
    ReceiptInputMismatch,
    ReceiptOutputMismatch,
    DeliveryPacketMismatch,
    AdmissionRouteMismatch,
    PolicyHashMismatch,
    MessagePolicyRejected,
    EvidenceRequired,
    InvalidRoleClaim,
    RoleClaimAdmissionMismatch,
    RoleClaimReceiptMismatch,
    RoleClaimExpired,
    ClaimRangeOverlap,
    InvalidRelayTransitBundle,
    RelayTransitHopMissing,
    RelayTransitReceiptMismatch,
    InvalidRelayProofCustodyAck,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettlementResult {
    pub user: PublicKey,
    pub worker: NodeId,
    pub amount: u64,
    pub user_balance_after: u64,
    pub worker_balance_after: u64,
    pub admission_spent_after: u64,
    pub receipt_hash: Hash,
}

impl EdgeWire for SettlementResult {
    fn encode_wire(&self, out: &mut WireWriter) {
        self.user.encode_wire(out);
        self.worker.encode_wire(out);
        self.amount.encode_wire(out);
        self.user_balance_after.encode_wire(out);
        self.worker_balance_after.encode_wire(out);
        self.admission_spent_after.encode_wire(out);
        self.receipt_hash.encode_wire(out);
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        Ok(Self {
            user: <PublicKey as EdgeWire>::decode_wire(input)?,
            worker: <NodeId as EdgeWire>::decode_wire(input)?,
            amount: <u64 as EdgeWire>::decode_wire(input)?,
            user_balance_after: <u64 as EdgeWire>::decode_wire(input)?,
            worker_balance_after: <u64 as EdgeWire>::decode_wire(input)?,
            admission_spent_after: <u64 as EdgeWire>::decode_wire(input)?,
            receipt_hash: <Hash as EdgeWire>::decode_wire(input)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayTransitCustodyCheckResult {
    pub custody_hash: Hash,
    pub custody_requirement_hash: Hash,
}

impl EdgeWire for RelayTransitCustodyCheckResult {
    fn encode_wire(&self, out: &mut WireWriter) {
        self.custody_hash.encode_wire(out);
        self.custody_requirement_hash.encode_wire(out);
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        Ok(Self {
            custody_hash: <Hash as EdgeWire>::decode_wire(input)?,
            custody_requirement_hash: <Hash as EdgeWire>::decode_wire(input)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayTransitCustodySettlementResult {
    pub settlement: SettlementResult,
    pub custody: RelayTransitCustodyCheckResult,
}

impl EdgeWire for RelayTransitCustodySettlementResult {
    fn encode_wire(&self, out: &mut WireWriter) {
        self.settlement.encode_wire(out);
        self.custody.encode_wire(out);
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        Ok(Self {
            settlement: SettlementResult::decode_wire(input)?,
            custody: RelayTransitCustodyCheckResult::decode_wire(input)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettlementPruneResult {
    pub removed_admission: bool,
    pub removed_receipts: u64,
    pub refunded_unspent_budget: u64,
}

pub struct DeliverySettlementEvidence<'a> {
    pub admission: &'a WorkAdmission,
    pub receipt: &'a WorkReceipt,
    pub relay_input: &'a OrderedChannelEnvelope,
    pub recipient_delivery: &'a OrderedChannelEnvelope,
    pub recipient: &'a NodeIdentity,
    pub recipient_policy: &'a RecipientMessagePolicy,
    pub recipient_proof: &'a ChannelProof,
    pub previous_transit_hash: Hash,
    pub now_unix_ms: u64,
}

pub struct RelayTransitSettlementEvidence<'a> {
    pub admission: &'a WorkAdmission,
    pub receipt: &'a WorkReceipt,
    pub bundle: &'a RelayTransitBundle,
    pub recipient_delivery: &'a OrderedChannelEnvelope,
    pub recipient: &'a NodeIdentity,
    pub recipient_policy: &'a RecipientMessagePolicy,
    pub recipient_proof: &'a ChannelProof,
    pub source_node_id: NodeId,
    pub destination_node_id: NodeId,
    pub now_unix_ms: u64,
    pub max_hops: usize,
}

pub struct RelayTransitCustodyEvidence<'a> {
    pub bundle: &'a RelayTransitBundle,
    pub custody_ack: &'a RelayProofCustodyAck,
    pub requirement: RelayTransitCustodyRequirement,
    pub now_unix_ms: u64,
}

pub struct RelayTransitCustodyChainEvidence<'a> {
    pub bundle: &'a RelayTransitBundle,
    pub custody_acks: &'a [RelayProofCustodyAck],
    pub requirement: RelayTransitCustodyRequirement,
    pub now_unix_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelayTransitCustodyRequirement {
    pub expected_packed_proof_root: Hash,
    pub required_custody_kind: u16,
}

impl EdgeWire for RelayTransitCustodyRequirement {
    fn encode_wire(&self, out: &mut WireWriter) {
        self.expected_packed_proof_root.encode_wire(out);
        self.required_custody_kind.encode_wire(out);
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        Ok(Self {
            expected_packed_proof_root: <Hash as EdgeWire>::decode_wire(input)?,
            required_custody_kind: <u16 as EdgeWire>::decode_wire(input)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PaidClaimRange {
    worker: NodeId,
    admission_hash: Hash,
    work_kind: u16,
    sequence_start: u64,
    sequence_end: u64,
}

#[derive(Clone, Debug, Default)]
pub struct SettlementLedger {
    user_balances: BTreeMap<PublicKey, u64>,
    worker_balances: BTreeMap<NodeId, u64>,
    reserved_admission_budget: BTreeMap<Hash, u64>,
    reserved_admission_user: BTreeMap<Hash, PublicKey>,
    admission_spend: BTreeMap<Hash, u64>,
    admission_receipts: BTreeMap<Hash, BTreeSet<Hash>>,
    paid_receipts: BTreeSet<Hash>,
    paid_claims: BTreeSet<Hash>,
    paid_claim_ranges: BTreeSet<PaidClaimRange>,
}

impl SettlementLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn deposit_user_credit(&mut self, user: PublicKey, amount: u64) -> u64 {
        let balance = self.user_balances.entry(user).or_default();
        *balance = balance.saturating_add(amount);
        *balance
    }

    pub fn user_balance(&self, user: &PublicKey) -> u64 {
        *self.user_balances.get(user).unwrap_or(&0)
    }

    pub fn worker_balance(&self, worker: &NodeId) -> u64 {
        *self.worker_balances.get(worker).unwrap_or(&0)
    }

    pub fn admission_spent(&self, admission_hash: &Hash) -> u64 {
        *self.admission_spend.get(admission_hash).unwrap_or(&0)
    }

    pub fn reserved_admission_budget(&self, admission_hash: &Hash) -> u64 {
        *self
            .reserved_admission_budget
            .get(admission_hash)
            .unwrap_or(&0)
    }

    pub fn reserve_admission_budget(
        &mut self,
        admission: &WorkAdmission,
    ) -> Result<Hash, SettlementError> {
        if !verify_work_admission(admission) {
            return Err(SettlementError::InvalidAdmission);
        }
        let admission_hash = work_admission_hash(admission)?;
        if self.reserved_admission_budget.contains_key(&admission_hash) {
            return Err(SettlementError::DuplicateAdmission);
        }
        let user_balance = self
            .user_balances
            .get_mut(&admission.user)
            .ok_or(SettlementError::UnknownUser)?;
        if *user_balance < admission.admitted_budget {
            return Err(SettlementError::InsufficientBalance);
        }
        *user_balance -= admission.admitted_budget;
        self.reserved_admission_budget
            .insert(admission_hash, admission.admitted_budget);
        self.reserved_admission_user
            .insert(admission_hash, admission.user);
        self.admission_spend.entry(admission_hash).or_insert(0);
        Ok(admission_hash)
    }

    pub fn is_receipt_paid(&self, receipt_hash: &Hash) -> bool {
        self.paid_receipts.contains(receipt_hash)
    }

    pub fn is_role_claim_paid(&self, claim_hash: &Hash) -> bool {
        self.paid_claims.contains(claim_hash)
    }

    pub fn paid_receipt_count(&self) -> usize {
        self.paid_receipts.len()
    }

    pub fn tracked_admission_count(&self) -> usize {
        self.admission_spend.len()
    }

    pub fn can_settle_receipt_unchecked_evidence(
        &self,
        admission: &WorkAdmission,
        receipt: &WorkReceipt,
    ) -> Result<(), SettlementError> {
        if receipt.worker.role == NODE_ROLE_RELAY {
            return Err(SettlementError::EvidenceRequired);
        }
        self.can_settle_receipt_common(admission, receipt)
    }

    pub fn can_settle_delivery(
        &self,
        evidence: &DeliverySettlementEvidence<'_>,
    ) -> Result<(), SettlementError> {
        self.can_settle_receipt_common(evidence.admission, evidence.receipt)?;
        verify_delivery_evidence(evidence)?;
        Ok(())
    }

    pub fn can_settle_relay_transit_hop(
        &self,
        evidence: &RelayTransitSettlementEvidence<'_>,
    ) -> Result<(), SettlementError> {
        self.can_settle_receipt_common(evidence.admission, evidence.receipt)?;
        verify_relay_transit_settlement_evidence(evidence)?;
        Ok(())
    }

    pub fn can_settle_relay_transit_hop_with_custody(
        &self,
        transit: &RelayTransitSettlementEvidence<'_>,
        custody: &RelayTransitCustodyChainEvidence<'_>,
    ) -> Result<RelayTransitCustodyCheckResult, SettlementError> {
        self.can_settle_relay_transit_hop(transit)?;
        if custody.bundle.request_hash != transit.bundle.request_hash
            || custody.bundle.admission_hash != transit.bundle.admission_hash
            || custody.bundle.bundle_root != transit.bundle.bundle_root
        {
            return Err(SettlementError::InvalidRelayProofCustodyAck);
        }
        let custody_hash = verify_relay_transit_custody_chain_evidence(custody)?;
        let custody_requirement_hash = relay_transit_custody_requirement_hash(&custody.requirement);
        Ok(RelayTransitCustodyCheckResult {
            custody_hash,
            custody_requirement_hash,
        })
    }

    pub fn can_settle_role_claim(
        &self,
        admission: &WorkAdmission,
        receipt: &WorkReceipt,
        claim: &RoleWorkClaim,
        now_unix_ms: u64,
    ) -> Result<(), SettlementError> {
        self.can_settle_receipt_common(admission, receipt)?;
        verify_role_claim_common(admission, receipt, claim, now_unix_ms)?;
        let claim_hash = role_work_claim_hash(claim);
        if self.paid_claims.contains(&claim_hash) {
            return Err(SettlementError::DuplicateReceipt);
        }
        if self.claim_range_overlaps(claim) {
            return Err(SettlementError::ClaimRangeOverlap);
        }
        Ok(())
    }

    pub fn settle_receipt_unchecked_evidence(
        &mut self,
        admission: &WorkAdmission,
        receipt: &WorkReceipt,
    ) -> Result<SettlementResult, SettlementError> {
        self.can_settle_receipt_unchecked_evidence(admission, receipt)?;
        self.commit_settlement(admission, receipt)
    }

    pub fn settle_receipt(
        &mut self,
        admission: &WorkAdmission,
        receipt: &WorkReceipt,
    ) -> Result<SettlementResult, SettlementError> {
        self.can_settle_receipt_common(admission, receipt)?;
        self.commit_settlement(admission, receipt)
    }

    pub fn settle_receipt_batch(
        &mut self,
        admission: &WorkAdmission,
        receipts: &[WorkReceipt],
    ) -> Result<BatchSettlementResult, BatchSettlementError> {
        self.settle_receipt_batch_unchecked_evidence(admission, receipts)
    }

    pub fn settle_delivery(
        &mut self,
        evidence: &DeliverySettlementEvidence<'_>,
    ) -> Result<SettlementResult, SettlementError> {
        self.can_settle_delivery(evidence)?;
        self.commit_settlement(evidence.admission, evidence.receipt)
    }

    pub fn settle_relay_transit_hop(
        &mut self,
        evidence: &RelayTransitSettlementEvidence<'_>,
    ) -> Result<SettlementResult, SettlementError> {
        self.can_settle_relay_transit_hop(evidence)?;
        self.commit_settlement(evidence.admission, evidence.receipt)
    }

    pub fn settle_relay_transit_hop_with_custody(
        &mut self,
        transit: &RelayTransitSettlementEvidence<'_>,
        custody: &RelayTransitCustodyChainEvidence<'_>,
    ) -> Result<RelayTransitCustodySettlementResult, SettlementError> {
        let custody = self.can_settle_relay_transit_hop_with_custody(transit, custody)?;
        let settlement = self.commit_settlement(transit.admission, transit.receipt)?;
        Ok(RelayTransitCustodySettlementResult {
            settlement,
            custody,
        })
    }

    pub fn settle_role_claim(
        &mut self,
        admission: &WorkAdmission,
        receipt: &WorkReceipt,
        claim: &RoleWorkClaim,
        now_unix_ms: u64,
    ) -> Result<SettlementResult, SettlementError> {
        self.can_settle_role_claim(admission, receipt, claim, now_unix_ms)?;
        let result = self.commit_settlement(admission, receipt)?;
        self.paid_claims.insert(role_work_claim_hash(claim));
        self.paid_claim_ranges.insert(PaidClaimRange {
            worker: claim.worker.node_id,
            admission_hash: claim.admission_hash,
            work_kind: claim.work_kind,
            sequence_start: claim.sequence_start,
            sequence_end: claim.sequence_end,
        });
        Ok(result)
    }

    fn claim_range_overlaps(&self, claim: &RoleWorkClaim) -> bool {
        self.paid_claim_ranges.iter().any(|paid| {
            paid.worker == claim.worker.node_id
                && paid.admission_hash == claim.admission_hash
                && paid.work_kind == claim.work_kind
                && ranges_overlap(
                    paid.sequence_start,
                    paid.sequence_end,
                    claim.sequence_start,
                    claim.sequence_end,
                )
        })
    }

    fn can_settle_receipt_common(
        &self,
        admission: &WorkAdmission,
        receipt: &WorkReceipt,
    ) -> Result<(), SettlementError> {
        if !verify_work_admission(admission) {
            return Err(SettlementError::InvalidAdmission);
        }
        if !verify_work_receipt(receipt) {
            return Err(SettlementError::InvalidReceipt);
        }
        let admission_hash = work_admission_hash(admission)?;
        if receipt.admission_hash != admission_hash {
            return Err(SettlementError::ReceiptAdmissionMismatch);
        }
        if receipt.request_hash != admission.request_hash {
            return Err(SettlementError::ReceiptAdmissionMismatch);
        }
        let receipt_hash = work_receipt_hash(receipt)?;
        if self.paid_receipts.contains(&receipt_hash) {
            return Err(SettlementError::DuplicateReceipt);
        }
        let already_spent = self.admission_spent(&admission_hash);
        let next_spend = already_spent.saturating_add(receipt.total_claim);
        let Some(reserved_budget) = self.reserved_admission_budget.get(&admission_hash) else {
            return Err(SettlementError::AdmissionBudgetNotReserved);
        };
        if next_spend > *reserved_budget {
            return Err(SettlementError::ClaimExceedsAdmissionBudget);
        }
        Ok(())
    }

    fn commit_settlement(
        &mut self,
        admission: &WorkAdmission,
        receipt: &WorkReceipt,
    ) -> Result<SettlementResult, SettlementError> {
        let admission_hash = work_admission_hash(admission)?;
        let receipt_hash = work_receipt_hash(receipt)?;
        let already_spent = self.admission_spent(&admission_hash);
        let next_spend = already_spent.saturating_add(receipt.total_claim);
        self.paid_receipts.insert(receipt_hash);
        self.admission_receipts
            .entry(admission_hash)
            .or_default()
            .insert(receipt_hash);
        self.admission_spend.insert(admission_hash, next_spend);
        let worker_balance = self
            .worker_balances
            .entry(receipt.worker.node_id)
            .or_default();
        *worker_balance = worker_balance.saturating_add(receipt.total_claim);
        let worker_balance_after = *worker_balance;
        let user_balance_after = self.user_balance(&admission.user);
        Ok(SettlementResult {
            user: admission.user,
            worker: receipt.worker.node_id,
            amount: receipt.total_claim,
            user_balance_after,
            worker_balance_after,
            admission_spent_after: next_spend,
            receipt_hash,
        })
    }

    pub fn prune_finalized_admission(&mut self, admission_hash: &Hash) -> SettlementPruneResult {
        let reserved_budget = self
            .reserved_admission_budget
            .remove(admission_hash)
            .unwrap_or(0);
        let reserved_user = self.reserved_admission_user.remove(admission_hash);
        let spent = self.admission_spend.remove(admission_hash);
        let removed_admission = spent.is_some();
        let refunded_unspent_budget = reserved_budget.saturating_sub(spent.unwrap_or(0));
        if refunded_unspent_budget != 0
            && let Some(user) = reserved_user
        {
            let balance = self.user_balances.entry(user).or_default();
            *balance = balance.saturating_add(refunded_unspent_budget);
        }
        let receipts = self
            .admission_receipts
            .remove(admission_hash)
            .unwrap_or_default();
        let removed_receipts = receipts.len() as u64;
        for receipt_hash in receipts {
            self.paid_receipts.remove(&receipt_hash);
        }
        SettlementPruneResult {
            removed_admission,
            removed_receipts,
            refunded_unspent_budget,
        }
    }
}

pub fn verify_role_claim_common(
    admission: &WorkAdmission,
    receipt: &WorkReceipt,
    claim: &RoleWorkClaim,
    now_unix_ms: u64,
) -> Result<(), SettlementError> {
    if !verify_role_work_claim(claim) {
        return Err(SettlementError::InvalidRoleClaim);
    }
    let admission_hash = work_admission_hash(admission)?;
    if claim.admission_hash != admission_hash
        || claim.request_hash != admission.request_hash
        || claim.policy_hash != admission.policy_hash
        || claim.controlling_admission_node_id != admission.admission_node.node_id
    {
        return Err(SettlementError::RoleClaimAdmissionMismatch);
    }
    if claim.valid_until_unix_ms < now_unix_ms || admission.valid_until_unix_ms < now_unix_ms {
        return Err(SettlementError::RoleClaimExpired);
    }
    if receipt.worker != claim.worker
        || receipt.request_hash != claim.request_hash
        || receipt.admission_hash != claim.admission_hash
        || receipt.input_hash != claim.input_root
        || receipt.output_hash != claim.output_root
        || receipt.units_used != claim.units_used
        || receipt.total_claim != claim.total_claim
        || receipt.sequence != claim.sequence_end
    {
        return Err(SettlementError::RoleClaimReceiptMismatch);
    }
    Ok(())
}

fn ranges_overlap(a_start: u64, a_end: u64, b_start: u64, b_end: u64) -> bool {
    a_start <= b_end && b_start <= a_end
}

pub fn verify_delivery_evidence(
    evidence: &DeliverySettlementEvidence<'_>,
) -> Result<Hash, SettlementError> {
    let receipt = evidence.receipt;
    if receipt.worker.role != NODE_ROLE_RELAY {
        return Err(SettlementError::WrongWorkerRole);
    }
    if receipt.worker.node_id != receipt.relay_node_id {
        return Err(SettlementError::WrongRelay);
    }
    if recipient_message_policy_hash(evidence.recipient_policy) != evidence.admission.policy_hash {
        return Err(SettlementError::PolicyHashMismatch);
    }
    if evidence.admission.assigned_route_commitment != evidence.relay_input.envelope.route_hash
        || evidence.admission.assigned_channel.channel_id
            != evidence.relay_input.envelope.channel_id
    {
        return Err(SettlementError::AdmissionRouteMismatch);
    }
    if evidence.recipient_delivery.envelope.from != receipt.worker.node_id {
        return Err(SettlementError::WrongRelay);
    }
    if evidence.recipient_delivery.envelope.to != evidence.recipient.node_id {
        return Err(SettlementError::WrongRecipient);
    }
    if evidence.relay_input.envelope.packet_hash != evidence.recipient_delivery.envelope.packet_hash
    {
        return Err(SettlementError::DeliveryPacketMismatch);
    }
    let WorkPacket::NetworkMessage(message) = &evidence.relay_input.envelope.packet else {
        return Err(SettlementError::DeliveryPacketMismatch);
    };
    if recipient_message_policy_allows(evidence.recipient_policy, message, evidence.now_unix_ms)
        .is_err()
    {
        return Err(SettlementError::MessagePolicyRejected);
    }
    if message.via_relay != receipt.worker.node_id
        || evidence.relay_input.envelope.to != receipt.worker.node_id
    {
        return Err(SettlementError::WrongRelay);
    }
    if message.to != evidence.recipient.node_id {
        return Err(SettlementError::WrongRecipient);
    }
    if verify_channel_proof_for_ordered_with_policy(
        evidence.recipient_proof,
        evidence.recipient,
        receipt.worker.node_id,
        evidence.recipient_delivery,
        evidence.admission.policy_hash,
    )
    .is_err()
    {
        return Err(SettlementError::InvalidRecipientProof);
    }
    let expected_input = ordered_message_input_hash(evidence.relay_input);
    if receipt.input_hash != expected_input {
        return Err(SettlementError::ReceiptInputMismatch);
    }
    let transit_hash = packet_transit_hash(&PacketTransitHashInput {
        node_id: receipt.worker.node_id,
        from: evidence.relay_input.envelope.from,
        to: evidence.recipient.node_id,
        channel_id: evidence.relay_input.envelope.channel_id,
        route_hash: evidence.recipient_delivery.envelope.route_hash,
        packet_hash: evidence.recipient_delivery.envelope.packet_hash,
        sequence: receipt.sequence,
        previous_transit_hash: evidence.previous_transit_hash,
    });
    let expected_output = relay_delivery_output_hash(
        transit_hash,
        evidence.recipient_delivery.envelope.packet_hash,
        channel_proof_hash(evidence.recipient_proof),
    );
    if receipt.output_hash != expected_output {
        return Err(SettlementError::ReceiptOutputMismatch);
    }
    Ok(transit_hash)
}

pub fn verify_relay_transit_settlement_evidence(
    evidence: &RelayTransitSettlementEvidence<'_>,
) -> Result<Hash, SettlementError> {
    let receipt = evidence.receipt;
    if receipt.worker.role != NODE_ROLE_RELAY {
        return Err(SettlementError::WrongWorkerRole);
    }
    if receipt.worker.node_id != receipt.relay_node_id {
        return Err(SettlementError::WrongRelay);
    }
    if evidence.admission.assigned_relay_path != evidence.bundle.relay_path {
        return Err(SettlementError::AdmissionRouteMismatch);
    }
    if evidence.admission.assigned_route_commitment
        != relay_transit_bundle_route_commitment(evidence.bundle)
    {
        return Err(SettlementError::AdmissionRouteMismatch);
    }
    if evidence.destination_node_id != evidence.recipient.node_id
        || evidence.bundle.destination_node_id != evidence.recipient.node_id
    {
        return Err(SettlementError::WrongRecipient);
    }
    if recipient_message_policy_hash(evidence.recipient_policy) != evidence.admission.policy_hash {
        return Err(SettlementError::PolicyHashMismatch);
    }
    let final_relay = evidence
        .admission
        .assigned_relay_path
        .last()
        .ok_or(SettlementError::AdmissionRouteMismatch)?;
    if evidence.recipient_delivery.envelope.from != *final_relay {
        return Err(SettlementError::WrongRelay);
    }
    if evidence.recipient_delivery.envelope.to != evidence.recipient.node_id {
        return Err(SettlementError::WrongRecipient);
    }
    let delivered_packet_hash = packet_bytes(&evidence.recipient_delivery.envelope.packet)
        .map(|bytes| blake3_hash(&bytes))
        .map_err(|_| SettlementError::PacketHashFailed)?;
    if delivered_packet_hash != evidence.bundle.packet_hash
        || evidence.recipient_delivery.envelope.packet_hash != evidence.bundle.packet_hash
    {
        return Err(SettlementError::DeliveryPacketMismatch);
    }
    let WorkPacket::NetworkMessage(message) = &evidence.recipient_delivery.envelope.packet else {
        return Err(SettlementError::DeliveryPacketMismatch);
    };
    if recipient_message_policy_allows(evidence.recipient_policy, message, evidence.now_unix_ms)
        .is_err()
    {
        return Err(SettlementError::MessagePolicyRejected);
    }
    if verify_channel_proof_for_ordered_with_policy(
        evidence.recipient_proof,
        evidence.recipient,
        *final_relay,
        evidence.recipient_delivery,
        evidence.admission.policy_hash,
    )
    .is_err()
    {
        return Err(SettlementError::InvalidRecipientProof);
    }
    let admission_hash = work_admission_hash(evidence.admission)?;
    let context = RelayTransitBundleContext {
        request_hash: evidence.admission.request_hash,
        admission_hash,
        controlling_admission_node_id: evidence.admission.admission_node.node_id,
        source_node_id: evidence.source_node_id,
        destination_node_id: evidence.destination_node_id,
        relay_path: &evidence.admission.assigned_relay_path,
        packet_hash: evidence.bundle.packet_hash,
        final_delivery_proof_hash: channel_proof_hash(evidence.recipient_proof),
        max_hops: evidence.max_hops,
    };
    let bundle_root = verify_relay_transit_bundle(evidence.bundle, &context)
        .map_err(|_| SettlementError::InvalidRelayTransitBundle)?;
    let hop = evidence
        .bundle
        .hops
        .iter()
        .find(|hop| hop.relay_node_id == receipt.worker.node_id && hop.sequence == receipt.sequence)
        .ok_or(SettlementError::RelayTransitHopMissing)?;
    if receipt.request_hash != evidence.bundle.request_hash
        || receipt.admission_hash != evidence.bundle.admission_hash
        || receipt.input_hash != hop.input_hash
        || receipt.output_hash != hop.transit_hash
        || receipt.units_used == 0
    {
        return Err(SettlementError::RelayTransitReceiptMismatch);
    }
    Ok(bundle_root)
}

pub fn verify_relay_transit_custody_evidence(
    evidence: &RelayTransitCustodyEvidence<'_>,
) -> Result<Hash, SettlementError> {
    verify_relay_transit_custody_requirement(&evidence.requirement)?;
    if evidence.custody_ack.packed_proof_root != evidence.requirement.expected_packed_proof_root
        || evidence.custody_ack.custody_kind != evidence.requirement.required_custody_kind
    {
        return Err(SettlementError::InvalidRelayProofCustodyAck);
    }
    verify_relay_proof_custody_ack_for_bundle(
        evidence.custody_ack,
        evidence.bundle,
        evidence.now_unix_ms,
    )
    .map_err(|_| SettlementError::InvalidRelayProofCustodyAck)
}

pub fn verify_relay_transit_custody_chain_evidence(
    evidence: &RelayTransitCustodyChainEvidence<'_>,
) -> Result<Hash, SettlementError> {
    verify_relay_transit_custody_requirement(&evidence.requirement)?;
    verify_relay_proof_custody_chain(
        evidence.bundle,
        evidence.custody_acks,
        evidence.requirement.expected_packed_proof_root,
        evidence.requirement.required_custody_kind,
        evidence.now_unix_ms,
    )
    .map_err(|_| SettlementError::InvalidRelayProofCustodyAck)
}

pub fn verify_relay_transit_custody_requirement(
    requirement: &RelayTransitCustodyRequirement,
) -> Result<(), SettlementError> {
    if requirement.expected_packed_proof_root == [0u8; 32] {
        return Err(SettlementError::InvalidRelayProofCustodyAck);
    }
    if !matches!(
        requirement.required_custody_kind,
        RELAY_PROOF_CUSTODY_KIND_PACKED
            | RELAY_PROOF_CUSTODY_KIND_NOTARIZED
            | RELAY_PROOF_CUSTODY_KIND_STORED
    ) {
        return Err(SettlementError::InvalidRelayProofCustodyAck);
    }
    Ok(())
}

pub fn relay_transit_custody_requirement_hash(
    requirement: &RelayTransitCustodyRequirement,
) -> Hash {
    HashBuilder::domain(RELAY_TRANSIT_CUSTODY_REQUIREMENT_DOMAIN)
        .hash(&requirement.expected_packed_proof_root)
        .u16(requirement.required_custody_kind)
        .finish()
}

pub fn settlement_result_hash(result: &SettlementResult) -> Hash {
    HashBuilder::domain(SETTLEMENT_RESULT_DOMAIN)
        .hash(&result.user)
        .hash(&result.worker)
        .u64(result.amount)
        .u64(result.user_balance_after)
        .u64(result.worker_balance_after)
        .u64(result.admission_spent_after)
        .hash(&result.receipt_hash)
        .finish()
}

pub fn relay_transit_custody_check_result_hash(result: &RelayTransitCustodyCheckResult) -> Hash {
    HashBuilder::domain(RELAY_TRANSIT_CUSTODY_CHECK_RESULT_DOMAIN)
        .hash(&result.custody_hash)
        .hash(&result.custody_requirement_hash)
        .finish()
}

pub fn relay_transit_custody_settlement_result_hash(
    result: &RelayTransitCustodySettlementResult,
) -> Hash {
    HashBuilder::domain(RELAY_TRANSIT_CUSTODY_SETTLEMENT_RESULT_DOMAIN)
        .hash(&settlement_result_hash(&result.settlement))
        .hash(&relay_transit_custody_check_result_hash(&result.custody))
        .finish()
}

pub fn work_admission_hash(admission: &WorkAdmission) -> Result<Hash, SettlementError> {
    packet_bytes(&WorkPacket::WorkAdmission(admission.clone()))
        .map(|bytes| blake3_hash(&bytes))
        .map_err(|_| SettlementError::PacketHashFailed)
}

pub fn work_receipt_hash(receipt: &WorkReceipt) -> Result<Hash, SettlementError> {
    packet_bytes(&WorkPacket::WorkReceipt(receipt.clone()))
        .map(|bytes| blake3_hash(&bytes))
        .map_err(|_| SettlementError::PacketHashFailed)
}

pub fn receipt_id_for_claim(
    request_hash: Hash,
    admission_hash: Hash,
    worker: NodeId,
    input_hash: Hash,
    output_hash: Hash,
    sequence: u64,
) -> Hash {
    HashBuilder::domain(RECEIPT_ID_DOMAIN)
        .hash(&request_hash)
        .hash(&admission_hash)
        .node_id(&worker)
        .hash(&input_hash)
        .hash(&output_hash)
        .u64(sequence)
        .finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::{CHANNEL_KIND_MEMORY, ChannelEndpoint, ChannelEnvelope};
    use crate::codec::{packet_bytes, wire_bytes, wire_from_bytes};
    use crate::delivery_proof::{channel_proof_for_allowed_ordered_message, channel_proof_hash};
    use crate::signing::{empty_signature, sign_work_admission, sign_work_receipt};
    use crate::transit_proof::{
        PacketTransitHashInput, RELAY_PROOF_CUSTODY_KIND_NOTARIZED,
        RELAY_PROOF_CUSTODY_KIND_PACKED, RELAY_PROOF_CUSTODY_KIND_STORED, RelayProofCustodyAck,
        RelayTransitBundle, finalize_relay_transit_bundle, relay_proof_custody_ack_hash,
        relay_transit_bundle_root, relay_transit_hop_evidence, relay_transit_route_commitment,
        sign_relay_proof_custody_ack, sign_relay_proof_custody_ack_for_bundle,
    };
    use crate::{
        SimNode, blake3_hash, open_recipient_message_policy, sign_network_message_payload,
        sign_recipient_message_policy,
    };
    use alloc::vec;
    use alloc::vec::Vec;
    use edgerun_crypto::Ed25519SigningKey;

    fn public_key_from_seed(seed: u8) -> PublicKey {
        let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
        let mut out = [0u8; 32];
        out.copy_from_slice(key.verifying_key().as_bytes());
        out
    }

    fn relay_admission(
        admission: &SimNode,
        user: PublicKey,
        request_hash: Hash,
        relay_path: Vec<NodeId>,
        budget: u64,
        policy_hash: Hash,
        route_commitment: Hash,
    ) -> WorkAdmission {
        sign_work_admission(
            &admission.key,
            WorkAdmission {
                abi_version: WORK_WIRE_ABI_VERSION,
                admission_id: blake3_hash(b"settlement-relay-transit-admission"),
                dao_id: admission.identity.public_key,
                user,
                admission_node: admission.identity.clone(),
                request_hash,
                assigned_route_commitment: route_commitment,
                assigned_channel: ChannelEndpoint::new(
                    blake3_hash(b"settlement-relay-transit-channel"),
                    CHANNEL_KIND_MEMORY,
                    Vec::new(),
                    "memory".into(),
                ),
                assigned_relay_path: relay_path,
                admitted_budget: budget,
                policy_hash,
                sequence: 1,
                valid_until_unix_ms: u64::MAX,
                signature: empty_signature(),
            },
        )
    }

    fn relay_receipt(
        relay: &SimNode,
        request_hash: Hash,
        admission_hash: Hash,
        input_hash: Hash,
        output_hash: Hash,
        sequence: u64,
        claim: u64,
    ) -> WorkReceipt {
        sign_work_receipt(
            &relay.key,
            WorkReceipt {
                abi_version: WORK_WIRE_ABI_VERSION,
                receipt_id: receipt_id_for_claim(
                    request_hash,
                    admission_hash,
                    relay.identity.node_id,
                    input_hash,
                    output_hash,
                    sequence,
                ),
                request_hash,
                admission_hash,
                worker: relay.identity.clone(),
                relay_node_id: relay.identity.node_id,
                input_hash,
                output_hash,
                units_used: 1,
                total_claim: claim,
                sequence,
                signature: empty_signature(),
            },
        )
    }

    fn relay_custody_packed_proof_root() -> Hash {
        blake3_hash(b"relay-custody-packed-proof-root")
    }

    fn relay_custody_requirement() -> RelayTransitCustodyRequirement {
        RelayTransitCustodyRequirement {
            expected_packed_proof_root: relay_custody_packed_proof_root(),
            required_custody_kind: RELAY_PROOF_CUSTODY_KIND_STORED,
        }
    }

    fn relay_custody_ack(
        bundle: &RelayTransitBundle,
        valid_until_unix_ms: u64,
    ) -> RelayProofCustodyAck {
        let custodian = SimNode::from_seed(178, NODE_ROLE_STORAGE);
        sign_relay_proof_custody_ack_for_bundle(
            &custodian.key,
            custodian.identity,
            bundle,
            relay_custody_packed_proof_root(),
            RELAY_PROOF_CUSTODY_KIND_STORED,
            valid_until_unix_ms,
        )
        .expect("relay custody ack")
    }

    fn relay_custody_ack_for_kind(
        bundle: &RelayTransitBundle,
        seed: u8,
        role: u16,
        custody_kind: u16,
    ) -> RelayProofCustodyAck {
        let custodian = SimNode::from_seed(seed, role);
        sign_relay_proof_custody_ack_for_bundle(
            &custodian.key,
            custodian.identity,
            bundle,
            relay_custody_packed_proof_root(),
            custody_kind,
            10,
        )
        .expect("relay custody ack for kind")
    }

    struct RelayBundleFixture {
        admission: WorkAdmission,
        user: PublicKey,
        relays: [SimNode; 3],
        recipient: SimNode,
        recipient_policy: RecipientMessagePolicy,
        recipient_delivery: OrderedChannelEnvelope,
        recipient_proof: ChannelProof,
        source: NodeId,
        destination: NodeId,
        bundle: RelayTransitBundle,
    }

    fn relay_bundle_fixture() -> RelayBundleFixture {
        let admission_node = SimNode::from_seed(171, NODE_ROLE_ADMISSION);
        let user = public_key_from_seed(172);
        let relays = [
            SimNode::from_seed(173, NODE_ROLE_RELAY),
            SimNode::from_seed(174, NODE_ROLE_RELAY),
            SimNode::from_seed(175, NODE_ROLE_RELAY),
        ];
        let relay_path = relays
            .iter()
            .map(|relay| relay.identity.node_id)
            .collect::<Vec<_>>();
        let request_hash = blake3_hash(b"settlement-relay-transit-request");
        let source_node = SimNode::from_seed(176, NODE_ROLE_MESSAGE);
        let recipient = SimNode::from_seed(177, NODE_ROLE_MESSAGE);
        let source = source_node.identity.node_id;
        let destination = recipient.identity.node_id;
        let packet = WorkPacket::NetworkMessage(sign_network_message_payload(
            &source_node.key,
            blake3_hash(b"settlement-relay-transit-message"),
            [0u8; 32],
            source,
            destination,
            relay_path[2],
            DEPARTMENT_MESSAGE,
            WORK_TYPE_MESSAGE_DELIVER,
            1,
            b"settlement multi-hop relay delivery".to_vec(),
        ));
        let packet_hash = packet_bytes(&packet)
            .map(|bytes| blake3_hash(&bytes))
            .expect("packet hash");
        let mut recipient_policy =
            open_recipient_message_policy(recipient.identity.clone(), 1, u64::MAX);
        recipient_policy.allowed_relays.push(relay_path[2]);
        recipient_policy = sign_recipient_message_policy(&recipient.key, recipient_policy);
        let recipient_delivery = OrderedChannelEnvelope {
            envelope: ChannelEnvelope::new(
                [33u8; 32],
                relay_path[2],
                destination,
                [43u8; 32],
                packet_hash,
                packet,
            ),
            sequence: 1,
            previous_message_hash: [0u8; 32],
        };
        let recipient_proof = channel_proof_for_allowed_ordered_message(
            &recipient.key,
            &recipient.identity,
            relay_path[2],
            &recipient_delivery,
            &recipient_policy,
            1,
        )
        .expect("recipient proof");
        let final_delivery_proof_hash = channel_proof_hash(&recipient_proof);
        let hops = vec![
            relay_transit_hop_evidence(
                PacketTransitHashInput {
                    node_id: relay_path[0],
                    from: source,
                    to: relay_path[1],
                    channel_id: [31u8; 32],
                    route_hash: [41u8; 32],
                    packet_hash,
                    sequence: 1,
                    previous_transit_hash: [0u8; 32],
                },
                blake3_hash(b"relay-hop-0-input"),
                0,
            ),
            relay_transit_hop_evidence(
                PacketTransitHashInput {
                    node_id: relay_path[1],
                    from: relay_path[0],
                    to: relay_path[2],
                    channel_id: [32u8; 32],
                    route_hash: [42u8; 32],
                    packet_hash,
                    sequence: 1,
                    previous_transit_hash: [5u8; 32],
                },
                blake3_hash(b"relay-hop-1-input"),
                1,
            ),
            relay_transit_hop_evidence(
                PacketTransitHashInput {
                    node_id: relay_path[2],
                    from: relay_path[1],
                    to: destination,
                    channel_id: [33u8; 32],
                    route_hash: [43u8; 32],
                    packet_hash,
                    sequence: 1,
                    previous_transit_hash: [6u8; 32],
                },
                blake3_hash(b"relay-hop-2-input"),
                2,
            ),
        ];
        let route_commitment =
            relay_transit_route_commitment(source, destination, &relay_path, &hops);
        let admission = relay_admission(
            &admission_node,
            user,
            request_hash,
            relay_path.clone(),
            30,
            recipient_message_policy_hash(&recipient_policy),
            route_commitment,
        );
        let admission_hash = work_admission_hash(&admission).expect("admission hash");
        let bundle = finalize_relay_transit_bundle(RelayTransitBundle {
            abi_version: WORK_WIRE_ABI_VERSION,
            request_hash,
            admission_hash,
            controlling_admission_node_id: admission.admission_node.node_id,
            source_node_id: source,
            destination_node_id: destination,
            relay_path,
            packet_hash,
            final_delivery_proof_hash,
            hops,
            bundle_root: [0u8; 32],
        });
        RelayBundleFixture {
            admission,
            user,
            relays,
            recipient,
            recipient_policy,
            recipient_delivery,
            recipient_proof,
            source,
            destination,
            bundle,
        }
    }

    fn funded_relay_ledger(fixture: &RelayBundleFixture, balance: u64) -> SettlementLedger {
        let mut ledger = SettlementLedger::new();
        ledger.deposit_user_credit(fixture.user, balance);
        ledger
            .reserve_admission_budget(&fixture.admission)
            .expect("reserve relay admission budget");
        ledger
    }

    #[test]
    fn settlement_pays_relay_hop_only_with_verified_multi_relay_bundle() {
        let fixture = relay_bundle_fixture();
        let hop = &fixture.bundle.hops[1];
        let receipt = relay_receipt(
            &fixture.relays[1],
            fixture.bundle.request_hash,
            fixture.bundle.admission_hash,
            hop.input_hash,
            hop.transit_hash,
            hop.sequence,
            7,
        );
        let evidence = RelayTransitSettlementEvidence {
            admission: &fixture.admission,
            receipt: &receipt,
            bundle: &fixture.bundle,
            recipient_delivery: &fixture.recipient_delivery,
            recipient: &fixture.recipient.identity,
            recipient_policy: &fixture.recipient_policy,
            recipient_proof: &fixture.recipient_proof,
            source_node_id: fixture.source,
            destination_node_id: fixture.destination,
            now_unix_ms: 1,
            max_hops: 8,
        };

        let mut ledger = funded_relay_ledger(&fixture, 30);
        let result = ledger
            .settle_relay_transit_hop(&evidence)
            .expect("relay hop settles");
        assert_eq!(result.amount, 7);
        assert_eq!(
            ledger.worker_balance(&fixture.relays[1].identity.node_id),
            7
        );
    }

    #[test]
    fn settlement_verifies_relay_bundle_custody_ack_without_settling_payment() {
        let fixture = relay_bundle_fixture();
        let ack = relay_custody_ack(&fixture.bundle, 10);
        let evidence = RelayTransitCustodyEvidence {
            bundle: &fixture.bundle,
            custody_ack: &ack,
            requirement: relay_custody_requirement(),
            now_unix_ms: 9,
        };

        let ack_hash =
            verify_relay_transit_custody_evidence(&evidence).expect("custody evidence verifies");
        assert_eq!(ack_hash, relay_proof_custody_ack_hash(&ack));
    }

    #[test]
    fn settlement_rejects_invalid_relay_custody_requirement() {
        let fixture = relay_bundle_fixture();
        let ack = relay_custody_ack(&fixture.bundle, 10);

        assert_eq!(
            verify_relay_transit_custody_requirement(&RelayTransitCustodyRequirement {
                expected_packed_proof_root: [0u8; 32],
                required_custody_kind: RELAY_PROOF_CUSTODY_KIND_STORED,
            }),
            Err(SettlementError::InvalidRelayProofCustodyAck)
        );
        assert_eq!(
            verify_relay_transit_custody_requirement(&RelayTransitCustodyRequirement {
                expected_packed_proof_root: relay_custody_packed_proof_root(),
                required_custody_kind: 99,
            }),
            Err(SettlementError::InvalidRelayProofCustodyAck)
        );
        assert_eq!(
            verify_relay_transit_custody_evidence(&RelayTransitCustodyEvidence {
                bundle: &fixture.bundle,
                custody_ack: &ack,
                requirement: RelayTransitCustodyRequirement {
                    expected_packed_proof_root: [0u8; 32],
                    required_custody_kind: RELAY_PROOF_CUSTODY_KIND_STORED,
                },
                now_unix_ms: 9,
            }),
            Err(SettlementError::InvalidRelayProofCustodyAck)
        );
    }

    #[test]
    fn settlement_hashes_relay_custody_requirement_for_audit_refs() {
        let requirement = relay_custody_requirement();
        let same = relay_custody_requirement();
        let different_root = RelayTransitCustodyRequirement {
            expected_packed_proof_root: blake3_hash(b"different-custody-root"),
            required_custody_kind: RELAY_PROOF_CUSTODY_KIND_STORED,
        };
        let different_kind = RelayTransitCustodyRequirement {
            expected_packed_proof_root: relay_custody_packed_proof_root(),
            required_custody_kind: RELAY_PROOF_CUSTODY_KIND_NOTARIZED,
        };

        assert_eq!(
            relay_transit_custody_requirement_hash(&requirement),
            relay_transit_custody_requirement_hash(&same)
        );
        assert_ne!(
            relay_transit_custody_requirement_hash(&requirement),
            relay_transit_custody_requirement_hash(&different_root)
        );
        assert_ne!(
            relay_transit_custody_requirement_hash(&requirement),
            relay_transit_custody_requirement_hash(&different_kind)
        );

        let bytes = wire_bytes(&requirement).expect("custody requirement bytes");
        let decoded = wire_from_bytes::<
            RelayTransitCustodyRequirement,
            RelayTransitCustodyRequirement,
        >(&bytes)
        .expect("custody requirement roundtrip");
        assert_eq!(decoded, requirement);
        assert_eq!(
            relay_transit_custody_requirement_hash(&decoded),
            relay_transit_custody_requirement_hash(&requirement)
        );
    }

    #[test]
    fn settlement_rejects_relay_custody_ack_wrong_root_kind_replay_window_and_expiry() {
        let fixture = relay_bundle_fixture();
        let ack = relay_custody_ack(&fixture.bundle, 10);

        assert_eq!(
            verify_relay_transit_custody_evidence(&RelayTransitCustodyEvidence {
                bundle: &fixture.bundle,
                custody_ack: &ack,
                requirement: RelayTransitCustodyRequirement {
                    expected_packed_proof_root: blake3_hash(b"wrong-packed-proof-root"),
                    required_custody_kind: RELAY_PROOF_CUSTODY_KIND_STORED,
                },
                now_unix_ms: 9,
            }),
            Err(SettlementError::InvalidRelayProofCustodyAck)
        );

        assert_eq!(
            verify_relay_transit_custody_evidence(&RelayTransitCustodyEvidence {
                bundle: &fixture.bundle,
                custody_ack: &ack,
                requirement: RelayTransitCustodyRequirement {
                    expected_packed_proof_root: relay_custody_packed_proof_root(),
                    required_custody_kind: crate::transit_proof::RELAY_PROOF_CUSTODY_KIND_NOTARIZED,
                },
                now_unix_ms: 9,
            }),
            Err(SettlementError::InvalidRelayProofCustodyAck)
        );

        let mut replayed_bundle = fixture.bundle.clone();
        replayed_bundle.packet_hash = blake3_hash(b"replayed-relay-bundle-packet");
        replayed_bundle.bundle_root = relay_transit_bundle_root(&replayed_bundle);
        assert_eq!(
            verify_relay_transit_custody_evidence(&RelayTransitCustodyEvidence {
                bundle: &replayed_bundle,
                custody_ack: &ack,
                requirement: relay_custody_requirement(),
                now_unix_ms: 9,
            }),
            Err(SettlementError::InvalidRelayProofCustodyAck)
        );

        let mut wrong_window = relay_custody_ack(&fixture.bundle, 10);
        wrong_window.sequence_end = wrong_window.sequence_end + 1;
        wrong_window = sign_relay_proof_custody_ack(
            &SimNode::from_seed(178, NODE_ROLE_STORAGE).key,
            wrong_window,
        );
        assert_eq!(
            verify_relay_transit_custody_evidence(&RelayTransitCustodyEvidence {
                bundle: &fixture.bundle,
                custody_ack: &wrong_window,
                requirement: relay_custody_requirement(),
                now_unix_ms: 9,
            }),
            Err(SettlementError::InvalidRelayProofCustodyAck)
        );

        assert_eq!(
            verify_relay_transit_custody_evidence(&RelayTransitCustodyEvidence {
                bundle: &fixture.bundle,
                custody_ack: &ack,
                requirement: relay_custody_requirement(),
                now_unix_ms: 11,
            }),
            Err(SettlementError::InvalidRelayProofCustodyAck)
        );
    }

    #[test]
    fn settlement_verifies_relay_custody_chain_without_settling_payment() {
        let fixture = relay_bundle_fixture();
        let packed = relay_custody_ack_for_kind(
            &fixture.bundle,
            179,
            NODE_ROLE_VERIFIER,
            RELAY_PROOF_CUSTODY_KIND_PACKED,
        );
        let notarized = relay_custody_ack_for_kind(
            &fixture.bundle,
            180,
            NODE_ROLE_NOTARY,
            RELAY_PROOF_CUSTODY_KIND_NOTARIZED,
        );
        let stored = relay_custody_ack_for_kind(
            &fixture.bundle,
            181,
            NODE_ROLE_STORAGE,
            RELAY_PROOF_CUSTODY_KIND_STORED,
        );
        let chain = vec![packed, notarized, stored.clone()];

        let chain_hash =
            verify_relay_transit_custody_chain_evidence(&RelayTransitCustodyChainEvidence {
                bundle: &fixture.bundle,
                custody_acks: &chain,
                requirement: relay_custody_requirement(),
                now_unix_ms: 9,
            })
            .expect("custody chain verifies");
        assert_eq!(chain_hash, relay_proof_custody_ack_hash(&stored));
    }

    #[test]
    fn settlement_can_require_relay_custody_chain_before_payment() {
        let fixture = relay_bundle_fixture();
        let hop = &fixture.bundle.hops[1];
        let receipt = relay_receipt(
            &fixture.relays[1],
            fixture.bundle.request_hash,
            fixture.bundle.admission_hash,
            hop.input_hash,
            hop.transit_hash,
            hop.sequence,
            7,
        );
        let transit = RelayTransitSettlementEvidence {
            admission: &fixture.admission,
            receipt: &receipt,
            bundle: &fixture.bundle,
            recipient_delivery: &fixture.recipient_delivery,
            recipient: &fixture.recipient.identity,
            recipient_policy: &fixture.recipient_policy,
            recipient_proof: &fixture.recipient_proof,
            source_node_id: fixture.source,
            destination_node_id: fixture.destination,
            now_unix_ms: 1,
            max_hops: 8,
        };
        let packed = relay_custody_ack_for_kind(
            &fixture.bundle,
            179,
            NODE_ROLE_VERIFIER,
            RELAY_PROOF_CUSTODY_KIND_PACKED,
        );
        let notarized = relay_custody_ack_for_kind(
            &fixture.bundle,
            180,
            NODE_ROLE_NOTARY,
            RELAY_PROOF_CUSTODY_KIND_NOTARIZED,
        );
        let stored = relay_custody_ack_for_kind(
            &fixture.bundle,
            181,
            NODE_ROLE_STORAGE,
            RELAY_PROOF_CUSTODY_KIND_STORED,
        );
        let custody = RelayTransitCustodyChainEvidence {
            bundle: &fixture.bundle,
            custody_acks: &[packed, notarized, stored.clone()],
            requirement: relay_custody_requirement(),
            now_unix_ms: 9,
        };

        let mut ledger = funded_relay_ledger(&fixture, 30);
        let check = ledger
            .can_settle_relay_transit_hop_with_custody(&transit, &custody)
            .expect("relay hop preflight accepts custody chain");
        assert_eq!(check.custody_hash, relay_proof_custody_ack_hash(&stored));
        assert_eq!(
            check.custody_requirement_hash,
            relay_transit_custody_requirement_hash(&custody.requirement)
        );
        let result = ledger
            .settle_relay_transit_hop_with_custody(&transit, &custody)
            .expect("relay hop settles only with custody chain");
        assert_eq!(result.settlement.amount, 7);
        assert_eq!(result.custody, check);
        assert_eq!(
            result.custody.custody_hash,
            relay_proof_custody_ack_hash(&stored)
        );
        assert_eq!(
            result.custody.custody_requirement_hash,
            relay_transit_custody_requirement_hash(&custody.requirement)
        );
        let bytes = wire_bytes(&result).expect("custody settlement result bytes");
        let decoded = wire_from_bytes::<
            RelayTransitCustodySettlementResult,
            RelayTransitCustodySettlementResult,
        >(&bytes)
        .expect("custody settlement result roundtrip");
        assert_eq!(decoded, result);
        assert_eq!(decoded.custody, check);
        assert_eq!(
            relay_transit_custody_settlement_result_hash(&decoded),
            relay_transit_custody_settlement_result_hash(&result)
        );
        let mut different_settlement = result.clone();
        different_settlement.settlement.amount =
            different_settlement.settlement.amount.saturating_add(1);
        let mut different_custody = result.clone();
        different_custody.custody.custody_hash = blake3_hash(b"different-custody-hash");
        assert_ne!(
            settlement_result_hash(&different_settlement.settlement),
            settlement_result_hash(&result.settlement)
        );
        assert_ne!(
            relay_transit_custody_check_result_hash(&different_custody.custody),
            relay_transit_custody_check_result_hash(&result.custody)
        );
        assert_ne!(
            relay_transit_custody_settlement_result_hash(&different_settlement),
            relay_transit_custody_settlement_result_hash(&result)
        );
        assert_ne!(
            relay_transit_custody_settlement_result_hash(&different_custody),
            relay_transit_custody_settlement_result_hash(&result)
        );
        assert_eq!(
            ledger.worker_balance(&fixture.relays[1].identity.node_id),
            7
        );
    }

    #[test]
    fn settlement_rejects_relay_payment_with_unrelated_custody_chain() {
        let fixture = relay_bundle_fixture();
        let hop = &fixture.bundle.hops[1];
        let receipt = relay_receipt(
            &fixture.relays[1],
            fixture.bundle.request_hash,
            fixture.bundle.admission_hash,
            hop.input_hash,
            hop.transit_hash,
            hop.sequence,
            7,
        );
        let transit = RelayTransitSettlementEvidence {
            admission: &fixture.admission,
            receipt: &receipt,
            bundle: &fixture.bundle,
            recipient_delivery: &fixture.recipient_delivery,
            recipient: &fixture.recipient.identity,
            recipient_policy: &fixture.recipient_policy,
            recipient_proof: &fixture.recipient_proof,
            source_node_id: fixture.source,
            destination_node_id: fixture.destination,
            now_unix_ms: 1,
            max_hops: 8,
        };
        let mut unrelated_bundle = fixture.bundle.clone();
        unrelated_bundle.request_hash = blake3_hash(b"unrelated-custody-request");
        unrelated_bundle.bundle_root = relay_transit_bundle_root(&unrelated_bundle);
        let packed = relay_custody_ack_for_kind(
            &unrelated_bundle,
            179,
            NODE_ROLE_VERIFIER,
            RELAY_PROOF_CUSTODY_KIND_PACKED,
        );
        let notarized = relay_custody_ack_for_kind(
            &unrelated_bundle,
            180,
            NODE_ROLE_NOTARY,
            RELAY_PROOF_CUSTODY_KIND_NOTARIZED,
        );
        let stored = relay_custody_ack_for_kind(
            &unrelated_bundle,
            181,
            NODE_ROLE_STORAGE,
            RELAY_PROOF_CUSTODY_KIND_STORED,
        );
        let custody = RelayTransitCustodyChainEvidence {
            bundle: &unrelated_bundle,
            custody_acks: &[packed, notarized, stored],
            requirement: relay_custody_requirement(),
            now_unix_ms: 9,
        };

        let mut ledger = funded_relay_ledger(&fixture, 30);
        assert_eq!(
            ledger.settle_relay_transit_hop_with_custody(&transit, &custody),
            Err(SettlementError::InvalidRelayProofCustodyAck)
        );
        assert_eq!(
            ledger.worker_balance(&fixture.relays[1].identity.node_id),
            0
        );
    }

    #[test]
    fn settlement_rejects_relay_custody_chain_wrong_root_order_and_final_kind() {
        let fixture = relay_bundle_fixture();
        let packed = relay_custody_ack_for_kind(
            &fixture.bundle,
            182,
            NODE_ROLE_VERIFIER,
            RELAY_PROOF_CUSTODY_KIND_PACKED,
        );
        let notarized = relay_custody_ack_for_kind(
            &fixture.bundle,
            183,
            NODE_ROLE_NOTARY,
            RELAY_PROOF_CUSTODY_KIND_NOTARIZED,
        );
        let stored = relay_custody_ack_for_kind(
            &fixture.bundle,
            184,
            NODE_ROLE_STORAGE,
            RELAY_PROOF_CUSTODY_KIND_STORED,
        );

        assert_eq!(
            verify_relay_transit_custody_chain_evidence(&RelayTransitCustodyChainEvidence {
                bundle: &fixture.bundle,
                custody_acks: &[packed.clone(), notarized.clone(), stored.clone()],
                requirement: RelayTransitCustodyRequirement {
                    expected_packed_proof_root: blake3_hash(b"wrong-chain-packed-root"),
                    required_custody_kind: RELAY_PROOF_CUSTODY_KIND_STORED,
                },
                now_unix_ms: 9,
            }),
            Err(SettlementError::InvalidRelayProofCustodyAck)
        );
        assert_eq!(
            verify_relay_transit_custody_chain_evidence(&RelayTransitCustodyChainEvidence {
                bundle: &fixture.bundle,
                custody_acks: &[notarized.clone(), packed.clone(), stored.clone()],
                requirement: relay_custody_requirement(),
                now_unix_ms: 9,
            }),
            Err(SettlementError::InvalidRelayProofCustodyAck)
        );
        assert_eq!(
            verify_relay_transit_custody_chain_evidence(&RelayTransitCustodyChainEvidence {
                bundle: &fixture.bundle,
                custody_acks: &[packed.clone(), stored.clone()],
                requirement: relay_custody_requirement(),
                now_unix_ms: 9,
            }),
            Err(SettlementError::InvalidRelayProofCustodyAck)
        );
        assert_eq!(
            verify_relay_transit_custody_chain_evidence(&RelayTransitCustodyChainEvidence {
                bundle: &fixture.bundle,
                custody_acks: &[packed, notarized],
                requirement: relay_custody_requirement(),
                now_unix_ms: 9,
            }),
            Err(SettlementError::InvalidRelayProofCustodyAck)
        );
    }

    #[test]
    fn settlement_rejects_relay_bundle_not_admitted_by_same_path() {
        let fixture = relay_bundle_fixture();
        let admission_node = SimNode::from_seed(171, NODE_ROLE_ADMISSION);
        let mut wrong_path = fixture.bundle.relay_path.clone();
        wrong_path.swap(0, 1);
        let wrong_admission = relay_admission(
            &admission_node,
            fixture.user,
            fixture.bundle.request_hash,
            wrong_path,
            30,
            recipient_message_policy_hash(&fixture.recipient_policy),
            relay_transit_bundle_route_commitment(&fixture.bundle),
        );
        let wrong_admission_hash = work_admission_hash(&wrong_admission).expect("admission hash");
        let hop = &fixture.bundle.hops[0];
        let receipt = relay_receipt(
            &fixture.relays[0],
            fixture.bundle.request_hash,
            wrong_admission_hash,
            hop.input_hash,
            hop.transit_hash,
            hop.sequence,
            5,
        );
        let evidence = RelayTransitSettlementEvidence {
            admission: &wrong_admission,
            receipt: &receipt,
            bundle: &fixture.bundle,
            recipient_delivery: &fixture.recipient_delivery,
            recipient: &fixture.recipient.identity,
            recipient_policy: &fixture.recipient_policy,
            recipient_proof: &fixture.recipient_proof,
            source_node_id: fixture.source,
            destination_node_id: fixture.destination,
            now_unix_ms: 1,
            max_hops: 8,
        };
        let mut ledger = SettlementLedger::new();
        ledger.deposit_user_credit(fixture.user, 30);
        ledger
            .reserve_admission_budget(&wrong_admission)
            .expect("reserve wrong admission budget");
        assert_eq!(
            ledger.settle_relay_transit_hop(&evidence),
            Err(SettlementError::AdmissionRouteMismatch)
        );
    }

    #[test]
    fn settlement_rejects_relay_receipt_that_does_not_match_bundle_hop() {
        let fixture = relay_bundle_fixture();
        let hop = &fixture.bundle.hops[0];
        let receipt = relay_receipt(
            &fixture.relays[0],
            fixture.bundle.request_hash,
            fixture.bundle.admission_hash,
            blake3_hash(b"wrong-input"),
            hop.transit_hash,
            hop.sequence,
            5,
        );
        let evidence = RelayTransitSettlementEvidence {
            admission: &fixture.admission,
            receipt: &receipt,
            bundle: &fixture.bundle,
            recipient_delivery: &fixture.recipient_delivery,
            recipient: &fixture.recipient.identity,
            recipient_policy: &fixture.recipient_policy,
            recipient_proof: &fixture.recipient_proof,
            source_node_id: fixture.source,
            destination_node_id: fixture.destination,
            now_unix_ms: 1,
            max_hops: 8,
        };
        let mut ledger = funded_relay_ledger(&fixture, 30);
        assert_eq!(
            ledger.settle_relay_transit_hop(&evidence),
            Err(SettlementError::RelayTransitReceiptMismatch)
        );
    }

    #[test]
    fn settlement_rejects_forged_relay_bundle_summary_root() {
        let mut fixture = relay_bundle_fixture();
        fixture.bundle.hops[2].transit_hash = [88u8; 32];
        fixture.bundle.bundle_root = relay_transit_bundle_root(&fixture.bundle);
        let hop = &fixture.bundle.hops[2];
        let receipt = relay_receipt(
            &fixture.relays[2],
            fixture.bundle.request_hash,
            fixture.bundle.admission_hash,
            hop.input_hash,
            hop.transit_hash,
            hop.sequence,
            5,
        );
        let evidence = RelayTransitSettlementEvidence {
            admission: &fixture.admission,
            receipt: &receipt,
            bundle: &fixture.bundle,
            recipient_delivery: &fixture.recipient_delivery,
            recipient: &fixture.recipient.identity,
            recipient_policy: &fixture.recipient_policy,
            recipient_proof: &fixture.recipient_proof,
            source_node_id: fixture.source,
            destination_node_id: fixture.destination,
            now_unix_ms: 1,
            max_hops: 8,
        };
        let mut ledger = funded_relay_ledger(&fixture, 30);
        assert_eq!(
            ledger.settle_relay_transit_hop(&evidence),
            Err(SettlementError::InvalidRelayTransitBundle)
        );
    }

    #[test]
    fn settlement_rejects_multi_relay_bundle_without_valid_recipient_proof() {
        let mut fixture = relay_bundle_fixture();
        fixture.recipient_proof.relay_node_id = fixture.relays[0].identity.node_id;
        let hop = &fixture.bundle.hops[2];
        let receipt = relay_receipt(
            &fixture.relays[2],
            fixture.bundle.request_hash,
            fixture.bundle.admission_hash,
            hop.input_hash,
            hop.transit_hash,
            hop.sequence,
            5,
        );
        let evidence = RelayTransitSettlementEvidence {
            admission: &fixture.admission,
            receipt: &receipt,
            bundle: &fixture.bundle,
            recipient_delivery: &fixture.recipient_delivery,
            recipient: &fixture.recipient.identity,
            recipient_policy: &fixture.recipient_policy,
            recipient_proof: &fixture.recipient_proof,
            source_node_id: fixture.source,
            destination_node_id: fixture.destination,
            now_unix_ms: 1,
            max_hops: 8,
        };
        let mut ledger = funded_relay_ledger(&fixture, 30);
        assert_eq!(
            ledger.settle_relay_transit_hop(&evidence),
            Err(SettlementError::InvalidRecipientProof)
        );
    }

    #[test]
    fn settlement_rejects_multi_relay_bundle_route_not_committed_by_admission() {
        let mut fixture = relay_bundle_fixture();
        fixture.bundle.hops[1].route_hash = [91u8; 32];
        fixture.bundle.hops[1].transit_hash = packet_transit_hash(&PacketTransitHashInput {
            node_id: fixture.bundle.hops[1].relay_node_id,
            from: fixture.bundle.hops[1].from,
            to: fixture.bundle.hops[1].to,
            channel_id: fixture.bundle.hops[1].channel_id,
            route_hash: fixture.bundle.hops[1].route_hash,
            packet_hash: fixture.bundle.hops[1].packet_hash,
            sequence: fixture.bundle.hops[1].sequence,
            previous_transit_hash: fixture.bundle.hops[1].previous_transit_hash,
        });
        fixture.bundle.bundle_root = relay_transit_bundle_root(&fixture.bundle);
        let hop = &fixture.bundle.hops[1];
        let receipt = relay_receipt(
            &fixture.relays[1],
            fixture.bundle.request_hash,
            fixture.bundle.admission_hash,
            hop.input_hash,
            hop.transit_hash,
            hop.sequence,
            5,
        );
        let evidence = RelayTransitSettlementEvidence {
            admission: &fixture.admission,
            receipt: &receipt,
            bundle: &fixture.bundle,
            recipient_delivery: &fixture.recipient_delivery,
            recipient: &fixture.recipient.identity,
            recipient_policy: &fixture.recipient_policy,
            recipient_proof: &fixture.recipient_proof,
            source_node_id: fixture.source,
            destination_node_id: fixture.destination,
            now_unix_ms: 1,
            max_hops: 8,
        };
        let mut ledger = funded_relay_ledger(&fixture, 30);
        assert_eq!(
            ledger.settle_relay_transit_hop(&evidence),
            Err(SettlementError::AdmissionRouteMismatch)
        );
    }
}
