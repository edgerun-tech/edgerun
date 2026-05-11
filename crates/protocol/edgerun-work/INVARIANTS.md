# EdgeRun Work Protocol Invariants

This crate is not only a message-passing layer. It is a set of verifiable state transitions for admitted work, routed execution, receipts, and settlement.

The transport medium is deliberately not part of the protocol guarantee. Memory, TCP, WebSocket, browser worker `postMessage`, email import, QR import, USB, Bluetooth, or future transports may move the same bytes. Protocol validity is decided by signatures, hashes, route commitments, ordered-channel rules, typed payload verification, receipts, and settlement checks.

## Layer ownership

| Layer | Owns | Must not own |
|---|---|---|
| `protocol` | Durable wire/economic structs | Runtime behavior, storage policy, transport details |
| `codec` | rkyv bytes, hashes, signatures, canonical preimages | Admission policy, settlement policy |
| `request_auth` | User `WorkRequest` signature verification | User balance checks |
| `route_auth` / `route_plan` | Signed route ads and route snapshots | Packet ordering, settlement |
| `channel_order` | Ordered stream acceptance | Business validity |
| `work_channel` / transport adapters | Moving `ChannelEnvelope`s | Declaring work valid |
| `relay_role` | Relay forwarding and relay receipts | Admission, user balance, final settlement |
| `storage_payload` / `typed_storage_role` | Typed object store/retrieve payload verification | Admission, settlement |
| `settlement` / `batch_settlement` | Local model of receipt payment | Transport, route selection |
| `delivery_proof` / `transit_proof` | Proof objects for delivery/transit claims | Pricing policy |
| `std_runtime` | std-only sockets, threads, synchronized wrappers | Core protocol invariants |

## Identity invariant

Every signed node identity must satisfy:

```text
node_id == derive_node_id(public_key, role)
```

This is mandatory for every path that accepts a `NodeIdentity` as authority or payment destination.

Applies to:

- `WorkReceipt.worker`
- `WorkAdmission.admission_node`
- `RelayAssignment.assigned_by`
- `NodeAvailable.node`
- `NodeHeartbeat.node`
- `RouteAdvertisement.node`
- `RouteSnapshot.issued_by`
- `ChannelProof` recipient verification

A signature that verifies against a public key is not enough. The signed public key must also derive the claimed `node_id` for the claimed `role`.

Canonical helpers:

```rust
verify_node_identity(identity)
verify_signature(identity, signature, preimage)
```

All higher-level verifiers should flow through these helpers.

## User authorization invariant

A raw `WorkRequest` is untrusted until:

```rust
verify_work_request(&request) == true
```

Admission must not trust any of these fields before user signature verification:

- `request.user`
- `request.user_sequence`
- `request.recipient`
- `request.work_type`
- `request.department`
- `request.payload_hash`
- `request.input_root`
- `request.max_total_cost`
- `request.valid_until_unix_ms`

Portable ingress is allowed. A work request may arrive through TCP, WebSocket, email, QR code, file import, browser worker, or any other medium. The medium gives no authority. Only the user signature gives intent.

Canonical helper:

```rust
verify_work_request(request)
```

## Admission invariant

A `WorkAdmission` means a DAO-authorized admission node claims:

```text
This signed user request passed admission policy and may enter the network with this budget, route, and validity window.
```

Admission must commit to:

- `dao_id`
- `user`
- `admission_node`
- `request_hash`
- `assigned_route_hash`
- `assigned_channel`
- `admitted_budget`
- `policy_hash`
- `sequence`
- `valid_until_unix_ms`

Workers do not re-check user balances. Workers validate that work belongs to an admitted chain and then execute local role rules.

Canonical helper:

```rust
verify_work_admission(admission)
```

## Route invariant

A `RouteAdvertisement` is a signed statement by a node about how it is reachable and which roles/departments it claims.

Route signatures are valid for these route states:

- `AVAILABLE`
- `DRAINING`
- `UNAVAILABLE`

Only `AVAILABLE` routes should be selected for new work. `DRAINING` and `UNAVAILABLE` are still valid signed state transitions and are useful for route snapshots, audits, and shutdown.

Canonical helpers:

```rust
verify_route_advertisement(route)
verify_available_route_advertisement(route)
VerifiedRoutePlan::from_snapshot(snapshot)
```

## Ordered-channel invariant

Receivers accept an ordered envelope only when:

```text
packet_hash == hash(packet bytes)
route_hash == expected selected route hash
sequence == next expected sequence for (channel_id, from, to)
previous_message_hash == last accepted message hash for (channel_id, from, to)
```

