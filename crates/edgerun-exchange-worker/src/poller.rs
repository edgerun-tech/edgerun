//! Order polling worker — polls provider status for active orders.
//!
//! STUB: Not yet implemented. In production this will:
//! - Maintain a set of active order IDs from the event log
//! - Periodically poll each provider for status updates
//! - Emit ProviderStatusObserved events into the exchange stream
//! - Detect contradictions between provider status and canonical status
//!
//! Until implemented, no polling occurs and status is only updated
//! via explicit API calls.

use edgerun_log;

/// Placeholder poller configuration.
pub struct PollerConfig {
    pub interval_ms: u64,
    pub max_concurrent_polls: u32,
}

impl Default for PollerConfig {
    fn default() -> Self {
        Self {
            interval_ms: 30_000,
            max_concurrent_polls: 10,
        }
    }
}

/// Start the poller loop. Currently a no-op stub.
pub fn run_poller(_config: PollerConfig) {
    edgerun_log::info!("EXCHANGE_POLLER: stub — no active polling");
}
