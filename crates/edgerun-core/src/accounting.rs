/// Accounting types for the edgerun compute marketplace.
///
/// Defines the core data structures for tracking billable compute work.
/// All units are integers — no floating point in the critical path.
///
/// ## Primary Unit: Reference Core-Microseconds (RC-µs)
/// Physical core-µs are adjusted by a benchmark-derived multiplier to produce
/// billable reference core-µs. This ensures heterogeneous hardware is priced fairly.
///
/// ## Resource Units
/// | Resource     | Unit                  | Type   |
/// |-------------|-----------------------|--------|
/// | Compute     | core-microseconds     | u64    |
/// | Memory      | byte-microseconds     | u128   |
/// | Storage I/O | ops + bytes           | u64    |
/// | Network     | bytes                 | u64    |

use crate::fixed_point::FixedPoint16;
use crate::crypto::sha256;

// ===========================================================================
// Workload classification
// ===========================================================================

/// Classification of the workload type (affects pricing tiers).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkloadClass {
    /// Generic OCI container
    Container,
    /// ML inference
    Inference,
    /// Model compilation / training
    Compilation,
    /// Data processing / ETL
    DataProcessing,
    /// Media transcoding
    Transcoding,
    /// General compute task
    General,
    /// Custom / unknown
    Other(u32),
}

impl WorkloadClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkloadClass::Container => "container",
            WorkloadClass::Inference => "inference",
            WorkloadClass::Compilation => "compilation",
            WorkloadClass::DataProcessing => "data_processing",
            WorkloadClass::Transcoding => "transcoding",
            WorkloadClass::General => "general",
            WorkloadClass::Other(_) => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "container" => WorkloadClass::Container,
            "inference" => WorkloadClass::Inference,
            "compilation" => WorkloadClass::Compilation,
            "data_processing" => WorkloadClass::DataProcessing,
            "transcoding" => WorkloadClass::Transcoding,
            "general" => WorkloadClass::General,
            _ => WorkloadClass::Other(0),
        }
    }

    pub fn to_u32(&self) -> u32 {
        match self {
            WorkloadClass::Container => 1,
            WorkloadClass::Inference => 2,
            WorkloadClass::Compilation => 3,
            WorkloadClass::DataProcessing => 4,
            WorkloadClass::Transcoding => 5,
            WorkloadClass::General => 6,
            WorkloadClass::Other(v) => 1000 + *v,
        }
    }
}

/// Work priority level (affects pricing and scheduling).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkPriority {
    /// Best-effort, lowest cost
    Batch,
    /// Standard marketplace rate
    Standard,
    /// Expedited, premium rate
    Expedited,
}

impl WorkPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkPriority::Batch => "batch",
            WorkPriority::Standard => "standard",
            WorkPriority::Expedited => "expedited",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "batch" => WorkPriority::Batch,
            "standard" => WorkPriority::Standard,
            "expedited" => WorkPriority::Expedited,
            _ => WorkPriority::Standard,
        }
    }
}

/// Outcome of a workload execution.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkStatus {
    /// Completed successfully
    Completed,
    /// Failed during execution
    Failed,
    /// Terminated by requester
    Terminated,
    /// Preempted by higher-priority work
    Preempted,
}

impl WorkStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkStatus::Completed => "completed",
            WorkStatus::Failed => "failed",
            WorkStatus::Terminated => "terminated",
            WorkStatus::Preempted => "preempted",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "completed" => WorkStatus::Completed,
            "failed" => WorkStatus::Failed,
            "terminated" => WorkStatus::Terminated,
            "preempted" => WorkStatus::Preempted,
            _ => WorkStatus::Failed,
        }
    }
}

// ===========================================================================
// Performance Certificate
// ===========================================================================

/// A cryptographically signed performance proof for a node.
///
/// Produced at node init (and periodically thereafter) by running
/// built-in deterministic benchmarks. The certificate is stored in
/// the node's event stream and can be verified by any peer.
///
/// The multipliers derived from this certificate convert physical
/// core-µs into billable reference core-µs.
#[derive(Clone, Debug)]
pub struct PerformanceCertificate {
    /// Node's ECDSA P-256 public key (64 bytes, uncompressed without 0x04 prefix)
    pub node_id: [u8; 64],
    /// CPU integer math throughput score (higher = faster)
    pub cpu_int_score: u64,
    /// CPU crypto (SHA-256 / AES) throughput score
    pub cpu_crypto_score: u64,
    /// Memory bandwidth in MB/s
    pub mem_bandwidth_mbps: u64,
    /// Memory latency in nanoseconds (lower = better)
    pub mem_latency_ns: u64,
    /// Storage random IOPS
    pub storage_random_iops: u64,
    /// Storage sequential throughput MB/s
    pub storage_seq_mbps: u64,
    /// Storage event append IOPS (protobuf encode + file append + fsync)
    pub storage_event_iops: u64,
    /// Storage blob put/get ops/s (encrypt + write / read + decrypt)
    pub storage_blob_ops: u64,
    /// Storage object put/get ops/s (encode + encrypt + blob + index)
    pub storage_object_ops: u64,
    /// Mesh frame encode/decode ops/s
    pub net_frame_encode_decode_ops: u64,
    /// Mesh frame sign/verify ops/s
    pub net_frame_sign_verify_ops: u64,
    /// UDP raw throughput ops/s (send + recv loopback)
    pub net_udp_throughput_ops: u64,
    /// Mesh router next-hop lookup ops/s
    pub net_router_lookup_ops: u64,
    /// Optional GPU compute score
    pub gpu_score: Option<u64>,
    /// Optional NPU compute score
    pub npu_score: Option<u64>,
    /// Benchmark start time (µs since Unix epoch)
    pub benchmark_started_us: u64,
    /// Benchmark end time (µs since Unix epoch)
    pub benchmark_completed_us: u64,
    /// SHA-256 of all above fields (integrity check)
    pub digest: [u8; 32],
    /// ECDSA P-256 signature of the digest (64 bytes: r || s)
    pub signature: [u8; 64],
}

