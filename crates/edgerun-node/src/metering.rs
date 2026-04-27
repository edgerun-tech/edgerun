/// Work metering — tracks resource consumption of a running workload.
///
/// A `WorkMeter` is created when a workload starts and finalized when it
/// completes. It measures elapsed time, I/O operations, and computes
/// billable reference-core-microseconds using the provider's
/// PerformanceCertificate multipliers.
///
/// Zero external dependencies — uses only core atomics and target time.
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
#[cfg(target_os = "none")]
use edgerun_bare_rt::Instant;
#[cfg(not(target_os = "none"))]
use std::time::Instant;

use edgerun_core::accounting::{
    PerformanceCertificate, WorkAccounting, WorkPriority, WorkStatus, WorkloadClass,
};
use edgerun_core::fixed_point::FixedPoint16;

/// Monotonically increasing timestamp source.
/// Returns microseconds since Unix epoch.
fn now_us() -> u64 {
    #[cfg(not(target_os = "none"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before UNIX epoch")
            .as_micros() as u64
    }

    #[cfg(target_os = "none")]
    {
        0
    }
}

impl Clone for WorkMeter {
    fn clone(&self) -> Self {
        Self {
            work_id: self.work_id,
            requester_id: self.requester_id,
            provider_id: self.provider_id,
            delegation_hash: self.delegation_hash,
            start_time: self.start_time,
            start_monotonic_us: self.start_monotonic_us,
            allocated_cores: self.allocated_cores,
            allocated_memory_bytes: self.allocated_memory_bytes,
            storage_read_bytes: AtomicU64::new(
                self.storage_read_bytes
                    .load(Ordering::Relaxed),
            ),
            storage_written_bytes: AtomicU64::new(
                self.storage_written_bytes
                    .load(Ordering::Relaxed),
            ),
            storage_read_ops: AtomicU32::new(
                self.storage_read_ops
                    .load(Ordering::Relaxed),
            ),
            storage_write_ops: AtomicU32::new(
                self.storage_write_ops
                    .load(Ordering::Relaxed),
            ),
            network_sent_bytes: AtomicU64::new(
                self.network_sent_bytes
                    .load(Ordering::Relaxed),
            ),
            network_received_bytes: AtomicU64::new(
                self.network_received_bytes
                    .load(Ordering::Relaxed),
            ),
            workload_class: self.workload_class,
            priority: self.priority,
            provider_cert_digest: self.provider_cert_digest,
            cpu_multiplier: self.cpu_multiplier,
            memory_multiplier: self.memory_multiplier,
            storage_multiplier: self.storage_multiplier,
            gpu_core_us: self.gpu_core_us,
            npu_core_us: self.npu_core_us,
        }
    }
}

/// A running meter for a single workload's resource consumption.
pub struct WorkMeter {
    // === Identity ===
    work_id: [u8; 32],
    requester_id: [u8; 64],
    provider_id: [u8; 64],
    delegation_hash: [u8; 32],

    // === Timing ===
    start_time: Instant,
    start_monotonic_us: u64,

    // === Resource allocation ===
    allocated_cores: u32,
    allocated_memory_bytes: u64,

    // === I/O counters (atomic for thread-safe access) ===
    storage_read_bytes: AtomicU64,
    storage_written_bytes: AtomicU64,
    storage_read_ops: AtomicU32,
    storage_write_ops: AtomicU32,
    network_sent_bytes: AtomicU64,
    network_received_bytes: AtomicU64,

    // === Classification ===
    workload_class: WorkloadClass,
    priority: WorkPriority,

    // === Performance multipliers ===
    provider_cert_digest: [u8; 32],
    cpu_multiplier: FixedPoint16,
    memory_multiplier: FixedPoint16,
    storage_multiplier: FixedPoint16,

    // === GPU/NPU ===
    gpu_core_us: u64,
    npu_core_us: u64,
}

impl WorkMeter {
    /// Create a new work meter.
    pub fn new(
        work_id: [u8; 32],
        requester_id: [u8; 64],
        provider_id: [u8; 64],
        delegation_hash: [u8; 32],
        allocated_cores: u32,
        allocated_memory_bytes: u64,
        workload_class: WorkloadClass,
        priority: WorkPriority,
        cert: &PerformanceCertificate,
    ) -> Self {
        Self {
            work_id,
            requester_id,
            provider_id,
            delegation_hash,
            start_time: Instant::now(),
            start_monotonic_us: now_us(),
            allocated_cores,
            allocated_memory_bytes,
            storage_read_bytes: AtomicU64::new(0),
            storage_written_bytes: AtomicU64::new(0),
            storage_read_ops: AtomicU32::new(0),
            storage_write_ops: AtomicU32::new(0),
            network_sent_bytes: AtomicU64::new(0),
            network_received_bytes: AtomicU64::new(0),
            workload_class,
            priority,
            provider_cert_digest: cert.digest,
            cpu_multiplier: cert.cpu_core_multiplier(),
            memory_multiplier: cert.memory_multiplier(),
            storage_multiplier: cert.storage_multiplier(),
            gpu_core_us: 0,
            npu_core_us: 0,
        }
    }

