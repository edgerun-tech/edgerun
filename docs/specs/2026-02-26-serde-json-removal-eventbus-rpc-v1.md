# Serde JSON Removal and RPC-Typed Payloads V1

## Goal

Remove direct `serde_json` usage from runtime-proto, snapshotter, and containerd shim paths by switching internal state and message handling to typed/protobuf-backed structures aligned with existing RPC/eventbus contracts.

## Non-goals

- Replacing containerd’s upstream ttrpc protocol with a custom transport.
- Redesigning containerd OCI bundle file format (`config.json`) or crun wire formats.
- Introducing new runtime classes or changing runtime selection semantics.

## Security and Constraint Requirements

- Preserve existing task lifecycle and snapshotter behavior.
- Keep containerd shim communication over typed ttrpc/protobuf (`containerd.task.v2.Task`, shim RPC `task_api`).
- Keep deterministic on-disk snapshotter state encoding/decoding with explicit schema versioning.
- No new JS package manager workflows; no port `8080` usage.
- Minimize dependency growth and remove direct `serde_json` crate dependencies from touched crates.

## Acceptance Criteria

1. No direct `serde_json` usage remains in:
   - `crates/edgerun-runtime-proto`
   - `crates/edgerun-snapshotter`
   - `crates/edgerun-containerd-shim`
2. `FsIntentEvent` metadata uses typed fields (non-JSON value tree).
3. Snapshotter persistent state is encoded/decoded via typed protobuf schema, not JSON text.
4. Containerd shim bundle and runtime-selection config reads are typed (OCI spec model), not `serde_json::Value`.
5. Containerd shim state handling for task status does not parse JSON with `serde_json`.
6. Workspace validation passes:
   - `cargo check --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`

## Rollout Notes

- Backward compatibility for existing JSON snapshotter state files is not guaranteed in V1; operators should allow regeneration or migrate state before rollout.
- Deploy shim/snapshotter binaries together so runtime-proto schema and state codec changes stay aligned.

## Rollback Notes

- Revert this change set to restore prior JSON-based handling.
- If rollback is required after state-file format changes, remove/regenerate snapshotter state file to match reverted binary expectations.
