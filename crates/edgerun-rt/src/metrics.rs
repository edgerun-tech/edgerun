//! Metrics collection for runtime

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub struct Metrics {
    tasks_spawned: AtomicU64,
    tasks_completed: AtomicU64,
    tasks_failed: AtomicU64,
    tasks_pending: AtomicUsize,
    cpu_time_ns: AtomicU64,
    io_bytes_read: AtomicU64,
    io_bytes_written: AtomicU64,
}

impl Metrics {
    pub const fn new() -> Self {
        Self {
            tasks_spawned: AtomicU64::new(0),
            tasks_completed: AtomicU64::new(0),
            tasks_failed: AtomicU64::new(0),
            tasks_pending: AtomicUsize::new(0),
            cpu_time_ns: AtomicU64::new(0),
            io_bytes_read: AtomicU64::new(0),
            io_bytes_written: AtomicU64::new(0),
        }
    }

    pub fn spawn_task(&self) {
        self.tasks_spawned.fetch_add(1, Ordering::Release);
        self.tasks_pending.fetch_add(1, Ordering::Release);
    }

    pub fn complete_task(&self) {
        self.tasks_completed.fetch_add(1, Ordering::Release);
        self.tasks_pending.fetch_sub(1, Ordering::Release);
    }

    pub fn fail_task(&self) {
        self.tasks_failed.fetch_add(1, Ordering::Release);
        self.tasks_pending.fetch_sub(1, Ordering::Release);
    }

    pub fn add_cpu_time(&self, ns: u64) {
        self.cpu_time_ns.fetch_add(ns, Ordering::Release);
    }

    pub fn add_read_bytes(&self, bytes: u64) {
        self.io_bytes_read.fetch_add(bytes, Ordering::Release);
    }

    pub fn add_write_bytes(&self, bytes: u64) {
        self.io_bytes_written.fetch_add(bytes, Ordering::Release);
    }

    pub fn tasks_spawned(&self) -> u64 {
        self.tasks_spawned.load(Ordering::Acquire)
    }

    pub fn tasks_completed(&self) -> u64 {
        self.tasks_completed.load(Ordering::Acquire)
    }

    pub fn tasks_pending(&self) -> usize {
        self.tasks_pending.load(Ordering::Acquire)
    }

    pub fn cpu_time_ns(&self) -> u64 {
        self.cpu_time_ns.load(Ordering::Acquire)
    }

    pub fn io_read_bytes(&self) -> u64 {
        self.io_bytes_read.load(Ordering::Acquire)
    }

    pub fn io_write_bytes(&self) -> u64 {
        self.io_bytes_written.load(Ordering::Acquire)
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}