# EdgeRun Work Protocol Invariants

This crate is not only a message-passing layer. It is a set of verifiable state transitions for admitted work, routed execution, capability access, receipts, proofs, and settlement.

The transport medium is deliberately not part of the protocol guarantee. Memory, TCP, WebSocket, browser worker `postMessage`, email import, QR import, USB, Bluetooth, QUIC, WebTransport, or future transports may move the same bytes. Protocol validity is decided by identities, signatures, hashes, admission-defined routes, ordered-channel rules, typed payload verification, proofs, receipts, and settlement checks.

## Universal capability model

An Edgerun node is any addressable capability endpoint with an Ed25519 identity. A node may be a whole machine, but it may also be a single local or remote capability:

- webcam
- microphone
- local object store
- browser tab
- code editor executor
- shell/process adapter
- GPU/NPU worker
- sensor
- database adapter
- application UI

The protocol treats these uniformly. A capability owns a keypair, has `node_id == Ed25519 public key`, exposes one or more roles/departments/work types, and only accepts packets admitted for that capability. Generic resource access uses `NODE_ROLE_CAPABILITY`, `DEPARTMENT_CAPABILITY`, and the capability work types (`CAPABILITY_REQUEST`, `CAPABILITY_INVOKE`, `CAPABILITY_EVENT`, `CAPABILITY_CLOSE`) so devices and services do not invent transport-specific control protocols.

Generic capability messages carry a typed `CapabilityEnvelope` as the
`NetworkMessage.payload`. The outer `NetworkMessage` owns routing, admission
matching, sender signature, and packet ordering. The inner capability envelope
owns capability session/id/kind/operation/content-type/sequence metadata and
the payload hash for media, input, render, object, or control bytes.

Capability ids are deterministic:
`capability_id_from_descriptor(provider_node_id, descriptor)`. Sessions are
deterministically derived from admission hash, source identity, target identity,
capability id, and sequence. Invocations are deterministically derived from
session id, operation, sequence, and payload hash. Timestamps are packet
metadata, not authority, unless admission policy explicitly admits a
time-dependent rule.

The security goal is not to trust relays or transports. Relays are untrusted packet movers. They can drop, delay, or fail to forward packets, but they must not be able to forge capability identity, user intent, admissions, work outputs, receipts, or delivery proofs.

The canonical flow is:

```text
capability keypair
  -> capability NodeId
  -> relay connects to admission with signed NodeAvailable(relay endpoint)
  -> capability asks admission for relay assignment
  -> admission returns signed RelayAssignment with ChannelEndpoint
  -> capability connects to that admission-assigned endpoint
  -> capability proves the assigned path by sending signed protocol traffic over it
  -> sender asks admission for work/capability access
  -> admission returns signed WorkAdmission with predefined relay path
  -> sender signs NetworkMessage and feeds packets to its relay
  -> relays forward along the admission-defined path
  -> capability executes only admitted role/work inputs
  -> receipts/proofs bind execution and delivery
```

Local hardware, application UIs, and remote services use the same rules. A
webcam on the local machine, a storage daemon in a browser, an app-owned UI
node, and a remote compute worker are all identity-bound capabilities routed
through admission-assigned relays under policy. Developers build against the
app SDK and UI/runtime bridge; they do not need to know whether the current
endpoint is memory, WebSocket, TCP, a file adapter, a device adapter, or a
network interface.

Specialized roles such as storage, compute, and message delivery are optimized libraries on top of the same model. They do not create a separate routing authority.

Capability payload verification must be deterministic:

- `NetworkMessage.department == DEPARTMENT_CAPABILITY`
- `NetworkMessage.work_type` matches `CapabilityEnvelope.kind`
- `NetworkMessage.from == CapabilityEnvelope.source_node_id`
- `NetworkMessage.to == CapabilityEnvelope.target_node_id`
- `CapabilityEnvelope.operation` is valid for `CapabilityEnvelope.content_type`
- `CapabilityEnvelope.payload_hash == hash(CapabilityEnvelope.payload)`
- `NetworkMessage.payload_hash == hash(encoded CapabilityEnvelope)`

## Trust Container Decryption Invariant

Application UI nodes must not be decryption authorities for user data. A chat
app may compose outgoing plaintext, seal payloads to a recipient, store sealed
message metadata, and request plaintext for display, but the private user
decryption key belongs to the portable Trust Container capability.

