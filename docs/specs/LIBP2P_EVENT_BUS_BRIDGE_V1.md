# LIBP2P_EVENT_BUS_BRIDGE_V1

## Goal
Enable event-bus message propagation over libp2p so scheduler-published event bus envelopes can be distributed peer-to-peer and ingested by subscribed nodes.

## Non-goals
- Replace storage-backed event bus persistence.
- Add consensus semantics for cross-node ordering.
- Introduce custom p2p auth beyond signed libp2p identities.

## Security and Constraints
- P2P bridge remains opt-in via env vars.
- Event payload transport uses gossipsub topic scoped by explicit topic name.
- Invalid inbound payloads are rejected without crashing runtime.
- Local storage-backed policy and nonce validation remain authoritative on ingest.

## Acceptance Criteria
1. `edgerun-p2p` exposes event bus runtime with publish + subscribe channels over libp2p gossipsub.
2. Scheduler publishes local event bus envelopes to libp2p topic.
3. Scheduler ingests remote libp2p envelopes into local `StorageBackedEventBus` with validation.
4. Worker can subscribe to p2p event bus topic and observe traffic.
5. Build/check/tests pass for touched crates or blockers are reported.

## Rollout
- Enable on selected nodes with `EDGERUN_P2P_ENABLED=true` and shared topic.
- Bootstrap peers configured via `EDGERUN_P2P_BOOTSTRAP_PEERS`.

## Rollback
- Disable p2p bridge via `EDGERUN_P2P_ENABLED=false`.
- Revert change set if runtime behavior needs full rollback.