/// Reference scores for a baseline machine (e.g., Raspberry Pi 4 or t3.micro).
/// These are the denominator for the multiplier calculation.
pub const REFERENCE_CPU_INT_SCORE: u64 = 1_000_000;
pub const REFERENCE_CPU_CRYPTO_SCORE: u64 = 500_000;
pub const REFERENCE_MEM_BW_MBPS: u64 = 5_000;
pub const REFERENCE_MEM_LATENCY_NS: u64 = 100;
pub const REFERENCE_STORAGE_IOPS: u64 = 3_000;
pub const REFERENCE_STORAGE_SEQ_MBPS: u64 = 50;
pub const REFERENCE_STORAGE_EVENT_IOPS: u64 = 500;
pub const REFERENCE_STORAGE_BLOB_OPS: u64 = 1_000;
pub const REFERENCE_STORAGE_OBJECT_OPS: u64 = 500;
pub const REFERENCE_NET_FRAME_ENCODE_DECODE_OPS: u64 = 10_000;
pub const REFERENCE_NET_FRAME_SIGN_VERIFY_OPS: u64 = 1_000;
pub const REFERENCE_NET_UDP_THROUGHPUT_OPS: u64 = 50_000;
pub const REFERENCE_NET_ROUTER_LOOKUP_OPS: u64 = 100_000;

impl PerformanceCertificate {
    /// Compute the SHA-256 digest of all performance fields (excludes digest and signature).
    pub fn compute_digest(&self) -> [u8; 32] {
        let mut buf = [0u8; 64 + 8 * 15 + 8 + 8]; // 200 bytes
        buf[0..64].copy_from_slice(&self.node_id);
        buf[64..72].copy_from_slice(&self.cpu_int_score.to_le_bytes());
        buf[72..80].copy_from_slice(&self.cpu_crypto_score.to_le_bytes());
        buf[80..88].copy_from_slice(&self.mem_bandwidth_mbps.to_le_bytes());
        buf[88..96].copy_from_slice(&self.mem_latency_ns.to_le_bytes());
        buf[96..104].copy_from_slice(&self.storage_random_iops.to_le_bytes());
        buf[104..112].copy_from_slice(&self.storage_seq_mbps.to_le_bytes());
        buf[112..120].copy_from_slice(&self.storage_event_iops.to_le_bytes());
        buf[120..128].copy_from_slice(&self.storage_blob_ops.to_le_bytes());
        buf[128..136].copy_from_slice(&self.storage_object_ops.to_le_bytes());
        buf[136..144].copy_from_slice(&self.net_frame_encode_decode_ops.to_le_bytes());
        buf[144..152].copy_from_slice(&self.net_frame_sign_verify_ops.to_le_bytes());
        buf[152..160].copy_from_slice(&self.net_udp_throughput_ops.to_le_bytes());
        buf[160..168].copy_from_slice(&self.net_router_lookup_ops.to_le_bytes());
        buf[168..176].copy_from_slice(&(self.gpu_score.unwrap_or(0)).to_le_bytes());
        buf[176..184].copy_from_slice(&(self.npu_score.unwrap_or(0)).to_le_bytes());
        buf[184..192].copy_from_slice(&self.benchmark_started_us.to_le_bytes());
        buf[192..200].copy_from_slice(&self.benchmark_completed_us.to_le_bytes());
        let hash = sha256(&buf);
        let mut out = [0u8; 32];
        out.copy_from_slice(&hash);
        out
    }

    /// Verify the certificate's integrity (digest matches) and ECDSA signature.
    /// Returns `true` if both the digest and signature are valid.
    pub fn verify(&self) -> bool {
        // 1. Check digest matches
        if self.digest != self.compute_digest() {
            return false;
        }

        // 2. Verify ECDSA P-256 signature over the digest
        self.verify_ecdsa_signature().is_ok()
    }

    fn verify_ecdsa_signature(&self) -> Result<(), &'static str> {
        use p256::ecdsa::{Signature, VerifyingKey, signature::hazmat::PrehashVerifier};
        use p256::EncodedPoint;

        // Reconstruct the public key from node_id (64 bytes: x || y, uncompressed without 0x04)
        let mut pk_bytes = [0u8; 65];
        pk_bytes[0] = 0x04;
        pk_bytes[1..33].copy_from_slice(&self.node_id[..32]);
        pk_bytes[33..].copy_from_slice(&self.node_id[32..]);

        let encoded_point = EncodedPoint::from_bytes(pk_bytes).map_err(|_| "Invalid public key")?;
        let verifying_key = VerifyingKey::from_encoded_point(&encoded_point).map_err(|_| "Invalid verifying key")?;

