# 2026-02-28 Overlay Event Bus gRPC Transport v1

## Goal
- Use gRPC as transport for `edge-internal` overlay event communication.
- Keep event-bus semantics (topic + overlay + push subscription) separate from storage.
- Support multi-process components on one node with typed contracts.

## Non-Goals
- This phase does not finalize `edge-cluster` transport.
- This phase does not introduce consumer-group assignment/rebalance yet.
- This phase does not replace all existing callers with gRPC clients.

## Security and Constraints
- Transport endpoint is Unix domain socket by default for `edge-internal`.
- Fail-closed publish path when ingestion enforcement is enabled.
- Topic and overlay remain explicit in each request/envelope.

## Design
- Add proto contract in `crates/edgerun-event-bus/proto/edge_internal/v1/event_bus.proto`.
- Service:
  - `Publish(PublishRequest) -> PublishResponse`
  - `Subscribe(SubscribeRequest) -> stream EventEnvelope`
- Scheduler starts gRPC broker bound to UDS path under scheduler data dir.
- Runtime bus remains in-memory push/sub core; gRPC is a transport adapter.

## Acceptance Criteria
1. `edgerun-event-bus` compiles generated gRPC/protobuf code.
2. Scheduler starts `edge-internal` broker via gRPC adapter.
3. `cargo check --workspace` passes.
4. `cargo clippy -p edgerun-event-bus --all-targets -- -D warnings` passes.
5. `cargo clippy -p edgerun-scheduler --all-targets -- -D warnings` passes.

## Rollout and Rollback
- Rollout:
  - Enable `EDGERUN_EVENT_BUS_EDGE_INTERNAL_ENABLED=true`.
  - Broker path defaults to `${EDGERUN_SCHEDULER_DATA_DIR}/event-bus/edge-internal.sock`.
- Rollback:
  - Revert gRPC adapter wiring commit to restore prior adapter.
