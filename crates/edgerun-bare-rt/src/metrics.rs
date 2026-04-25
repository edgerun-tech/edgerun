//! Metrics stubs.

#![no_std]

// ===========================================================================
// Metrics
// ===========================================================================

pub struct RuntimeMetrics {
    pub total_spawned: u64,
    pub total_completed: u64,
    pub total_aborted: u64,
    pub active_tasks: u64,
    pub blocking_threads: u64,
}

impl RuntimeMetrics {
    pub fn new() -> Self {
        Self {
            total_spawned: 0,
            total_completed: 0,
            total_aborted: 0,
            active_tasks: 0,
            blocking_threads: 0,
        }
    }
}