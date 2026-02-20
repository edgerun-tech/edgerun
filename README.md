# EdgeRun Master TODO (Whitepaper Convergence)

This file is the single source of truth for project convergence to `Whitepaper.md` (Phase 1 spec).
It replaces prior root README/roadmap aggregation for day-to-day tracking.

Status snapshot (2026-02-20):

- Overall convergence to whitepaper target: ~72%
- Runtime determinism/security track: ~85%
- Scheduler/worker policy + attestability track: ~70%
- On-chain settlement track: ~68%
- Storage/service hardening track: ~58%

## A. Whitepaper alignment checklist

Legend: `[x] done`, `[~] partial`, `[ ] open`

### A1. Core protocol constraints (`Whitepaper.md` sections 1-3)

- [x] BLAKE3-256 hashing and Ed25519 signing primitives in shared crates.
- [x] Runtime ID + ABI policy enforcement in runtime/worker/scheduler paths.
- [~] Bundle schema alignment.
- [ ] Add optional bundle `meta` fields (`content_type`, `note`) in canonical schema.
- [ ] Enforce whitepaper I/O rule: `write_output` exactly once.
- [ ] Reconcile/lock hostcall signature contract vs whitepaper text (`read_input` shape).

### A2. Deterministic runtime (`Whitepaper.md` section 2)

- [x] No-float opcode validation.
- [x] Import allowlist/signature checks.
- [x] Strict export policy (`_start`, `memory`) + memory constraints.
- [x] Fuel/memory limit enforcement and deterministic error taxonomy.
- [x] Replay corpus tooling + cross-host determinism matrix in CI.
- [x] Output hash verification mode for replay.
- [x] Hash-only execution mode for worker path (no output byte buffering in worker loop).
- [~] Streaming output policy complete for all runtime surfaces.
- [ ] Remove buffered-only behavior from runtime CLI path (or formalize as accepted exception).

### A3. Scheduler + worker off-chain network (`Whitepaper.md` sections 4, 6, 7)

- [x] Scheduler endpoints for heartbeat, assignments, result/failure/replay ingestion.
- [x] Worker heartbeats and assignment polling.
- [x] Signed assignment policy verification (runtime_id/abi/lifecycle window/key tuple).
- [x] ABI rollover assignment integration test (`N` and `N-1`).
- [~] Committee and quorum flow at scheduler level.
- [ ] Implement explicit committee assignment API/logic (not only direct single-worker assignment on create).
- [ ] Implement scheduler-side quorum tracking and finalize trigger flow from observed worker results.
- [ ] Add scheduler chain watcher loop for on-chain job/result/finalization observability.
- [ ] Add worker capability admission logic (`runtime_ids`, capacity) to assignment decisions.

### A4. On-chain Solana program (`Whitepaper.md` section 5)

- [x] Program instruction surface exists for config/stake/job/result/finalize/cancel/slash.
- [x] Program tests include submit-result signature pre-instruction checks and finalize/cancel/slash paths.
- [~] Exact whitepaper economics and authority modes frozen in code/docs.
- [ ] Confirm and lock all whitepaper constants/policies in one program-facing policy doc:
- [ ] lock formula, payout split, slash distribution, cancellation/refund policy.
- [ ] Decide whether to keep scheduler-authority finalize-only in MVP or move to permissionless finalize path.
- [ ] Add explicit contradictory-result slashing integration coverage against scheduler-triggered flow.

### A5. Storage service and bundle serving (`Whitepaper.md` sections 3-4)

- [x] Content-addressed bundle storage path present in scheduler scaffold (`/bundle/{bundle_hash}`).
- [~] Storage roadmap execution in `crates/edgerun-storage` is advanced but incomplete.
- [ ] Complete storage roadmap Phase 2 model closure (ACK quorum vs consensus model clarity).
- [ ] TLS/mTLS transport plan + implementation path for replication links.
- [ ] Compatibility matrix and migration/upgrade policy completion.
- [ ] Service operability endpoints + runtime/admin surface completion.

### A6. Security, supply chain, and provenance (`Whitepaper.md` section 8 + deployment checklist)

- [x] Runtime fuzz sanity CI.
- [x] Runtime vulnerability audit gate + triage process.
- [x] Runtime SBOM generation in CI.
- [x] Runtime release provenance workflow with artifact attestation.
- [x] External review packet + findings ledger + CI validation scaffold.
- [x] UB-focused runtime checks (Miri/ASan/UBSan) in CI.
- [ ] External third-party audit execution and final sign-off.
- [ ] Resolve temporary advisory ignore list via upstream dependency upgrades.
- [ ] Supply-chain pinning policy document (toolchains/images/dependency update policy).

### A7. Testing and release readiness (`Whitepaper.md` section 10 + deployment checklist)

- [x] Strict workspace CI gates (`fmt`, `check`, strict `clippy`, tests).
- [x] Integration suites (scheduler API, e2e lifecycle, policy rotation, ABI rollover).
- [x] Workspace coverage reporting (`llvm-cov`) in CI.
- [~] Whitepaper-mandated deterministic multi-machine evidence depth.
- [ ] Expand deterministic golden corpus breadth and edge-case vectors.
- [ ] Add whitepaper-aligned 10-machine determinism campaign artifact process.
- [ ] Add explicit release bundle checklist artifact collation (tests, fuzz, replay, coverage, provenance).

## B. Immediate execution queue (ordered)

- [ ] B1: enforce `write_output` exactly once in runtime and add regression tests.
- [ ] B2: add bundle `meta` fields with canonical encoding/validation and compatibility tests.
- [ ] B3: implement scheduler committee/quorum orchestration loop aligned to whitepaper finalization flow.
- [ ] B4: finalize on-chain economics/authority policy doc and align program + scheduler defaults.
- [ ] B5: expand deterministic golden corpus and publish repeatable multi-host conformance artifact process.
- [ ] B6: complete external audit sign-off cycle using `SECURITY_FINDINGS.json` completion criteria.

## C. Source mapping

This TODO is consolidated from:

- `Whitepaper.md`
- `crates/edgerun-runtime/PRODUCTION_READINESS.md`
- `crates/edgerun-runtime/SECURITY_REVIEW.md`
- `crates/edgerun-storage/ROADMAP.md`
- `crates/edgerun-storage/PRODUCTION_PLAN.md`
- `program/README.md`
