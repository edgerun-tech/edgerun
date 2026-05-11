use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use crate::codec::{blake3_hash, verify_work_admission, verify_work_receipt};
use crate::protocol::*;
use crate::settlement::{work_admission_hash, work_receipt_hash, SettlementError, SettlementLedger};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchSettlementError {
    EmptyBatch,
    Settlement(SettlementError),
    DuplicateInBatch,
    DuplicateAlreadyPaid,
    ReceiptHashFailed,
    InvalidBatchRoot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiptBatch {
    pub admission_hash: Hash,
    pub receipt_hashes: Vec<Hash>,
    pub total_claim: u64,
    pub batch_root: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchSettlementResult {
    pub batch_root: Hash,
    pub receipt_count: u64,
    pub total_claim: u64,
    pub user_balance_after: u64,
    pub worker_count: u64,
}

impl SettlementLedger {
    pub fn settle_receipt_batch_unchecked_evidence(
        &mut self,
        admission: &WorkAdmission,
        receipts: &[WorkReceipt],
    ) -> Result<BatchSettlementResult, BatchSettlementError> {
        let batch = build_receipt_batch(admission, receipts)?;
        let mut committed = self.clone();
        for receipt in receipts {
            committed
                .settle_receipt_unchecked_evidence(admission, receipt)
                .map_err(|error| match error {
                    SettlementError::DuplicateReceipt => BatchSettlementError::DuplicateAlreadyPaid,
                    other => BatchSettlementError::Settlement(other),
                })?;
        }
        let mut workers = BTreeSet::new();
        for receipt in receipts {
            workers.insert(receipt.worker.node_id);
        }
        let user_balance_after = committed.user_balance(&admission.user);
        *self = committed;
        Ok(BatchSettlementResult {
            batch_root: batch.batch_root,
            receipt_count: receipts.len() as u64,
            total_claim: batch.total_claim,
            user_balance_after,
            worker_count: workers.len() as u64,
        })
    }
}

pub fn build_receipt_batch(
    admission: &WorkAdmission,
    receipts: &[WorkReceipt],
) -> Result<ReceiptBatch, BatchSettlementError> {
    if receipts.is_empty() {
        return Err(BatchSettlementError::EmptyBatch);
    }
    if !verify_work_admission(admission) {
        return Err(BatchSettlementError::Settlement(SettlementError::InvalidAdmission));
    }
    let admission_hash = work_admission_hash(admission).map_err(BatchSettlementError::Settlement)?;
    let mut receipt_hashes = Vec::with_capacity(receipts.len());
    let mut seen = BTreeSet::new();
    let mut total_claim = 0u64;
    for receipt in receipts {
        if !verify_work_receipt(receipt) {
            return Err(BatchSettlementError::Settlement(SettlementError::InvalidReceipt));
        }
        if receipt.admission_hash != admission_hash || receipt.request_hash != admission.request_hash {
            return Err(BatchSettlementError::Settlement(SettlementError::ReceiptAdmissionMismatch));
        }
        let hash = work_receipt_hash(receipt).map_err(BatchSettlementError::Settlement)?;
        if !seen.insert(hash) {
            return Err(BatchSettlementError::DuplicateInBatch);
        }
        receipt_hashes.push(hash);
        total_claim = total_claim.saturating_add(receipt.total_claim);
    }
    if total_claim > admission.admitted_budget {
        return Err(BatchSettlementError::Settlement(SettlementError::ClaimExceedsAdmissionBudget));
    }
    let batch_root = receipt_batch_root(admission_hash, &receipt_hashes, total_claim);
    Ok(ReceiptBatch {
        admission_hash,
        receipt_hashes,
        total_claim,
        batch_root,
    })
}

pub fn receipt_batch_root(admission_hash: Hash, receipt_hashes: &[Hash], total_claim: u64) -> Hash {
    let mut leaves = Vec::with_capacity(receipt_hashes.len());
    for hash in receipt_hashes {
        let mut leaf = Vec::new();
        leaf.extend_from_slice(&admission_hash);
        leaf.extend_from_slice(hash);
        leaves.push(blake3_hash(&leaf));
    }
    let merkle = merkle_root(leaves);
    let mut root = Vec::new();
    root.extend_from_slice(&admission_hash);
    root.extend_from_slice(&total_claim.to_be_bytes());
    root.extend_from_slice(&(receipt_hashes.len() as u64).to_be_bytes());
    root.extend_from_slice(&merkle);
    blake3_hash(&root)
}

pub fn merkle_root(mut leaves: Vec<Hash>) -> Hash {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    while leaves.len() > 1 {
        let mut next = Vec::with_capacity(leaves.len().div_ceil(2));
        let mut i = 0;
        while i < leaves.len() {
            let left = leaves[i];
            let right = if i + 1 < leaves.len() { leaves[i + 1] } else { left };
            let mut bytes = Vec::with_capacity(64);
            bytes.extend_from_slice(&left);
            bytes.extend_from_slice(&right);
            next.push(blake3_hash(&bytes));
            i += 2;
        }
        leaves = next;
    }
    leaves[0]
}

pub fn worker_claim_totals(receipts: &[WorkReceipt]) -> BTreeMap<NodeId, u64> {
    let mut totals = BTreeMap::new();
    for receipt in receipts {
        let value = totals.entry(receipt.worker.node_id).or_insert(0u64);
        *value = value.saturating_add(receipt.total_claim);
    }
    totals
}
