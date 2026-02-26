# Route Candidates Control Plane V1

## Goal
- Introduce canonical transport route candidates for peer routing discovery.
- Keep route resolution compatible during migration by retaining legacy `reachable_urls` as a fallback field.

## Non-goals
- No STUN/TURN/ICE or NAT traversal behavior in this slice.
- No dynamic path scoring changes in routing policy.

## Security and constraints
- Preserve existing signed registration model (`route_register_signing_message`) by signing normalized candidate URIs.
- Accept only known transport schemes (`quic`, `websocket`, `wireguard`) when normalizing candidates.
- Keep scheduler challenge/heartbeat token lifecycle unchanged.

## Acceptance criteria
1. Control-plane route contracts include `candidates` with `kind`, `uri`, `priority`, and `metadata`.
2. Scheduler route registration normalizes candidate payloads and stores canonical candidates.
3. Discovery consumes canonical candidates and only falls back to legacy `reachable_urls` when candidates are absent.
4. Existing route announcer flow continues functioning with candidate-aware registration payloads.

## Rollout
- Phase 1: introduce dual-field contracts (`candidates` + `reachable_urls`) and migrate scheduler/discovery/announcer.
- Phase 2: migrate remaining clients to candidate-first behavior.
- Phase 3: remove legacy fallback field after all clients are migrated.

## Rollback
- Revert this change set to restore URL-only route advertisements.
