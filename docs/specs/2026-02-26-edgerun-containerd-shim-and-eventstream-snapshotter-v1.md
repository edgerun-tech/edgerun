# EdgeRun Containerd Shim + Eventstream Snapshotter V1

## Goal

Introduce an EdgeRun-native container runtime integration path in containerd with a dedicated runtime type and a snapshotter contract aligned with eventstream-backed virtual filesystem semantics.

## Scope (V1)

- Add workspace crates for:
  - runtime message/types contract (`edgerun-runtime-proto`)
  - shim lifecycle bootstrap (`edgerun-containerd-shim`)
  - snapshotter lifecycle contract (`edgerun-snapshotter`)
- Add shim Unix socket `serve` endpoint for control-plane request/response transport.
- Add shim library task client adapter (`ContainerdTaskClient`) that maps containerd task-service calls (`create/start/state/kill/delete/wait`) onto protobuf `task_api` requests over Unix socket.
- Add shim task-service skeleton (`ShimTaskTtrpcService`) exposing RPC-shaped methods (`create/start/state/kill/delete/wait`) delegating to the adapter.
- Add containerd ttrpc task service implementation (`ContainerdTaskTtrpcService`) for `containerd.task.v2.Task` and registration helper (`register_task_ttrpc_service`).
- Add persistent snapshot metadata state file support.
- Materialize snapshot mount roots on disk for `prepare/view/commit/remove/cleanup` so returned mounts are backed by real filesystem paths.
- Add snapshotter Unix socket `serve` endpoint for long-running control operations.
- Add systemd unit templates + installer workflow for snapshotter + shim backend host services.
- Add containerd config apply/verify script for runtime snippet import path.
- Emit runtime and snapshot materialization events into `edgerun-storage` timeline.
- Keep the rollout additive; no replacement of existing `crun`/`runc` runtime paths yet.
- Provide selectable runtime classes in containerd config:
  - default OCI runtime class: `crun` (`io.containerd.runc.v2`, `BinaryName=/usr/bin/crun`)
  - EdgeRun runtime class: `edgerun` (`io.containerd.edgerun.v1`, `BinaryName=containerd-shim-edgerun-v2`)

## Non-goals (V1)

- Full containerd shim gRPC/ttrpc implementation.
  - Current status: task service methods (`Create/Start/State/Kill/Delete/Wait/Connect/Shutdown`) are implemented with containerd ttrpc trait bindings; broader shim lifecycle integration remains follow-up work.
- Production-ready snapshotter daemon wiring into containerd plugin registration.
- Full overlayfs lowerdir/upperdir/workdir layering semantics beyond V1 bind-backed materialization.
- Full POSIX parity over event storage.

## Contracts

### Runtime Type

- runtime_type identifier target: `io.containerd.edgerun.v1`
- binary name target (containerd lifecycle entrypoint): `containerd-shim-edgerun-v2`
- shim transport endpoint target: Unix socket (`serve` command) with framed protobuf protocol.
- backend shim socket for lifecycle entrypoint is configurable via `EDGERUN_SHIM_SOCKET` (default `/run/edgerun-shim/edgerun.sock`).
- containerd selectable runtime target:
  - `default_runtime_name = "crun"` for OCI paths
  - explicit runtime class `edgerun` for EdgeRun/WASI paths

### Shim Socket Ops (V1, Protobuf)

- `health`
- `print_subject`
- `emit_event`
- `task_state`
- `task_api` (`create|start|state|kill|delete|wait`)

`task_api` operations `create`, `start`, and `kill` also publish runtime task lifecycle events into timeline (`runtime_task_event_v1_pb`) so control-plane RPC and timeline streams stay aligned.

One-shot CLI parity:
- `containerd-shim-edgerun-v1 rpc` consumes one framed protobuf request from stdin and writes one framed protobuf response to stdout.

TTRPC server mode:
- `containerd-shim-edgerun-v1 serve-ttrpc --ttrpc-socket-path <path> --shim-socket-path <path>` starts a `containerd.task.v2.Task` ttrpc service and forwards calls to the shim protobuf `task_api` backend.

### Snapshotter Name

- snapshotter name target: `edgerun`
- method surface target (containerd-aligned):
  - `Prepare`
  - `View`
  - `Commit`
  - `Mounts`
  - `Remove`
  - `Stat`
  - `Update`
  - `Usage`
  - `Walk`
  - `Cleanup`

### Snapshotter Socket Ops (V1, Protobuf)

- `health`
- `prepare`
- `view`
- `commit`
- `remove`
- `walk`
- `cleanup`
- `stat`
- `mounts`
- `usage`

Response payloads are typed protobuf `oneof` bodies (`snapshot`, `snapshots`, `mounts`, `usage`, `removed`, `cleanup`) with no JSON payload wrapper in the transport.

One-shot CLI parity:
- `edgerun-snapshotterd rpc` consumes one framed protobuf request from stdin and writes one framed protobuf response to stdout.

### Event Subject Families

- `edgerun.runtime.task.<namespace>.<task_id>.<lane>`
- `edgerun.runtime.io.<namespace>.<task_id>.<stream>`
- `edgerun.storage.snapshot.<snapshot_key>.<lane>`

### Timeline Payload Types

- `runtime_task_event_v1_pb` (protobuf bytes)
- `snapshot_materialized_v1_pb` (protobuf bytes)

## Durability and Consistency Defaults

- Durability default for runtime/snapshot state events: `AckDurable`.
- Write conflict strategy: optimistic with stale-base rejection (declared at intent model level).
- Binary data strategy: CAS blob references (defined for subsequent phases).

## Acceptance Criteria

1. New crates compile and are part of workspace.
2. Shim binary can emit validated runtime events into timeline.
3. Snapshotter binary can emit materialized snapshot events into timeline.
4. Snapshotter API surface includes all containerd-aligned lifecycle methods.
5. Snapshotter state survives process restart via persistent state file.
6. Snapshotter operations create/update/remove corresponding mount-root directories on disk and `usage` reflects on-disk content.
7. Runtime config template renders `crun` as default runtime and keeps `edgerun` as explicit opt-in runtime.
8. Runtime matrix smoke script exists for `ctr` to validate both runtime classes:
   - `io.containerd.runc.v2`
   - `io.containerd.edgerun.v1`
9. System service install script exists to build/install binaries and install/enable host units for snapshotter + shim backend.
10. Containerd config apply script exists to install snippet in `conf.d`, validate effective config, and optionally run runtime matrix smoke.
11. Runtime WASM smoke script exists and validates `edgerun-runtime replay-corpus` inside a container started with `io.containerd.edgerun.v1`.

## Rollout

1. Keep existing runtime defaults unchanged.
2. Integrate runtime class for canary workloads only.
3. Add true shim/snapshotter transport bindings in a follow-up spec.

## Rollback

- Remove the new crates from workspace members.
- Keep timeline events append-only; no destructive cleanup required.
