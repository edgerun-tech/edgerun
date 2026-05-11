use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use crate::codec::{blake3_hash, packet_bytes, verify_work_admission, verify_work_receipt};
use crate::protocol::*;

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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettlementResult {
    pub user: PublicKey,
    pub worker: NodeId,
    pub amount: u64,
    pub user_balance_after: u64,
    pub worker_balance_after: u64,
    pub receipt_hash: Hash,
}

#[derive(Clone, Debug, Default)]
pub struct SettlementLedger {
    user_balances: BTreeMap<PublicKey, u64>,
    worker_balances: BTreeMap<NodeId, u64>,
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

    pub fn settle_receipt(
        &mut self,
        user: PublicKey,
        admission: &WorkAdmission,
        receipt: &WorkReceipt,
    ) -> Result<SettlementResult, SettlementError> {
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
        if receipt.total_claim > admission.admitted_budget {
            return Err(SettlementError::ClaimExceedsAdmissionBudget);
        }
        let receipt_hash = work_receipt_hash(receipt)?;
        if !self.paid_receipts.insert(receipt_hash) {
            return Err(SettlementError::DuplicateReceipt);
        }
        let user_balance = self.user_balances.get_mut(&user).ok_or(SettlementError::UnknownUser)?;
        if *user_balance < receipt.total_claim {
            self.paid_receipts.remove(&receipt_hash);
            return Err(SettlementError::InsufficientBalance);
        }
        *user_balance -= receipt.total_claim;
        let worker_balance = self.worker_balances.entry(receipt.worker.node_id).or_default();
        *worker_balance = worker_balance.saturating_add(receipt.total_claim);
        Ok(SettlementResult {
            user,
            worker: receipt.worker.node_id,
            amount: receipt.total_claim,
            user_balance_after: *user_balance,
            worker_balance_after: *worker_balance,
            receipt_hash,
        })
    }
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
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&request_hash);
    bytes.extend_from_slice(&admission_hash);
    bytes.extend_from_slice(&worker);
    bytes.extend_from_slice(&input_hash);
    bytes.extend_from_slice(&output_hash);
    bytes.extend_from_slice(&sequence.to_be_bytes());
    blake3_hash(&bytes)
}
