# EdgeRun Snapshotter Decommission V1

## Goal

Remove the in-repo EdgeRun snapshotter implementation and its runtime wiring so containerd runtime paths only use stable, protocol-compatible snapshotters.

## Non-goals

- Changing the `io.containerd.edgerun.v1` runtime class behavior.
- Replacing containerd upstream snapshotter implementations.
- Introducing a new custom snapshotter protocol implementation in this step.

## Security and Constraint Requirements

- Preserve deterministic runtime startup and task lifecycle behavior.
- Keep containerd default runtime and snapshotter configuration explicit.
- Remove service/unit/config paths that point at unsupported snapshotter socket APIs.
- Avoid introducing new dependency or package manager workflows.

## Acceptance Criteria

1. `crates/edgerun-snapshotter` is removed from workspace membership.
2. Containerd snippet/config no longer configures `proxy_plugins.edgerun` snapshotter socket wiring.
3. Systemd service wiring no longer depends on `edgerun-snapshotter.service`.
4. Workspace builds/tests pass for remaining crates.
5. Runtime smoke path remains valid with stable snapshotter (`overlayfs`) and `io.containerd.edgerun.v1` runtime.

## Rollout Notes

- Apply updated containerd snippet and restart containerd.
- Ensure workloads use `overlayfs` (or another containerd-native snapshotter) for runtime execution.

## Rollback Notes

- Revert this change set to restore prior snapshotter crate and wiring.
- Re-enable snapshotter service and proxy plugin settings only with a protocol-compatible server implementation.