    /// Record a storage read operation.
    pub fn add_storage_read(&self, bytes: u64) {
        self.storage_read_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.storage_read_ops.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a storage write operation.
    pub fn add_storage_write(&self, bytes: u64) {
        self.storage_written_bytes
            .fetch_add(bytes, Ordering::Relaxed);
        self.storage_write_ops.fetch_add(1, Ordering::Relaxed);
    }

    /// Record network data sent.
    pub fn add_network_sent(&self, bytes: u64) {
        self.network_sent_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record network data received.
    pub fn add_network_received(&self, bytes: u64) {
        self.network_received_bytes
            .fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record GPU compute time (µs).
    pub fn add_gpu_compute(&mut self, core_us: u64) {
        self.gpu_core_us += core_us;
    }

    /// Record NPU compute time (µs).
    pub fn add_npu_compute(&mut self, core_us: u64) {
        self.npu_core_us += core_us;
    }

    /// Elapsed time in microseconds.
    fn elapsed_us(&self) -> u64 {
        self.start_time.elapsed().as_micros() as u64
    }

    /// Finalize the meter and produce an immutable WorkAccounting record.
    pub fn finalize(mut self, status: WorkStatus, exit_code: Option<i32>) -> WorkAccounting {
        let elapsed_us = self.elapsed_us();
        let completed_at_us = self.start_monotonic_us + elapsed_us;

        // Physical core-µs = allocated_cores × duration
        let physical_core_us = (self.allocated_cores as u64) * elapsed_us;

        // Memory: GB and duration in seconds
        let physical_memory_gb = (self.allocated_memory_bytes / (1024 * 1024 * 1024)) as u32;
        let memory_duration_seconds = (elapsed_us / 1_000_000) as u32;

        // Billable RC-µs = physical_core_us × cpu_multiplier
        let billable_compute_rc_us = self.cpu_multiplier.mul_u64(physical_core_us);

        WorkAccounting {
            work_id: self.work_id,
            requester_id: self.requester_id,
            provider_id: self.provider_id,
            delegation_hash: self.delegation_hash,
            started_at_us: self.start_monotonic_us,
            completed_at_us,
            physical_core_us,
            physical_memory_gb,
            memory_duration_seconds,
            gpu_core_us: self.gpu_core_us,
            npu_core_us: self.npu_core_us,
            storage_read_bytes: self.storage_read_bytes.load(Ordering::Relaxed),
            storage_written_bytes: self.storage_written_bytes.load(Ordering::Relaxed),
            storage_read_ops: self.storage_read_ops.load(Ordering::Relaxed),
            storage_write_ops: self.storage_write_ops.load(Ordering::Relaxed),
            network_sent_bytes: self.network_sent_bytes.load(Ordering::Relaxed),
            network_received_bytes: self.network_received_bytes.load(Ordering::Relaxed),
            workload_class: self.workload_class,
            priority: self.priority,
            status,
            exit_code,
            provider_cert_digest: self.provider_cert_digest,
            cpu_multiplier: self.cpu_multiplier,
            memory_multiplier: self.memory_multiplier,
            storage_multiplier: self.storage_multiplier,
            billable_compute_rc_us,
        }
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Compute a SHA-256 work ID from a workload spec string.
pub fn compute_work_id(spec: &[u8]) -> [u8; 32] {
    let hash = edgerun_core::crypto::sha256(spec);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hash);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cert() -> PerformanceCertificate {
        PerformanceCertificate {
            node_id: [1u8; 64],
            cpu_int_score: 2_000_000, // 2x reference
            cpu_crypto_score: 1_000_000,
            mem_bandwidth_mbps: 10_000,
            mem_latency_ns: 50,
            storage_random_iops: 6_000,
            storage_seq_mbps: 100,
            storage_event_iops: 1_000,
            storage_blob_ops: 2_000,
            storage_object_ops: 1_000,
            net_frame_encode_decode_ops: 20_000,
            net_frame_sign_verify_ops: 2_000,
            net_udp_throughput_ops: 100_000,
            net_router_lookup_ops: 200_000,
            gpu_score: None,
            npu_score: None,
            benchmark_started_us: 0,
            benchmark_completed_us: 0,
            digest: [0xAB; 32],
            signature: [0u8; 64],
        }
    }

    #[test]
    fn work_meter_produces_accounting() {
        let cert = test_cert();
        let meter = WorkMeter::new(
            [0xAA; 32],
            [0xBB; 64],
            [0xCC; 64],
            [0xDD; 32],
            4,                      // 4 cores
            8 * 1024 * 1024 * 1024, // 8GB
            WorkloadClass::Container,
            WorkPriority::Standard,
            &cert,
        );

        meter.add_storage_read(1048576);
        meter.add_storage_write(524288);
        meter.add_network_sent(4194304);
        meter.add_network_received(2097152);

        let accounting = meter.finalize(WorkStatus::Completed, Some(0));

        assert_eq!(accounting.workload_class, WorkloadClass::Container);
        assert_eq!(accounting.status, WorkStatus::Completed);
        assert_eq!(accounting.storage_read_ops, 1);
        assert_eq!(accounting.storage_write_ops, 1);
        assert_eq!(accounting.network_sent_bytes, 4194304);

        // CPU multiplier is 2x, 4 cores × elapsed
        assert_eq!(
            accounting.billable_compute_rc_us,
            accounting
                .cpu_multiplier
                .mul_u64(accounting.physical_core_us)
        );
    }

    #[test]
    fn compute_work_id_is_deterministic() {
        let spec = b"container:ubuntu:latest:cores=4:memory=8G";
        let id1 = compute_work_id(spec);
        let id2 = compute_work_id(spec);
        assert_eq!(id1, id2);
    }
}
