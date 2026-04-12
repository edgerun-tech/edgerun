# Edgerun Compute Marketplace — Design Document

## Overview

Edgerun nodes sell spare compute capacity on a peer-to-peer mesh. When a buyer sends an `ExecuteWorkload` command, the seller pulls an OCI container image, runs it with resource limits, and bills in **Reference Core-Microseconds (RC-µs)** — a benchmark-adjusted unit that ensures heterogeneous hardware is priced fairly.

```
Buyer Node                    Seller Node
────────                      ───────────
  │                              │
  │── ExecuteWorkload ──────────>│  ← Command signed, delegation chain verified
  │                              │
  │                              │  1. Start WorkMeter (timing begins)
  │                              │  2. Pull OCI image from registry
  │                              │  3. Apply resource limits (cgroups v2)
  │                              │  4. Run container (namespaces + pivot_root)
  │                              │  5. Finalize meter → WorkAccounting
  │                              │  6. Append to accounting ledger
  │                              │
  │<── work completed ──────────│  ← Response includes billable RC-µs
```

---

## The Unit: Reference Core-Microseconds (RC-µs)

### The Problem

A CPU core on a Raspberry Pi 4 is not equal to a core on an AMD EPYC 9654. If both charge "per core-second," buyers would overpay for slow hardware and sellers of fast hardware would be undercompensated. Nodes would have incentive to lie about specs.

### The Solution: Benchmark-Derived Multipliers

Each node runs a deterministic benchmark suite at init, producing a signed **PerformanceCertificate**:

| Benchmark | Measures | Reference Baseline |
|-----------|----------|-------------------|
| CPU Integer | ALU throughput (hash + sieve) | 1,000,000 ops/s |
| CPU Crypto | SHA-256 throughput | 500,000 hashes/s |
| Memory BW | Sequential read bandwidth | 5,000 MB/s |
| Memory Latency | Random pointer-chasing | 100 ns |
| Storage IOPS | 4K random read+write | 3,000 ops/s |
| Storage Seq | 16 MB sequential R+W | 50 MB/s |

The **cpu_core_multiplier** = `cpu_int_score / REFERENCE_CPU_INT_SCORE`.

**Billable RC-µs = physical_core_us × cpu_core_multiplier**

| Hardware | Score | Multiplier | 1M physical µs → billable |
|----------|-------|------------|---------------------------|
| Raspberry Pi 4 | 500,000 | 0.5x | 500,000 RC-µs |
| Reference (t3.micro) | 1,000,000 | 1.0x | 1,000,000 RC-µs |
| Modern desktop CPU | 5,000,000 | 5.0x | 5,000,000 RC-µs |
| AMD EPYC 9654 | 10,000,000 | 10.0x | 10,000,000 RC-µs |

**Why this prevents lying:**
- If a node claims a faster CPU but the benchmark says otherwise, the certificate's signature won't match — peers reject it
- If a node has a faster CPU and benchmarks honestly, it earns MORE per physical µs → correct incentive
- The benchmark is deterministic — any peer can re-run it and verify

---

## Architecture

### Crate Dependency Graph

```
edgerun-node (edgerund)                    ← The daemon
├── edgerun-core                           ← Protocol + accounting types
│   ├── accounting.rs                      ← WorkAccounting, PerformanceCertificate,
│   │                                      ← ComputeAdvertisement, WorkSettlement
│   ├── benchmark.rs                       ← 6 benchmarks → PerformanceCertificate
│   ├── fixed_point.rs                     ← 16.16 fixed-point math (no deps)
│   └── crypto.rs                          ← SHA-256 (inline, no deps)
├── edgerun-oci-registry                   ← Docker Hub / OCI registry client
│   └── (uses edgerun-core::crypto::sha256)← No vendored sha2
├── edgerun-oci-runtime                    ← Container runtime (namespaces, cgroups)
├── edgerun-storage                        ← Append-only event log + accounting ledger
│   └── file_index.rs                      ← work_accounting.bin persistence
└── metering (module in edgerun-node/src/metering.rs)
    ← WorkMeter runtime resource tracker
    ← NOT a standalone crate — lives as a module inside edgerun-node
```

### Data Flow: ExecuteWorkload Command

