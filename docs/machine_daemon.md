# Machine daemon: trust establishment, delegation bootstrap, and transport

## Goal
Run the Rust machine daemon as the bootstrap layer that can:

- load local controller and trust-root state
- load query-policy defaults
- establish signed peer sessions
- optionally advertise direct routes to known peers
- provide transport/session scaffolding for later delegation and executor policy work

## Current Rust implementation

A first Rust scaffold now exists in `rust/crates/edgerun-machine-daemon`.

It currently provides:

- machine config loading from YAML or JSON
- bootstrap projection into local validator state
- signed `SessionHello` generation
- signed `SessionAccept` verification
- signed `RouteAdvertisement` generation
- abstract peer transport hooks so the daemon is not tied to a specific RPC stack yet
- tests for config parsing, bootstrap projection, successful peer establishment, and nonce mismatch rejection

## Config schema

```yaml
version: 1
controllers:
  - node_server
trust_roots:
  - node_server
query_policy:
  proof_bundle_max_bytes: 1048576
  metadata_only_query_classes:
    - QUERY_CLASS_OBJECT_FETCH
peers:
  - name: node_phone
    target_node: node_phone
    address: quic://10.10.10.42:8080
    transport_features: ["proto", "grpc-over-quic"]
    protocol_versions: [1]
    reconnect_interval: 30s
    advertise_route: true
    route_ttl: 2m
```

## Current flow

1. Load machine config.
2. Project controllers, trust roots, and query policy into bootstrap state.
3. Build a signed `SessionHello`.
4. Send it over an abstract peer transport.
5. Verify the returned `SessionAccept`:
   - nonce echo matches
   - selected protocol version is supported
   - selected transport features are a subset of what we offered
   - signature verifies against the known peer identity
6. Optionally advertise a signed direct route.

## Why the transport is abstract right now

The current implementation intentionally avoids locking the daemon to one transport dependency too early.
That keeps the trust/session logic reusable whether the eventual transport becomes:

- a custom framed transport
- local IPC
- a capability-session bridge

## What is still missing

This is bootstrap scaffolding, not the finished federation runtime.

Still missing:

- actual network client/server transport implementation
- persistent session store
- trust-root and controller mutation flows from signed records
- full delegation-chain evaluation in daemon command paths
- route expiry handling and route scoring
- executor integration
- machine daemon binary / service wrapper

## Best next step

Build the concrete transport adapter next, then hang the current signed session/bootstrap logic off it instead of redoing the trust logic inside the transport layer.
