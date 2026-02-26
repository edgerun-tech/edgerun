# TRANSPORT_STACK_REMOVAL_V1

## Goal
Remove the current transport stack from the Rust workspace immediately by deleting transport-specific crates and their orchestration/discovery layers.

## Non-goals
- No replacement transport framework in this change.
- No behavioral redesign of existing scheduler control WebSocket or WebRTC signaling endpoints.
- No frontend architecture change.

## Security and Constraints
- Preserve existing control-plane signing behavior used by scheduler/worker/term-server.
- Keep workspace deterministic and compileable after crate removal.
- Do not leave dangling path dependencies or orphaned workspace members.
- Keep changes scoped to in-repo code; no mock transport shims.

## Acceptance Criteria
1. Workspace no longer includes:
   - `crates/edgerun-transport-core`
   - `crates/edgerun-transport-quic`
   - `crates/edgerun-transport-ws`
   - `crates/edgerun-transport-wireguard`
   - `crates/edgerun-discovery`
   - `crates/edgerun-routing`
2. Scheduler, worker, and term-server compile without depending on removed transport crates.
3. Control-plane signing message generation remains available and unchanged in behavior.
4. Required validation commands complete successfully and are reported with pass/fail evidence.

## Rollout
- Single-step removal in current branch.
- If downstream code depends on removed crates, follow-up work must introduce a new explicit architecture rather than reintroducing legacy transport crates.

## Rollback
- Revert this change set to restore removed transport crates and previous dependency graph.
