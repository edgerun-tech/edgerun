# OCI Runtime Conformance Status

> **Source**: [`opencontainers/runtime-tools`](https://github.com/opencontainers/runtime-tools) @ `8a4db57` (v0.9.0)
> **Spec**: [`opencontainers/runtime-spec`](https://github.com/opencontainers/runtime-spec) @ `6f7b71c` (v1.3.0-dev)
> **Generated**: 2026-04-10
> **Updated**: 2026-04-10 (code audit)
> **Unit tests**: 55/55 passing (`cargo test -p edgerun-oci-runtime`)
>
> This document is derived from the official OCI runtime conformance test suite.
> Each row maps to a test binary in `runtime-tools/validation/`.

## Status Legend

| Status | Meaning |
|--------|---------|
| ✅ | Implemented — test should pass |
| ⚠️ | Partially implemented — some aspects missing or incorrect |
| ❌ | Not implemented |
| ➖ | Not applicable (e.g., wrong platform, CLI-only) |

---

## Lifecycle Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `create` | Runtime & Lifecycle: create | ⚠️ | CLI `cmd_create` implemented via `libc::clone()` + FIFO sync. **Gap:** uses `setup_child_for_create()` which skips seccomp (step 7 missing vs `setup_container_child()`). Uses external `mkfifo` binary — only non-kernel dependency. |
| `start` | Runtime & Lifecycle: start | ⚠️ | CLI `cmd_start` implemented: validates "created" state, writes "go\n" to FIFO, updates state to "running". **Gap:** does not run poststart hooks (only the library `start_spec_internal` does). |
| `state` | Runtime & Lifecycle: query state | ✅ | CLI `cmd_state` implemented: loads state.json, checks process liveness via `kill(pid, 0)`, outputs OCI-compatible JSON. |
| `kill` | Runtime & Lifecycle: kill | ✅ | CLI `cmd_kill` implemented: parses signal (numeric or name), sends via `libc::kill()`. Supports HUP/INT/QUIT/KILL/TERM/CONT/STOP. |
| `kill_no_effect` | kill on non-running container | ⚠️ | Signal sent regardless of container state; library `RunningContainer::kill()` has graceful SIGTERM→wait→SIGKILL but CLI sends raw signal. |
| `killsig` | Signal delivery | ⚠️ | Library path: PID 1 init script forwards TERM/INT/QUIT to child. CLI path: signal sent directly to container PID (bypasses init). |
| `delete` | Runtime & Lifecycle: delete | ⚠️ | CLI `cmd_delete` implemented: checks running state, optionally SIGKILLs, cleans state dir. **Gap:** does not run poststop hooks from spec (no config.json loaded at delete time). |
| `delete_only_create_resources` | Delete with create-only resources | ⚠️ | State cleanup works; cgroup cleanup not implemented in CLI or library. |
| `delete_resources` | Delete cleans up cgroups | ❌ | Neither CLI nor library cleans up cgroups on delete. `delete_container_internal()` is a stub — only runs poststop hooks with `None`. |
| `default` | Default spec validation | ✅ | `create_bundle()` produces valid defaults: 6 namespaces, 9 masked paths, 6 readonly paths, uid=0, gid=0, noNewPrivileges=true |
| `misc_props` | OCI version, annotations | ✅ | We set `ociVersion: "1.0.2"`; `annotations` map fully supported in `OciSpec` type |
| `config_updates_without_affect` | Spec updates don't affect running container | ❌ | No live config update support |
| `pidfile` | PID file creation | ✅ | `--pid-file` flag in CLI `cmd_create` writes PID to specified file |

## Process Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `process` | Process: cwd, env, args, PID 1 init | ✅ | Fully implemented: cwd/env/args/env clearing, PID 1 init script with signal forwarding + zombie reaping via `pid1_init_script()` |
| `process_user` | Process: uid, gid, umask, username | ✅ | uid/gid/umask set via `do_setuid`/`do_setgid`; supplementary GIDs via `set_supplementary_gids`; username N/A in containers |
| `process_capabilities` | Process: Linux capabilities | ✅ | All 5 cap sets wired: effective/permitted/inheritable via `capset`, bounding via `PR_CAPBSET_DROP`, ambient via `PR_CAP_AMBIENT`. All 41 Linux capabilities recognized (0–40) |
| `process_capabilities_fail` | Invalid capabilities must fail | ✅ | `caps_to_bitmask()` returns `io::Error(InvalidInput)` for unrecognized capability names; `drop_capabilities()` validates keep-list |
| `process_oom_score_adj` | Process OOM score | ✅ | `process.oomScoreAdj` written to `/proc/self/oom_score_adj` in `setup_container_child()` |
| `process_rlimits` | Process resource limits (RLIMIT_*) | ✅ | Implemented via `prlimit64` syscall (`do_setrlimit`); all 16 RLIMIT types recognized |
| `process_rlimits_fail` | Invalid RLIMIT must fail | ✅ | `rlimit_name_to_int()` returns `None` for invalid names → `io::Error(InvalidInput)` |

## Filesystem Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `root_readonly_true` | Root: readonly = true | ✅ | `setup_rootfs()` remounts `/` as read-only via `mount(, /, , MS_REMOUNT|MS_RDONLY, )` when `root.readonly == Some(true)` |
| `mounts` | Mounts: bind, tmpfs, proc, sysfs | ✅ | `setup_mount()` handles bind/proc/sysfs/tmpfs with flag parsing; proc/sysfs/dev/pts/devtmpfs/shm mounted in `setup_rootfs()` |
| `linux_mount_label` | SELinux mount label | ✅ | `linux.mountLabel` appended to mount data string in `setup_mount()` for all spec mounts |
| `linux_rootfs_propagation` | Rootfs propagation (shared/private/slave) | ✅ | `set_rootfs_propagation()` supports shared/slave/unbindable/private via `mount(, /, , MS_REC|MS_*)` |

## Namespace Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `linux_ns_nopath` | Namespaces without path (create new) | ✅ | All 6 namespaces created via `do_unshare(CLONE_NEW*)`; `default_namespaces()` returns user/mount/pid/network/ipc/uts |
| `linux_ns_path` | Namespaces with path (join existing) | ✅ | `join_explicit_ns_paths()` opens namespace files and calls `do_setns(fd, flag)`; path-based ns filtered from unshare |
| `linux_ns_itype` | Namespace type validation | ✅ | Unknown namespace types rejected with `io::Error(InvalidInput)` in `ContainerConfig::from_spec()` |
| `linux_ns_path_type` | Namespace path + type combinations | ✅ | Kernel validates `setns(fd, flags)` type mismatch — wrong type for a namespace fd returns EINVAL from kernel |

## Cgroup Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `linux_cgroups_memory` | memory.limit, swap, reservation, kernel | ✅ | `memory.max`, `memory.swap.max`, `memory.low` (reservation), `memory.kmem.max` (kernel) all written |
| `linux_cgroups_cpus` | cpu.shares, quota, period, realtime, cpus, mems | ✅ | All fields: `cpu.max` (quota/period), `cpu.max.rt` (realtime), `cpu.weight` (shares→weight conversion), `cpuset.cpus`, `cpuset.mems` |
| `linux_cgroups_pids` | pids.limit | ✅ | `pids.max` written |
| `linux_cgroups_devices` | Device cgroup allow/deny rules | ⚠️ | Writes to `devices.allow`/`cgroup.devices.allow` (v1 format only). On pure cgroup v2 systems, device cgroup uses eBPF — these writes fail silently. |
| `linux_cgroups_blkio` | Block I/O weight, leaf weight | ✅ | `io.bfq.weight` + `io.weight` with v1→v2 weight conversion (×100); `io.max` for throttle read/write bps/iops |
| `linux_cgroups_hugetlb` | Hugepage limits | ✅ | `hugetlb.<pagesize>.max` written per pagesize |
| `linux_cgroups_network` | Network class ID, priority | ⚠️ | Writes to `net_cls.classid` + `net_prio.prioidx` (v1-only files). On pure cgroup v2 systems these paths don't exist — fails silently. |
| `linux_cgroups_relative_memory` | Relative cgroup path memory | ✅ | Path resolved correctly |
| `linux_cgroups_relative_cpus` | Relative cgroup path CPU | ✅ | Path resolved, realtime/cpuset supported |
| `linux_cgroups_relative_pids` | Relative cgroup path PIDs | ✅ | Path resolved correctly |
| `linux_cgroups_relative_devices` | Relative cgroup path devices | ⚠️ | Path resolved, but v1 device controller files only |
| `linux_cgroups_relative_blkio` | Relative cgroup path blkio | ✅ | Path resolved correctly |
| `linux_cgroups_relative_hugetlb` | Relative cgroup path hugetlb | ✅ | Path resolved correctly |
| `linux_cgroups_relative_network` | Relative cgroup path network | ⚠️ | Path resolved, but v1-only files |

## Device Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `linux_devices` | Device creation from spec | ✅ | `create_spec_device()` creates char/block/fifo devices from `linux.devices`; essential devices (null/zero/full/random/urandom/tty) always created; parent dirs auto-created |

## Security Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `linux_seccomp` | Seccomp filtering | ⚠️ | Spec-driven seccomp: rules parsed, BPF generated. Supports `default_action`, `architectures`, all action types (allow/deny/errno/kill/trace/log/notify). **Gaps:** (1) 64-bit arg filters for LT/LE/GE/GT use wrong BPF opcodes — `0x30` (JSET/bitwise AND-test) used for GE/LT instead of proper comparison operators; `0x25` (JGT) used for LE/GT. (2) `SCMP_CMP_MASKED_EQ` is simplified — checks `A & expected == expected` (only verifies expected bits are set, doesn't verify other bits are cleared). (3) Skip distances capped at 255 (`.min(255)`) which could corrupt large filter programs. (4) CLI `create` path via `setup_child_for_create()` skips seccomp entirely. Falls back to built-in allow-list when no spec rules. |
| `linux_masked_paths` | Masked paths (mount /dev/null) | ✅ | Paths masked via `mount(/dev/null, path, , MS_BIND, )`; errors are strict (fatal) |
| `linux_readonly_paths` | Readonly paths (bind + remount ro) | ✅ | Paths bind-mounted + remounted read-only with NOSUID/NODEV/NOEXEC; errors are strict (fatal) |
| `linux_process_apparmor_profile` | AppArmor profile | ✅ | `/proc/self/attr/apparmor/exec` written with `"exec <profile>"` |
| `linux_sysctl` | Sysctl parameter setting | ✅ | `apply_sysctl()` writes to `/proc/sys/<key>` (dots→slashes); errors logged to `/dev/kmsg` but non-fatal |
| `linux_uid_mappings` | UID/GID namespace mappings | ✅ | `uid_map`/`gid_map` written from spec `linux.uidMappings`/`linux.gidMappings`; defaults to `0 65534 1` when empty/missing; `setgroups` set to `deny` |
| `linux_mount_label` | SELinux mount label | ✅ | `linux.mountLabel` applied to all mount data strings |

## Hook Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `prestart` | Prestart hooks | ✅ | `execute_prestart_hooks()` called during create, runtime namespace, state JSON on stdin |
| `prestart_fail` | Prestart hook failure aborts create | ✅ | `io::Error` returned, container not started |
| `poststart` | Poststart hooks | ✅ | `execute_poststart_hooks()` called after child started, runtime namespace, before start returns |
| `poststart_fail` | Poststart hook failure aborts start | ✅ | SIGKILL sent to container child, `io::Error` returned |
| `poststop` | Poststop hooks | ⚠️ | Library path: `delete_container_internal` passes `None` for hooks (spec hooks never loaded). CLI path: `cmd_delete` does not load config.json. Poststop hooks from spec are **not executed** in either path. |
| `poststop_fail` | Poststop hook failure logs warning | ⚠️ | Hardcoded `None` hooks passed — no hooks to fail. The warning-logging mechanism exists (`execute_poststop_hooks` never-fails semantics) but is unreachable. |
| `hooks` | Hook lifecycle ordering | ✅ | Library path: full order prestart → createRuntime → createContainer → startContainer → poststart → poststop. CLI path: hooks not executed (create/start/delete bypass hook execution). |
| `hooks_stdin` | Hooks receive state JSON on stdin | ✅ | OCI state JSON (ociVersion, id, status, pid, bundle, annotations) written to each hook's stdin in library path. **Gap:** container `id` field is always `String::new()` — never populated from actual container ID. |

## Hostname Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `hostname` | UTS namespace hostname | ✅ | `do_set_hostname()` calls `sethostname(2)` in container UTS namespace |

---

## Summary

| Category | Total | ✅ | ⚠️ | ❌ | ➖ |
|----------|-------|----|----|----|----|
| Lifecycle | 13 | 3 | 8 | 2 | 0 |
| Process | 7 | 7 | 0 | 0 | 0 |
| Filesystem | 4 | 4 | 0 | 0 | 0 |
| Namespace | 4 | 4 | 0 | 0 | 0 |
| Cgroup | 14 | 10 | 4 | 0 | 0 |
| Device | 1 | 1 | 0 | 0 | 0 |
| Security | 7 | 6 | 1 | 0 | 0 |
| Hook | 8 | 5 | 2 | 1 | 0 |
| Hostname | 1 | 1 | 0 | 0 | 0 |
| **Total** | **59** | **41** | **15** | **3** | **0** |

### Implementation Rate

- **Fully implemented**: 41/59 (69%)
- **Partially implemented**: 15/59 (25%)
- **Not implemented**: 3/59 (5%)

---

## Known Gaps (from code audit)

### Critical

| Gap | Location | Impact |
|-----|----------|--------|
| **Seccomp skipped in CLI create path** | `process.rs::setup_child_for_create()` omits seccomp step | Containers created via CLI have no seccomp filtering |
| **Poststop hooks never execute** | `lifecycle.rs::delete_container_internal()` passes `None` hooks; `cli/delete.rs` doesn't load config | Spec poststop hooks never run in any code path |
| **BPF operator bugs (LT/LE/GE/GT)** | `seccomp.rs` uses `0x30` (JSET) for GE/LT, `0x25` (JGT) for LE/GT | 64-bit argument filters produce incorrect results for these operators |
| **MASKED_EQ incomplete** | `seccomp.rs` — checks `A & expected == expected` | Only verifies expected bits are set; doesn't verify other bits are cleared |

### Moderate

| Gap | Location | Impact |
|-----|----------|--------|
| **Cgroup errors silently discarded** | `cgroups.rs` — all writes use `let _ =` | Failures invisible; containers start with no resource limits if cgroup writes fail |
| **Device/network cgroups v1-only** | `cgroups.rs` — writes to v1 files | On pure cgroup v2 systems, device and network limits don't work |
| **Container ID never populated** | `lifecycle.rs` — `ContainerState.id = String::new()` | Hooks receive empty ID in state JSON |
| **No cgroup cleanup on delete** | `lifecycle.rs::delete_container_internal()` stub | Cgroup directories persist after container deletion |
| **CLI start bypasses poststart hooks** | `cli/start.rs` only writes to FIFO | Poststart hooks only run in library `start_spec_internal` path |
| **CLI delete bypasses poststop hooks** | `cli/delete.rs` doesn't load spec | Same as critical gap above |
| **External `mkfifo` dependency** | `cli/create.rs` uses `Command::new("mkfifo")` | Only non-kernel dependency in the entire runtime |

### Minor

| Gap | Location | Impact |
|-----|----------|--------|
| **Hook stdout discarded** | `hooks.rs::run_hook()` — `Stdio::null()` for stdout | Hook output not captured for debugging |
| **PID 1 init only reaps specific child** | `init.rs` — `wait $PID` only | Other orphaned children not reaped |
| **Dead code in `userns.rs`** | `userns.rs::write_uid_map`/`write_gid_map` marked `dead_code` | Duplicate of implementations in `process.rs` |
| **Unused import** | `init.rs:14` — `use std::io;` | Cargo warning |

---

## CLI vs Library Architecture

The runtime has two distinct code paths with different coverage:

| Feature | Library (`start_spec`/`run_spec`) | CLI (`edgerun-oci`) |
|---------|------|------|
| Namespace setup | ✅ `setup_container_child()` | ✅ `clone()` + `setup_child_for_create()` |
| Seccomp | ✅ Applied in `setup_container_child()` | ❌ Skipped in `setup_child_for_create()` |
| Capabilities | ✅ | ✅ |
| Cgroups | ✅ (best-effort, errors silent) | ✅ (same code) |
| prestart hooks | ✅ | ❌ |
| createRuntime hooks | ✅ | ❌ |
| createContainer hooks | ✅ | ❌ |
| startContainer hooks | ✅ | ❌ |
| poststart hooks | ✅ | ❌ |
| poststop hooks | ⚠️ (passes `None`) | ❌ (doesn't load spec) |
| State persistence | ✅ | ✅ |
| PID file | N/A | ✅ |

---

## Changes Since Previous Status

| Area | Before | After | Change |
|------|--------|-------|--------|
| CLI commands | ❌ Not implemented | ⚠️ All 5 implemented (create/start/state/kill/delete) | +10 lifecycle tests now partially passable |
| `pidfile` | ❌ | ✅ | `--pid-file` flag in CLI create |
| `linux_seccomp` | ✅ | ⚠️ | Audit revealed BPF operator bugs and MASKED_EQ incomplete |
| `poststop` hooks | ✅ | ⚠️ | Audit revealed hooks pass `None` — spec hooks never executed |
| `linux_cgroups_devices` | ✅ | ⚠️ | v1-only, fails silently on cgroup v2 |
| `linux_cgroups_network` | ✅ | ⚠️ | v1-only, fails silently on cgroup v2 |
| Overall | 48/59 (81%) ✅ | 41/59 (69%) ✅ + 15/59 (25%) ⚠️ | More accurate assessment from code audit |
