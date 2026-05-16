# edgerun-oci accountable runtime

`edgerun-oci` should remain an OCI-compatible runtime first. The new accountability layer must not fork the OCI bundle format, require Kubernetes, require a TPM, or make ordinary OCI workloads stop working.

The approach is to add an optional accountable execution profile on top of normal OCI metadata. When enabled, the runtime emits a tamper-evident event log for facts the runtime can observe directly: config hash, container id, bundle path, PID, state transitions, cgroup setup, start signal, exit/delete handling, and cleanup.

Application-level claims stay separate. The runtime can prove that a workload was launched under specific runtime constraints; an application wrapper or SDK can separately sign higher-level domain events from inside the workload. Verifiers join both streams by `run_id`.

These streams are execution evidence and local projections. They become network
authority only when admitted work, receipts, or capability packets bind to the
runtime evidence through `edgerun-work`.

## Positioning

This is not hardware attestation yet. It is runtime-level accountable execution.

The trust ladder is:

```text
L0: unsigned runtime events
L1: software-key runtime signatures
L2: controller-provisioned runtime key
L3: OS-bound keyring / KMS / Vault-backed key
L4: TPM / HSM / TEE-backed runtime key
L5: TPM-backed key plus measured boot / PCR quote
```

The event schema must survive all levels. Stronger signer backends only upgrade the evidence attached to the same lifecycle events.

## OCI compatibility rules

1. A normal OCI bundle without EdgeRun annotations must run normally.
2. Accountability is opt-in via OCI annotations or runtime configuration.
3. EdgeRun metadata lives in annotations, sidecar manifests, or referrers; not in a forked `config.json` schema.
4. Runtime facts and application facts are separate local evidence streams.
5. Missing signer hardware must not prevent basic hash-chained logging.
6. Isolation-critical failures must stay separate from accountability logging failures. Logging should never silently weaken container isolation.

## Opt-in annotations

Initial annotations:

```json
{
  "annotations": {
    "org.edgerun.accountability.version": "v0",
    "org.edgerun.accountability.enabled": "true",
    "org.edgerun.accountability.log": "state://events.log",
    "org.edgerun.accountability.signer": "none",
    "org.edgerun.accountability.capabilities": "sha256:<optional-manifest-digest>"
  }
}
```

For v0, `state://events.log` resolves to:

```text
/run/edgerun-oci/<container-id>/events.log
```

or the equivalent rootless state directory already used by `edgerun-oci`.

A later collector can stream or preserve the event log before OCI `delete` removes the state directory. Long-term archival should move into the EdgeRun controller/storage layer, not into the OCI runtime state directory.

## Event stream split

Runtime stream:

```text
edgerun-oci → runtime facts
```

Examples:

```text
ContainerCreateRequested
BundleConfigHashed
ContainerCreated
CgroupsApplied
DeviceCgroupsApplied
ContainerStarted
PoststartHooksExecuted
ContainerStopped
PoststopCleanupCompleted
ContainerDeleted
RuntimeError
```

Application stream:

```text
app wrapper / SDK → domain facts
```

Examples:

```text
HttpRequestMade
FileWritten
JobAccepted
JobResultProduced
PaymentClaimed
MessageSent
CapabilityUsed
```

The verifier joins streams using:

```text
run_id = sha256(runtime || container_id || bundle_hash || create_nonce)
```

## v0 event shape

```json
{
  "schema": "edgerun.runtime.event.v0",
  "seq": 3,
  "prev_hash": "sha256:...",
  "event_hash": "sha256:...",
  "timestamp_unix_ms": 1777815000000,
  "runtime": "edgerun-oci",
  "container_id": "example",
  "event": "ContainerStarted",
  "status": "running",
  "pid": 12345,
  "bundle": "/path/to/bundle",
  "payload": {
    "cgroup_path": "/example"
  },
  "trust": {
    "level": "runtime-software-hash-chain",
    "signer": "none",
    "hash": "sha256"
  }
}
```

For the first implementation slice, `event_hash` is the SHA-256 digest of the canonical event body excluding `event_hash`. `prev_hash` is the digest of the previous event line. This gives immediate tamper evidence without adding new dependencies or requiring key management.

## First implementation slice

Implement now:

1. Add `crates/edgerun-oci/src/accountability.rs`.
2. Add a small append-only JSONL writer.
3. Resolve the event log path from the existing container state directory.
4. Gate logging on `org.edgerun.accountability.version` or `org.edgerun.accountability.enabled=true`.
5. Emit `ContainerCreated` and `ContainerStarted` events from lifecycle code.
6. Treat logging failures as warnings unless strict mode is later introduced.

Defer:

1. Ed25519/P-256 signing.
2. TPM-backed signing.
3. PCR quotes and measured boot evidence.
4. External event collector.
5. OCI artifact/referrer publication.
6. Application wrapper event stream.

## Why this belongs in the runtime

A wrapper can only report what the application cooperates with. The OCI runtime can report what actually happened at the container boundary: PID, state, spec snapshot, cgroups, namespaces, mounts, hooks, start signal, exit handling, and cleanup.

That makes `edgerun-oci` a compatibility bridge: ordinary containers still run, but workloads can opt into cryptographically chained execution records without requiring a new orchestrator or hardware attestation on day one.
