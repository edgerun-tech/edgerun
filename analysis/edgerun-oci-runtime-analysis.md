# edgerun-oci-runtime Analysis

## Overview

`edgerun-oci-runtime` is a **minimal OCI-compliant container runtime** written in pure Rust. It implements the [Open Container Initiative (OCI) Runtime Specification](https://github.com/opencontainers/runtime-spec), enabling the creation and management of Linux containers without external dependencies like runc, Docker, or systemd.

### Key Characteristics

- **Zero external dependencies**: No libc crate, no libseccomp, no cgroup libraries — only raw syscalls
- **Kernel-native**: Uses direct Linux kernel primitives (namespaces, cgroups v2, pivot_root, mount)
- **OCI spec compliant**: Implements the full container lifecycle with hook integration
- **Security-first**: Seccomp-BPF filtering, capability bounding, no_new_privs, masked paths
- **Architecture-aware**: Supports both x86_64 and aarch64 with architecture-specific syscall numbers and seccomp rules

## Module Architecture

### Core Modules

| Module | Purpose | Lines |
|--------|---------|-------|
| `lib.rs` | Public API, OCI spec types, namespace helpers, tests | ~658 |
| `json.rs` | OCI spec type definitions with serde serialization | ~450 |
| `syscalls.rs` | Raw syscall FFI declarations and wrappers | ~250 |
| `seccomp.rs` | Seccomp-BPF program generation and application | ~849 |
| `process.rs` | Child process setup in pre_exec closures | ~400 |
| `rootfs.rs` | Root filesystem setup: pivot_root, mounts, devices | ~350 |
| `cgroups.rs` | Cgroups v2 resource management | ~200 |
| `userns.rs` | UID/GID mapping and capability management | ~200 |
| `lifecycle.rs` | Container lifecycle: create, start, delete with hooks | ~250 |
| `hooks.rs` | OCI runtime hook execution (prestart, createRuntime, etc.) | ~450 |
| `bundle.rs` | OCI bundle creation and config.json serialization | ~70 |
| `container.rs` | Container lifecycle facade (run_bundle, start_bundle) | ~50 |
| `handle.rs` | RunningContainer handle for await/kill operations | ~70 |
| `init.rs` | Minimal PID 1 init process for containers | ~60 |
| `state.rs` | Container state persistence to /run/edgerun-oci/ | ~70 |

### CLI Module

| File | Purpose |
|------|---------|
| `cli/mod.rs` | Argument parsing, global options, command dispatch |
| `cli/create.rs` | Create command: clone() with namespaces, FIFO setup |
| `cli/start.rs` | Start command: signal container via FIFO |
| `cli/state.rs` | State command: output container state JSON |
| `cli/kill.rs` | Kill command: send signals to container |
| `cli/delete.rs` | Delete command: cleanup container resources |
| `bin/edgerun-oci.rs` | CLI binary entry point |

## OCI Specification Implementation

### Type System (`json.rs`)

The crate implements a comprehensive subset of the OCI Runtime Spec v1.0.2:

#### OciSpec
- `ociVersion`: String
- `process`: OciProcess (args, env, cwd, user, capabilities, rlimits, noNewPrivileges, oomScoreAdj, apparmorProfile)
- `root`: OciRoot (path, readonly)
- `hostname`: String
- `linux`: OciLinux (namespaces, resources, devices, masked_paths, readonly_paths, sysctl, hooks, seccomp)
- `mounts`: Vec<OciMount>
- `annotations`: HashMap<String, String>

#### Security Types
- **OciLinuxSeccomp**: Full seccomp configuration with default_action, architectures, syscalls, argument filters
- **OciSeccompAction**: All 9 actions (Kill, KillProcess, KillThread, Trap, Errno, Trace, Allow, Notify, Log)
- **OciCapabilities**: All 5 capability sets (bounding, effective, inheritable, permitted, ambient)
- **OciLinuxDevice**: Device creation from spec (char, block, fifo)

#### Resource Types
- **OciLinuxResources**: memory, cpu, pids, block_io, hugepage_limits, network
- **OciLinuxMemory**: limit, reservation, swap, kernel, kernelTCP
- **OciLinuxCpu**: shares, quota, period, realtime_runtime, realtime_period, cpus, mems
- **OciLinuxBlockIO**: weight, throttle devices (read/write bps/iops)
- **OciLinuxHugepageLimit**: pagesize, limit
- **OciLinuxNetwork**: class_id, priorities

## Container Lifecycle

### OCI Lifecycle Implementation

The runtime follows the official OCI runtime lifecycle:

```
create → (prestart → createRuntime → createContainer) → created
start  → (startContainer → exec process → poststart)   → running
(process exits)                                        → stopped
delete → (undo create → poststop)                      → deleted
```

### Two Execution Modes

#### 1. Library API (`run_spec` / `start_spec`)

Used by `edgerun-node` for programmatic container management:

```rust
// Blocking execution
let exit_status = run_spec(&oci_spec)?;

// Non-blocking execution
let container = start_spec(&oci_spec)?;
// ... do other work ...
container.wait()?;
delete_container(&container);
```

**Implementation flow** (`lifecycle.rs`):
1. Parse `ContainerConfig` from OCI spec
2. Extract hooks from spec
3. Execute **prestart** hooks (runtime namespace)
4. Execute **createRuntime** hooks (runtime namespace)
5. Build `Command` with process args
6. Register `pre_exec` closure that:
   - Calls `setup_container_child()` (namespaces, rootfs, security)
   - Executes **createContainer** hooks (container namespace)
   - Executes **startContainer** hooks (container namespace)
7. Spawn child process
8. Setup cgroups
9. Execute **poststart** hooks (runtime namespace)
10. Return `RunningContainer` handle

#### 2. CLI API (`edgerun-oci` binary)

Used by OCI conformance tests (`runtime-tools`):

```bash
edgerun-oci create <container-id> --bundle /path/to/bundle
edgerun-oci start <container-id>
edgerun-oci state <container-id>
edgerun-oci kill <container-id> <signal>
edgerun-oci delete <container-id>
```

**Implementation flow** (`cli/create.rs`):
1. Read `config.json` from bundle directory
2. Create FIFO at `/run/edgerun-oci/<id>/start.fifo` for start synchronization
3. Calculate namespace clone flags from spec
4. Call `libc::clone()` with namespace flags and custom stack
5. Child process runs `cloned_child_main()` which:
   - Reads config from bundle
   - Calls `setup_child_for_create()` (rootfs, security, privileges)
   - Waits on FIFO for start signal
   - Execs container process
6. Parent saves state to `/run/edgerun-oci/<id>/state.json`
7. Parent writes PID to pid-file if `--pid-file` specified

**Start command** (`cli/start.rs`):
1. Load state from disk
2. Verify container is in "created" state
3. Write "go\n" to FIFO to unblock child
4. Update state to "running"

### PID 1 Init Process (`init.rs`)

When PID namespace is enabled, the runtime generates a minimal PID 1 init shim:

```sh
#!/bin/sh
# Forward signals to the child
cleanup() {
    kill -$1 $PID 2>/dev/null
}

trap 'cleanup 15' TERM
trap 'cleanup 2' INT
trap 'cleanup 3' QUIT

# Run workload in background
{workload} &
PID=$!

# Reap zombies in a loop
while true; do
    wait $PID 2>/dev/null
    EXIT_CODE=$?
    while kill -0 $PID 2>/dev/null; do
        sleep 0.1
    done
    exit $EXIT_CODE
done
```

**Key features**:
- Signal forwarding (SIGTERM, SIGINT, SIGQUIT) to child workload
- Zombie reaping via wait loop
- Exit code propagation

## Security Implementation

### 1. Namespace Isolation (`syscalls.rs`, `process.rs`)

Six Linux namespaces isolated by default:
- **user**: UID/GID mapping (user namespaces)
- **mount**: Filesystem namespace
- **pid**: Process ID namespace
- **network**: Network stack isolation
- **ipc**: Inter-process communication
- **uts**: Hostname and domainname

**Clone flags** (stable since kernel 2.6):
```rust
NEWNS:     0x00020000  // mount
NEWCGROUP: 0x02000000  // cgroup
NEWUTS:    0x04000000  // uts
NEWIPC:    0x08000000  // ipc
NEWUSER:   0x10000000  // user
NEWPID:    0x20000000  // pid
NEWNET:    0x40000000  // network
CONTAINER: 0x6e020000  // all six namespaces
```

### 2. UID/GID Mapping (`process.rs`)

- Only writes uid_map/gid_map if actually in a user namespace (compares `/proc/self/ns/user` vs `/proc/1/ns/user`)
- Falls back to current UID if not in user namespace
- Writes "deny" to `/proc/self/setgroups` to disable setgroups (required for user namespaces)

### 3. Seccomp-BPF Filtering (`seccomp.rs`)

**Two modes of operation**:

#### A. Spec-driven seccomp (from OCI config)
- Parses `linux.seccomp` from OCI spec
- Generates BPF program from seccomp rules
- Supports all 9 seccomp actions: Kill, KillProcess, KillThread, Trap, Errno, Trace, Allow, Notify, Log
- Supports 64-bit argument filters with operators: EQ, NE, LT, LE, GE, GT
- Architecture validation (x86_64: 0xc000003e, aarch64: 0xc00000b7)
- Custom errno return values

#### B. Built-in allow-list fallback
When no spec rules provided, uses a built-in allow-list:
- **x86_64**: ~300 syscalls allowed
- **aarch64**: ~300 syscalls allowed
- Fail-closed: if seccomp can't be applied, container startup aborts

**BPF program structure**:
1. Validate architecture (kill if mismatch)
2. Load syscall number
3. Check against allow-list/deny-list via JEQ jumps
4. Return RET_ALLOW, RET_ERRNO, or RET_KILL_THREAD

### 4. Capability Management (`userns.rs`)

**Five capability sets** (OCI spec):
1. **bounding**: Drop all capabilities not in bounding set via `prctl(PR_CAPBSET_DROP)`
2. **effective**: Set via `capset()` syscall
3. **inheritable**: Set via `capset()` syscall
4. **permitted**: Set via `capset()` syscall
5. **ambient**: Set via `prctl(PR_CAP_AMBIENT, RAISE)` (requires CAP_SETPCAP)

**All 41 Linux capabilities recognized** (0-40):
```
CAP_CHOWN, CAP_DAC_OVERRIDE, CAP_DAC_READ_SEARCH, CAP_FOWNER, CAP_FSETID,
CAP_KILL, CAP_SETGID, CAP_SETUID, CAP_SETPCAP, CAP_LINUX_IMMUTABLE,
CAP_NET_BIND_SERVICE, CAP_NET_BROADCAST, CAP_NET_ADMIN, CAP_NET_RAW,
CAP_IPC_LOCK, CAP_IPC_OWNER, CAP_SYS_MODULE, CAP_SYS_RAWIO, CAP_SYS_CHROOT,
CAP_SYS_PTRACE, CAP_SYS_PACCT, CAP_SYS_ADMIN, CAP_SYS_BOOT, CAP_SYS_NICE,
CAP_SYS_RESOURCE, CAP_SYS_TIME, CAP_SYS_TTY_CONFIG, CAP_MKNOD, CAP_LEASE,
CAP_AUDIT_WRITE, CAP_AUDIT_CONTROL, CAP_SETFCAP, CAP_MAC_OVERRIDE, CAP_MAC_ADMIN,
CAP_SYSLOG, CAP_WAKE_ALARM, CAP_BLOCK_SUSPEND, CAP_AUDIT_READ, CAP_PERFMON,
CAP_BPF, CAP_CHECKPOINT_RESTORE
```

Invalid capability names cause errors (per OCI spec requirement).

### 5. Security Hardening (`userns.rs`)

- **no_new_privs**: Set via `prctl(PR_SET_NO_NEW_PRIVS, 1)`
- **non-dumpable**: Set via `prctl(PR_SET_DUMPABLE, 0)` to prevent core dumps and ptrace
- Applied before capabilities and seccomp

### 6. Rootfs Security (`rootfs.rs`)

- **Read-only rootfs**: When `root.readonly == true`, remounts `/` as read-only AFTER pivot_root but BEFORE mounting writable filesystems
- **Masked paths**: Mount `/dev/null` over sensitive paths (/proc/acpi, /proc/kcore, etc.)
- **Readonly paths**: Bind mount and remount read-only (/proc/sys, /proc/asound, etc.)
- **Mount validation**: Reject mount destinations that escape rootfs (path traversal check)

### 7. Resource Limits (`syscalls.rs`, `process.rs`)

- **RLIMIT_***: All 16 resource limit types via `prlimit64` syscall
- Set in pre_exec before execve
- Examples: RLIMIT_NOFILE, RLIMIT_NPROC, RLIMIT_MEMLOCK, RLIMIT_CORE

## Root Filesystem Setup (`rootfs.rs`)

### Pivot Root Process

1. **Bind mount rootfs** to make it a mount point (`mount(rootfs, rootfs, "bind", MS_BIND|MS_REC)`)
2. **Make / private** to prevent mount propagation to host (`mount("", "/", "", MS_PRIVATE|MS_REC)`)
3. **Create old_root** inside rootfs (`.oci-old-root`)
4. **Change CWD** to rootfs (pivot_root requires CWD under new_root)
5. **Pivot root** (`pivot_root(".", ".oci-old-root")`)
6. **Unmount old root** (`umount2("/.oci-old-root", MNT_DETACH)`)
7. **Remove old root** directory

### Filesystem Mounts

**Automatically mounted**:
- `/proc` (proc, nosuid|nodev|noexec)
- `/sys` (sysfs, nosuid|nodev|noexec)
- `/dev` (tmpfs, nosuid|strictatime, mode=755, size=65536k)
- `/dev/pts` (devpts, nosuid|noexec, newinstance)
- `/dev/shm` (tmpfs, nosuid|nodev, mode=1777)

**Essential devices created**:
- `/dev/null` (1:3, mode 0666)
- `/dev/zero` (1:5, mode 0666)
- `/dev/full` (1:7, mode 0666)
- `/dev/random` (1:8, mode 0444)
- `/dev/urandom` (1:9, mode 0444)
- `/dev/tty` (5:0, mode 0666)

**Overlay whiteout handling**:
- OCI-style `.wh.<name>` prefix files
- Overlayfs char device whiteouts (0:0 device, rdev() == 0)

### Sysctl Support

- Converts dots to slashes: `net.ipv4.ip_forward` → `/proc/sys/net/ipv4/ip_forward`
- Best-effort: failures logged to `/dev/kmsg`

### Rootfs Propagation

Supports all four modes:
- **shared** (MS_REC | MS_SHARED = 0x100)
- **slave** (MS_REC | MS_SLAVE = 0x200)
- **unbindable** (MS_REC | MS_UNBINDABLE = 0x400)
- **private** (MS_REC | MS_PRIVATE = 1<<18)

## Cgroups v2 Implementation (`cgroups.rs`)

Pure file I/O to `/sys/fs/cgroup/<path>/` — no cgroup library dependency.

### Supported Controllers

| Controller | Files Written | Notes |
|------------|---------------|-------|
| **Memory** | `memory.max`, `memory.swap.max`, `memory.low`, `memory.kmem.max` | limit, swap, reservation, kernel |
| **CPU** | `cpu.max`, `cpu.max.rt`, `cpu.weight`, `cpuset.cpus`, `cpuset.mems` | quota/period, realtime, shares→weight conversion |
| **PIDs** | `pids.max` | Max process count |
| **Block I/O** | `io.bfq.weight`, `io.weight`, `io.max` | Weight + throttle (rbps/wbps/riops/wiops) |
| **Hugepages** | `hugetlb.<pagesize>.max` | Per pagesize limits |
| **Network** | `net_cls.classid`, `net_prio.prioidx` | v1 compatibility |
| **Devices** | `devices.allow`, `cgroup.devices.allow` | v1 compatibility (v2 uses eBPF) |

### CPU Shares → Weight Conversion

Legacy cgroup v1 `cpu.shares` (2-262144) mapped to v2 `cpu.weight` (1-10000):

```rust
pub fn shares_to_weight(shares: u64) -> u64 {
    if shares <= 2 { return 1; }
    let w = 1 + (shares - 2).saturating_mul(9999) / 262142;
    w.min(10000).max(1)
}
```

Uses saturating arithmetic to prevent overflow.

## Hook System (`hooks.rs`)

### Hook Types

| Hook | Namespace | Timing | Failure Behavior |
|------|-----------|--------|------------------|
| **prestart** | Runtime | After env created, before pivot_root (deprecated) | Stop container |
| **createRuntime** | Runtime | After env created, before pivot_root | Stop container |
| **createContainer** | Container | After namespace setup, before pivot_root | Stop container |
| **startContainer** | Container | Before exec | Stop container |
| **poststart** | Runtime | After exec starts | Stop container |
| **poststop** | Runtime | After container deleted | Log warning, continue |

### Hook Execution

- Hooks receive container state JSON on stdin
- State format matches OCI runtime spec state schema:
  ```json
  {
    "ociVersion": "1.0.2",
    "id": "container-id",
    "status": "creating",
    "pid": 12345,
    "bundle": "/path/to/bundle",
    "annotations": {}
  }
  ```
- Hooks executed in listed order
- Timeout support (per-hook `timeout` field in seconds)
- Hooks run as external processes with stdin/stdout/stderr

### Hook Error Semantics

- **prestart/createRuntime/createContainer/startContainer/poststart**: Non-zero exit or timeout → stop container, jump to cleanup
- **poststop**: Non-zero exit or timeout → log warning to `/dev/kmsg`, continue with remaining hooks

## State Management (`state.rs`)

Container state persisted to `/run/edgerun-oci/<id>/`:

```
/run/edgerun-oci/
  <id>/
    state.json    # Container state
    start.fifo    # FIFO for create/start synchronization
```

### State JSON Format

```json
{
  "ociVersion": "1.0.2",
  "id": "container-id",
  "status": "created",  // "creating" | "created" | "running" | "stopped"
  "pid": 12345,
  "bundle": "/path/to/bundle",
  "annotations": {}
}
```

## Child Process Setup (`process.rs`)

The `setup_container_child()` function runs in a `pre_exec` closure and implements all container setup in order:

1. **Unshare namespaces** (`do_unshare(cfg.ns_flags)`)
2. **Join explicit namespaces** (`do_setns()` on namespace file descriptors)
3. **UID/GID mapping** (write to `/proc/self/uid_map` and `/proc/self/gid_map` if in user namespace)
4. **Hostname** (`sethostname()`)
5. **Security hardening** (no_new_privs, non-dumpable)
6. **Capabilities** (capset, cap_bset_drop, cap_ambient)
7. **Seccomp-BPF** (fail-closed: aborts if filter can't be applied)
8. **Resource limits** (`prlimit64` for each rlimit)
9. **OOM score** (write to `/proc/self/oom_score_adj`)
10. **AppArmor** (write to `/proc/self/attr/apparmor/exec`)
11. **Umask** (via syscall)
12. **Rootfs setup** (pivot_root, mounts, devices, masked paths, readonly paths)
13. **Rootfs propagation** (shared/slave/private/unbindable)
14. **Sysctl** (write to `/proc/sys/...`)
15. **Supplementary groups** (`setgroups()`)
16. **Drop GID** (`setgid()`)
17. **Drop UID** (`setuid()`)

## Syscall Interface (`syscalls.rs`)

### Raw Syscall FFI

No libc crate — direct `extern "C"` declarations:

```rust
extern "C" {
    pub fn unshare(flags: c_int) -> c_int;
    pub fn syscall(number: c_long, ...) -> c_long;
    pub fn kill(pid: c_int, sig: c_int) -> c_int;
    pub fn mount(source: *const c_char, target: *const c_char, ...) -> c_int;
    pub fn pivot_root(new_root: *const c_char, put_old: *const c_char) -> c_int;
    pub fn umount2(target: *const c_char, flags: c_int) -> c_int;
    pub fn mknod(path: *const c_char, mode: c_uint, dev: c_uint) -> c_int;
    pub fn setuid(uid: u32) -> c_int;
    pub fn setgid(gid: u32) -> c_int;
    pub fn setgroups(size: usize, list: *const u32) -> c_int;
    pub fn prctl(option: c_int, ...) -> c_int;
    pub fn sethostname(name: *const c_char, len: usize) -> c_int;
}
```

### Architecture-Specific Syscall Numbers

| Syscall | x86_64 | aarch64 |
|---------|--------|---------|
| seccomp | 317 | 277 |
| capset | 126 | 94 |
| umask | 95 | 166 |
| prlimit64 | 302 | 267 |
| setns | 308 | 268 |

## OCI Conformance Status

From `corpus/oci-conformance/STATUS.md` (updated 2026-04-10 code audit):

**Unit tests**: 55/55 passing (`cargo test -p edgerun-oci-runtime`)

### Implemented Features (✅)

- ✅ Process: cwd, env, args, PID 1 init
- ✅ Process user: uid, gid, umask
- ✅ Process capabilities (all 5 sets, all 41 capabilities)
- ✅ Process OOM score adjustment
- ✅ Process resource limits (all 16 RLIMIT types)
- ✅ Root readonly enforcement
- ✅ Mounts: bind, tmpfs, proc, sysfs
- ✅ SELinux mount labels
- ✅ Rootfs propagation (shared/private/slave)
- ✅ Namespaces without path (create new)
- ✅ Namespaces with path (join existing)
- ✅ Namespace type validation
- ✅ Cgroups: memory, cpu, pids, blkio, hugetlb
- ✅ Device creation from spec
- ✅ Seccomp filtering (spec-driven; ⚠️ BPF operator bugs in LT/LE/GE/GT, MASKED_EQ incomplete)
- ✅ Masked paths and readonly paths
- ✅ Hostname via UTS namespace
- ✅ Hook lifecycle (prestart/createRuntime/createContainer/startContainer/poststart)
- ✅ CLI commands: create, start, state, kill, delete
- ✅ PID file via `--pid-file` flag

### Partially Implemented (⚠️)

- ⚠️ Seccomp: CLI create path skips seccomp entirely (`setup_child_for_create()`)
- ⚠️ Poststop hooks: never executed — `delete_container_internal()` passes `None`
- ⚠️ Cgroup device/network: v1-only files, fail silently on cgroup v2
- ⚠️ Cgroup errors: all silently discarded (`let _ =`)
- ⚠️ CLI hooks: create/start/delete bypass hook execution
- ⚠️ No cgroup cleanup on container delete

### Not Implemented (❌)

- ❌ Live config updates (`config_updates_without_affect`)
- ❌ Cgroup cleanup on delete (`delete_resources`)
- ❌ Live config updates to running containers
- ❌ Kill on non-running containers

## Dependencies

### Internal Dependencies

- `edgerun-json`: JSON serialization via serde (path dependency)

### External Dependencies

- `serde` v1 (with derive feature)
- `serde_json` v1
- `libc` v0.2 (minimal usage — only for clone stack, kill, open, read, close in CLI)

**No**: libseccomp, libcgroup, libcap, systemd, Docker, runc

## Usage Patterns

### Programmatic Usage (Library API)

```rust
use edgerun_oci_runtime::{
    create_bundle, write_bundle, start_bundle, 
    RunningContainer, delete_container
};
use std::path::Path;

// Create OCI bundle
let spec = create_bundle(
    "/var/lib/containers/myapp/rootfs",
    vec!["/usr/bin/myapp".into(), "--config".into(), "/etc/myapp.conf"],
    Some(vec!["PATH=/usr/bin".into(), "APP_ENV=production".into()]),
    Some("mycontainer".into()),
);
write_bundle(Path::new("/var/lib/containers/myapp"), &spec)?;

// Start container (non-blocking)
let container = start_bundle(Path::new("/var/lib/containers/myapp"))?;
println!("Container PID: {}", container.pid());

// Wait for completion
let exit_status = container.wait()?;
println!("Container exited with status: {}", exit_status);

// Cleanup
delete_container(&container);
```

### Resource-Constrained Container

```rust
use edgerun_oci_runtime::json::*;

let mut spec = create_bundle("/rootfs", vec!["/bin/sh".into()], None, None);

// Add resource limits
spec.linux = Some(OciLinux {
    resources: Some(OciLinuxResources {
        memory: Some(OciLinuxMemory {
            limit: Some(536870912),  // 512 MB
            swap: Some(0),           // No swap
            ..Default::default()
        }),
        cpu: Some(OciLinuxCpu {
            quota: Some(50000),      // 50% of one CPU
            period: Some(100000),    // 100ms period
            shares: Some(512),       // Half default weight
            ..Default::default()
        }),
        pids: Some(OciLinuxPids { limit: 128 }),
        ..Default::default()
    }),
    cgroups_path: Some("/edgerun/mycontainer".into()),
    namespaces: Some(default_namespaces()),
    ..Default::default()
});
```

### CLI Usage

```bash
# Create a container (enters created state)
edgerun-oci create mycontainer --bundle /path/to/bundle

# Check state
edgerun-oci state mycontainer

# Start the container
edgerun-oci start mycontainer

# Send signal
edgerun-oci kill mycontainer SIGTERM

# Delete container resources
edgerun-oci delete mycontainer
```

## Integration with edgerun-node

The `edgerun-node` crate uses `edgerun-oci-runtime` as an optional dependency:

```toml
edgerun-oci-runtime = { path = "../edgerun-oci-runtime", optional = true }
```

Used in `command_dispatch.rs` for container management:
- Parse OCI spec from registry
- Apply resource limits from node configuration
- Start container via `start_bundle()`
- Track container lifecycle via `RunningContainer` handle

## Testing

### Test Coverage

54 unit tests covering:

- **OCI spec serialization/deserialization**: Round-trip tests for all types
- **Bundle creation**: Default values, empty args, hostname defaults
- **Namespace flags**: Empty, single, combinations, unknown types, duplicates
- **Shares to weight conversion**: Zero, one, default, high shares, overflow prevention, monotonicity
- **Seccomp BPF**: Non-empty, valid structure, contains allow/deny instructions
- **BPF instruction encoding**: 8-byte output, correct field encoding
- **Bundle writing**: Directory creation, config.json content, round-trip
- **Mount serialization**: OciMount round-trip with options
- **Resource serialization**: OciLinuxResources with memory, cpu, pids
- **Capability serialization**: OciCapabilities round-trip
- **Hook execution**: Empty hooks (OK), successful hooks, failing hooks, timeout handling
- **Poststop hooks**: Never fails even with broken hooks (per OCI spec)

### Test Structure

Tests are organized in `lib.rs` and individual module `#[cfg(test)]` blocks:
- Module-specific tests for serialization, helpers
- Integration-style tests for spec round-trips
- Error case tests (invalid input, missing fields)

## Performance Characteristics

### Memory Usage

- **Zero-copy syscalls**: No intermediate buffers for kernel interactions
- **Minimal BPF programs**: Seccomp filters generated as compact byte arrays
- **Stack allocation**: Clone stack uses 1MB pre-allocated vector

### Execution Time

- **Fast startup**: No external process spawning (no runc/Docker overhead)
- **Direct syscalls**: No library indirection
- **Lazy cgroup setup**: Only writes controllers that have resource limits

## Security Considerations

### Strengths

1. **Fail-closed seccomp**: Container startup aborts if seccomp can't be applied
2. **Capability validation**: Invalid capability names cause errors (not silent failures)
3. **Namespace isolation**: Six namespaces isolated by default
4. **No external dependencies**: No attack surface from third-party libraries
5. **Mount path validation**: Rejects paths that escape rootfs
6. **Resource limits**: PID limits prevent fork bombs, memory limits prevent OOM

### Potential Concerns

1. **Raw syscalls**: No libc safety wrappers (e.g., `mount` doesn't validate string pointers)
2. **CString creation**: Null bytes in paths cause early returns (not errors in some cases)
3. **Privilege requirements**: Needs CAP_SYS_ADMIN for most operations (typically runs as root)
4. **User namespace limitations**: uid_map/gid_map writing skipped if not in user namespace
5. **Cgroup v1 fallback**: Some cgroup controllers use v1 paths (may fail on v2-only systems)

## Comparison to Other Runtimes

| Feature | edgerun-oci-runtime | runc | crun | youki |
|---------|---------------------|------|------|-------|
| **Language** | Rust | Go | C | Rust |
| **Binary Size** | ~2 MB (with LTO) | ~8 MB | ~200 KB | ~5 MB |
| **External Deps** | 1 (`mkfifo` for CLI) | libseccomp, libcap | libseccomp, libcap, libyajl | libseccomp, libcap |
| **OCI Spec** | v1.0.2 | v1.x | v1.x | v1.x |
| **Cgroups** | v2 (v1 fallback, some controllers v1-only) | v1 + v2 | v1 + v2 | v1 + v2 |
| **Seccomp** | Built-in BPF gen (⚠️ operator bugs) | libseccomp | libseccomp | libseccomp |
| **Hooks** | Library: full OCI hooks; CLI: none | Full OCI hooks | Full OCI hooks | Full OCI hooks |
| **Rootless** | Partial (user ns) | Full | Full | Partial |
| **Systemd Integration** | No | Yes | Yes | No |
| **Conformance** | 41/59 ✅, 15/59 ⚠️ (69%+25%) | ~100% | ~100% | ~95% |

## Known Bugs (from 2026-04-10 code audit)

1. **Seccomp BPF operator errors**: `SCMP_CMP_LT`/`SCMP_CMP_GE` use `0x30` (JSET, bitwise AND-test) instead of comparison operators. `SCMP_CMP_LE`/`SCMP_CMP_GT` use `0x25` (JGT). These produce incorrect 64-bit argument filtering.
2. **Seccomp MASKED_EQ incomplete**: Checks `A & expected == expected` — only verifies expected bits are set, doesn't verify other bits are cleared.
3. **Seccomp skipped in CLI create**: `setup_child_for_create()` does not call seccomp — containers created via CLI have no filtering.
4. **Poststop hooks never run**: `delete_container_internal()` passes `None` for hooks; CLI delete doesn't load config.json.
5. **Container ID never populated**: `ContainerState.id` is always `String::new()` in hooks.
6. **Cgroup errors silently discarded**: All writes use `let _ =`.
7. **No cgroup cleanup on delete**: `delete_container_internal()` is a stub.

## Future Enhancements

### Missing OCI Spec Features

1. **Solaris/Windows/VMware support**: Platform-specific extensions (Linux-only currently)
2. **Linux integrity digest**: Root filesystem verification
3. **Scheduler policy**: SCHED_FIFO, SCHED_RR configuration
4. **Personality**: Execution domain personality
5. **AppArmor fully wired**: Profile application only, no stack/allow rules
6. **SELinux mount label**: Written but not enforced beyond data string

### Bug Fixes Needed

1. Fix BPF operator encoding for SCMP_CMP_LT/LE/GE/GT (use proper comparison ops)
2. Implement full SCMP_CMP_MASKED_EQ (XOR then JEQ on result==0)
3. Wire seccomp into `setup_child_for_create()` CLI path
4. Load spec and run poststop hooks in delete path
5. Populate `ContainerState.id` from actual container ID
6. Add cgroup cleanup to `delete_container_internal()`
7. Add error reporting for cgroup setup failures

### CLI Improvements

1. **Exec command**: Run additional processes in running container
2. **Pause/Resume**: Cgroup freezer integration
3. **Checkpoint/Restore**: CRIU integration for live migration
4. **Events**: Container event streaming
5. **Update**: Live cgroup updates without restart

### Security Hardening

1. **Landlock LSM**: Sandboxing for filesystem access
2. **BPF-based device filtering**: eBPF cgroup device controller
3. **Seccomp notify**: Userspace seccomp handler integration
4. **Time namespaces**: Isolate clock adjustments
5. **Cgroup namespace**: Container sees its own cgroup hierarchy

## Conclusion

`edgerun-oci-runtime` is a well-architected, security-focused minimal OCI runtime that prioritizes:

- **Simplicity**: 1 external dependency (`mkfifo` for CLI), direct kernel syscalls
- **Correctness**: 55/55 unit tests passing, OCI spec compliance (41/59 fully, 15/59 partially)
- **Security**: Seccomp-BPF, capabilities, namespace isolation, fail-closed design

**Current conformance: 41/59 ✅ (69%), 15/59 ⚠️ (25%), 3/59 ❌ (5%).**

The library API is solid — the main gaps are in the CLI layer (hooks not wired, seccomp skipped in create path, poststop hooks never executed) and several seccomp BPF bugs that need fixing before production use.

It's suitable for:
- Embedded/container edge deployments (minimal footprint)
- Security-conscious environments (no third-party library trust)
- Custom container orchestrators (programmatic API)
- Learning/teaching container internals (clear syscall-level implementation)

The main limitations are:
- No advanced features (checkpoint/restore, live migration)
- CLI lifecycle partially implemented (library API is primary)
- Rootless containers need user namespace configuration
- Cgroup v2 primary (v1 fallback may fail on hybrid systems)
