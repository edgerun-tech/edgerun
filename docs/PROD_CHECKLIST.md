# EdgeRun Production Checklist

Last reviewed: 2026-02-20
Reviewer: Codex

## Release Blockers (Must Pass)

- [x] `Program attestation verification enforced`
  - Gate: on-chain `submit_result` must verify Ed25519 attestation path.
  - Evidence: enforced in `program/programs/edgerun_program/src/lib.rs` via `verify_worker_attestation`.

- [x] `Program integration tests cover real lifecycle`
  - Gate: tests must validate config/stake/job/assign/submit/finalize/cancel/slash behavior.
  - Evidence: `program/tests/edgerun.ts` now covers initialize/register/deposit/post/assign/submit/finalize/cancel/slash flows, including attestation pass/fail paths.

- [x] `Strict lint gate clean`
  - Gate: `cargo clippy --workspace --all-targets -- -D warnings` must pass.
  - Current: PASS (verified 2026-02-20).

- [ ] `Runtime production gates Phase C/D complete`
  - Gate: determinism CI matrix, replay expectation mode, fuel calibration, security hardening closure, perf/SLO CI gates.
  - Evidence: open items in `crates/edgerun-runtime/PRODUCTION_READINESS.md:150` and `crates/edgerun-runtime/PRODUCTION_READINESS.md:156`.

- [ ] `Scheduler chain dependency policy finalized`
  - Gate: no placeholder tx path in production mode, or explicit documented fail-open policy with safeguards.
  - Evidence: fallback placeholder behavior at `crates/edgerun-scheduler/src/main.rs:242` and `crates/edgerun-scheduler/src/main.rs:524`.

- [ ] `CI covers all production components`
  - Gate: CI must include storage tests/lints and program test pipeline (or equivalent separate required workflow).
  - Evidence: current CI scope in `.github/workflows/ci.yml` runs runtime/worker tests + integration only.

## Currently Passing (Good Signals)

- [x] `Workspace compiles`
  - Command: `cargo check --workspace`

- [x] `Formatting check`
  - Command: `cargo fmt --all --check`

- [x] `Workspace tests`
  - Command: `cargo test --workspace`

- [x] `Scheduler/worker integration scripts`
  - Commands:
    - `./scripts/integration_scheduler_api.sh`
    - `./scripts/integration_e2e_lifecycle.sh`
    - `./scripts/integration_policy_rotation.sh`

- [x] `Program local harness runs`
  - Command: `./program/scripts/test-bun-local`
  - Note: currently smoke-level only.

## Pre-Prod Exit Criteria

- [ ] All "Release Blockers" above are checked.
- [ ] Working tree clean before tag (`git status` has no unintended changes).
- [ ] Versioned release notes include:
  - runtime ABI/runtime_id policy
  - rollback plan
  - incident/replay runbook links

## Suggested Execution Order

1. Implement program attestation verification + lifecycle tests.
2. Fix clippy `-D warnings` failures (start with `crates/edgerun-storage`).
3. Expand CI to enforce program + storage quality gates.
4. Close runtime readiness Phase C/D items.
5. Lock scheduler production behavior for chain context failures.
6. Re-run full release checklist and tag only when all gates pass.
