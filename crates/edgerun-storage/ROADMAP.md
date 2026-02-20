<!-- SPDX-License-Identifier: GPL-2.0-only -->
# Storage Engine Roadmap (2026)

License direction: `GPL-2.0-only` (kept as-is).

This roadmap replaces the prior phase-only list with a productionization plan.

## Current baseline

- Async `io_uring` write/read/fsync reactor is implemented.
- Checkpoint durability chain is implemented.
- Replication ACK transport (single + batched, authenticated) is implemented.
- Crash harness exists and 10k campaign passed (`10000/10000`).
- Perf gates and mixed RW sweep tooling exist.

## Phase 1: Consistency and Product Narrative (1-2 weeks)

Goal: ensure docs and guarantees match real behavior.

Deliverables:

1. Align `PRODUCTION_PLAN.md`, `README.md`, and crate docs so "implemented vs not implemented" is consistent.
2. Publish explicit guarantee table:
   - durability semantics
   - replication semantics
   - crash/recovery guarantees
3. Publish non-goals and known limitations.

Exit criteria:

- No stale contradictions across roadmap/plan/docs.
- Architecture and guarantees are externally understandable without code-reading.

## Phase 2: Replication Correctness Upgrade (2-4 weeks)

Goal: move from ACK-transport durability to stronger commit semantics.

Deliverables:

1. Define target model:
   - either "durable quorum ACK store" (current direction)
   - or full consensus log protocol (Raft/Paxos-class)
2. If consensus path selected:
   - leader election
   - term/indexed log replication
   - commit index + replay safety
3. Fault-injection tests for partition/rejoin and stale leader behavior.

Exit criteria:

- Formalized safety model with tests mapping directly to model claims.
- No ambiguity between transport ACK and consensus commit guarantees.

## Phase 3: Security Hardening (2-3 weeks)

Goal: production-safe posture for multi-node deployments.

Deliverables:

1. Replication transport security:
   - TLS/mTLS support
   - key rotation procedure
   - peer identity policy docs
2. At-rest encryption strategy (or explicit deferral with rationale).
3. CI security gates:
   - `cargo audit` already present
   - add advisory triage policy and fail/allowlist process.

Exit criteria:

- Encrypted in-transit replication path available.
- Security runbook documented for key management and incident response.

## Phase 4: Operability and Runtime Surface (2-4 weeks)

Goal: usable as a service, not just a library/tool bundle.

Deliverables:

1. Production runtime binary:
   - config file schema + validation
   - startup/health/readiness endpoints
   - structured logging + metrics endpoint
2. Admin operations:
   - checkpoint trigger/status
   - compaction status
   - replication peer status
3. SLO dashboard schema and alert thresholds.

Exit criteria:

- Can run as a managed service with observable health and failure modes.

## Phase 5: Compatibility and Upgrade Policy (1-2 weeks)

Goal: safe long-term operation across versions.

Deliverables:

1. On-disk format versioning policy:
   - manifest
   - segment
   - SST metadata
2. Backward/forward compatibility matrix.
3. Migration/upgrade tool or documented online/offline migration path.

Exit criteria:

- Version upgrade path is explicit, tested, and documented.

## Phase 6: Performance Qualification and Release Readiness (ongoing)

Goal: reproducible release gates on target hardware.

Deliverables:

1. Hardware-matrix benchmark profiles (NVMe classes, core counts).
2. `tools/perf_gate.sh` thresholds calibrated per target profile.
3. Release artifact bundle:
   - crash campaign report
   - perf sweep report
   - test/check logs

Exit criteria:

- Release can be justified by auditable gate artifacts.

## Immediate next 3 actions

1. Complete Phase 1 doc consistency pass in `PRODUCTION_PLAN.md`.
2. Decide and document replication target model (ACK quorum vs consensus log).
3. Add TLS/mTLS replication transport design + implementation plan.
