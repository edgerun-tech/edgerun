//! On-chain watcher placeholder.
//!
//! The Solana-specific chain watcher was removed. This module is retained
//! for future on-chain integration with non-Solana chains.

pub struct ChainWatcher;

impl ChainWatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn poll(&self) {
        // Placeholder for future chain integration
    }
}
