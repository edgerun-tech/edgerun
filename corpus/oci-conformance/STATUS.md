# OCI Runtime Conformance Status

> **Source**: [`opencontainers/runtime-tools`](https://github.com/opencontainers/runtime-tools) @ `8a4db57` (v0.9.0)
> **Spec**: [`opencontainers/runtime-spec`](https://github.com/opencontainers/runtime-spec) @ `6f7b71c` (v1.3.0-dev)
> **Generated**: 2026-04-10
> **Updated**: 2026-04-10 (post-audit fixes)
> **Unit tests**: 60/60 passing (`cargo test -p edgerun-oci-runtime`)
> **External dependencies**: 0 (pure kernel syscalls; `nsenter` for exec, `/bin/sh` for hooks)

---

## Summary

| Category | Total | ✅ | ⚠️ | ❌ |
|----------|-------|----|----|----|
| Lifecycle | 13 | 10 | 2 | 1 |
| Process | 7 | 7 | 0 | 0 |
| Filesystem | 4 | 4 | 0 | 0 |
| Namespace | 4 | 4 | 0 | 0 |
| Cgroup | 14 | 10 | 4 | 0 |
| Device | 1 | 1 | 0 | 0 |
| Security | 7 | 6 | 1 | 0 |
| Hook | 8 | 8 | 0 | 0 |
| Hostname | 1 | 1 | 0 | 0 |
| **Total** | **59** | **51** | **7** | **1** |

### Implementation Rate

- **Fully implemented**: 51/59 (86%)
- **Partially implemented**: 7/59 (12%)
- **Not implemented**: 1/59 (2%)

---

## Remaining Gaps

### Moderate (4)

| Gap | Spec Section | Impact |
|-----|-------------|--------|
| **Device cgroup v1-only** | `linux_cgroups_devices` | `devices.allow` is a v1 controller. On pure cgroup v2, device restrictions require eBPF — not implemented. Errors logged to `/dev/kmsg`. |
| **Network cgroup v1-only** | `linux_cgroups_network` | `net_cls.classid` / `net_prio.prioidx` are v1-only. On cgroup v2, network QoS is managed via tc/ebpf. Errors logged to `/dev/kmsg`. |
| **kill signal bypasses PID 1 init** | `killsig` | CLI `kill` sends signal directly to container PID. Library path's PID 1 init script forwards TERM/INT/QUIT to the child workload. |
| **kill_no_effect: raw signal** | `kill_no_effect` | CLI sends signal via `libc::kill` regardless of container state (with is_process_alive guard). OCI conformance test expects specific behavior for signals on stopped containers. |

### Low (3)

| Gap | Spec Section | Impact |
|-----|-------------|--------|
| **config_updates_without_affect** | Live config update | No `update`-through-spec mechanism. The `update` command writes cgroup files directly but doesn't update `config.json`. |
| **Dead clone code** | N/A | `CloneChildData`, `cloned_child_main`, `setup_child_for_create` in `process.rs` are unused. If wired up, they'd have weaker security (no AppArmor, no umask, no seccomp). |
| **No seccomp notify** | `listenerPath` | OCI spec v1.2.0+ `listenerPath`/`listenerMetadata` fields parsed but never used. |

### Not Implemented (1)

| Gap | Spec Section | Impact |
|-----|-------------|--------|
| **platform os_features** | `platform` | `os_features` field deserialized but never validated. Only `os` and `arch` are checked against the host. |

---

## Fixed Gaps (resolved in this session)

| Gap | Fix |
|-----|-----|
| **BPF operator bugs (LT/LE/GE/GT)** | Fixed opcode encoding: LT→JGE(skip,0), LE→JGT(skip,0), GE→JGE(0,skip), GT→JGT(0,skip). 5 new tests verify correct BPF encoding. |
| **MASKED_EQ incomplete** | Now does `(A & mask) == valueTwo` for both 32-bit halves. |
| **Seccomp skipped in CLI create** | CLI create now uses `fork_container_child()` → `setup_container_child()` → full seccomp. |
| **Poststop hooks never execute** | CLI delete loads spec and calls `run_poststop_and_cleanup()`. Library path stores hooks in `RunningContainer`. |
| **No cgroup cleanup on delete** | `cgroup.kill` written before `remove_dir_all` on cgroup directory. 50ms grace period. |
| **CLI start bypasses poststart hooks** | `cmd_start` loads spec and calls `run_poststart_hooks()`. |
| **ContainerState.id empty in hooks** | `make_state()` and `run_poststop_and_cleanup()` populate ID and version from spec. Annotations now from `spec.annotations` (not sysctl). |
| **External `mkfifo` dependency** | Replaced with `libc::mkfifo()` FFI syscall. Zero external binaries. |
| **Platform validation missing** | Added `OciPlatform` type with `matches_host()` check. `fork_container_child()` rejects bundles for wrong OS/arch. |

---

## CLI Commands

| Command | Hooks | Description |
|---------|-------|-------------|
| `create <id>` | prestart, createRuntime, createContainer | Fork child (blocked on FIFO), save state |
| `start <id>` | startContainer, poststart | Signal FIFO, cgroups, poststart hooks |
| `state <id>` | — | Output OCI-compatible state JSON |
| `kill <id> [signal]` | — | Send signal (no-op if process dead) |
| `delete <id> [--force]` | poststop | Kill, hooks, cleanup state/FIFO/cgroup |
| `exec <id> <cmd...>` | — | Run additional process in container namespaces via `nsenter` |
| `update <id> [flags...]` | — | Live cgroup resource updates |

---

## Architecture

```
CLI create → run_prestart_hooks() → run_create_runtime_hooks() → fork_container_child() → save_created_state()
CLI start  → signal_start() → setup_container_cgroups() → run_poststart_hooks() → update_state_running()
CLI delete → load_spec() → run_poststop_and_cleanup() → delete_state() → remove_fifo() → remove_cgroup()
CLI exec   → nsenter --target <pid> --mount --uts --ipc --net --pid
CLI update → write cgroup files directly (memory, cpu, pids, blkio, cpuset)
```

All hooks run in the correct namespace:
- **Runtime namespace** (parent process): prestart, createRuntime, poststart, poststop
- **Container namespace** (child `pre_exec`): createContainer, startContainer (between FIFO unblock and exec)
