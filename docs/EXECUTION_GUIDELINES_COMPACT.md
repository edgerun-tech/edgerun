# Edgerun Execution Guidelines (Compact Canonical)

## Use This Loop For Every Task
1. Scope the change.
2. If non-trivial, write/update spec first (`goal`, `non-goals`, `security/constraints`, `acceptance`, `rollout/rollback`).
3. Implement minimal deterministic slice.
4. Run required checks.
5. Run relevant tests.
6. Report evidence with exact commands and pass/fail.

## Mandatory Validation Baseline
For frontend-impacting work:
```bash
cd frontend && bun run check
cd frontend && bun run build
```

For behavior/UI changes, add or update Cypress E2E tests and run relevant specs.

For Rust/storage/scheduler core work:
```bash
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Evidence Report Format (Required)
- `Scope`: exact files/behavior changed.
- `Commands run`: exact commands.
- `Results`: pass/fail per command.
- `Artifacts`: outputs and paths (if relevant).
- `Known limitations`: explicit blockers or deferred gaps.

## Execution Guardrails
- Do not introduce `npm`/`pnpm` workflows for repo-default JS flow; use `bun`.
- Do not create parallel frontend roots outside `frontend/`.
- Keep generated/build/temp artifacts in `out/`.
- Do not declare done without proof.
- Do not leave known regressions unreported.
- Do not use mock chain sources for chain-derived UI/data.

## Active Priority Backlog (Execution Order)
1. Reduce cold-start/event query full-history scans in storage event/timeline paths.
2. Implement or remove unused `EMERGENCY_SEAL_ONLY` phase path so proto and runtime match.
3. Unify chain progress canonical ingress (event-bus path vs direct storage session path).
4. Tighten protocol drift checks between documented contracts and runtime/CLI behavior.

## Contract Checks Before Merging Behavior Changes
- Envelope validation deterministic and explicit.
- Idempotency rules defined and tested.
- Query cursor/limit semantics stable and tested.
- Producer/consumer mappings documented for changed contracts.
- Rollback path documented for any stateful change.

## Frontend Test Intent Checklist
- Rendering/state correctness from canonical data sources.
- Routing + hydration/interactivity behavior.
- Architecture-critical wiring (worker/runtime/protocol contracts).
- No snapshot-only confidence.

## Ops Quick Commands (When Relevant)
Unified root workflow:
```bash
bun run drift:check
bun run ecosystem:check
bun run ecosystem:build
bun run ecosystem:test
bun run ecosystem:verify
```

Cloudflare frontend target verification:
```bash
bun run cloudflare:targets:check
```

Frontend:
```bash
cd frontend && bun install
cd frontend && bun run check
cd frontend && bun run build
cd frontend && bun run test:e2e
```

Compose terminal stack E2E:
```bash
cd frontend && bun run e2e:compose
```

Systemd local services:
```bash
systemctl --user daemon-reload
systemctl --user enable --now edgerun-scheduler.service
journalctl --user -u edgerun-scheduler.service -f
```

Cloudflare tunnel/smoke paths are maintained in `scripts/cloudflared/README.md`.

## When To Escalate
Escalate only for:
1. Missing credentials/secrets/access.
2. Destructive/high-risk decisions.
3. Product-direction ambiguity that materially changes scope.

Ask one concrete question and include exact command/context already attempted.

## Compact Canonical Pair
- Mental model + constraints: `docs/REPO_CONTEXT_COMPACT.md`
- Action loop + commands + evidence: `docs/EXECUTION_GUIDELINES_COMPACT.md`

Other docs are detail references; this pair defines default working mentality.