```
CommandEnvelope arrives
    │
    ▼
command_dispatch.rs: dispatch_execute_workload()
    │
    ├── Parse workload spec: "alpine:latest:cores=4:memory=8G"
    │   └── → image="alpine:latest", cores=4, memory=8GB
    │
    ├── Load PerformanceCertificate (from disk cache)
    │   └── If missing → run_full_benchmark() → cache to perf_cert.bin
    │
    ├── Create WorkMeter(start_time, allocated_resources, cert_multipliers)
    │
    ├── PHASE 1: Pull OCI Image
    │   ├── RegistryClient::resolve_manifest() → get layer sizes
    │   ├── meter.add_network_received(total_layer_bytes)
    │   ├── RegistryClient::pull() → download + extract layers
    │   ├── meter.add_storage_write(rootfs_size)
    │   └── meter.add_storage_read(cached_blobs)
    │
    ├── PHASE 2: Apply Resource Limits
    │   ├── Parse config.json → OciSpec
    │   ├── Set cgroup memory.limit = 8GB
    │   ├── Set cgroup cpu.shares = 4096 (4 × 1024)
    │   └── Set cgroup pids.max = 256
    │
    ├── PHASE 3: Run Container
    │   ├── edgerun_oci_runtime::run_bundle()
    │   │   ├── fork() + unshare(namespaces)
    │   │   ├── pivot_root(rootfs)
    │   │   ├── mount proc, sysfs, devpts
    │   │   ├── drop privileges (setuid/setgid)
    │   │   ├── execve(entrypoint)
    │   │   └── parent: setup cgroups v2, wait()
    │   └── → ExitStatus
    │
    ├── Finalize: meter.finalize(status, exit_code)
    │   └── → WorkAccounting {
    │         physical_core_us: cores × elapsed,
    │         billable_compute_rc_us: physical × cpu_multiplier,
    │         storage_read_bytes, network_received_bytes, ...
    │       }
    │
    ├── Persist: store.record_work_accounting(&accounting)
    │   └── → work_accounting.bin (append-only)
    │
    └── Cleanup: remove temp bundle directory
```

---

## Storage: Work Accounting Ledger

### File Format: `work_accounting.bin`

Append-only binary log. Each record:

```
[data_len: u64][WorkAccounting bytes...][record_hash: str][requester_hex: str]
[provider_hex: str][workload_class: str][status: str][started_at_us: u64]
[billable_rc_us: u64]
```

### Query Functions (all O(n) scan, n is small)

| Function | Purpose |
|----------|---------|
| `total_billable_for_requester(id)` | Total RC-µs a buyer has consumed |
| `total_billable_for_provider(id)` | Total RC-µs a seller has delivered |
| `work_in_time_range(from, to)` | All work in a time window |
| `work_by_class("inference")` | Filter by workload type |
| `work_by_status("completed")` | Filter by outcome |
| `list_all_work()` | Full audit log |

### WorkAccounting Serialized Fields

```rust
work_id: [u8; 32]           // SHA-256 of workload spec
requester_id: [u8; 64]      // Buyer's ECDSA pubkey
provider_id: [u8; 64]       // Seller's ECDSA pubkey
delegation_hash: [u8; 32]   // Authorization chain link
started_at_us: u64          // Unix timestamp µs
completed_at_us: u64
physical_core_us: u64       // cores × duration
physical_memory_gb: u32     // Allocated memory
memory_duration_seconds: u32
gpu_core_us: u64            // GPU compute (if applicable)
npu_core_us: u64            // NPU compute (if applicable)
storage_read_bytes: u64
storage_written_bytes: u64
storage_read_ops: u32
storage_write_ops: u32
network_sent_bytes: u64
network_received_bytes: u64
workload_class: enum        // Container, Inference, Compilation, ...
priority: enum              // Batch, Standard, Expedited
status: enum                // Completed, Failed, Terminated, Preempted
exit_code: Option<i32>
provider_cert_digest: [u8; 32]  // Which cert was used
cpu_multiplier: FixedPoint16    // At time of execution
memory_multiplier: FixedPoint16
storage_multiplier: FixedPoint16
billable_compute_rc_us: u64     // THE BILLABLE AMOUNT
```

---

## The Three Phases of Work

### Phase 1: Pull (Network I/O)

The OCI registry client downloads layers. The meter records:
- **Network received** = total layer sizes from manifest
- **Storage written** = extracted rootfs size
- **Storage read** = cached blob files

