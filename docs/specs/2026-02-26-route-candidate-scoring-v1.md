# Route Candidate Scoring V1

## Goal
- Improve transport endpoint selection quality by scoring candidates using route metadata instead of kind-only ordering.
- Preserve deterministic fallback behavior while reducing relay usage when direct paths are available.

## Non-goals
- No STUN/TURN/ICE signaling changes in this slice.
- No protocol wire-format changes beyond already introduced candidate schema fields.

## Security and constraints
- Keep policy deterministic: score uses explicit endpoint fields only (`kind`, `priority`, metadata).
- Do not introduce network probing side effects in route selection path.
- Maintain existing feature gate behavior (`allow_ws_fallback`, multiplexing requirement).

## Acceptance criteria
1. Routing policy continues to prefer QUIC over WebSocket/WireGuard by default.
2. For same transport kind, direct candidates are preferred over relay candidates when metadata is present.
3. For otherwise equivalent candidates, lower advertised RTT is preferred.
4. Existing route resolution flow remains compatible and builds cleanly.

## Rollout
- Phase 1: metadata-aware scoring in `edgerun-transport-core`.
- Phase 2: producer-side metadata quality improvements (RTT freshness, relay labels, NAT hints).
- Phase 3: adaptive runtime re-scoring based on observed path health.

## Rollback
- Revert policy-scoring changes to restore previous kind + priority ordering.
