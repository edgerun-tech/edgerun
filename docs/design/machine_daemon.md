# Machine daemon: edgerund

## Goal

The `edgerund` binary (from the `edgerun-node` crate) is the primary machine daemon. It provides:

- TPM/YubiKey/software key provisioning for hardware-backed node identity
- Mesh networking for peer discovery and routing (via `edgerun-mesh`,
  `edgerun-mesh-link`, `edgerun-mesh-session`, and `edgerun-mesh-daemon`)
- Command processing with ECDSA signature verification and delegation chain validation
- OCI workload execution (container runtime with namespaces, cgroups v2, pivot_root)
- Resource metering and RC-µs billing (compute marketplace)
- Structured logging and health endpoint
- Graceful signal handling (SIGINT/SIGTERM) and optional PID 1 init mode

## Current implementation

The daemon lives in `crates/edgerun-node/` with the binary entry point at `crates/edgerun-node/src/main.rs`.

### CLI interface

```bash
# Generate node identity (TPM, YubiKey, or software key)
edgerund init --config node.yaml [--name NAME] [--software]

# Start the daemon
edgerund run --config node.yaml \
  [--listen 0.0.0.0:8080] \
  [--health-port 8888] \
  [--log-level info] \
  [--init]

# Show node identity
edgerund status --config node.yaml
```

### Key provisioning (`edgerund init`)

1. Detects available hardware backends in priority order: TPM 2.0 (`/dev/tpmrm0`) → YubiKey (PIV slot 9a) → software key (`--software` flag)
2. For TPM: creates ECDSA P-256 signing key via TSS2 ESAPI, persists to a TPM persistent handle (0x81000001–0x810000FF), derives NodeID from public key coordinates
3. For YubiKey: reads existing ECDSA P-256 key from PIV slot 9a
4. For software: generates an insecure in-memory key (dev-only)
5. Writes YAML config with NodeID, signer type, and handle reference
6. Runs performance benchmarks and caches `perf_cert.bin`

### Daemon runtime (`edgerund run`)

1. Loads config and signer (TPM, YubiKey, or software)
2. Initializes structured logging
3. Optionally installs PID 1 signal handlers (`--init` flag or when running as PID 1)
4. Creates the no_std/bare runtime pieces used by the node (`edgerun-rt`)
5. Initializes mesh networking stack (`edgerun-mesh` + `edgerun-mesh-link` + `edgerun-mesh-session` + `edgerun-mesh-daemon`)
6. Starts TCP listener for peer connections (`--listen`)
7. Starts health HTTP endpoint (`--health-port`)
8. Enters main event loop: process commands, route mesh frames, manage workloads

### Mesh networking

The mesh stack provides peer-to-peer networking:

- **edgerun-mesh**: Frame types (130-byte header with dest/src NodeID, TTL, frame type), ECDSA-signed frames, routing table
- **edgerun-mesh-link**: Raw Ethernet and UDP link layer, multicast discovery
- **edgerun-mesh-daemon**: Poll loop tying links, router state, sessions, capability handling, commands, and metrics together

### Command processing

Commands arrive via TCP or mesh frames. The processing pipeline:
1. Parse `CommandEnvelope` (protobuf)
2. Verify ECDSA signature against sender's public key
3. Verify delegation chain and controller authorization
4. Dispatch to handler (e.g., `ExecuteWorkload`, queries)
5. Record events to append-only log
6. Return signed response

### Workload execution (ExecuteWorkload)

Three-phase execution:
1. **Pull**: Download OCI image from registry, track network/storage I/O
2. **Resource limits**: Apply cgroups v2 limits (memory, CPU weight, PIDs)
3. **Run**: Fork, unshare namespaces, pivot_root, drop privileges, exec container

The `WorkMeter` (in `edgerun-node/src/metering.rs`) tracks resource consumption and calculates billable RC-µs using the node's `PerformanceCertificate`.

### Storage

- `edgerun-storage`: Append-only event log, rebuildable binary/file indexes, replay cache, and encrypted blob store
- AES-GCM encryption for stored objects with persistent blob keys

## Architecture diagram

```
┌─────────────────────────────────────────────────────────┐
│                        edgerund                          │
├─────────────────────────────────────────────────────────┤
│  CLI: init / run / status                               │
├─────────────────────────────────────────────────────────┤
│  Identity: TPM 2.0 │ YubiKey │ Software (dev only)      │
├─────────────────────────────────────────────────────────┤
│  Mesh: mesh + mesh-link + mesh-session + mesh-daemon    │
│    Frame types │ ECDSA signing │ Bellman-Ford routing   │
├─────────────────────────────────────────────────────────┤
│  Command dispatch: signature verify │ delegation chain  │
├─────────────────────────────────────────────────────────┤
│  Workload: OCI pull │ cgroups v2 │ namespaces │ exec    │
│  Metering: WorkMeter → RC-µs billing                   │
├─────────────────────────────────────────────────────────┤
│  Storage: event log │ rebuildable indexes │ blobs      │
├─────────────────────────────────────────────────────────┤
│  Health endpoint: HTTP /health on configurable port     │
└─────────────────────────────────────────────────────────┘
```

## Systemd integration

Two service files are provided in `systemd/`:

- **edgerund.service**: Standard daemon with `Type=notify`, security hardening (NoNewPrivileges, ProtectSystem, PrivateTmp, etc.), device access for TPM/ALSA/V4L2/input
- **edgerund-init.service**: PID 1 replacement with `Type=idle`, `--init` flag, less restrictive sandbox (PID 1 needs broader access)

## Historical note

An earlier design iteration used a separate `edgerun-machine-daemon` crate with a YAML-based peer configuration, `SessionHello`/`SessionAccept` handshakes, and abstract peer transport hooks. That design has been superseded by the current `edgerun-node` architecture, which integrates mesh networking, command processing, and workload execution into a single unified daemon.
