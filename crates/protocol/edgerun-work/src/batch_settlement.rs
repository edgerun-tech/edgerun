use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use crate::preimage::HashBuilder;
use crate::protocol::*;
use crate::settlement::{work_admission_hash, work_receipt_hash, SettlementError, SettlementLedger};
use crate::signing::{verify_work_admission, verify_work_receipt};

const RECEIPT_BATCH_LEAF_DOMAIN: &[u8] = b"edgerun:v1:work:receipt-batch-leaf";
const RECEIPT_BATCH_ROOT_DOMAIN: &[u8] = b"edgerun:v1:work:receipt-batch-root";
const MERKLE_PARENT_DOMAIN: &[u8] = b"edgerun:v1:work:merkle-parent";

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
    for receipt_hash in receipt_hashes {
        leaves.push(receipt_batch_leaf_hash(admission_hash, *receipt_hash));
    }
    let merkle = merkle_root(leaves);
    HashBuilder::domain(RECEIPT_BATCH_ROOT_DOMAIN)
        .hash(&admission_hash)
        .u64(total_claim)
        .u64(receipt_hashes.len() as u64)
        .hash(&merkle)
        .finish()
}

pub fn receipt_batch_leaf_hash(admission_hash: Hash, receipt_hash: Hash) -> Hash {
    HashBuilder::domain(RECEIPT_BATCH_LEAF_DOMAIN)
        .hash(&admission_hash)
        .hash(&receipt_hash)
        .finish()
}

pub fn merkle_parent_hash(left: Hash, right: Hash) -> Hash {
    HashBuilder::domain(MERKLE_PARENT_DOMAIN)
        .hash(&left)
        .hash(&right)
        .finish()
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
            next.push(merkle_parent_hash(left, right));
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