        // Parse signature (r || s format, 64 bytes total)
        let r_bytes: [u8; 32] = self.signature[..32].try_into().map_err(|_| "Invalid r")?;
        let s_bytes: [u8; 32] = self.signature[32..].try_into().map_err(|_| "Invalid s")?;

        let sig = Signature::from_scalars(r_bytes, s_bytes).map_err(|_| "Invalid signature")?;

        // Verify signature over the digest
        verifying_key.verify_prehash(&self.digest, &sig).map_err(|_| "Signature verification failed")
    }

    /// Compute and return a new certificate with the digest field populated.
    /// Convenience method for benchmark construction.
    pub fn with_digest(mut self) -> Self {
        self.digest = self.compute_digest();
        self
    }

    /// CPU core multiplier relative to reference.
    /// E.g., 2.5x means 1 physical core-µs = 2.5 billable RC-µs.
    pub fn cpu_core_multiplier(&self) -> FixedPoint16 {
        FixedPoint16::from_ratio(self.cpu_int_score, REFERENCE_CPU_INT_SCORE)
    }

    /// Memory quality factor (combines bandwidth and latency).
    pub fn memory_multiplier(&self) -> FixedPoint16 {
        let bw_factor = FixedPoint16::from_ratio(self.mem_bandwidth_mbps, REFERENCE_MEM_BW_MBPS);
        let lat_factor = if self.mem_latency_ns > 0 {
            FixedPoint16::from_ratio(REFERENCE_MEM_LATENCY_NS, self.mem_latency_ns)
        } else {
            FixedPoint16::ONE
        };
        bw_factor.mul_fp(lat_factor)
    }

    /// Storage I/O multiplier.
    pub fn storage_multiplier(&self) -> FixedPoint16 {
        FixedPoint16::from_ratio(self.storage_random_iops, REFERENCE_STORAGE_IOPS)
    }

    /// Serialize the certificate to bytes for wire transport.
    /// Format: node_id(64) + scores(15*8) + times(2*8) + digest(32) + signature(64) = 296 bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(296);
        buf.extend_from_slice(&self.node_id);
        buf.extend_from_slice(&self.cpu_int_score.to_le_bytes());
        buf.extend_from_slice(&self.cpu_crypto_score.to_le_bytes());
        buf.extend_from_slice(&self.mem_bandwidth_mbps.to_le_bytes());
        buf.extend_from_slice(&self.mem_latency_ns.to_le_bytes());
        buf.extend_from_slice(&self.storage_random_iops.to_le_bytes());
        buf.extend_from_slice(&self.storage_seq_mbps.to_le_bytes());
        buf.extend_from_slice(&self.storage_event_iops.to_le_bytes());
        buf.extend_from_slice(&self.storage_blob_ops.to_le_bytes());
        buf.extend_from_slice(&self.storage_object_ops.to_le_bytes());
        buf.extend_from_slice(&self.net_frame_encode_decode_ops.to_le_bytes());
        buf.extend_from_slice(&self.net_frame_sign_verify_ops.to_le_bytes());
        buf.extend_from_slice(&self.net_udp_throughput_ops.to_le_bytes());
        buf.extend_from_slice(&self.net_router_lookup_ops.to_le_bytes());
        buf.extend_from_slice(&(self.gpu_score.unwrap_or(0)).to_le_bytes());
        buf.extend_from_slice(&(self.npu_score.unwrap_or(0)).to_le_bytes());
        buf.extend_from_slice(&self.benchmark_started_us.to_le_bytes());
        buf.extend_from_slice(&self.benchmark_completed_us.to_le_bytes());
        buf.extend_from_slice(&self.digest);
        buf.extend_from_slice(&self.signature);
        buf
    }

    /// Deserialize from bytes.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 296 {
            return None;
        }
        let mut node_id = [0u8; 64];
        node_id.copy_from_slice(&data[0..64]);
        let cpu_int_score = u64::from_le_bytes(data[64..72].try_into().ok()?);
        let cpu_crypto_score = u64::from_le_bytes(data[72..80].try_into().ok()?);
        let mem_bandwidth_mbps = u64::from_le_bytes(data[80..88].try_into().ok()?);
        let mem_latency_ns = u64::from_le_bytes(data[88..96].try_into().ok()?);
        let storage_random_iops = u64::from_le_bytes(data[96..104].try_into().ok()?);
        let storage_seq_mbps = u64::from_le_bytes(data[104..112].try_into().ok()?);
        let storage_event_iops = u64::from_le_bytes(data[112..120].try_into().ok()?);
        let storage_blob_ops = u64::from_le_bytes(data[120..128].try_into().ok()?);
        let storage_object_ops = u64::from_le_bytes(data[128..136].try_into().ok()?);
        let net_frame_encode_decode_ops = u64::from_le_bytes(data[136..144].try_into().ok()?);
        let net_frame_sign_verify_ops = u64::from_le_bytes(data[144..152].try_into().ok()?);
        let net_udp_throughput_ops = u64::from_le_bytes(data[152..160].try_into().ok()?);
        let net_router_lookup_ops = u64::from_le_bytes(data[160..168].try_into().ok()?);
        let gpu_raw = u64::from_le_bytes(data[168..176].try_into().ok()?);
        let npu_raw = u64::from_le_bytes(data[176..184].try_into().ok()?);
        let benchmark_started_us = u64::from_le_bytes(data[184..192].try_into().ok()?);
        let benchmark_completed_us = u64::from_le_bytes(data[192..200].try_into().ok()?);
        let mut digest = [0u8; 32];
        digest.copy_from_slice(&data[200..232]);
        let mut signature = [0u8; 64];
        signature.copy_from_slice(&data[232..296]);

        let gpu_score = if gpu_raw > 0 { Some(gpu_raw) } else { None };
        let npu_score = if npu_raw > 0 { Some(npu_raw) } else { None };

        Some(Self {
            node_id, cpu_int_score, cpu_crypto_score, mem_bandwidth_mbps,
            mem_latency_ns, storage_random_iops, storage_seq_mbps,
            storage_event_iops, storage_blob_ops, storage_object_ops,
            net_frame_encode_decode_ops, net_frame_sign_verify_ops,
            net_udp_throughput_ops, net_router_lookup_ops,
            gpu_score, npu_score, benchmark_started_us, benchmark_completed_us,
            digest, signature,
        })
    }
}

