# LIBP2P_P2P_NETWORKING_V1

## Goal
Introduce libp2p-based peer-to-peer networking primitives into the Edgerun Rust workspace and wire them into scheduler/worker process startup as an opt-in runtime service.

## Non-goals
- No migration of existing control-plane APIs from WebSocket/HTTP to p2p in this change.
- No DHT, pubsub, or custom protocol message routing yet.
- No frontend networking changes.

## Security and Constraints
- P2P must be disabled by default.
- Transport security must use Noise over authenticated libp2p identities.
- Configuration must be deterministic via env vars (listen addrs, bootstrap peers, optional static key seed).
- Startup must not panic on malformed optional p2p env values; return actionable errors.

## Acceptance Criteria
1. New crate `crates/edgerun-p2p` exists with:
   - libp2p Swarm initialization
   - `ping` and `identify` behaviours
   - helper to spawn an async runtime loop
2. Scheduler and worker can opt-in p2p runtime via env var toggle.
3. Runtime supports configured listen addresses and bootstrap dial targets.
4. Unit tests cover critical config parsing/identity derivation logic.
5. Build/check/tests run with evidence.

## Rollout
- Phase 1 (this change): foundational libp2p runtime enabled through env vars.
- Phase 2 (follow-up): protocol-specific p2p request/response for Edgerun jobs/routes.

## Rollback
- Disable by setting `EDGERUN_P2P_ENABLED=false`.
- Revert commit to remove crate and startup integration.
