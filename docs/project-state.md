# Project State Assessment

Status date: 2026-04-29.

This assessment is code-grounded. It describes what is implemented in this
checkout, what is protocol/design material, what is generated catalog material,
and where the implementation is still partial or blocked.

## Summary

Edgerun Core is a real alpha implementation of the v0 protocol core, not just a
crate list or design document. The strongest parts are the protocol validators,
signed stream construction/verification, durable event/object storage, and the
hosted node command path. Those areas have substantial tests and preserve the
important v0 authority rule: commands are requests until a target node validates
and records an outcome event.

The project is not yet a complete production edge platform. Several broad
surfaces are partial: federation now has signed-fragment aggregation, but not a
full peer-selection and remote-query orchestration loop; service stacks vary by
protocol; and bare-metal/ESP32 paths include bring-up code and stubs. The
repository also contains generated web-platform protobuf catalogs and historical
RFC/design files that should not be read as implemented browser/runtime support.

## Verified Workspace Facts

- Implemented in code: `cargo metadata --no-deps --format-version 1` succeeds
  and reports 109 packages/workspace members.
- Implemented in code: `crates/` has 109 first-level directories, all with
  `Cargo.toml` manifests in this checkout.
- Implemented in code: the root `Cargo.toml` textual `members` list has 109
  unique entries.
- Generated type/catalog material: `proto/edgerun/v0` contains 44 protobuf
  files, including core protocol families and generated web-platform catalogs.
- Protocol/design requirement: `edgerun_core_protocol_v0_single_file.md` is the
  v0 working draft and remains the semantic source of truth.

## What Is Implemented

Protocol core is implemented in `edgerun-core` and `edgerun-proto`:

- protobuf-generated records are the canonical protocol structs;
- canonical bytes are prost encodings, with signatures cleared for signable
  forms;
- hashes and signatures are domain-separated;
- validators cover commands, streams, delegations, revocations, assurance
  claims, identity records, snapshots, query requests/results, proofs, route
  advertisements, reachability hints, relay envelopes, and session handshakes;
- command validation includes target checks, replay checks, timing, signature
  verification, delegation-chain continuity, attenuation, revocation, assurance,
  scope, rate/use limits, and contextual constraints.

Stream handling is implemented in `edgerun-stream`:

- stream writers create signed genesis events and contiguous signed appends;
- event hashes use canonical signable event bytes;
- validation rejects missing genesis, sequence gaps, bad previous hashes,
  missing/bad signatures, tampering, and wrong writer keys.

Durable storage is implemented in `edgerun-storage`:

- file, memory, and block-backed event logs exist;
- `NodeStore` appends events as authoritative records and maintains rebuildable
  indexes;
- object storage distinguishes logical object identity from representation
  identity;
- blob/content paths include encrypted storage and integrity checks;
- stream-chain validation with writer verification is covered by tests.

Hosted node command/query plumbing is implemented in `edgerun-node`:

- the library node records command accept/reject outcomes into its stream;
- daemon command dispatch uses `validate_command`, persistent and in-memory
  replay caches, controller projection, workload policy, rate limits, and
  command-result events;
- local query execution supports heads, event ranges, object existence/fetch,
  snapshots, and signed denial/result fragments against `NodeStore`.
- config projection replays committed `UpdateConfig` result objects from the
  event log, so mutable node configuration is rebuilt from authoritative
  events rather than only from the startup YAML.
- federation has an advisory aggregation API that validates signed remote
  `QueryResultFragment`s, enforces trusted responders and responder limits,
  stores accepted fragments as immutable objects, and returns a signed aggregate
  fragment with a `FederatedAggregateDescriptor` proof object.

Mesh and remote capabilities are implemented as usable building blocks:

- mesh frames are identity-addressed and signed around P-256 `NodeID`s;
- mesh link/session/daemon crates provide raw Ethernet, UDP, multicast/tunnel,
  ECDH session, replay/rekey, and daemon wiring;
- remote capability crates define signed requests/results, transports,
  session grants, policy decisions, and adapters for hardware capability traits.

Runtime, service, and hardware support is broad but uneven:

- `edgerun-rt` is a no_std-first async runtime with many hosted tests;
- HTTP/TLS/QUIC/DNS/DHCP/email/proxy/OCI crates contain real protocol code, but
  readiness differs per protocol and conformance document;
- Linux/ALSA/V4L2/Goodix/Bluetooth/TPM/YubiKey/Android adapter crates expose
  concrete host or provider integrations where supported by the host.

## Partial Or Blocked Areas

- Currently partial: federated query behavior. The core validates query and
  proof artifacts, the node can answer local queries, and signed remote
  fragments can now be aggregated. What remains is end-to-end peer selection,
  network fanout, timeout handling, and fetch-back of remote referenced
  objects/events.
- Currently partial: several service protocol crates are better described as
  protocol stacks/building blocks than production-certified services. The HTTP
  docs still call out TODO/conformance gaps, especially HTTP/3.
- Bare-target/stubbed: ESP32-S3 Wi-Fi/BLE paths include deterministic stubs and
  bring-up diagnostics. The Wi-Fi MMIO doc records current RX/TX and ROM-state
  blockers.
- Bare-target/stubbed or host-only: many hardware adapters are meaningful on
  Linux or specific devices, while bare targets often expose the trait surface
  without full device support.
- Generated type/catalog material: the HTML/CSS/ECMAScript/DOM/WebIDL protobuf
  families and spec snapshots are schema/catalog/reference assets. They are not
  evidence of a full browser engine.
- Protocol/design requirement: older RFCs document design history. Some old
  "not implemented" notes have since been surpassed by code; verify against
  current source before treating an RFC as status.

## Verification Run

The following package-level checks passed in this checkout:

```bash
cargo test -p edgerun-core
cargo test -p edgerun-stream
cargo test -p edgerun-storage
cargo test -p edgerun-node
```

Results:

- `edgerun-core`: 580 tests passed.
- `edgerun-stream`: 60 tests passed.
- `edgerun-storage`: 64 tests passed.
- `edgerun-node`: 43 tests passed.

These checks cover the core authority model well. They do not prove full
workspace production readiness, hardware availability, service conformance, or
bare-metal bootability on every target.

## Honest Read

The repository is strongest as a protocol-first reference core with a serious
amount of implementation behind the model. The core invariants are not merely
documented; many are enforced in validators, stream code, storage code, and
tests.

The weak point is breadth versus maturity. There are many crates and many
protocol/device surfaces, but several should be treated as alpha, partial,
host-only, generated, or bring-up work. The next highest-value work is not more
surface area; it is tightening end-to-end flows, marking feature readiness per
crate, completing the remaining projections/federation paths, and adding
conformance/interop evidence for service and hardware claims.
