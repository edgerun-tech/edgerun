# 2026-02-28 Networking v1 Architecture Contract

## Goal
- Converge networking to a minimal, secure, local-first architecture.
- Eliminate duplicate cluster networking paths.
- Keep node-local composition deterministic and fast.

## Non-Goals
- No global canonical state merge.
- No immediate removal of all legacy networking code in one step.
- No replacement of local UDS IPC with cluster protocols.

## Architecture Contract

### 1) `edge-internal` network (same node)
- Transport: gRPC over Unix domain sockets.
- Scope: process-local control/events between node services.
- Reliability model: local IPC, fail-closed for required control paths.
- Current implementation remains canonical.

### 2) `edge-cluster` network (multi-node)
- Primary substrate: libp2p only.
- Required libp2p primitives:
  - authenticated node identity,
  - encrypted channels,
  - multiplexed streams,
  - pubsub for distributed event fanout,
  - request/response for directed RPC.
- Scheduler must not be a mandatory rendezvous path for peer-to-peer connectivity.

### 3) Discovery
- Primary discovery path: libp2p-derived peer/route data.
- No scheduler fallback path for cluster discovery.
- Discovery must fail closed when libp2p route data is unavailable.

### 4) Security
- Node identity must be stable and signed.
- All intent/event payloads remain signed at application layer.
- Capability leases remain policy-gated and time-bounded.
- Partition behavior: continue local execution with security checks intact.

## Code Reduction Targets
- Remove scheduler WebSocket route control protocol as cluster dependency after parity.
- Remove custom WebSocket stream mux for cluster plane.
- Remove WireGuard adapter path for cluster plane unless explicitly required.
- Keep direct protocol adapters only where they serve non-cluster external requirements.

## Acceptance Criteria
1. `edgerun-p2p` is a first-class workspace crate.
2. Discovery layer supports `libp2p-first` with scheduler fallback.
3. Cluster networking design is documented and enforceable by code boundaries.
4. Workspace builds and tests pass for touched crates.

## Rollout / Rollback
- Rollout:
  - feature/env gated migration with strict libp2p discovery.
  - move services to libp2p-first discovery incrementally.
- Rollback:
  - disable libp2p-first consumer integration and revert to previous commit state.