Incoming chat plaintext is released only through an admitted capability invoke:

```text
chat app node
  -> WorkRequest / WorkAdmission
  -> CapabilityEnvelope {
       capability_id = Trust Container,
       operation = message decrypt,
       payload = sealed message bytes
     }
  -> Trust Container role verifies recipient identity and opens only messages
     encrypted to the unlocked user
  -> plaintext response to that admitted app session
```

This keeps routing authority separate from decryption authority. Device
admission, app admission, relays, network adapters, storage, and other apps may
route or store sealed bytes, but they must not be able to decrypt every message.

## Layer ownership

| Layer | Owns | Must not own |
|---|---|---|
| `protocol` | Durable wire/economic structs | Runtime behavior, storage policy, transport details |
| `codec` | rkyv bytes, hashes, signatures, canonical preimages | Admission policy, settlement policy |
| `request_auth` | User `WorkRequest` signature verification | User balance checks |
| `route_binding` / `route_policy` | Derived route bindings and transport policy | Packet ordering, settlement, admission-defined work authorization, node admission authority |
| `channel_order` | Ordered stream acceptance | Business validity |
| `work_channel` / transport adapters | Moving `ChannelEnvelope`s | Declaring work valid |
| `relay_role` | Relay forwarding, packet transit hashes, and final relay receipts once receiver proof exists | Admission, user balance, final settlement |
| `delivery_proof` / `transit_proof` | Recipient proofs and per-node packet-transit evidence | Pricing policy |
| `storage_payload` / `typed_storage_role` | Typed object store/retrieve payload verification | Admission, settlement |
| `settlement` / `batch_settlement` | Local model of receipt payment | Transport, route selection |
| `std_runtime` | std-only sockets, threads, synchronized wrappers | Core protocol invariants |

## Identity invariant

Every signed node identity must satisfy:

```text
node_id == public_key
```

This is mandatory for every path that accepts a `NodeIdentity` as authority or payment destination.

Roles are capabilities of a node identity. They do not create a different node id. The same public key can be used with different role declarations only where policy explicitly allows that capability, and role execution must check the local identity role before accepting work.

Applies to:

- `WorkReceipt.worker`
- `WorkAdmission.admission_node`
- `RelayAssignment.assigned_by`
- `NodeAvailable.node`
- `NodeHeartbeat.node`
- `ChannelProof` recipient verification

A signature that verifies against a public key is not enough. The signed public key must also equal the claimed `node_id`, and the claimed role must be accepted by the current protocol/policy context.

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

## Admission And Route Invariant

For user-owned resources, the user is the admission authority. A user may run the
admission node locally or delegate admission through a user-signed policy, but
work for that user's resources must originate from the user's signature. No
capability node, relay, transport adapter, or derived route cache can create
authority for user resources on its own.

A `WorkAdmission` means a DAO-authorized admission node claims:

```text
This signed user request passed admission policy and may enter the network with this budget, predefined relay path, and validity window.
```

Admission must commit to:

- `dao_id`
- `user`
- `admission_node`
- `request_hash`
- `assigned_route_commitment`
- `assigned_channel`
- `assigned_relay_path`
- `admitted_budget`
- `policy_hash`
- `sequence`
- `valid_until_unix_ms`

The admission-defined route is the only canonical route for new work. It must be derived from the signed `WorkRequest`, signed `WorkAdmission`, source node id, target role, and admission-assigned relay path.

`assigned_route_commitment` is an admission commitment to the selected route or
channel state. It is not node-authored availability and it is not accepted
unless it is inside a valid admission chain.

Node route bindings are not protocol authority. A node cannot bind itself into availability, select its own relay path, authorize its own capability access, or cause another node to accept work. Availability comes from admission state, and work access comes from the user/admission signature chain.

Workers do not re-check user balances. Workers validate that work belongs to an admitted chain, that the packet matches the admitted role/department/work type, and then execute local role rules.

Canonical helper:

```rust
verify_work_admission(admission)
admitted_capability_route_from_admission(...)
verify_admission_defined_route(...)
```

## Relay Topology Invariant

Admission is the control plane. Relays are the data plane. Capability nodes are leaves.

