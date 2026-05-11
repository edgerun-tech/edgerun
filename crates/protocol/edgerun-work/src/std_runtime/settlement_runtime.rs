use std::sync::{Arc, Mutex};

use crate::batch_settlement::{BatchSettlementError, BatchSettlementResult};
use crate::protocol::{Hash, NodeId, PublicKey, WorkAdmission, WorkReceipt};
use crate::settlement::{SettlementError, SettlementLedger, SettlementPruneResult, SettlementResult};

#[derive(Clone, Debug, Default)]
pub struct ThreadSafeSettlementLedger {
    inner: Arc<Mutex<SettlementLedger>>,
}

impl ThreadSafeSettlementLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_ledger(ledger: SettlementLedger) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ledger)),
        }
    }

    pub fn deposit_user_credit(&self, user: PublicKey, amount: u64) -> u64 {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .deposit_user_credit(user, amount)
    }

    pub fn user_balance(&self, user: &PublicKey) -> u64 {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .user_balance(user)
    }

    pub fn worker_balance(&self, worker: &NodeId) -> u64 {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .worker_balance(worker)
    }

    pub fn settle_receipt(
        &self,
        admission: &WorkAdmission,
        receipt: &WorkReceipt,
    ) -> Result<SettlementResult, SettlementError> {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .settle_receipt(admission, receipt)
    }

    pub fn settle_receipt_batch(
        &self,
        admission: &WorkAdmission,
        receipts: &[WorkReceipt],
    ) -> Result<BatchSettlementResult, BatchSettlementError> {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .settle_receipt_batch(admission, receipts)
    }

    pub fn prune_finalized_admission(&self, admission_hash: &Hash) -> SettlementPruneResult {
        self.inner
            .lock()
            .expect("settlement ledger poisoned")
            .prune_finalized_admission(admission_hash)
    }
}
