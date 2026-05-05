//! Exchange routing policy.
//!
//! Defines how quotes are routed to providers based on asset pairs,
//! amount thresholds, and fee optimization.

extern crate alloc;

#[derive(Debug, Clone)]
pub struct RoutingPolicy {
    pub prefer_stable: bool,
    pub max_slippage_bps: u32,
    pub timeout_secs: u32,
    /// EdgeRun settlement commission in basis points.
    pub edgerun_bps: u32,
    /// Minimum remaining quote lifetime required before returning a route.
    pub min_quote_ttl_ms: u64,
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self {
            prefer_stable: true,
            max_slippage_bps: 50,
            timeout_secs: 30,
            edgerun_bps: 30,
            min_quote_ttl_ms: 15_000,
        }
    }
}
