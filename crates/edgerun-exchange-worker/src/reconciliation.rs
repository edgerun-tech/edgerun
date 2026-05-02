//! Reconciliation worker — detects provider/canonical status contradictions.
//!
//! STUB: Not yet implemented. In production this will:
//! - Compare canonical status (derived from events) with latest provider status
//! - When providers contradict terminal state, emit ManualReviewRequired
//! - Handle provider disagreement (multiple providers for same order)
//! - Generate reconciliation reports for audit trail

use edgerun_log;

/// Placeholder reconciliation configuration.
pub struct ReconciliationConfig {
    pub interval_ms: u64,
}

impl Default for ReconciliationConfig {
    fn default() -> Self {
        Self {
            interval_ms: 60_000,
        }
    }
}

/// Run reconciliation pass. Currently a no-op stub.
pub fn run_reconciliation(_config: ReconciliationConfig) {
    edgerun_log::info!("EXCHANGE_RECONCILIATION: stub — no active reconciliation");
}
