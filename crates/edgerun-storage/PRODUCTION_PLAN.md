<!-- SPDX-License-Identifier: GPL-2.0-only -->
# Storage Engine Production Plan (Reality-Based)

Last updated: 2026-02-19

## 1. Current Architecture (Implemented)

Write path:

1. Event encode on worker thread
2. In-memory index updates
3. Segment append in memory
4. Centralized I/O through process-wide reactor thread (`io_reactor`)
5. Disk persistence via `io_uring`

I/O architecture:

- Single process-wide reactor (`OnceLock<Arc<IoReactor>>`)
- Dedicated I/O thread owns the ring
- Worker threads submit commands over channel
- Reactor batches SQEs and drains CQEs

Core capabilities currently implemented:

- Batched async read/write/fsync in reactor
- Linked durability chain (write + fsync)
- Linked checkpoint chain:
  - `WRITE segment -> FSYNC segment -> WRITE manifest -> FSYNC manifest`
- Registered file table (`register_files`, `register_files_update`)
- Registered fixed buffers (`register_buffers`)
- `WRITE_FIXED` / `READ_FIXED` when operation is eligible
- Ring setup flags:
  - `IORING_SETUP_CLAMP`
  - `IORING_SETUP_COOP_TASKRUN`
  - `IORING_SETUP_SINGLE_ISSUER`
  - optional `IORING_SETUP_SQPOLL`
- Segment encryption foundation (userspace):
  - chunk-level XChaCha20-Poly1305 AEAD container format
  - HKDF key hierarchy (`K_store -> K_seg -> K_chunk`)
  - transport-root verification without decryption
  - encrypted segment reader/writer APIs for at-rest confidentiality

## 2. Durability Semantics (Current)

`DurabilityLevel` behavior in async writer:

- `AckLocal` / `AckBuffered`
  - buffered writes, no fsync requirement
- `AckDurable`
  - linked write+fsync per flushed chunk
- `AckCheckpointed`
  - supported via explicit checkpoint flush API with 4-op linked chain
  - API: `AsyncSegmentWriter::flush_checkpointed(manifest_bytes)`
- `AckReplicatedN`
  - currently local durable write; distributed quorum enforcement not yet implemented

## 3. Performance Reality

Recent local benchmark (`tools/async_writer_benchmark.rs`):

- `end_to_end` producers=1: ~130-150 MB/s
- `end_to_end` producers=8: ~580-640 MB/s
- `io_only` producers=1: ~2.3-2.6 GB/s
- `io_only` producers=8: lower due to producer/queue coordination overhead in benchmark mode

Interpretation:

- Reactor and durability pipeline are working correctly
- Remaining gap is now mostly LSM lookup/index structure efficiency at higher scale

## 4. What Is NOT Done Yet

1. Full direct-I/O pipeline
- O_DIRECT is configurable and attempted, but not enforced end-to-end for every path
- Full 4KiB alignment discipline still partial

2. Continuous deep queueing on write path
- Many paths still wait on completion in caller flow
- This limits in-flight depth under some workloads

3. LSM compaction engine
- LSM framework exists, but compaction implementation is still incomplete

4. Distributed durability (`AckReplicatedN`)
- No quorum commit protocol yet

5. Crash campaign at production scale
- Harness exists; 10k randomized crash pass is not yet complete

6. Encryption rollout depth
- Core encrypted segment format exists, but not yet fully wired through:
  - async writer + replication payload paths
  - key-management backends (TPM/keyring/Android Keystore)
  - key epoch rotation policies and tooling
 - Key-management foundation now includes pluggable providers:
   - env key provider
   - passphrase Argon2id provider
   - wrapped-file provider
   - Linux keyring adapter (keyctl-based)
   - Android Keystore adapter stub for later platform wiring

## 5. Immediate Roadmap (Ordered)

## Phase A: Throughput and Latency Stabilization (Completed)

Completed:

1. Async writer queue/reap path with sustained in-flight batching
2. Reactor-side batching + registered files + fixed-buffer usage
3. Segment preallocation and automatic rollover
4. Async compaction scheduling + telemetry counters
5. Restart-robust SST metadata (`entry_count` reconstruction)
6. Enqueue failure signaling for ticket-based reactor APIs
7. Perf gate script: `tools/perf_gate.sh`