// ===========================================================================
// Work Accounting Record
// ===========================================================================

/// A single billable work unit — immutable once written.
///
/// Records both physical resource consumption and billable
/// reference-core-microseconds (adjusted by the provider's
/// PerformanceCertificate at the time of execution).
#[derive(Clone, Debug)]
pub struct WorkAccounting {
    // === Identity ===
    /// SHA-256 of workload spec (unique work identifier)
    pub work_id: [u8; 32],
    /// ECDSA public key of the buyer (64 bytes)
    pub requester_id: [u8; 64],
    /// ECDSA public key of the seller/this node (64 bytes)
    pub provider_id: [u8; 64],
    /// SHA-256 of the delegation record that authorized this work
    pub delegation_hash: [u8; 32],

    // === Timing (microseconds since Unix epoch) ===
    pub started_at_us: u64,
    pub completed_at_us: u64,

    // === Physical resource consumption ===
    /// Physical core-microseconds (cores × duration)
    pub physical_core_us: u64,
    /// Physical memory allocated in GB (for billing)
    pub physical_memory_gb: u32,
    /// Memory allocation duration in seconds
    pub memory_duration_seconds: u32,
    /// GPU compute unit microseconds
    pub gpu_core_us: u64,
    /// NPU MAC array microseconds
    pub npu_core_us: u64,

    // === I/O ===
    pub storage_read_bytes: u64,
    pub storage_written_bytes: u64,
    pub storage_read_ops: u32,
    pub storage_write_ops: u32,
    pub network_sent_bytes: u64,
    pub network_received_bytes: u64,

    // === Classification ===
    pub workload_class: WorkloadClass,
    pub priority: WorkPriority,

    // === Outcome ===
    pub status: WorkStatus,
    pub exit_code: Option<i32>,

    // === Performance multipliers (from provider's cert at execution time) ===
    pub provider_cert_digest: [u8; 32],
    pub cpu_multiplier: FixedPoint16,
    pub memory_multiplier: FixedPoint16,
    pub storage_multiplier: FixedPoint16,

    // === Billable units (derived) ===
    /// billable = physical_core_us × cpu_multiplier
    pub billable_compute_rc_us: u64,
}

impl WorkAccounting {
    /// Compute SHA-256 of all accounting fields for integrity.
    /// Reuses `to_bytes()` to avoid duplicating serialization logic.
    pub fn compute_record_hash(&self) -> [u8; 32] {
        let bytes = self.to_bytes();
        let hash = sha256(&bytes);
        let mut out = [0u8; 32];
        out.copy_from_slice(&hash);
        out
    }

