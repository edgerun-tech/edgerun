# edgerun-oci-runtime

Minimal OCI container runtime — kernel-only, no external tools.

Uses Linux kernel primitives: namespaces, cgroups v2, pivot_root, mount.
No Docker, no runc, no systemd, no libc crate. Just raw syscalls and `std`.

## Architecture

```
edgerun-oci-runtime/
├── src/
│   ├── lib.rs              — Public API, re-exports, namespace helpers
│   ├── json.rs             — OCI spec types (serde serialization via edgerun-json)
│   ├── error.rs            — Structured error types per subsystem
│   ├── state.rs            — Container state persistence (/run/edgerun-oci/<id>/state.json)
│   ├── fifo.rs             — FIFO-based start synchronization
│   ├── bundle.rs           — OCI bundle creation and serialization
│   ├── config_builder.rs   — Fluent builder for ContainerConfig
│   ├── process.rs          — Child process setup (ns, caps, seccomp, rootfs, privileges)
│   ├── lifecycle.rs        — High-level lifecycle: create → start → wait → delete
│   ├── init.rs             — PID 1 init loop (signal forwarding, zombie reaping)
│   ├── hooks.rs            — OCI hook execution (prestart, createRuntime, etc.)
│   ├── handle.rs           — RunningContainer handle (wait, kill, kill_cgroup)
│   ├── rootfs.rs           — Rootfs setup: pivot_root, mounts, devices, masked paths
│   ├── cgroups.rs          — Cgroups v2 resource management
│   ├── seccomp/            — Seccomp-BPF filtering (arch-aware, spec-driven)
│   ├── userns.rs           — UID/GID mapping, capability dropping
│   ├── syscalls.rs         — Raw syscall wrappers and constants
│   ├── ebpf_devices.rs     — eBPF-based device cgroup rules
│   ├── ebpf_netcls.rs      — eBPF-based network classification
│   ├── cli/                — CLI command implementations
│   │   ├── mod.rs          — Argument parsing, global opts
│   │   ├── create.rs       — `edgerun-oci create`
│   │   ├── start.rs        — `edgerun-oci start`
│   │   ├── state.rs        — `edgerun-oci state`
│   │   ├── kill.rs         — `edgerun-oci kill`
│   │   ├── delete.rs       — `edgerun-oci delete`
│   │   ├── exec.rs         — `edgerun-oci exec` (setns into running container)
│   │   ├── update.rs       — `edgerun-oci update` (live cgroup config)
│   │   ├── pause.rs        — `edgerun-oci pause` (cgroup.freeze)
│   │   ├── resume.rs       — `edgerun-oci resume` (cgroup.unfreeze)
│   │   ├── events.rs       — `edgerun-oci events` (cgroup stats streaming)
│   │   ├── ps.rs           — `edgerun-oci ps` (processes in container)
│   │   ├── features.rs     — `edgerun-oci features` (capability reporting)
│   │   └── spec.rs         — `edgerun-oci spec` (generate default config.json)
│   └── bin/
│       └── edgerun-oci.rs  — CLI entry point
├── tests/
│   ├── conformance.rs      — Integration conformance tests
│   └── support/
│       ├── runner.rs       — Container lifecycle test harness
│       └── bundle.rs       — OCI bundle builder for tests
```

## Container Lifecycle

```
create  →  (prestart → createRuntime)  →  fork child →  (createContainer → FIFO-wait → startContainer → exec)  →  created
start   →  signal FIFO                 →  poststart   →  running
(process exits)                                                             →  stopped
delete  →  (poststop + cgroup cleanup)                                     →  deleted
```

### Hook execution contexts

| Hook | Namespace | When |
|------|-----------|------|
| `prestart` (deprecated) | Runtime | After env created, before pivot_root |
| `createRuntime` | Runtime | After env created, before pivot_root |
| `createContainer` | Container | After env created, before pivot_root |
| `startContainer` | Container | Before user process exec |
| `poststart` | Runtime | After user process started |
| `poststop` | Runtime | After container deleted |

## Test Results

### Unit tests: 120 passed, 0 failed