### Phase 2: Resource Limits (Cgroups v2)

Before running, the OCI config is modified to enforce the buyer's resource allocation:

```
cgroup/memory.max    = allocated_memory_bytes  (e.g., 8GB)
cgroup/cpu.weight    = shares_to_weight(cores × 1024)
cgroup/pids.max      = 256
```

This ensures the container cannot consume more resources than billed.

### Phase 3: Run (CPU Time)

The OCI runtime forks, unshares namespaces, pivot_roots, drops privileges, and execs the container. The WorkMeter measures wall-clock time from meter creation to finalization.

```
physical_core_us = allocated_cores × elapsed_microseconds
billable_rc_us   = physical_core_us × cpu_multiplier
```

---

## Billing Flow

1. **Seller publishes** a `ComputeAdvertisement` on the mesh:
   ```
   available_cores: 8
   price_per_million_rc_us: 1000 micro-credits
   cert_digest: <hash of PerformanceCertificate>
   ```

2. **Buyer verifies** the certificate:
   - Fetches cert from seller's event stream
   - Verifies ECDSA signature
   - Calculates effective cores = advertised × multiplier

3. **Buyer sends** `ExecuteWorkload` command with workload spec

4. **Seller executes** and returns the billable RC-µs count

5. **Both sign** a `WorkSettlement`:
   ```
   work_id, total_core_us, billable_rc_us
   price_micro_credits
   buyer_signature, seller_signature
   ```

6. **Settlement is stored** in both nodes' ledgers as cryptographic proof of agreement.

---

## External Dependencies: Zero

Everything uses only Rust stdlib or existing project code:

| Component | Dependency | Source |
|-----------|-----------|--------|
| SHA-256 | `edgerun_core::crypto::sha256` | Inline FIPS 180-4 implementation |
| Fixed-point math | `FixedPoint16` | 16.16 integer math, no floats |
| OCI pull | `ureq`, `serde_json`, `flate2`, `tar`, `zstd` | crates.io |
| OCI runtime | Raw syscalls (`unshare`, `pivot_root`, `mknod`) | `extern "C"` declarations |
| Cgroups v2 | File I/O to `/sys/fs/cgroup/` | std::fs |
| Accounting storage | Binary file append | std::fs |

No tokio, no async runtime, no databases, no external crypto libraries.

---

## Security Hardening

### User Namespace Isolation

Container root (UID 0) maps to an unprivileged host user (UID 65534, `nobody`).
Even if the container escapes the rootfs, it has no host privileges.

```
write_uid_map(0, 65534, 1)   → container root → host nobody
write_gid_map(0, 65534, 1)   → container root GID → host nogroup
```

Implemented in `edgerun-oci-runtime/src/lib.rs` — writes `/proc/self/uid_map`
and `/proc/self/gid_map` in the child's `pre_exec` closure before `pivot_root`.

### Seccomp-BPF Syscall Filter

Containers are restricted to ~75 essential syscalls (read, write, socket,
execve, exit, futex, etc.). Everything else returns EPERM.

```rust
// BPF program: arch check → syscall check → ALLOW or ERRNO(EPERM)
// Built as sock_filter instructions, applied via seccomp() syscall.
fn seccomp_bpf_prog() -> Vec<u8> { ... }
```

Applied after `prctl(PR_SET_NO_NEW_PRIVS, 1)` in the child's `pre_exec`.
Fails open gracefully if the kernel doesn't support seccomp.

### Benchmark at Node Init

Benchmarks run during `edgerund init`, not on first work. The PerformanceCertificate
is cached as `perf_cert.bin` in the config directory. Node startup shows:

```
Running performance benchmarks...
  CPU:      2.50x reference
  Mem BW:   45000 MB/s
  Mem Lat:  45 ns
  Stor IOPS: 150000
  Cert:     /path/to/node.yaml/perf_cert.bin
```

### Real-Time I/O Metering

`RegistryClient` tracks actual bytes downloaded during layer pulls:

```rust
// In download_blob():
let n = io::copy(&mut reader, &mut file)?;
self.bytes_downloaded += n;  // ← real bytes, not manifest estimate
```

After pull, `client.bytes_downloaded()` reports the true count. This replaces
the manifest-size estimation — billing is based on actual network I/O.

---
