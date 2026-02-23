# Edgerun Repo Context (Compact Canonical)

## Purpose
This is the canonical high-density context for how to think and operate in this repository.

## Verification Basis (2026-02-23)
- Repo policy source: `AGENTS.md`.
- Frontend build/test source: `frontend/package.json`, `frontend/README.md`.
- Event foundation implementation source: `crates/edgerun-storage/src/event_bus.rs`, `crates/edgerun-storage/src/timeline.rs`, `crates/edgerun-cli/src/commands/execution.rs`, `crates/edgerun-scheduler/src/main.rs`.
- Protocol source: `docs/ROUTED_TERMINAL_PROTOCOL_V2.mdx`, `frontend/control-panel/WS_PROTOCOL.md`.

## Canonical Operating Mentality
- Determinism first: architecture, runtime behavior, and protocol decisions must be reproducible and verifiable.
- Proof over claims: no completion without command-backed evidence.
- Spec-first for non-trivial change: write/update spec before implementation.
- Minimal complexity: reduce dependencies, avoid duplicate roots/paths, keep boundaries explicit.
- Real sources only for on-chain views: use real RPC/chain networks; no mocked chain-derived UI signals.

## Hard Invariants
- JavaScript package/runtime default: `bun`.
- Frontend canonical root: `frontend/` only.
- Generated/build/temp artifacts: `out/` at repo root.
- Architecture must remain deterministic and dependency-disciplined.
- On-chain derived views must use real chain/RPC sources.
- Do not bind anything to port `8080`.

## Definition Of Done (Non-Negotiable)
A change is done only when all are true:
1. Requested behavior exists.
2. Required checks/tests pass.
3. Evidence report includes exact commands + pass/fail.
4. Known regressions are explicitly reported.

## System Boundaries (Merged From Specs)
- UI layer (`os-ui`/frontend): rendering + interaction surfaces.
- Execution layer (substrate/runtime): deterministic job execution contracts.
- Event bus + storage layer: canonical append/query/state boundary.
- Adapter layer: external providers/integrations behind explicit contracts.
- Contract-first preference: proto/schema-based interfaces, avoid hidden compatibility shims.

## Event Foundation (Current Implementation Snapshot)
- Implemented:
  1. Event/timeline envelopes are validated and reject malformed required fields.
  2. Duplicate `event_id` handling is idempotent for identical payloads and rejected for conflicts.
  3. CLI execution lifecycle writes queryable timeline events (`intent_submitted`, `execution_started`, `execution_step_started`, `execution_step_finished`, `execution_finished`).
  4. Event-bus phases `AwaitingInit`, `Running`, `Halted` plus `resume_from` flow are enforced.
- Current gaps inferred from code:
  1. Event-bus state still reconstructs from full history on cold start (no persisted index snapshot).
  2. Event-bus/timeline query paths scan segment history per query call.
  3. `BUS_PHASE_V1_EMERGENCY_SEAL_ONLY` exists in proto but has no transition logic in runtime code.
  4. Chain progress handling exists in both storage session APIs and scheduler event-bus sink paths.

## Protocol/Contract Snapshot
- Routed Terminal Protocol v2: framed lifecycle (`open`, `ack`, `input`, `output`, `resize`, `close`, `exit`, `error`) with strict validation + session/routing semantics.
- Long-running job contract: explicit lifecycle events (`job.opened`, `action.planned`, `action.result`) with durable envelope and replay-friendly logging.
- Control panel WS protocol: pinned request/response/push envelope shapes.

## Runtime + Chain Principles (Whitepapers)
- Deterministic wasm execution, restricted hostcalls/imports, canonical bundle/hash validation.
- Runtime identity must be explicit and enforced.
- Settlement/finality authority remains chain-verifiable.
- Phase-2 direction: reduced scheduler authority, stronger DA/finality reliability, deterministic selection semantics.

## Security + Trust Posture
- Zero-trust partitioning for storage/event streams where specified.
- No hidden auto-healing shims that mask contract violations.
- Security constraints and rollback notes must be written in specs before behavior changes.

## Frontend/UX Operating Rules
- Static generation bias and minimal runtime dependencies.
- Style/theming centralized; style guide is authoritative.
- E2E tests assert user-visible and architecture-critical behavior.

## Deploy/Ops Snapshot
- Systemd/cloudflared docs define local operational pathways for scheduler/workers/terminal routing.
- Always keep observability/log-tail commands in evidence for operational changes.

## Legal/Brand Constraints
- Licensing matrix and trademark notices remain authoritative for distribution/branding boundaries.

## Coverage Map (what this compact doc absorbs)
- Governance: `AGENTS.md`
- Architecture/specs/proposals: historical content consolidated from removed docs (kept in git history)
- Frontend protocol/run docs: `frontend/README.md`, `frontend/control-panel/*.md`
- Script ops docs: `scripts/cloudflared/README.md`, `scripts/systemd/**/*.md`
- Runtime/storage/app readmes + whitepapers + licensing/trademark docs.
