# edgerun-oci

Minimal OCI container runtime — kernel-only, no external tools.

Uses Linux kernel primitives: namespaces, cgroups v2, pivot_root, mount.
No Docker, no runc, no systemd, no libc crate. Just raw syscalls and `std`.

## Current status

The workspace package is `edgerun-oci`. Source lives in `crates/edgerun-oci/`,
with the CLI entry point at `src/bin/edgerun-oci.rs`.

As of 2026-04-29, `cargo metadata --no-deps --format-version 1` succeeds for
the root workspace. The runtime is not yet production-ready as a `runc`/`crun`
replacement. It has working OCI bundle lifecycle paths and targeted runc
compatibility fixes, but full unskipped runc/contest coverage is still being
triaged.

Status labels in this document mean:

- **Implemented in code**: code path exists and is intended to run.
- **Validated**: verified by the focused command shown below or by the listed
  project tests.
- **Partial**: works for the covered path but still has compatibility gaps.
- **Blocked / unverified**: code or tests are missing for production claims.

## Architecture

```
crates/edgerun-oci/
├── src/
│   ├── lib.rs              — Public API, re-exports, namespace helpers
│   ├── json.rs             — OCI spec types (serde serialization via edgerun-json)
│   ├── error.rs            — Structured error types per subsystem
│   ├── state.rs            — Container state persistence (/run/edgerun-oci/<id>/state.json)
│   ├── fifo.rs             — FIFO-based start synchronization
│   ├── bundle.rs           — OCI bundle creation and serialization
│   ├── config_builder.rs   — Fluent builder for ContainerProcessConfig
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

**Last focused run:** 2026-04-29

Validated commands from this documentation update:

```bash
cargo build -p edgerun-oci --features std --target x86_64-unknown-linux-musl
cd crates/edgerun-oci/tests/runc/src/github.com/opencontainers/runc
sudo -E PATH="$PATH" script -q -e -c 'bats -f "^update devices \[minimal transition rules\]$" -t tests/integration'
sudo -E PATH="$PATH" script -q -e -c 'bats -f "^list$" -t tests/integration'
```

Validated result:

```text
ok 1 update devices [minimal transition rules]
ok 1 list
```

Earlier project-local conformance snapshot, retained for context:

- Unit tests: 125 passed, 0 failed.
- Integration tests: 24 passed, 0 failed.
- Clippy warnings: 0.

Treat the earlier counts as a historical project-local snapshot, not as proof of
full `runc`/`crun` compatibility.

### Project-local integration tests from the earlier snapshot

| Test | What it verifies |
|------|-----------------|
| `conformance_default` | Minimal container runs (`/bin/true`) |
| `conformance_hostname` | Hostname set correctly in container |
| `conformance_process_args` | Custom args work (`/bin/sleep 0.1`) |
| `conformance_mounts` | Proc mount works |
| `conformance_masked_paths` | Path masking works |
| `conformance_readonly_paths` | Readonly bind mounts work |
| `conformance_lifecycle_state` | Full create→start→state→delete lifecycle with state transitions |
| `conformance_duplicate_id` | Second create with same ID fails |
| `conformance_cgroup_memory` | `memory.max` set correctly |
| `conformance_cgroup_pids` | `pids.max` set correctly |
| `conformance_cgroup_cpu` | `cpu.weight` converted from shares, in valid range |
| `conformance_cgroup_cleanup` | Cgroup directory cleaned up after delete |
| `bundle_*` (6 tests) | Bundle config.json roundtrip, minimal fields, cgroup, args, hostname, masked paths, mount |
| `copy_runtime_libs` | Dynamic linker/libc copied into rootfs |
| `runner_id_format` | Container ID format validation |
| **`device_uid_gid_ownership_in_container`** | Device chown() after mknod() — runs real container with spec device uid/gid |
| **`time_namespace_accepted_by_validator`** | "time" in KNOWN_NAMESPACES — validates spec with time namespace |
| **`cgroup_weight_device_per_device_written`** | weightDevice global weight written to cgroup (per-device needs BFQ scheduler) |

Run project-local integration tests with:

```bash
sudo cargo test -p edgerun-oci --test conformance -- --test-threads=1
```

Run focused runc compatibility checks from the vendored runc harness with:

```bash
cargo build -p edgerun-oci --features std --target x86_64-unknown-linux-musl
cp target/x86_64-unknown-linux-musl/debug/ert crates/edgerun-oci/tests/runc/src/github.com/opencontainers/runc/runc
chmod +x crates/edgerun-oci/tests/runc/src/github.com/opencontainers/runc/runc
cd crates/edgerun-oci/tests/runc/src/github.com/opencontainers/runc
sudo -E PATH="$PATH" script -q -e -c 'bats -f "^update devices \[minimal transition rules\]$" -t tests/integration'
sudo -E PATH="$PATH" script -q -e -c 'bats -f "^list$" -t tests/integration'
```

## OCI Spec Compliance

### Target Version: v1.0.2

| Category | Types Defined | Runtime Applied | Status |
|----------|--------------|-----------------|--------|
| Top-level spec | 8/8 | 8/8 | ✅ |
| `process` fields | 15/15 | 15/15 | ✅ |
| `root` fields | 2/2 | 2/2 | ✅ |
| `mounts` fields | 7/7 | 5/7 | ⚠️ (recursive + idmapped types only) |
| `linux` fields | 14/14 | 14/14 | ✅ |
| Namespaces | 8/8 | 8/8 | ✅ |
| `domainname` | 1/1 | 1/1 | ✅ |
| Cgroup v2 resources | 32/32 | 31/32 | ⚠️ (leaf weights v1-only) |
| Seccomp fields | 7/7 | 7/7 | ✅ |
| Hook types | 6/6 | 6/6 | ✅ |
| Linux devices | 7/7 | 7/7 | ✅ |
| Device cgroup rules (eBPF) | 4/4 | 4/4 | ✅ Validated for focused runc transition-rule path |
| OCI 1.1 features | 6/6 | 4/6 | ⚠️ (recursive + idmapped types only) |
| OCI 1.2 features | 3/3 | 2/3 | ⚠️ (personality applied, time namespace accepted) |

### What Is Actually Applied at Runtime

| Feature | Runtime Code | Integration Test | Status |
|---------|-------------|-----------------|--------|
| `linux.devices[].uid/gid` | `chown()` after `mknod()` in `create_spec_device()` | `device_uid_gid_ownership_in_container` | ✅ Applied + Tested |
| `blockIO.weightDevice` | Per-device writes to `io.weight` / `io.bfq.weight` via append | `cgroup_weight_device_per_device_written` | ✅ Code applied (kernel needs BFQ for per-device) |
| `linux.time` namespace | `"time"` in `KNOWN_NAMESPACES`, flag mapped | `time_namespace_accepted_by_validator` | ✅ Applied + Tested |
| `process.consoleSize` | `TIOCSWINSZ` ioctl after PTY allocation | — | ✅ Code applied |
| `linux.personality` | `personality(2)` syscall in `setup_container_child` | — | ✅ Applied |
| `seccomp.listenerMetadata` | Written to `<bundle>/.edgerun-seccomp-metadata` | — | ✅ Applied |
| `resources.rdma` | Writes to `rdma/<name>/max` | — | ✅ Applied |
| `cpu.idle` (v1.1.0) | Writes `1` to `cpu.idle` when idle != 0 | — | ✅ Applied |
| `cpu.burst` (v1.1.0) | Writes burst value to `cpu.max.burst` | — | ✅ Applied |
| `hugetlb[].rsvd` (v1.1.0) | Writes to `hugetlb.<size>.rsvd.max` when rsvd=true | — | ✅ Applied |
| `process.ioPriority` (v1.1.0) | `ioprio_set(2)` syscall with class+priority | — | ✅ Applied |
| `domainname` (v1.1.0) | `setdomainname(2)` syscall | — | ✅ Applied |
| `intelRdt.schemata` (v1.3.0) | Combined schemata field overrides individual fields | — | ✅ Applied |
| `intelRdt.enableMonitoring` (v1.3.0) | Creates `monitors/` directory in clos | — | ✅ Applied |
| `mount.recursive` | `mount_setattr` with `MOUNT_ATTR_REC`, error logged to kmsg on failure | — | ✅ Applied (error logged) |
| `mount.uidMappings/gidMappings` | Full `setup_idmapped_mount` (fork+unshare+open_tree+mount_setattr+move_mount), error logged to kmsg on failure | — | ✅ Applied (error logged) |
| `mount.recursive` dest (v1.2.0) | Relative mount destinations allowed with escape checking | — | ✅ Applied |
| `mount.idmap/ridmap` opts (v1.2.0) | Option strings recognized (handled via uidMappings) | — | ✅ Applied |
| `memory.checkBeforeUpdate` (v1.1.0) | Type defined (runtime hint) | — | ⚠️ Type only |
| `leafWeight/leafWeightDevice` | Type defined, v1-only, intentionally skipped | — | ⚠️ Type only (correct for v2) |

### runc compatibility work validated on 2026-04-29

| Area | Status | Notes |
|------|--------|-------|
| Bundle-mode `run -d --console-socket` | ✅ Validated | Focused runc update-devices test starts detached workload and hands off console fd |
| Device cgroup eBPF | ✅ Validated for targeted path | Correct cgroup-device BPF ABI, attach type, access masks, and transition-rule timing |
| `update --pids-limit` while device rules are active | ✅ Validated | Repeated 30x in `update devices [minimal transition rules]` |
| `/dev/kmsg` deny with `/dev/null` still usable | ✅ Validated | Targeted runc test asserts no container output leaks |
| `list`, `list -q`, `list --format table`, `list --format json` | ✅ Validated | Vendored runc `list` integration test passes |
| Delete/poststop rootfs cleanup | ✅ Validated for targeted path | Relative bundle rootfs is resolved and detached before harness teardown |

### Current production-readiness assessment

`edgerun-oci` is not currently production-ready as a general `runc` or `crun`
replacement. It should be treated as a compatibility target under active
hardening.

Production blockers:

1. Full unskipped runc and contest suites have not passed.
2. Some runtime paths still use best-effort behavior for host-dependent mounts
   and cgroup writes; isolation-critical failures must not be silently bypassed.
3. Rootless behavior needs separate coverage from rootful behavior.
4. Seccomp, mount, namespace, hook, update, and exec edge cases need broader
   compatibility coverage.
5. CLI parity is still incomplete beyond the commands already listed.

### Remaining Gaps

#### Integration tests needed

1. **full unskipped runc/contest pass** — Required before replacement claims.
2. **rootless runc compatibility sweep** — Rootless behavior must be tracked independently.
3. **personality end-to-end** — Code applies `personality(2)` but no container-level test verifies it.
4. **consoleSize end-to-end** — Code applies `TIOCSWINSZ` but no container-level test verifies terminal dimensions.
5. **listenerMetadata end-to-end** — Code writes metadata file but no test verifies file creation.
6. **cpu.idle end-to-end** — `cpu.idle` cgroup write but no container-level test verifies idle state.
7. **cpu.burst end-to-end** — `cpu.max.burst` cgroup write but no container-level test verifies burst behavior.
8. **I/O priority end-to-end** — `ioprio_set(2)` but no container-level test verifies I/O priority.
9. **domainname end-to-end** — `setdomainname(2)` but no container-level test verifies NIS domain.
10. **hugetlb rsvd end-to-end** — `hugetlb.<size>.rsvd.max` write but no container-level test verifies reserved accounting.

#### Intentionally skipped

11. **leafWeight / leafWeightDevice** — Cgroup v1-only. This runtime is v2-only. Correct to skip.
12. **memory.checkBeforeUpdate** — Runtime hint only, not a cgroup file. Type defined for spec compliance.

#### Future OCI versions (v1.3.0 VM support)

13. **hwConfig / VM containers** — Entire new runtime model requiring hypervisor integration. Major feature.
14. **netDevices** — Network device configuration for VM containers.
15. **memoryPolicy** — NUMA memory policy configuration.

#### Previously Fixed

- ~~Terminal/PTY~~ — PTY with SCM_RIGHTS + I/O relay
- ~~Seccomp NOTIFY~~ — `SECCOMP_FILTER_FLAG_NEW_LISTENER`
- ~~Cgroup namespace~~ — in `default_namespaces()`
- ~~Platform validation~~ — enforced on create
- ~~exec PID namespace~~ — double-fork
- ~~Seccomp >255 rules~~ — `bpf_long_skip()`
- ~~`--root` override~~ — `state::set_state_dir()`
- ~~State bundle path~~ — uses `--bundle` from CLI
- ~~Poststop error logging~~ — stderr + kmsg fallback
- ~~Seccomp listenerMetadata~~ — written to bundle directory
- ~~bundle_path threading~~ — full lifecycle propagation
- ~~Device cgroup transition-rule path~~ — focused runc test passes
- ~~`list` command~~ — focused runc test passes
- ~~Relative bundle rootfs cleanup~~ — focused runc teardown exits cleanly

## Features Summary

### Implemented CLI Commands

| Command | Status | Notes |
|---------|--------|-------|
| `create` | ✅ Full | Namespaces, rootfs, hooks, FIFO sync |
| `start` | ✅ Full | FIFO signal, poststart hooks |
| `state` | ✅ Full | Reads `/run/edgerun-oci/<id>/state.json` |
| `kill` | ✅ Full | Signal parsing, no-op for stopped |
| `delete` | ✅ Full | Poststop hooks, cgroup cleanup |
| `list` | ✅ Partial | runc-compatible quiet/table/json output validated |
| `exec` | ✅ Full | setns join, double-fork for PID namespace, PTY relay |
| `update` | ✅ Partial | Memory, CPU, PIDs, block I/O; focused device-rule reapply path validated |
| `pause` | ✅ Full | cgroup.freeze |
| `resume` | ✅ Full | cgroup.unfreeze |
| `events` | ✅ Basic | Cgroup stats streaming |
| `ps` | ✅ Basic | Process listing from /proc scanning |
| `features` | ✅ Full | Capability reporting |
| `spec` | ✅ Full | Default config.json generation |

### Implemented OCI Spec Features

| Feature | Status |
|---------|--------|
| Namespaces (mount, pid, net, ipc, uts, user, cgroup, time) | ✅ Applied + Tested |
| Cgroups v2 (memory, cpu, pids, blockIO, hugepages, rdma) | ✅ Applied |
| Seccomp-BPF (NOTIFY, >255 rules, listenerMetadata) | ✅ Applied |
| OCI Hooks (all 6 types, timeout + stop-on-failure) | ✅ Applied |
| UID/GID mapping (user namespace) | ✅ Applied |
| Capabilities (5 sets) | ✅ Applied |
| Rootfs (pivot_root, mounts, devices with uid/gid) | ✅ Applied + Tested |
| Masked/readonly paths | ✅ Applied |
| Sysctl parameters | ✅ Applied |
| Resource limits (rlimits) | ✅ Applied |
| OOM score adjustment | ✅ Applied |
| AppArmor profile | ✅ Applied |
| SELinux label | ✅ Applied |
| Process scheduler (SCHED_*) | ✅ Applied |
| Intel RDT | ✅ Applied (best-effort) |
| Platform validation | ✅ Applied |
| Terminal/PTY (SCM_RIGHTS + I/O relay, consoleSize) | ✅ Applied |
| eBPF device cgroup rules | ✅ Applied |
| eBPF network class/priority | ✅ Applied |
| Live cgroup updates | ✅ Applied |
| Per-device block I/O weight | ✅ Applied (BFQ needed for per-device) |
| CPU idle (OCI 1.1.0) | ✅ Applied |
| CFS burst (OCI 1.1.0) | ✅ Applied |
| Hugepage rsvd (OCI 1.1.0) | ✅ Applied |
| I/O priority (OCI 1.1.0) | ✅ Applied |
| Domainname (OCI 1.1.0) | ✅ Applied |
| Intel RDT schemata (OCI 1.3.0) | ✅ Applied |
| Intel RDT monitoring (OCI 1.3.0) | ✅ Applied |
| Relative mount destinations (OCI 1.2.0) | ✅ Applied |
| Personality (OCI 1.2) | ✅ Applied |
| Rootfs propagation | ✅ Configurable, no default |
| Mount recursive | ✅ Applied (error logged) |
| Idmapped mounts (OCI 1.1/1.2) | ✅ Applied (error logged) |

## Code Metrics

| Metric | Value |
|--------|-------|
| Source lines | ~11,800 (Rust) |
| Unit tests | 125 |
| Integration tests | 24 |
| Clippy warnings | 0 |
| External dependencies | 3 (`edgerun-json`, `serde`, `libc`) |
| Async runtime | None (blocking syscalls only) |
| Architectures | x86_64, aarch64 |

## Dependencies

| Crate | Purpose |
|-------|---------|
| `edgerun-json` | JSON serialization (drop-in for serde_json) |
| `serde` | Derive macros for Serialize/Deserialize |
| `libc` | Raw syscall FFI, constants |

No async runtime, no external libraries (no libseccomp, no libcontainer, no libcap).

## Building

```bash
cargo build -p edgerun-oci                  # library + binary
cargo build -p edgerun-oci --release         # optimized binary
cargo test -p edgerun-oci --lib              # unit tests
sudo cargo test -p edgerun-oci --test conformance -- --test-threads=1
cargo clippy -p edgerun-oci
```

The binary is `edgerun-oci` / `ert`. It is not yet a production drop-in
replacement for `runc` or `crun`; use it as an active compatibility target and
validate required workloads explicitly.

## Running

```bash
# Create a bundle
mkdir -p mybundle/rootfs
# ... populate rootfs ...
edgerun-oci spec

# Run container (blocking)
edgerun-oci --bundle mybundle create mycontainer
edgerun-oci start mycontainer
edgerun-oci state mycontainer
edgerun-oci kill mycontainer
edgerun-oci delete mycontainer
```
