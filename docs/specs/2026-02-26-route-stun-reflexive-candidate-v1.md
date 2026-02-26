# Route STUN Reflexive Candidate V1

## Goal
- Add STUN-based reflexive endpoint discovery in term-server route announcements.
- Publish discovered public endpoint as a canonical route candidate for direct peer connectivity.

## Non-goals
- No TURN relay allocation in this slice.
- No full ICE negotiation graph.

## Security and constraints
- STUN discovery is best-effort and must not crash route announcer loop.
- Keep route signature model intact by signing normalized advertised URLs.
- Preserve existing challenge/register/heartbeat scheduler control workflow.

## Acceptance criteria
1. Term-server supports STUN server configuration with default `172.245.67.49:3478`.
2. When local/private route base is configured, term-server attempts STUN discovery and advertises reflexive public URL.
3. Candidate metadata marks STUN-discovered entries (`source=stun`, `direct=true`, `relay=false`).
4. Build/check passes for modified crates.

## Rollout
- Phase 1: add STUN probe + candidate publication in route announcer.
- Phase 2: feed measured NAT hints/RTT into candidate metadata and route scoring.

## Rollback
- Revert STUN probe logic; announcer returns to static URL advertisement only.