    /// Serialize to bytes for persistence.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.work_id);
        buf.extend_from_slice(&self.requester_id);
        buf.extend_from_slice(&self.provider_id);
        buf.extend_from_slice(&self.delegation_hash);
        buf.extend_from_slice(&self.started_at_us.to_le_bytes());
        buf.extend_from_slice(&self.completed_at_us.to_le_bytes());
        buf.extend_from_slice(&self.physical_core_us.to_le_bytes());
        buf.extend_from_slice(&self.physical_memory_gb.to_le_bytes());
        buf.extend_from_slice(&self.memory_duration_seconds.to_le_bytes());
        buf.extend_from_slice(&self.gpu_core_us.to_le_bytes());
        buf.extend_from_slice(&self.npu_core_us.to_le_bytes());
        buf.extend_from_slice(&self.storage_read_bytes.to_le_bytes());
        buf.extend_from_slice(&self.storage_written_bytes.to_le_bytes());
        buf.extend_from_slice(&self.storage_read_ops.to_le_bytes());
        buf.extend_from_slice(&self.storage_write_ops.to_le_bytes());
        buf.extend_from_slice(&self.network_sent_bytes.to_le_bytes());
        buf.extend_from_slice(&self.network_received_bytes.to_le_bytes());
        buf.extend_from_slice(&self.workload_class.to_u32().to_le_bytes());
        buf.extend_from_slice(&(match self.priority {
            WorkPriority::Batch => 0u32,
            WorkPriority::Standard => 1,
            WorkPriority::Expedited => 2,
        }).to_le_bytes());
        buf.extend_from_slice(&(match self.status {
            WorkStatus::Completed => 0u32,
            WorkStatus::Failed => 1,
            WorkStatus::Terminated => 2,
            WorkStatus::Preempted => 3,
        }).to_le_bytes());
        if let Some(code) = self.exit_code {
            buf.extend_from_slice(&1u32.to_le_bytes());
            buf.extend_from_slice(&code.to_le_bytes());
        } else {
            buf.extend_from_slice(&0u32.to_le_bytes());
            buf.extend_from_slice(&0i32.to_le_bytes());
        }
        buf.extend_from_slice(&self.provider_cert_digest);
        buf.extend_from_slice(&self.cpu_multiplier.to_raw().to_le_bytes());
        buf.extend_from_slice(&self.memory_multiplier.to_raw().to_le_bytes());
        buf.extend_from_slice(&self.storage_multiplier.to_raw().to_le_bytes());
        buf.extend_from_slice(&self.billable_compute_rc_us.to_le_bytes());
        buf
    }

    /// Deserialize from bytes.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        const MIN_SIZE: usize = 32+64+64+32 + 8+8 + 8+4+4+8+8 + 8+8+4+4 + 8+8 + 4+4+4+8 + 32+4+4+4 + 8;
        if data.len() < MIN_SIZE {
            return None;
        }

        struct Reader<'a> {
            data: &'a [u8],
            offset: usize,
        }
        impl<'a> Reader<'a> {
            fn read32(&mut self) -> Option<[u8; 32]> {
                if self.offset + 32 > self.data.len() { return None; }
                let v: [u8; 32] = self.data[self.offset..self.offset+32].try_into().ok()?;
                self.offset += 32;
                Some(v)
            }
            fn read64(&mut self) -> Option<u64> {
                if self.offset + 8 > self.data.len() { return None; }
                let v = u64::from_le_bytes(self.data[self.offset..self.offset+8].try_into().ok()?);
                self.offset += 8;
                Some(v)
            }
            fn read32u(&mut self) -> Option<u32> {
                if self.offset + 4 > self.data.len() { return None; }
                let v = u32::from_le_bytes(self.data[self.offset..self.offset+4].try_into().ok()?);
                self.offset += 4;
                Some(v)
            }
            fn read_i32(&mut self) -> Option<i32> {
                if self.offset + 4 > self.data.len() { return None; }
                let v = i32::from_le_bytes(self.data[self.offset..self.offset+4].try_into().ok()?);
                self.offset += 4;
                Some(v)
            }
            fn read64arr(&mut self) -> Option<[u8; 64]> {
                if self.offset + 64 > self.data.len() { return None; }
                let mut out = [0u8; 64];
                out.copy_from_slice(&self.data[self.offset..self.offset+64]);
                self.offset += 64;
                Some(out)
            }
        }

        let mut r = Reader { data, offset: 0 };

        let work_id = r.read32()?;
        let requester_id = r.read64arr()?;
        let provider_id = r.read64arr()?;
        let delegation_hash = r.read32()?;
        let started_at_us = r.read64()?;
        let completed_at_us = r.read64()?;
        let physical_core_us = r.read64()?;
        let physical_memory_gb = r.read32u()?;
        let memory_duration_seconds = r.read32u()?;
        let gpu_core_us = r.read64()?;
        let npu_core_us = r.read64()?;
        let storage_read_bytes = r.read64()?;
        let storage_written_bytes = r.read64()?;
        let storage_read_ops = r.read32u()?;
        let storage_write_ops = r.read32u()?;
        let network_sent_bytes = r.read64()?;
        let network_received_bytes = r.read64()?;
        let workload_class = WorkloadClass::from_str(
            match r.read32u()? {
                1 => "container",
                2 => "inference",
                3 => "compilation",
                4 => "data_processing",
                5 => "transcoding",
                6 => "general",
                _ => "other",
            }
        );
        let priority = match r.read32u()? {
            0 => WorkPriority::Batch,
            1 => WorkPriority::Standard,
            2 => WorkPriority::Expedited,
            _ => WorkPriority::Standard,
        };
        let status = match r.read32u()? {
            0 => WorkStatus::Completed,
            1 => WorkStatus::Failed,
            2 => WorkStatus::Terminated,
            3 => WorkStatus::Preempted,
            _ => WorkStatus::Failed,
        };
        let has_exit = r.read32u()?;
        let exit_code = if has_exit == 1 { Some(r.read_i32()?) } else { r.read_i32(); None };
        let provider_cert_digest = r.read32()?;
        let cpu_multiplier = FixedPoint16::from_raw(r.read32u()?);
        let memory_multiplier = FixedPoint16::from_raw(r.read32u()?);
        let storage_multiplier = FixedPoint16::from_raw(r.read32u()?);
        let billable_compute_rc_us = r.read64()?;

        Some(Self {
            work_id, requester_id, provider_id, delegation_hash,
            started_at_us, completed_at_us,
            physical_core_us, physical_memory_gb, memory_duration_seconds,
            gpu_core_us, npu_core_us,
            storage_read_bytes, storage_written_bytes,
            storage_read_ops, storage_write_ops,
            network_sent_bytes, network_received_bytes,
            workload_class, priority, status, exit_code,
            provider_cert_digest, cpu_multiplier, memory_multiplier, storage_multiplier,
            billable_compute_rc_us,
        })
    }

    /// Total storage I/O operations.
    pub fn total_storage_ops(&self) -> u64 {
        (self.storage_read_ops as u64) + (self.storage_write_ops as u64)
    }

    /// Total network bytes (sent + received).
    pub fn total_network_bytes(&self) -> u64 {
        self.network_sent_bytes.saturating_add(self.network_received_bytes)
    }
}