## Phase B: True checkpoint API integration

Completed:

1. Wired `AckCheckpointed` in engine-level append session API
2. Manifest attach/write path is now driven from `ManifestManager` prepare/commit flow
3. Added restart/crash-boundary tests around linked checkpoint chains

Success target:

- deterministic checkpoint durability semantics across restart

## Phase C: LSM lookup/compaction optimization

Completed:

1. Persisted sparse block index in SST metadata (v2 header)
2. Block-local binary-search lookup path with bloom prefilter
3. Level interval pruning + overlap-aware fanout reduction for point lookups
4. Added mixed read/write/compaction benchmark tool:
   - `tools/mixed_rw_compaction_benchmark.rs`
   - example: `cargo run -q --bin mixed_rw_compaction_benchmark -- --duration 8 --writers 2 --readers 4 --write-batch 512 --read-batch 2048`
   - recent baseline (2026-02-20): ~620k writes/s, ~39k reads/s, compaction scheduled=7 completed=6 failed=0

Success target:

- stable lookup throughput at 1M+ keys while compaction runs

## Phase D: Distributed durability

Completed:

1. Defined replication ACK contract for `AckReplicatedN` via `QuorumTracker`
2. Implemented quorum tracking + timeout/error semantics
3. Added unit/integration tests for quorum satisfied/not-met/invalid cases and engine append wiring
4. Wired real network ACK collection path (TCP ACK request/response) for `AckReplicatedN`
5. Added multi-node durability/consistency integration harness (healthy/partitioned/slow replicas + op-id consistency checks)
6. Added authenticated peer identity in ACK transport (signed request/response frames bound to peer `store_uuid`)
7. Added stream transport semantics for ACK collection (connection reuse, retries/backoff, idempotency-window dedupe)
8. Added multi-op framed ACK batch protocol (legacy + authenticated v2) over long-lived pooled channels
9. Added engine-level append group-commit API to replicate many ops in one durable flush + one batched ACK sweep
10. Added replication group-commit benchmark tool (`tools/replication_group_commit_benchmark.rs`) for single-vs-batch ACK path measurement
11. Added stream-chunking producer-loop API (`EngineAppendSession::append_stream_with_durability`) so replicated workloads can use group commit without manual batch slicing
12. Replication benchmark harness producer loops now route through stream-chunking API in both single and batch modes (batch-size 1 vs N)
13. Added session-level replication batching config (`set_replication_batch_size`) and config-driven stream helper (`append_replicated_stream`)

Success target:

- correct N-peer durability behavior under partition/failure tests

## 6. Quality Gates

Required before calling this production-ready:

1. `RUSTFLAGS='-D warnings' cargo check --all-targets` clean
2. Full test suite clean
3. Perf gate clean: `tools/perf_gate.sh`
   - now includes mixed RW sweep gate (configurable via env):
     - `MIXED_RW_SWEEP_DURATION` (default `4`)
     - `MIXED_RW_SWEEP_MAX_CASES` (default `4`)
     - sweep thresholds:
       - `MIN_TOP_SCORE` (default `700000`)
       - `MIN_TOP_WRITES_OPS` (default `250000`)
       - `MIN_TOP_READS_OPS` (default `80000`)
       - `MAX_TOP_COMP_FAILED` (default `0`)
4. 10k crash-injection campaign with no corruption/divergence
   - automation runner available: `cargo run -q --bin crash_campaign -- --iterations 10000 --data-dir /tmp/crash_campaign_10k --target-mb 2 --random true --keep-failed true`
   - latest run (2026-02-20): passed `10000/10000`, report `/tmp/crash_campaign_10k/campaign_report.json`
5. Documented SLOs achieved on target hardware
   - tuning sweep runner available: `tools/mixed_rw_tuning_sweep.sh`
   - example: `tools/mixed_rw_tuning_sweep.sh --duration 8`
   - latest sweep (2026-02-20): `/tmp/mixed_rw_tuning_sweep_20260220_120702/summary.md`
     - top score profile: writers=4, readers=10, write_batch=1024, read_batch=4096

## 7. Historical Notes

This document replaces older planning assumptions that were no longer accurate (notably the prior claim that io_uring execution was only a sync shim). The current codebase now runs real `io_uring` submission/completion in a centralized reactor.