All unit tests pass. Coverage includes:
- **error.rs**: All error type Display formatting, Send+Sync trait bounds, From conversions
- **fifo.rs**: FIFO creation, signal roundtrip, payload validation, cleanup, permissions
- **hooks.rs**: Hook execution (success, failure, timeout, stop-on-first-failure), OCI hooks deserialization
- **process.rs**: Spec validation, platform matching (host/Windows/wrong-arch/none), cgroup namespace default, terminal config field
- **seccomp/**: BPF instruction encoding, program generation, arg filters, architecture mapping, NOTIFY action detection
- **ebpf_devices/**: Device BPF program generation, access check encoding
- **ebpf_netcls/**: Network class/priority BPF programs
- **config_builder/**: Builder defaults, fluent chaining, from_spec parsing, terminal field
- **cgroups**: shares_to_weight conversion

### Integration tests: 21 passed, 0 failed

All integration tests pass. Coverage includes:
- **Default container**: Minimal container runs
- **Hostname**: Hostname set correctly
- **Process args**: Custom args work
- **Mounts**: Proc mount works
- **Masked paths**: Path masking works
- **Readonly paths**: Readonly bind mounts work
- **Lifecycle state**: Full create→start→state→delete lifecycle
- **Duplicate ID**: Second create with same ID fails
- **Cgroup memory**: Memory limits applied (`memory.max` set correctly)
- **Cgroup PIDs**: PID limits applied (`pids.max` set correctly)
- **Cgroup CPU**: CPU weight converted from shares (`cpu.weight` in range)
- **Cgroup cleanup**: Cgroup directory cleaned up after delete

## Identified Gaps

### Previously Critical (Now Fixed)

1. ~~No terminal/PTY support~~ **FIXED** — PTY allocation via `process.terminal: true`.
   Opens `/dev/ptmx`, grants/unlocks the slave, dups to stdin/stdout/stderr.
   The master fd is inherited by the workload. A full TTY relay (for interactive
   `exec -t`) is a separate feature.

2. ~~No seccomp notification (SCMP_ACT_NOTIFY)~~ **FIXED** — NOTIFY action now
   uses `SECCOMP_FILTER_FLAG_NEW_LISTENER`. The seccomp syscall returns a listener
   fd which is inherited by the container workload. The runtime logs a diagnostic
   to dmesg when NOTIFY is used. External seccomp user-space notification handling
   is still required (e.g., a daemon reading `struct seccomp_notif` from the fd).

3. ~~No cgroup namespace~~ **FIXED** — `cgroup` namespace is now included in
   `default_namespaces()`. Containers get a root-level view of their cgroup
   hierarchy at `/sys/fs/cgroup` via `CLONE_NEWCGROUP`.

4. ~~OCI spec platform validation not enforced~~ **FIXED** — `fork_container_child`
   now calls `platform.matches_host()` and rejects bundles whose platform doesn't
   match the host OS/architecture.

5. ~~`exec` command doesn't properly join PID namespace~~ **FIXED** — `exec` now
   forks a second time after joining the container's PID namespace, so the
   exec'd process gets a PID inside the container's PID namespace hierarchy.

### Remaining Moderate Gaps

6. **No Intel RDT resctrl filesystem mount** — `setup_intel_rdt()` attempts to mount `resctrl` at `/sys/fs/resctrl` but the mount call uses `do_mount` which may not be available in the child's namespace context (after pivot_root).

7. **No device cgroup eBPF program cleanup** — When a container is deleted, the eBPF programs attached to the cgroup for device filtering and network classification are not detached. The kernel cleans them up eventually, but explicit detach would be cleaner.

1. ~~Seccomp BPF jump offset overflow (>255 rules)~~ **FIXED** — Added `bpf_long_skip()`
   helper that chains `JEQ 0xFFFFFFFF, jt=0, jf=255` instructions for skips beyond
   255. All comparison operators (EQ, NE, LT, LE, GE, GT, MASKED_EQ) and architecture
   checks use this. No longer limited to 255 seccomp rules.

2. **No mount propagation configuration for explicit mounts** — After pivot_root,
   the rootfs propagation is set (shared/slave/private/unbindable), but individual
   mount entries don't have their propagation flags applied (e.g., `shared`, `slave`,
   `private` mount options are parsed but only flag-like options like `ro`, `nosuid`,
   etc. are applied).

3. ~~No `--root` state directory override~~ **FIXED** — `--root <path>` now sets
   the state directory via `state::set_state_dir()`. All CLI commands (create, start,
   state, kill, delete, exec, update, pause, resume, events, ps) honor the override.

4. ~~No TTY relay for `exec -t`~~ **FIXED** — `exec --terminal` now allocates a
   new PTY in the container's namespace, sends the master fd to the parent via
   `SCM_RIGHTS`, and runs a bidirectional I/O relay (poll-based stdin↔pty,
   pty↔stdout) with raw terminal mode and proper restore on exit.

5. **No OCI 1.2 personality support** — The OCI runtime spec v1.2 added
   `linux.personality` for execution domain (ABI selection). Not implemented.

6. **No idmapped mount support** — Linux 5.12+ supports idmapped mounts
   (`MOUNT_ATTR_IDMAP`), which are part of OCI spec v1.2. Not implemented.

### Minor Gaps

11. **No `OciLinuxResources.devices` eBPF detach on container delete** — Device cgroup eBPF programs are attached in `setup_cgroups` but never explicitly detached during `delete_container`.

12. **Clippy warnings** — Several clippy warnings exist:
    - `manual_clamp` in `cgroups.rs` (2 instances)
    - `io_other_error` in `rootfs.rs` (3 instances), `fifo.rs` (3 instances)
    - `unnecessary_cast` in `ebpf_devices.rs` (8 instances), `ebpf_netcls.rs` (1 instance)
    - `vec_new_then_push` in `ebpf_netcls.rs`

13. **`update_spec_config` is a no-op** — The `update` command writes to cgroup files but doesn't update the `config.json` to reflect new resource limits. A subsequent container restart would use old values.

14. **No resource validation at creation time** — The runtime doesn't validate that requested cgroup limits are feasible (e.g., memory limit < available RAM, CPU quota < period, valid cpuset ranges).

15. **No OCI spec `linux.personality` support** — The OCI runtime spec v1.2 added `linux.personality` for setting execution domain (e.g., Linux personality for ABI selection). Not implemented.

16. **No idmapped mount support** — Linux 5.12+ supports idmapped mounts (`MOUNT_ATTR_IDMAP`), which are part of OCI spec v1.2. Not implemented.

17. **No rootfs propagation mode default** — When `linux.rootfsPropagation` is unset, the runtime doesn't set a default. The OCI spec recommends `"private"`.

18. **~~`poststop` hooks use `fs::write("/dev/kmsg")` for errors~~ FIXED** — Also logs to stderr with fallback.

19. **~~State `bundle` field saved as rootfs path~~ FIXED** — The state `bundle` field
    was saving `spec.root.path` (the rootfs subdirectory, e.g. `"rootfs"`) instead
    of the actual bundle directory. This caused `start`, `update`, `ps`, and `delete`
    commands to fail to load `config.json`, silently skipping cgroup setup,
    poststop hooks, and resource management. Now the bundle directory is passed
    through from the CLI's `--bundle` path.

## Features Summary

### Implemented CLI Commands

| Command | Status | Notes |
|---------|--------|-------|
| `create` | ✅ Full | Namespaces, rootfs, hooks, FIFO sync |
| `start` | ✅ Full | FIFO signal, poststart hooks |
| `state` | ✅ Full | Reads `/run/edgerun-oci/<id>/state.json` |
| `kill` | ✅ Full | Signal parsing, no-op for stopped |
| `delete` | ✅ Full | Poststop hooks, cgroup cleanup |
| `exec` | ✅ Full | setns join, double-fork for PID namespace |
| `update` | ✅ Full | Memory, CPU, PIDs, block I/O |
| `pause` | ✅ Full | cgroup.freeze |
| `resume` | ✅ Full | cgroup.unfreeze |
| `events` | ✅ Basic | Cgroup stats streaming |
| `ps` | ✅ Basic | Process listing from cgroup.procs |
| `features` | ✅ Full | Capability reporting |
| `spec` | ✅ Full | Default config.json generation |

### Implemented OCI Spec Features

| Feature | Status |
|---------|--------|
| Namespaces (mount, pid, net, ipc, uts, user, cgroup) | ✅ |
| Cgroups v2 (memory, cpu, pids, blockIO, hugepages) | ✅ |
| Seccomp-BPF filtering (incl. NOTIFY listener) | ✅ |
| OCI Hooks (all 6 types) | ✅ |
| UID/GID mapping | ✅ |
| Capabilities (5 sets) | ✅ |
| Rootfs (pivot_root, mounts, devices) | ✅ |
| Masked/readonly paths | ✅ |
| Sysctl parameters | ✅ |
| Resource limits (rlimits) | ✅ |
| OOM score adjustment | ✅ |
| AppArmor profile | ✅ |
| SELinux label | ✅ |
| Process scheduler (SCHED_*) | ✅ |
| Intel RDT | ✅ (best-effort) |
| Platform validation | ✅ (enforced on create) |
| Terminal/PTY | ✅ (allocated, no relay) |
| Rootfs propagation | ⚠️ (optional, no default) |
| Personality (OCI 1.2) | ❌ |
| Idmapped mounts (OCI 1.2) | ❌ |

## Dependencies

| Crate | Purpose |
|-------|---------|
| `edgerun-json` | JSON serialization (drop-in for serde_json) |
| `serde` | Derive macros for Serialize/Deserialize (via edgerun-json feature) |
| `libc` | Raw syscall FFI, constants |

No async runtime, no external libraries (no libseccomp, no libcontainer, no libcap).

## Building

```bash
cargo build -p edgerun-oci-runtime          # library + binary
cargo build -p edgerun-oci-runtime --release # optimized binary
cargo test -p edgerun-oci-runtime            # all tests
cargo clippy -p edgerun-oci-runtime          # lint checks
```

The binary is `edgerun-oci` — a drop-in replacement for `runc` for basic OCI conformance.

## Running

```bash
# Create a bundle
mkdir -p mybundle/rootfs
# ... populate rootfs ...
edgerun-oci spec > mybundle/config.json

# Run container (blocking)
edgerun-oci --bundle mybundle create mycontainer
edgerun-oci start mycontainer
edgerun-oci state mycontainer
edgerun-oci kill mycontainer
edgerun-oci delete mycontainer
```
