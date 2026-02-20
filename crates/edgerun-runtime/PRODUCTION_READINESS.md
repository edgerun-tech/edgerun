# EdgeRun Runtime Production Readiness

Last updated: 2026-02-20

## Scope

This document defines the production bar for `crates/edgerun-runtime`:

- Deterministic execution of constrained Wasm bundles
- Stable, versioned host ABI
- Enforced resource limits and failure semantics
- Reproducible, auditable outputs (`output_hash`)

## Current status

Implemented:

- Canonical bundle decode + bundle hash invariant (hash raw canonical bytes first)
- Structural validation:
  - no float ops
  - only `edgerun::{input_len,read_input,write_output}` imports
  - exact hostcall signatures
  - required exports `_start` and `memory`
  - export allowlist (no extra exports)
  - memory export must be 32-bit and have explicit max pages
- Wasm execution via `wasmi`
- Fuel metering mapped to `max_instructions`
- Store memory limiter mapped to `max_memory_bytes`
- Runtime tests for determinism, instruction cap, and memory/output limits
- Stable runtime error taxonomy at strict API boundary:
  - `RuntimeErrorCode`
  - structured `RuntimeError { code, message, trap_code, fuel_limit, fuel_remaining }`
- Execution telemetry in `ExecutionReport`:
  - `abi_version`
  - `runtime_id`
  - `fuel_limit`
  - `fuel_remaining`
- Replay artifact contract documented: `REPLAY_ARTIFACT_CONTRACT.md`
- CI fuzz sanity job for decoder/validator/hostcall targets (bounded-time regression runs)
- CI dependency vulnerability audit (`cargo audit`)
- CI runtime SBOM artifact generation (`cargo cyclonedx`)
- Runtime security review and hardening backlog tracked in `SECURITY_REVIEW.md`
- Runtime ABI compatibility policy documented in `ABI_COMPATIBILITY_POLICY.md` (`N` + `N-1` window)
- Scheduler/worker ABI rollover integration test (`scripts/integration_abi_rollover.sh`)
- Signed runtime release provenance workflow (`.github/workflows/runtime-provenance.yml`)
- Hash-only runtime execution mode for worker path (no output byte buffering in worker execution loop)
- External security review packet + findings ledger validation pipeline
- UB-focused CI checks for runtime crate (`miri`, `asan`, `ubsan`)

Not yet production-ready:

- Comprehensive deterministic conformance suite across platforms
- Deep fuzzing/security hardening for hostcall boundary behavior
- Calibration model for `max_instructions` cost predictability
- Strong observability/replay tooling for disputes and incident response
- Streaming output policy for all runtime surfaces (`run` CLI still returns buffered output bytes)
- Security posture artifacts (supply-chain pinning policy)
- External security review final sign-off

## Production gates

### Gate 1: Determinism

Requirements:

- Fixed interpreter and config per runtime release
- Bit-stable `output_hash` for the same bundle across supported environments
- Deterministic error classing (same input => same failure class)

Acceptance:

- Cross-host replay corpus passes (Linux kernels/CPUs)
- Golden vector suite committed for all core hostcall scenarios

### Gate 2: Resource safety

Requirements:

- Hard execution fuel cap with clear out-of-fuel failure
- Hard memory cap (instantiate + grow + hostcall buffer writes)
- Safe pointer arithmetic and bounds checks for hostcalls

Acceptance:

- Adversarial tests: OOB reads/writes, negative/overflow offsets, giant lengths
- Fuzz harness coverage for hostcall argument combinations

### Gate 3: ABI stability

Requirements:

- Versioned runtime ABI document
- Signature checks enforced in validator
- Policy for additive/deprecated hostcalls

Acceptance:

- Compatibility tests across N-1 runtime versions
- Explicit ABI changelog with migration policy

### Gate 4: Security + robustness

Requirements:

- Fuzzing for parser/validator/executor surfaces
- Panic-free error handling under malformed bundles
- Clear trap mapping to protocol-level error codes

Acceptance:

- Continuous fuzz jobs and corpus regression in CI (or scheduled manual runs until CI exists)
- No known untriaged high-severity runtime issues

### Gate 5: Operability

Requirements:

- Structured runtime report (fuel used, trap code, memory pages, runtime version)
- Replay command for postmortems/disputes
- Performance SLOs and regression thresholds

Acceptance:

- On-call runbook with replay steps
- Perf baselines and alert thresholds tracked

## Manual cadence (pre-CI)

- Weekly fuzz run:
  - `scripts/weekly_fuzz.sh`
  - crash triage template: `scripts/fuzz_triage_template.md`
- Replay corpus run:
  - `scripts/run_replay_corpus.sh`
  - compares debug/release artifacts locally and fails on mismatch
- Cross-machine replay check:
  - run `edgerun-runtime replay-corpus` on each target host profile
  - compare artifacts with `scripts/compare_replay_profiles.sh`

## Implementation plan

### Phase A (in progress)

- [x] Exact import ABI signature validation
- [x] Strict export/memory policy validation
- [x] Fuel-based instruction limit enforcement
- [x] Runtime memory limiter

### Phase B (next)

- [x] Add explicit trap/error taxonomy at runtime API boundary
- [x] Add runtime_id compatibility checks (bundle vs expected worker assignment runtime_id)
- [x] Add ABI version field into execution report and worker/runtime policy checks
- [x] Add hostcall boundary tests for zero-length/partial/boundary/invalid pointer cases
- [x] Add fuzz targets for decoder/validator/hostcall boundaries with seed corpus
- [x] Add typed replay artifact CLI (`edgerun-runtime replay`) for disputes

### Phase C

- [x] Cross-machine determinism CI matrix
- [x] Replay tool (`edgerun-runtime replay --bundle ... --expect-output-hash ...`)
- [x] Instruction/fuel calibration framework

### Phase D

- [x] Security review + hardening backlog closure
- [x] SLO and performance regression gates in CI

## Immediate next implementation tasks

1. Add boundary tests for hostcalls:
   - zero-length reads/writes
   - partial reads near end-of-input
   - maximum legal write at memory boundary
   - overflow/negative pointer handling
2. Add compatibility tests across rolling runtime upgrades:
   - verify worker/scheduler behavior during policy key and ABI window transitions
3. Add deterministic replay surfaces:
   - CLI/API mode returning stable JSON execution artifact for disputes
   - include error metadata (`code`, `trap_code`, fuel) on failure paths
4. Add CI security + quality gates:
   - fuzz targets for decoder/validator/hostcalls
   - Miri/ASan/UBSan job for runtime crate
   - cross-host determinism matrix and golden corpus regression