// ===========================================================================
// Compute Advertisement (for mesh discovery)
// ===========================================================================

/// A node's advertisement of available compute resources on the mesh.
/// Broadcast periodically to attract marketplace buyers.
#[derive(Clone, Debug)]
pub struct ComputeAdvertisement {
    /// Node's ECDSA public key (64 bytes)
    pub node_id: [u8; 64],
    /// Number of unreserved CPU cores available for sale
    pub available_cores: u32,
    /// Bytes of unreserved memory available
    pub available_memory_bytes: u64,
    /// Bytes of unreserved storage available
    pub available_storage_bytes: u64,
    /// Number of GPUs available
    pub gpu_count: u32,
    /// Number of NPUs available
    pub npu_count: u32,
    /// Hash of the node's PerformanceCertificate (buyers can fetch it)
    pub cert_digest: [u8; 32],
    /// Price per million reference-core-microseconds (in micro-credits)
    pub price_per_million_rc_us: u64,
    /// Minimum contract duration in microseconds
    pub min_contract_duration_us: u64,
    /// Advertisement timestamp (µs since epoch)
    pub timestamp_us: u64,
    /// ECDSA signature of all above fields
    pub signature: [u8; 64],
}

impl ComputeAdvertisement {
    /// Compute digest for signing.
    pub fn compute_digest(&self) -> [u8; 32] {
        let mut buf = [0u8; 64 + 4 + 8 + 8 + 4 + 4 + 32 + 8 + 8 + 8];
        buf[0..64].copy_from_slice(&self.node_id);
        buf[64..68].copy_from_slice(&self.available_cores.to_le_bytes());
        buf[68..76].copy_from_slice(&self.available_memory_bytes.to_le_bytes());
        buf[76..84].copy_from_slice(&self.available_storage_bytes.to_le_bytes());
        buf[84..88].copy_from_slice(&self.gpu_count.to_le_bytes());
        buf[88..92].copy_from_slice(&self.npu_count.to_le_bytes());
        buf[92..124].copy_from_slice(&self.cert_digest);
        buf[124..132].copy_from_slice(&self.price_per_million_rc_us.to_le_bytes());
        buf[132..140].copy_from_slice(&self.min_contract_duration_us.to_le_bytes());
        buf[140..148].copy_from_slice(&self.timestamp_us.to_le_bytes());
        sha256(&buf).try_into().unwrap()
    }

    /// Serialize to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(212);
        buf.extend_from_slice(&self.node_id);
        buf.extend_from_slice(&self.available_cores.to_le_bytes());
        buf.extend_from_slice(&self.available_memory_bytes.to_le_bytes());
        buf.extend_from_slice(&self.available_storage_bytes.to_le_bytes());
        buf.extend_from_slice(&self.gpu_count.to_le_bytes());
        buf.extend_from_slice(&self.npu_count.to_le_bytes());
        buf.extend_from_slice(&self.cert_digest);
        buf.extend_from_slice(&self.price_per_million_rc_us.to_le_bytes());
        buf.extend_from_slice(&self.min_contract_duration_us.to_le_bytes());
        buf.extend_from_slice(&self.timestamp_us.to_le_bytes());
        buf.extend_from_slice(&self.signature);
        buf
    }

    /// Deserialize from bytes.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 212 {
            return None;
        }
        let mut node_id = [0u8; 64];
        node_id.copy_from_slice(&data[0..64]);
        let available_cores = u32::from_le_bytes(data[64..68].try_into().ok()?);
        let available_memory_bytes = u64::from_le_bytes(data[68..76].try_into().ok()?);
        let available_storage_bytes = u64::from_le_bytes(data[76..84].try_into().ok()?);
        let gpu_count = u32::from_le_bytes(data[84..88].try_into().ok()?);
        let npu_count = u32::from_le_bytes(data[88..92].try_into().ok()?);
        let mut cert_digest = [0u8; 32];
        cert_digest.copy_from_slice(&data[92..124]);
        let price_per_million_rc_us = u64::from_le_bytes(data[124..132].try_into().ok()?);
        let min_contract_duration_us = u64::from_le_bytes(data[132..140].try_into().ok()?);
        let timestamp_us = u64::from_le_bytes(data[140..148].try_into().ok()?);
        let mut signature = [0u8; 64];
        signature.copy_from_slice(&data[148..212]);

        Some(Self {
            node_id, available_cores, available_memory_bytes, available_storage_bytes,
            gpu_count, npu_count, cert_digest, price_per_million_rc_us,
            min_contract_duration_us, timestamp_us, signature,
        })
    }
}

// ===========================================================================
// Work Settlement (dual-signed billing agreement)
// ===========================================================================