Transport adapters do not decide validity. They only move bytes or envelopes.

Canonical helper:

```rust
ChannelOrderBook::accept(ordered, expected_route_hash)
```

## Relay invariant

A relay is paid only for an admitted, ordered forwarding action.

A relay receipt must be based on:

- `request_hash`
- `admission_hash`
- relay `worker` identity
- `relay_node_id`
- ordered input message hash
- forwarded packet hash
- claim amount
- relay receipt sequence

Relay forwarding must fail closed. If packet serialization fails, no receipt may be created and no fallback hash such as `blake3("")` may be used.

Current helper:

```rust
RelayRole::forward_ordered_on(channel, ordered, request_hash, admission_hash)
```

Production target: relay payment should require receiver delivery proof, not only a forwarded packet hash.

## Delivery proof invariant

A `ChannelProof` is a recipient-signed proof that a specific ordered channel message was accepted.

It binds:

- `channel_id`
- `relay_node_id`
- `from`
- `to`
- `message_hash`
- `sequence`
- recipient signature

Canonical helpers:

```rust
channel_proof_for_ordered(...)
verify_channel_proof_for_ordered(...)
```

Relay settlement should eventually require a valid `ChannelProof` hash in the relay receipt path.

## Storage invariant

Storage work must use typed payloads, not opaque bytes.

A store request must verify:

- `manifest_hash`
- `job_id`
- `shard_index`
- `shard_hash`
- `bytes`

A retrieve response must verify the same before reconstruction or payment.

Canonical helpers:

```rust
verify_store_request(request)
verify_retrieve_response(response)
TypedObjectStoreRole
```

A storage receipt without retrievability is not a complete production proof. The next proof target is tying storage payment to retrieval or availability proof.

## Erasure invariant

The current XOR 2+1 erasure coding is a proof scheme, not the final production erasure code.

It proves:

- manifest hashing
- shard hashing
- shard assignment to nodes
- one missing data shard recovery
- reconstruction verification against original hash

The interface should survive future replacement with a stronger k+n coding scheme.

## Settlement invariant

Settlement pays receipts, not promises.

A receipt can settle only if:

- `verify_work_admission(admission)`
- `verify_work_receipt(receipt)`
- `receipt.admission_hash == hash(admission)`
- `receipt.request_hash == admission.request_hash`
- receipt hash has not already been paid
- cumulative admission spend plus claim is at most `admitted_budget`
- `admission.user` has enough balance

Batch settlement must be atomic:

- build and verify the batch
- reject duplicates inside the batch
- reject already-paid receipts
- reject over-budget batches
- only mutate ledger after preflight succeeds

Canonical helpers:

```rust
SettlementLedger::can_settle_receipt(...)
SettlementLedger::settle_receipt(...)
SettlementLedger::settle_receipt_batch(...)
build_receipt_batch(...)
```

## Pruning/finalization invariant

`paid_receipts` and `admission_spend` may be pruned only after the admission is finalized and the challenge window is closed.

Pruning too early permits duplicate claims if the same admission is later reused. Treat pruning as a finalization operation, not routine cleanup.

Current helper:

```rust
SettlementLedger::prune_finalized_admission(admission_hash)
```

## WASM/browser invariant

Rust/WASM node code owns protocol verification and role execution.

Browser JavaScript owns transport plumbing:

```text
WebSocket bytes in/out
Worker postMessage bytes in/out
IndexedDB/local persistence later
```

The WASM boundary should move `ChannelEnvelope` bytes, not raw `WorkPacket` bytes, because the receiver needs channel and route metadata.

Canonical helpers:

```rust
channel_envelope_bytes(envelope)
channel_envelope_from_bytes(bytes)
WasmWorkerNode::accept_inbound_bytes(bytes, now_unix_ms)
WasmWorkerNode::drain_outbound_envelope_bytes()
```

## no_std invariant

Core protocol code should remain `no_std` + `alloc` compatible.

Allowed in core:

- protocol structs
- rkyv encoding
- signature verification
- route planning
- channel ordering
- role logic
- storage payloads
- erasure proof model
- settlement model

Keep in `std_runtime`:

- TCP sockets
- threads
- synchronized wrappers
- filesystem storage
- process/runtime concerns

## Review checklist for new code

For every new signed/economic object, answer:

1. What does this object claim?
2. Who signs it?
3. Which fields are covered by the signature?
4. What hash identifies it?
5. What canonical verifier exists?
6. What prior object does it depend on?
7. What later object consumes it?
8. What makes it payable?
9. What makes it slashable or challengeable?
10. What test proves bad data is rejected?

If these questions cannot be answered, the object is not ready for protocol use.
