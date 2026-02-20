# EdgeRun Runtime Security Review

Last updated: 2026-02-20

## Scope

This review covers `crates/edgerun-runtime` execution, validation, hostcall boundary behavior, and runtime supply-chain posture.

## Threat model

Primary adversary goals:

- Execute malformed or adversarial Wasm bundles to trigger panics, memory corruption, or nondeterminism.
- Exploit hostcall pointer/length handling to read/write out-of-bounds memory.
- Trigger resource exhaustion (fuel/memory/output) to degrade worker availability.
- Introduce known-vulnerable dependencies into runtime transitive closure.
- Undermine incident response by preventing reproducible replay artifacts.

Trust boundaries:

- Untrusted: bundle bytes (`wasm`, `input`, limits) and assignment payload contents.
- Trusted: runtime binary, interpreter/validator policy, host process memory, CI-produced release artifacts.

## Current controls

- Canonical bundle decode + hash invariant before execution.
- Strict validator policy:
  - hostcall import allowlist and exact signatures
  - export allowlist + required `_start` and `memory`
  - float-op rejection
  - memory export constraints
- Runtime resource controls:
  - fuel metering (`max_instructions`)
  - memory limiter (`max_memory_bytes`)
  - output size limits + trap mapping
- Hostcall boundary hardening tests for zero-length, partial, boundary, negative, and overflow cases.
- Determinism and replay controls:
  - replay artifact contract
  - replay corpus matrix in CI
  - expected output hash verification mode
- Security CI gates:
  - nightly fuzz sanity for decoder/validator/hostcall targets
  - dependency vulnerability audit (`cargo audit`)
  - SBOM generation artifact (`cargo cyclonedx`)

## Security hardening backlog

Closed items:

- [x] Add runtime fuzz targets for decoder/validator/hostcall boundaries.
- [x] Run bounded fuzz sanity continuously in CI.
- [x] Enforce dependency vulnerability audit in CI.
- [x] Emit runtime SBOM artifact in CI.
- [x] Add deterministic replay artifact and replay-corpus CI regression.

Open items:

- [ ] Signed release provenance for runtime artifacts (SLSA/in-toto or equivalent).
- [ ] Signed scheduler assignment policy distribution (runtime_id/abi policy moved from env/config).
- [ ] Formal external security review and findings sign-off.

## Advisory triage exceptions

The CI `cargo audit` gate currently carries temporary ignore entries in `.cargo/audit.toml` for:

- `RUSTSEC-2022-0093` (`ed25519-dalek` 1.x)
- `RUSTSEC-2024-0344` (`curve25519-dalek` 3.x)
- `RUSTSEC-2021-0145` (`atty`)

Rationale:

- Both advisories are currently introduced through shared Solana/scheduler dependency chains, not through direct `edgerun-runtime` dependency paths.
- Keeps the security gate actionable for newly introduced runtime-relevant vulnerabilities while upstream dependency updates are pending.

Exit criteria:

- Remove ignores once scheduler/Solana dependency chain upgrades eliminate the vulnerable crates.

## Residual risk notes

- Fuzz sanity runs are bounded-time smoke tests; they do not replace longer scheduled fuzzing.
- SBOM generation improves visibility but does not guarantee dependency integrity by itself.
- Release artifact provenance remains the largest remaining supply-chain gap.