```text
admission
  <-> relay mesh
        <-> assigned capability leaves
```

Relays connect to admission and to each other. Every non-relay node connects to its assigned relay. A sender feeds packets to its relay. The destination receives packets from its relay. If the destination is behind another relay, the route is a predefined relay path signed into the work admission.

Admission must be able to route through multiple relays:

```text
sender -> relay_a -> relay_b -> ... -> relay_n -> capability
```

The relay path is not discovered from the destination node at send time. It is chosen by admission from current availability and signed into `WorkAdmission.assigned_relay_path`.

If a relay drops its admission connection, admission must remove the relay and every node assigned behind it. Those nodes must start over by asking admission for a new relay assignment. Stale assignments and stale node route state must not continue to authorize work.

## Route Binding Invariant

Node route bindings are not part of the canonical authorization model.
They must not be used to authorize new work, bind availability, publish
capability access, select relays, build work admissions, or validate packet
delivery.

`RouteBinding` is derived runtime state installed by admission/runtime code. It
is not signed by a capability node and must not be accepted as a work authority
without the user/admission chain.

Runtime APIs may install a `RouteBinding` so bytes can move, but the caller must
already be operating under admission/runtime authority. Installing a binding does
not authorize work and does not prove that a capability is available.

Canonical helpers:

```rust
verify_work_request(request)
verify_work_admission(admission)
admitted_capability_route_from_admission(...)
verify_admission_defined_route(...)
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
The expected route hash must come from the admission-defined route for the work,
not from a node-authored route binding.

Canonical helper:

```rust
ChannelOrderBook::accept(ordered, expected_route_hash)
```

## Relay transit invariant

A relay proves packet work by hashing the packet it handled and committing that hash into a relay-local transit hash chain.

Transit evidence commits to:

- relay `node_id`
- original sender
- final recipient
- channel id
- route hash
- forwarded packet hash
- relay sequence
- previous relay transit hash

Canonical helpers:

```rust
packet_transit_hash(input)
packet_transit_chain_hash(hashes)
RelayRole::forward_ordered_on(...)
```

Relay forwarding must fail closed. If packet serialization fails, no receipt may be created and no fallback hash such as `blake3("")` may be used.

For admitted work, relay forwarding must also match the admission-defined route:

- the packet `via_relay` must be the first relay in the assigned path
- each relay hop must be one of the relays committed by admission
- the final target must match the admitted capability node
- the department and work type must match the admitted capability route

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
channel_proof_hash(proof)
```

## Relay payment invariant

A relay is paid only after forwarding an admitted ordered message and obtaining recipient delivery proof.

The payable relay receipt output hash must commit to:

- relay transit hash
- forwarded packet hash
- receiver `ChannelProof` hash

Canonical helpers:

```rust
relay_delivery_output_hash(transit_hash, forwarded_packet_hash, receiver_channel_proof_hash)
RelayRole::finalized_delivery_receipt(delivery, receiver_channel_proof_hash)
SettlementLedger::settle_delivery(evidence)
verify_delivery_evidence(evidence)
```

Generic unchecked receipt settlement must not be used for relay payments. Relay receipts require typed delivery evidence.

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

Common receipt settlement checks:

- `verify_work_admission(admission)`
- `verify_work_receipt(receipt)`
- `receipt.admission_hash == hash(admission)`
- `receipt.request_hash == admission.request_hash`
- receipt hash has not already been paid
- cumulative admission spend plus claim is at most `admitted_budget`
- `admission.user` has enough balance

Typed settlement paths must add proof-specific checks. For relay delivery, `settle_delivery` must verify recipient proof, transit hash, forwarded packet hash, admission route binding, and receipt output hash.

Canonical helpers:

```rust
SettlementLedger::settle_delivery(evidence)
SettlementLedger::can_settle_delivery(evidence)
SettlementLedger::settle_receipt_unchecked_evidence(admission, receipt)
SettlementLedger::settle_receipt_batch_unchecked_evidence(admission, receipts)
build_receipt_batch(...)
```

The `*_unchecked_evidence` APIs are low-level test helpers. They reject relay receipts and must not be used for relay payment.

Batch settlement must be atomic:

- build and verify the batch
- reject duplicates inside the batch
- reject already-paid receipts
- reject over-budget batches
- only mutate ledger after preflight succeeds

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
