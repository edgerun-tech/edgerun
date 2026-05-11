# Agent Instructions

This repository currently has active protocol work in `crates/protocol/edgerun-work`.

Do not treat `edgerun-work` as a generic message-passing crate. It is a verifiable work protocol: signed user intent enters through admission, work moves through ordered channels over arbitrary transports, roles execute typed work, proof objects make receipts payable, and settlement debits users and credits workers.

## Current state

`edgerun-work` currently includes these important modules:

- `protocol.rs`: core durable work/economic wire objects: `WorkSignature`, `NodeIdentity`, `NetworkMessage`, `WorkRequest`, `WorkAdmission`, `WorkReceipt`, `WorkPacket`, `WorkAck`.
- `node_control.rs`: node-control wire objects: `RelayEndpoint`, `NodeAvailable`, `NodeHeartbeat`, `RelayPeerList`, `RelayAssignment`.
- `codec.rs`: rkyv packet encoding/decoding and packet hash helpers only. Do not put signing or identity helpers back in `codec.rs`.
- `identity.rs`: `derive_node_id`, `verify_node_identity`, `node_identity_from_key`.
- `signing.rs`: signed-object preimages, `sign_*`, and `verify_*` helpers.
- `preimage.rs`: `PreimageBuilder` for signature preimages and `HashBuilder` for domain-separated BLAKE3 commitments.
- `types.rs`: typed wrappers for `NodeRole`, `WorkType`, and `Department`, while wire structs still use `u16`.
- `request_auth.rs`: signed user `WorkRequest` verification.
- `route_auth.rs` / `route_plan.rs`: signed route advertisements, route snapshots, route roots, and route selection.
- `channel_order.rs`: ordered channel state and ordered message hashes.
- `delivery_proof.rs` / `transit_proof.rs`: recipient delivery proofs and relay transit commitments.
- `relay_role.rs`: relay forwarding, transit receipts, and finalized relay delivery receipts.
- `storage_payload.rs` / `typed_storage_role.rs`: typed object store/retrieve payloads and typed storage role.
- `erasure_storage.rs`: XOR 2+1 proof erasure model. This is a proof/test scheme, not final production erasure coding.
- `settlement.rs` / `batch_settlement.rs`: local settlement model, typed relay delivery settlement, unchecked non-relay receipt settlement, and batch settlement.
- `std_runtime/*`: std-only runtime code such as TCP, threads, admission runtime, client runtime, and synchronized wrappers.

## Current working model

The working proof chain is:

```text
signed WorkRequest
→ signed WorkAdmission
→ signed RouteAdvertisement / RouteSnapshot
→ ordered ChannelEnvelope
→ role execution
→ delivery/transit/storage proof
→ signed WorkReceipt
→ settlement / batch settlement
```

Transport is not the trust boundary. Memory, TCP, WebSocket, browser worker `postMessage`, email import, QR import, USB, Bluetooth, QUIC, WebTransport, or future transports may move the same bytes. Protocol validity is decided by signatures, hashes, route commitments, ordered-channel checks, typed payload verification, proof objects, receipts, and settlement.

## Non-negotiable invariants

1. A `NodeIdentity` is valid only if:

```text
node_id == derive_node_id(public_key, role)
```

2. Admission must never trust a `WorkRequest` before `verify_work_request` succeeds.

3. Workers should not check user balances. Admission checks user signature, policy, balance, route, and budget. Workers check admitted chains and local role rules.

4. Route advertisements may validly be `AVAILABLE`, `DRAINING`, or `UNAVAILABLE`; route selection for new work must normally use only `AVAILABLE`.

5. Ordered channels accept messages only when packet hash, route hash, sequence, and previous-message hash match.

6. Relay payment must not use unchecked generic settlement. Relay payment requires typed delivery evidence: relay transit hash, forwarded packet hash, and recipient `ChannelProof` hash.

7. Storage payment must eventually be tied to typed storage/retrieval/availability evidence. Storage receipts without retrievability are not production-complete.

8. Batch settlement must be atomic: preflight the whole batch before mutating ledger state.

9. Do not prune paid receipt/admission tracking until an admission is finalized and its challenge window has closed.

10. Core protocol code should remain `no_std + alloc`. Sockets, threads, filesystem, and locks belong in `std_runtime`.

## Import boundaries

Do not import signing or identity helpers from `codec.rs`.

Use:

```rust
use crate::identity::{derive_node_id, node_identity_from_key, verify_node_identity};
use crate::signing::{empty_signature, sign_work_receipt, verify_work_receipt};
use crate::codec::{blake3_hash, packet_bytes, packet_hash};
```

Do not reintroduce compatibility re-exports from `codec.rs`.

For new hash/preimage code:

- Use `PreimageBuilder` for bytes that are signed.
- Use `HashBuilder` for BLAKE3 commitments and Merkle-like hashes.
- Every protocol/economic hash must have an explicit domain string.

## Test expectations

Before and after changes, run:

```bash
cargo test --manifest-path crates/protocol/edgerun-work/Cargo.toml
```

Also run no-std and wasm checks when touching core protocol, encoding, signing, route, settlement, storage, or WASM code:

```bash
cargo test --manifest-path crates/protocol/edgerun-work/Cargo.toml --no-default-features
cargo build --manifest-path crates/protocol/edgerun-work/Cargo.toml --target wasm32-unknown-unknown --release --no-default-features
```

Run the size script when touching WASM-facing code:

```bash
crates/protocol/edgerun-work/scripts/wasm-size.sh
```

## Current priority order

1. Keep the full test suite green.
2. Add/fill golden protocol hash tests for the current green state.
3. Replace broad `edgerun_work::*` imports in new tests with explicit imports.
4. Add shared test builders for admissions, receipts, routes, nodes, storage jobs, and delivery evidence.
5. Add typed verifier errors for `WorkRequest`, `WorkAdmission`, `WorkReceipt`, route ads, route snapshots, channel proofs, storage payloads, and settlement evidence.
6. Add settlement state/finalization before expanding pruning/challenge logic.
7. Add a `challenge.rs` module for challenge/evidence objects.
8. Only after protocol and proof settlement are stable, build a real `edgerun-work-node` binary.

## Things not to do

- Do not add more transports right now.
- Do not start Solana program work until proof objects and settlement evidence are stable.
- Do not use unchecked receipt settlement for relay receipts.
- Do not move std-only functionality into core modules.
- Do not silently change protocol hashes without updating golden hash tests.
- Do not hide security-sensitive imports through wildcard prelude tricks.

## Review checklist for any new protocol object

For every signed/economic/proof object, answer these before merging:

1. What does it claim?
2. Who signs it?
3. Which fields are covered by the signature?
4. What hash identifies it?
5. What canonical verifier exists?
6. What prior object does it depend on?
7. What later object consumes it?
8. What makes it payable?
9. What makes it slashable or challengeable?
10. What test proves bad data is rejected?
