# RBI State Stream Protocol V1

## Goal
- Define a deterministic protobuf contract for remote browser interaction that streams behavior/state updates instead of pixel frames.
- Provide a transport-agnostic envelope that can run over WebSocket, QUIC stream, or other ordered reliable channels.

## Non-goals
- No rendering engine implementation details.
- No pixel/video transport path.
- No protocol downgrade or legacy wire compatibility in this spec.

## Security and constraints
- Session frames must be scoped to a server-issued `session_id`.
- Inputs carry monotonic sequence identifiers for replay protection and deterministic processing.
- Origins and navigation policy are explicit in session negotiation.
- Binary payloads are optional and bounded (e.g., snapshots/chunks), with chunk metadata for backpressure-safe transfer.

## Acceptance criteria
1. Protobuf files exist under `src/proto/edgerun/browser/rbi/v1/`.
2. Contracts include:
   - Session lifecycle and policy negotiation
   - User input/event modeling
   - Render/state update modeling
   - Bidirectional envelope with ACK/error semantics
3. Messages are package/version namespaced as `edgerun.browser.rbi.v1`.
4. Contracts avoid transport lock-in (no WebSocket-only fields).

## Rollout
- Phase 1: commit protocol contracts and docs only.
- Phase 2: bind encoder/decoder in runtime/server crates behind feature flags.
- Phase 3: adopt in control plane and frontends with compatibility shims if needed.

## Rollback
- Revert protocol files and spec as one change set.
- Runtime behavior remains unchanged until integration phase begins.