/// A settlement record — both buyer and seller agree on what was delivered and charged.
#[derive(Clone, Debug)]
pub struct WorkSettlement {
    /// Links to the WorkAccounting record
    pub work_id: [u8; 32],
    pub total_core_us: u64,
    /// Derived from byte_us for human readability (GB-seconds)
    pub total_memory_gb_s: u64,
    pub total_storage_io: u64,
    pub total_network_bytes: u64,
    /// 1 credit = 1,000,000 micro_credits
    pub price_micro_credits: u64,
    /// Buyer's ECDSA signature approving the charge
    pub buyer_signature: [u8; 64],
    /// Seller's ECDSA signature confirming receipt
    pub seller_signature: [u8; 64],
    /// Settlement timestamp (µs since epoch)
    pub settled_at_us: u64,
    /// Pending, Settled, or Disputed
    pub status: SettlementStatus,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettlementStatus {
    Pending,
    Settled,
    Disputed,
}

impl SettlementStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SettlementStatus::Pending => "pending",
            SettlementStatus::Settled => "settled",
            SettlementStatus::Disputed => "disputed",
        }
    }
}

impl WorkSettlement {
    pub fn compute_digest(&self) -> [u8; 32] {
        let mut buf = Vec::with_capacity(32 + 8 + 8 + 8 + 8 + 8 + 64 + 64 + 8 + 1);
        buf.extend_from_slice(&self.work_id);
        buf.extend_from_slice(&self.total_core_us.to_le_bytes());
        buf.extend_from_slice(&self.total_memory_gb_s.to_le_bytes());
        buf.extend_from_slice(&self.total_storage_io.to_le_bytes());
        buf.extend_from_slice(&self.total_network_bytes.to_le_bytes());
        buf.extend_from_slice(&self.price_micro_credits.to_le_bytes());
        buf.extend_from_slice(&self.buyer_signature);
        buf.extend_from_slice(&self.seller_signature);
        buf.extend_from_slice(&self.settled_at_us.to_le_bytes());
        buf.extend_from_slice(&[match self.status {
            SettlementStatus::Pending => 0,
            SettlementStatus::Settled => 1,
            SettlementStatus::Disputed => 2,
        }]);
        sha256(&buf).try_into().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workload_class_roundtrip() {
        assert_eq!(WorkloadClass::from_str("inference"), WorkloadClass::Inference);
        assert_eq!(WorkloadClass::Inference.as_str(), "inference");
    }

    #[test]
    fn work_priority_roundtrip() {
        assert_eq!(WorkPriority::from_str("batch"), WorkPriority::Batch);
        assert_eq!(WorkPriority::Expedited.as_str(), "expedited");
    }

    #[test]
    fn work_status_roundtrip() {
        assert_eq!(WorkStatus::from_str("preempted"), WorkStatus::Preempted);
    }

    #[test]
    fn performance_certificate_digest_is_deterministic() {
        let mut cert = PerformanceCertificate {
            node_id: [1u8; 64],
            cpu_int_score: 2_000_000,
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
            benchmark_started_us: 1_000_000_000_000_000,
            benchmark_completed_us: 1_000_000_001_000_000,
            digest: [0u8; 32],
            signature: [0u8; 64],
        };
        let d1 = cert.compute_digest();
        let d2 = cert.compute_digest();
        assert_eq!(d1, d2);
        cert.digest = d1;
        assert!(cert.verify());
    }

    #[test]
    fn cpu_core_multiplier_fast_cpu() {
        let cert = PerformanceCertificate {
            node_id: [1u8; 64],
            cpu_int_score: 5_000_000, // 5x reference
            cpu_crypto_score: 0,
            mem_bandwidth_mbps: 0,
            mem_latency_ns: 0,
            storage_random_iops: 0,
            storage_seq_mbps: 0,
            storage_event_iops: 0,
            storage_blob_ops: 0,
            storage_object_ops: 0,
            net_frame_encode_decode_ops: 0,
            net_frame_sign_verify_ops: 0,
            net_udp_throughput_ops: 0,
            net_router_lookup_ops: 0,
            gpu_score: None,
            npu_score: None,
            benchmark_started_us: 0,
            benchmark_completed_us: 0,
            digest: [0u8; 32],
            signature: [0u8; 64],
        };
        let m = cert.cpu_core_multiplier();
        assert_eq!(m.to_int(), 5);
        // 4 physical cores × 1_000_000 µs = 4_000_000 physical core-µs
        // Billable: 4_000_000 × 5 = 20_000_000 RC-µs
        assert_eq!(m.mul_u64(4_000_000), 20_000_000);
    }

    #[test]
    fn cpu_core_multiplier_slow_cpu() {
        let cert = PerformanceCertificate {
            node_id: [1u8; 64],
            cpu_int_score: 500_000, // 0.5x reference
            cpu_crypto_score: 0,
            mem_bandwidth_mbps: 0,
            mem_latency_ns: 0,
            storage_random_iops: 0,
            storage_seq_mbps: 0,
            storage_event_iops: 0,
            storage_blob_ops: 0,
            storage_object_ops: 0,
            net_frame_encode_decode_ops: 0,
            net_frame_sign_verify_ops: 0,
            net_udp_throughput_ops: 0,
            net_router_lookup_ops: 0,
            gpu_score: None,
            npu_score: None,
            benchmark_started_us: 0,
            benchmark_completed_us: 0,
            digest: [0u8; 32],
            signature: [0u8; 64],
        };
        let m = cert.cpu_core_multiplier();
        assert_eq!(m.to_int(), 0); // truncates
        assert_eq!(m.to_raw(), 32768); // 0.5 in 16.16
        assert_eq!(m.mul_u64(1_000_000), 500_000);
    }

    #[test]
    fn work_accounting_serialization_roundtrip() {
        let accounting = WorkAccounting {
            work_id: [0xAA; 32],
            requester_id: [0xBB; 64],
            provider_id: [0xCC; 64],
            delegation_hash: [0xDD; 32],
            started_at_us: 1_000_000_000_000_000,
            completed_at_us: 1_000_000_001_000_000,
            physical_core_us: 4_000_000,
            physical_memory_gb: 8,
            memory_duration_seconds: 1,
            gpu_core_us: 0,
            npu_core_us: 0,
            storage_read_bytes: 1048576,
            storage_written_bytes: 524288,
            storage_read_ops: 100,
            storage_write_ops: 50,
            network_sent_bytes: 2097152,
            network_received_bytes: 1048576,
            workload_class: WorkloadClass::Inference,
            priority: WorkPriority::Standard,
            status: WorkStatus::Completed,
            exit_code: Some(0),
            provider_cert_digest: [0xEE; 32],
            cpu_multiplier: FixedPoint16::from_ratio(5, 2), // 2.5x
            memory_multiplier: FixedPoint16::from_ratio(3, 2),
            storage_multiplier: FixedPoint16::from_ratio(2, 1),
            billable_compute_rc_us: 10_000_000, // 4M × 2.5
        };

        let bytes = accounting.to_bytes();
        let restored = WorkAccounting::from_bytes(&bytes).unwrap();

        assert_eq!(restored.work_id, accounting.work_id);
        assert_eq!(restored.physical_core_us, accounting.physical_core_us);
        assert_eq!(restored.billable_compute_rc_us, accounting.billable_compute_rc_us);
        assert_eq!(restored.workload_class, WorkloadClass::Inference);
        assert_eq!(restored.status, WorkStatus::Completed);
        assert_eq!(restored.cpu_multiplier.to_raw(), accounting.cpu_multiplier.to_raw());
    }

    #[test]
    fn compute_advertisement_serialization() {
        let adv = ComputeAdvertisement {
            node_id: [0x42; 64],
            available_cores: 8,
            available_memory_bytes: 32 * 1024 * 1024 * 1024,
            available_storage_bytes: 500 * 1024 * 1024 * 1024,
            gpu_count: 1,
            npu_count: 0,
            cert_digest: [0xAB; 32],
            price_per_million_rc_us: 1000, // 1000 micro-credits per M RC-µs
            min_contract_duration_us: 1_000_000, // 1 second
            timestamp_us: 1_700_000_000_000_000,
            signature: [0xCD; 64],
        };
        let bytes = adv.to_bytes();
        let restored = ComputeAdvertisement::from_bytes(&bytes).unwrap();
        assert_eq!(restored.available_cores, 8);
        assert_eq!(restored.price_per_million_rc_us, 1000);
    }

    #[test]
    fn total_helpers() {
        let acc = WorkAccounting {
            work_id: [0; 32],
            requester_id: [0; 64],
            provider_id: [0; 64],
            delegation_hash: [0; 32],
            started_at_us: 0,
            completed_at_us: 0,
            physical_core_us: 0,
            physical_memory_gb: 0,
            memory_duration_seconds: 0,
            gpu_core_us: 0,
            npu_core_us: 0,
            storage_read_bytes: 0,
            storage_written_bytes: 0,
            storage_read_ops: 100,
            storage_write_ops: 50,
            network_sent_bytes: 1000,
            network_received_bytes: 2000,
            workload_class: WorkloadClass::General,
            priority: WorkPriority::Standard,
            status: WorkStatus::Completed,
            exit_code: None,
            provider_cert_digest: [0; 32],
            cpu_multiplier: FixedPoint16::ONE,
            memory_multiplier: FixedPoint16::ONE,
            storage_multiplier: FixedPoint16::ONE,
            billable_compute_rc_us: 0,
        };
        assert_eq!(acc.total_storage_ops(), 150);
        assert_eq!(acc.total_network_bytes(), 3000);
    }

    #[test]
    fn memory_multiplier_combined() {
        let cert = PerformanceCertificate {
            node_id: [1u8; 64],
            cpu_int_score: 0,
            cpu_crypto_score: 0,
            mem_bandwidth_mbps: 20_000,  // 4x reference
            mem_latency_ns: 50,           // 2x reference (half the latency)
            storage_random_iops: 0,
            storage_seq_mbps: 0,
            storage_event_iops: 0,
            storage_blob_ops: 0,
            storage_object_ops: 0,
            net_frame_encode_decode_ops: 0,
            net_frame_sign_verify_ops: 0,
            net_udp_throughput_ops: 0,
            net_router_lookup_ops: 0,
            gpu_score: None,
            npu_score: None,
            benchmark_started_us: 0,
            benchmark_completed_us: 0,
            digest: [0u8; 32],
            signature: [0u8; 64],
        };
        let m = cert.memory_multiplier();
        // 4x * 2x = 8x
        assert_eq!(m.to_int(), 8);
    }

    #[test]
    fn settlement_status_roundtrip() {
        assert_eq!(SettlementStatus::Pending.as_str(), "pending");
        assert_eq!(SettlementStatus::Settled.as_str(), "settled");
        assert_eq!(SettlementStatus::Disputed.as_str(), "disputed");
    }
}
