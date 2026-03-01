# 2026-03-01 Agent Tooling Surface Removal And Repo Coherence V1

## Goal
- Remove the `edgertool` Go CLI and its script wrapper surfaces.
- Remove obsolete agent/executor script entrypoints that currently depend on removed tooling.
- Restore a coherent, deterministic top-level workflow where every advertised command maps to existing paths.

## Non-Goals
- Replacing removed agent workflow tooling with a new framework in this change.
- Changing runtime protocol/event contracts in Rust crates.
- Introducing new services, ports, or external dependencies.

## Security and Constraints
- Fail closed on removed surfaces: no stale command should silently no-op.
- Keep Bun as JS runtime/package manager default.
- Preserve canonical frontend root (`frontend/`) and output root (`out/`).
- Do not bind anything to port `8080`.

## Design
1. Remove tooling source/binary path for `tooling/cmd/edgertool` and related `tooling` Go module assets.
2. Remove `scripts/agents/*` and `scripts/executors/*` compatibility wrappers.
3. Remove swarm executor-stack scripts that depend on removed executor scripts.
4. Update top-level `Makefile` targets and phony list to remove deleted entrypoints.
5. Repair `scripts/ecosystem-workflow.sh` to match current repo layout (remove references to non-existent `program/` and `cloud-os/` paths).
6. Keep historical specs as history; align active command surfaces and current docs/scripts only.

## Acceptance Criteria
1. `tooling/cmd/edgertool` and `scripts/agents` + `scripts/executors` are absent from the repo.
2. `make` targets do not reference removed paths.
3. `scripts/ecosystem-workflow.sh {check,build,test,verify}` runs against existing repo paths only.
4. Required verification commands pass for modified scope.

## Rollout
- Land spec and cleanup in one change set with command-backed validation.
- Use simplified canonical workflows (`make`, `scripts/ecosystem-workflow.sh`, frontend and cargo commands).

## Rollback
- Revert this cleanup commit to restore removed tooling and wrappers.
