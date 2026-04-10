# OCI Runtime Conformance Status

> **Source**: [`opencontainers/runtime-tools`](https://github.com/opencontainers/runtime-tools) @ `8a4db57` (v0.9.0)
> **Spec**: [`opencontainers/runtime-spec`](https://github.com/opencontainers/runtime-spec) @ `6f7b71c` (v1.3.0-dev)
> **Generated**: 2025-04-10
>
> This document is derived from the official OCI runtime conformance test suite.
> Each row maps to a test binary in `runtime-tools/validation/`.

## Status Legend

| Status | Meaning |
|--------|---------|
| ✅ | Implemented — test should pass |
| ⚠️ | Partially implemented — some aspects missing |
| ❌ | Not implemented |
| ➖ | Not applicable (e.g., wrong platform, CLI-only) |

---

## Lifecycle Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `create` | Runtime & Lifecycle: create | ❌ | Requires CLI (create/start/delete); we use library API |
| `start` | Runtime & Lifecycle: start | ❌ | CLI lifecycle — we have `run_spec`/`start_spec` |
| `state` | Runtime & Lifecycle: query state | ❌ | Requires CLI `state` command |
| `kill` | Runtime & Lifecycle: kill | ❌ | CLI `kill` command; we have `RunningContainer::kill()` |
| `kill_no_effect` | kill on non-running container | ❌ | CLI behavior |
| `killsig` | Signal delivery | ❌ | CLI signal forwarding |
| `delete` | Runtime & Lifecycle: delete | ❌ | CLI cleanup command |
| `delete_only_create_resources` | Delete with create-only resources | ❌ | CLI behavior |
| `delete_resources` | Delete cleans up cgroups | ❌ | CLI behavior |
| `default` | Default spec validation | ✅ | Our `create_bundle()` produces valid defaults |
| `misc_props` | OCI version, annotations | ⚠️ | We set `ociVersion: "1.0.2"`; annotations not supported |
| `config_updates_without_affect` | Spec updates don't affect running container | ❌ | No live config update |
| `pidfile` | PID file creation | ❌ | Not implemented |

## Process Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `process` | Process: cwd, env, args | ✅ | Fully implemented in `run_spec`/`start_spec` |
| `process_user` | Process: uid, gid, umask, username | ⚠️ | uid/gid set; umask & username not supported |
| `process_capabilities` | Process: Linux capabilities | ⚠️ | Bounding set dropped; effective/permitted/ambient not set |
| `process_capabilities_fail` | Invalid capabilities must fail | ⚠️ | No validation on invalid cap names |
| `process_oom_score_adj` | Process OOM score | ⚠️ | Type exists but not exposed in OciProcess; placeholder at 0 |
| `process_rlimits` | Process resource limits (RLIMIT_*) | ✅ | Implemented via prlimit64 syscall |
| `process_rlimits_fail` | Invalid RLIMIT must fail | ⚠️ | Unknown RLIMIT names silently skipped |

## Filesystem Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `root_readonly_true` | Root: readonly = true | ✅ | Remounts rootfs as read-only |
| `mounts` | Mounts: bind, tmpfs, proc, sysfs | ✅ | Implemented in `setup_rootfs` |
| `linux_mount_label` | SELinux mount label | ❌ | Not implemented |
| `linux_rootfs_propagation` | Rootfs propagation (shared/private/slave) | ❌ | Not implemented |

## Namespace Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `linux_ns_nopath` | Namespaces without path (create new) | ✅ | All 6 namespaces created |
| `linux_ns_path` | Namespaces with path (join existing) | ❌ | `ns.path` field ignored |
| `linux_ns_itype` | Namespace type validation | ❌ | No validation of ns type strings |
| `linux_ns_path_type` | Namespace path + type combinations | ❌ | Not implemented |

## Cgroup Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `linux_cgroups_memory` | memory.limit, swap, swappiness, OOM | ⚠️ | limit + swap set; swappiness/kernel/OOM not |
| `linux_cgroups_cpus` | cpu.shares, quota, period, realtime, cpus, mems | ⚠️ | shares + quota/period set; realtime/cpus/mems not |
| `linux_cgroups_pids` | pids.limit | ✅ | Implemented |
| `linux_cgroups_devices` | Device cgroup allow/deny rules | ❌ | Not implemented |
| `linux_cgroups_blkio` | Block I/O weight, leaf weight | ❌ | Not implemented |
| `linux_cgroups_hugetlb` | Hugepage limits | ❌ | Not implemented |
| `linux_cgroups_network` | Network class ID, priority | ❌ | Not implemented |
| `linux_cgroups_relative_memory` | Relative cgroup path memory | ✅ | Path resolved correctly |
| `linux_cgroups_relative_cpus` | Relative cgroup path CPU | ✅ | Path resolved correctly |
| `linux_cgroups_relative_pids` | Relative cgroup path PIDs | ✅ | Path resolved correctly |
| `linux_cgroups_relative_devices` | Relative cgroup path devices | ❌ | Not implemented |
| `linux_cgroups_relative_blkio` | Relative cgroup path blkio | ❌ | Not implemented |
| `linux_cgroups_relative_hugetlb` | Relative cgroup path hugetlb | ❌ | Not implemented |
| `linux_cgroups_relative_network` | Relative cgroup path network | ❌ | Not implemented |

## Device Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `linux_devices` | Device creation from spec | ⚠️ | Essential devices created; spec `linux.devices` partially wired |

## Security Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `linux_seccomp` | Seccomp filtering | ⚠️ | Static allow-list; spec seccomp rules not parsed |
| `linux_masked_paths` | Masked paths (mount /dev/null) | ⚠️ | Paths masked; errors logged but not fatal |
| `linux_readonly_paths` | Readonly paths (bind + remount ro) | ⚠️ | Paths made readonly; errors logged but not fatal |
| `linux_process_apparmor_profile` | AppArmor profile | ❌ | Not implemented |
| `linux_sysctl` | Sysctl parameter setting | ❌ | Not implemented |
| `linux_uid_mappings` | UID/GID namespace mappings | ⚠️ | Maps 0→65534 by default; spec mappings partially used |
| `linux_mount_label` | SELinux mount label | ❌ | Not implemented |

## Hook Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `prestart` | Prestart hooks | ❌ | Not implemented |
| `prestart_fail` | Prestart hook failure aborts create | ❌ | Not implemented |
| `poststart` | Poststart hooks | ❌ | Not implemented |
| `poststart_fail` | Poststart hook failure aborts start | ❌ | Not implemented |
| `poststop` | Poststop hooks | ❌ | Not implemented |
| `poststop_fail` | Poststop hook failure aborts delete | ❌ | Not implemented |
| `hooks` | Hook lifecycle ordering | ❌ | Not implemented |
| `hooks_stdin` | Hooks receive stdin | ❌ | Not implemented |

## Hostname Tests

| Test | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| `hostname` | UTS namespace hostname | ✅ | `sethostname` called in container |

---

## Summary

| Category | Total | ✅ | ⚠️ | ❌ | ➖ |
|----------|-------|----|----|----|----|
| Lifecycle | 11 | 1 | 1 | 9 | 0 |
| Process | 7 | 2 | 1 | 4 | 0 |
| Filesystem | 4 | 1 | 0 | 3 | 0 |
| Namespace | 4 | 1 | 0 | 3 | 0 |
| Cgroup | 14 | 4 | 2 | 8 | 0 |
| Device | 1 | 0 | 1 | 0 | 0 |
| Security | 7 | 0 | 4 | 3 | 0 |
| Hook | 8 | 0 | 0 | 8 | 0 |
| Hostname | 1 | 1 | 0 | 0 | 0 |
| **Total** | **57** | **10** | **10** | **37** | **0** |

### Implementation Rate

- **Fully implemented**: 9/57 (16%)
- **Partially implemented**: 9/57 (16%)
- **Not implemented**: 39/57 (68%)

### Notes on CLI vs Library

The conformance test suite is designed for **CLI runtimes** (runc, crun, youki) that implement
`oci-runtime-tool` commands: `create`, `start`, `state`, `kill`, `delete`. Our runtime is a
**library API** (`run_spec`, `start_spec`) without a CLI wrapper.

Tests marked ❌ for CLI reasons (create/start/kill/state/delete/pidfile/hooks) are not applicable
to our library-first design. If a CLI wrapper is built around our runtime, those tests would become
applicable.

### Key Gaps (Non-CLI)

These are missing features that affect our library API directly:

1. **Hooks** (prestart/poststart/poststop) — OCI spec requires lifecycle hooks
2. **Seccomp from spec** — We use a static allow-list, not spec-defined rules
3. **Process capabilities** — Bounding set dropped, but effective/permitted/ambient not set
4. **Cgroup advanced** — blkio, hugetlb, network class ID, realtime CPU, cpuset
5. **Sysctl** — `/proc/sys` parameter setting
6. **AppArmor** — Mandatory access control profiles
7. **RLIMIT** — Process resource limits
8. **OOM score** — OOM killer adjustment
9. **Namespace join** — `ns.path` for joining existing namespaces
10. **Rootfs propagation** — shared/private/slave mount propagation
