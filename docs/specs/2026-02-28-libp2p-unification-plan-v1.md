# 2026-02-28 Libp2p Unification Plan v1

## Goal
- Unify `edge-cluster` networking onto a single libp2p substrate.
- Remove duplicated custom transport/discovery control paths where libp2p can provide equivalent behavior.
- Keep `edge-internal` local process communication separate and local-first.

## Non-Goals
- No immediate replacement of local `edge-internal` gRPC-over-UDS event bus.
- No canonical global state merge semantics (per-node timelines remain independent).
- No full scheduler redesign in this phase.

## Security and Constraints
- Fail-closed behavior for control-plane critical flows.
- Authenticated peer identity required for cluster traffic.
- Signed intent/event payloads remain required end-to-end.
- Keep local operation viable during partitions.

## Current State Inventory

### Existing libp2p usage
- `crates/edgerun-p2p/src/lib.rs` contains a libp2p gossipsub runtime (`ping`, `identify`, signed `gossipsub`).
- Current status: dormant prototype.
  - Directory exists but no `Cargo.toml`.
  - Not listed in workspace members, not imported by active services.

### Active custom networking paths (cluster/control)
- Scheduler control and signaling:
  - `/v1/control/ws` binary/json RPC over WebSocket.
  - `/v1/webrtc/ws` signaling over WebSocket.
- Route registration protocol:
  - `route.challenge`, `route.register`, `route.heartbeat`, `route.resolve`.
  - candidate exchange (`quic://`, `ws://`, `wg://`) and STUN-discovered URLs.
- Discovery and routing:
  - `edgerun-discovery` resolves routes by dialing scheduler control WebSocket.
  - `edgerun-routing` selects endpoint + connector using policy.
- Transport implementations:
  - `edgerun-transport-quic`: direct Quinn connector.
  - `edgerun-transport-ws`: custom stream mux framing over WebSocket.
  - `edgerun-transport-wireguard`: UDP adapter-style connector.

### Active local internal networking
- `edgerun-event-bus` uses tonic gRPC over Unix domain socket for `edge-internal`.
- This is local IPC and should remain separate from cluster overlay.

## Unification Direction
- `edge-cluster`:
  - libp2p as the default and primary transport/discovery substrate.
  - use libp2p identity + authenticated sessions for node-to-node control/data channels.
  - use gossipsub/request-response/kad instead of scheduler-centric route-resolve WebSocket.
- `edge-internal`:
  - keep gRPC/UDS for same-node service composition.

## Migration Phases

1. **Phase 1: Activate libp2p as first-class crate**
- Add `crates/edgerun-p2p/Cargo.toml`.
- Add to workspace.
- Expose a stable API for:
  - peer identity bootstrap,
  - publish/subscribe,
  - direct request/response channel.
- Add integration test proving two local nodes exchange signed events.

2. **Phase 2: Introduce libp2p discovery provider**
- Add `Libp2pDiscoveryProvider` in `edgerun-discovery`.
- No scheduler fallback for the libp2p discovery path.
- Route planner/orchestrator consume libp2p discovery results without requiring scheduler resolve.

3. **Phase 3: Replace custom cluster transport adapters**
- Deprecate custom WS mux and WG adapter usage for cluster messaging.
- Provide libp2p stream adapter implementing `MuxedTransportSession` for routing stack compatibility.
- Keep QUIC connector only where explicitly needed outside libp2p swarm.

4. **Phase 4: Remove scheduler-centric route control dependency**
- Make route registration/heartbeat/resolve optional then removable for cluster traffic.
- Keep minimal compatibility bridge during rollout.

5. **Phase 5: Cleanup**
- Remove unused transport/discovery codepaths and env flags.
- Update docs/systemd/env profiles.

## Acceptance Criteria
1. libp2p crate is built and tested in workspace.
2. At least one active service path can discover/connect peers through libp2p without scheduler route-resolve WebSocket.
3. Cluster event exchange works over libp2p with signed payloads.
4. Legacy route control path can be disabled cleanly.
5. `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and relevant tests pass.

## Rollout / Rollback
- Rollout:
  - feature-flagged cutover from scheduler-route discovery to strict libp2p discovery.
- Rollback:
  - disable libp2p path via env/feature and revert to previous commit state.
  - no data migration required.
