use alloc::collections::{BTreeMap, BTreeSet};

use crate::batch_settlement::{BatchSettlementError, BatchSettlementResult};
use crate::channel::ChannelProof;
use crate::channel_order::OrderedChannelEnvelope;
use crate::codec::{blake3_hash, packet_bytes};
use crate::delivery_proof::{channel_proof_hash, verify_channel_proof_for_ordered_with_policy};
use crate::preimage::HashBuilder;
use crate::protocol::*;
use crate::recipient_policy::{
    RecipientMessagePolicy, recipient_message_policy_allows, recipient_message_policy_hash,
};
use crate::relay_role::ordered_message_input_hash;
use crate::signing::{verify_work_admission, verify_work_receipt};
use crate::transit_proof::{
    PacketTransitHashInput, packet_transit_hash, relay_delivery_output_hash,
};

const RECEIPT_ID_DOMAIN: &[u8] = b"edgerun:v1:work:receipt-id";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettlementError {
    InvalidAdmission,
    InvalidReceipt,
    UnknownUser,
    InsufficientBalance,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettlementPruneResult {
    pub removed_admission: bool,
    pub removed_receipts: u64,
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

#[derive(Clone, Debug, Default)]
pub struct SettlementLedger {
    user_balances: BTreeMap<PublicKey, u64>,
    worker_balances: BTreeMap<NodeId, u64>,
    admission_spend: BTreeMap<Hash, u64>,
    admission_receipts: BTreeMap<Hash, BTreeSet<Hash>>,
    paid_receipts: BTreeSet<Hash>,
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

    pub fn is_receipt_paid(&self, receipt_hash: &Hash) -> bool {
        self.paid_receipts.contains(receipt_hash)
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
        if next_spend > admission.admitted_budget {
            return Err(SettlementError::ClaimExceedsAdmissionBudget);
        }
        let user_balance = self
            .user_balances
            .get(&admission.user)
            .ok_or(SettlementError::UnknownUser)?;
        if *user_balance < receipt.total_claim {
            return Err(SettlementError::InsufficientBalance);
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
        let user_balance = self
            .user_balances
            .get_mut(&admission.user)
            .ok_or(SettlementError::UnknownUser)?;

        self.paid_receipts.insert(receipt_hash);
        self.admission_receipts
            .entry(admission_hash)
            .or_default()
            .insert(receipt_hash);
        *user_balance -= receipt.total_claim;
        self.admission_spend.insert(admission_hash, next_spend);
        let worker_balance = self
            .worker_balances
            .entry(receipt.worker.node_id)
            .or_default();
        *worker_balance = worker_balance.saturating_add(receipt.total_claim);
        Ok(SettlementResult {
            user: admission.user,
            worker: receipt.worker.node_id,
            amount: receipt.total_claim,
            user_balance_after: *user_balance,
            worker_balance_after: *worker_balance,
            admission_spent_after: next_spend,
            receipt_hash,
        })
    }

    pub fn prune_finalized_admission(&mut self, admission_hash: &Hash) -> SettlementPruneResult {
        let removed_admission = self.admission_spend.remove(admission_hash).is_some();
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
        }
    }
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
    if evidence.admission.assigned_route_hash != evidence.relay_input.envelope.route_hash
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
