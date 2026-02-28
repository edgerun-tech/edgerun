# Device Capability Host Adapter Linux V1

## Goal
- Add a `std` host adapter crate that augments `edgerun-device-cap-core` with runtime host-environment signals.
- Provide Linux-first capability probing for container context, cgroup memory limits, filesystem inventory, network inventory, and GPU inventory.
- Keep non-Linux targets buildable with deterministic `Unknown` fallback behavior.
- Ensure merged capability report represents effective/usable capabilities by applying runtime policy and permission gates.

## Non-goals
- No invasive hardware probing or privileged ioctls in V1.
- No Android-specific property/JNI probing in this phase.
- No Windows API integration in this phase.
- No mutation of system state.

## Security and constraint requirements
- Host adapter must remain read-only and side-effect free.
- Probes must avoid shelling out and use direct filesystem/process metadata reads.
- Probe failures must degrade to `Unknown`, not panic.
- Linux detection heuristics must be explicit and deterministic.

## Acceptance criteria
- New crate `crates/edgerun-device-cap-host` exists and is part of workspace.
- Crate exposes:
  - `probe_host_capabilities()` returning host capability signals,
  - `probe_device_domains()` returning domain status triplets (`detected`, `available`, `in_use`) for `cpu`, `storage`, `gpu`, `ram`, `usb`, `network`, `input`, `output`,
  - `probe_policy_context()` returning privilege/policy signals (`is_root`, selected Linux capability bits),
  - `probe_capabilities_with_host()` returning merged report using core schema.
- Linux runtime probing implemented for:
  - CPU runtime visibility (`/proc/cpuinfo` readability),
  - storage inventory (`/sys/block` or `/proc/partitions` readability),
  - RAM inventory (`/proc/meminfo` with finite `MemTotal`),
  - USB inventory (`/sys/bus/usb/devices`),
  - input inventory (`/dev/input` or `/sys/class/input`),
  - output inventory (`/sys/class/drm`, `/dev/fb0`, or `/dev/tty0`),
  - container detection (`/.dockerenv`, `/run/.containerenv`, `/proc/1/cgroup` markers),
  - memory limit detection (`/sys/fs/cgroup/memory.max`, `/sys/fs/cgroup/memory/memory.limit_in_bytes`),
  - filesystem inventory (`/proc/mounts` readability),
  - network inventory (`/sys/class/net` readability),
  - GPU inventory (`/dev/dri` or `/sys/class/drm`).
- Non-Linux targets compile and return `Unknown` host signals.
- Merged report domain statuses include detected hardware/service visibility, effective availability after policy gating, and runtime in-use heuristics.
- Unit tests validate parsing and marker-detection invariants.

## Rollout
1. Add host adapter crate and workspace wiring.
2. Switch capability consumers to call host-merged probe where `std` is available.
3. Add Android and Windows adapters in follow-up specs.

## Rollback
- Remove workspace member and host crate.
- Revert consumers to core-only probing.

## Spec alignment notes
- Preserves separation of concerns from `2026-02-28-device-capability-probing-core-v1.md`: core remains `no_std`; host adapter owns runtime/environment probing.
