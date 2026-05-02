//! Exchange routing policy.
//!
//! Defines how quotes are routed to providers based on asset pairs,
//! amount thresholds, and fee optimization.

extern crate alloc;

use alloc::vec::Vec;

pub struct RoutingPolicy {
    pub prefer_stable: bool,
    pub max_slippage_bps: u32,
    pub timeout_secs: u32,
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self {
            prefer_stable: true,
            max_slippage_bps: 50,
            timeout_secs: 30,
        }
    }
}