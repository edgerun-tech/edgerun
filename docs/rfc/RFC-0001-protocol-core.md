# RFC-0001: Protocol Core — Identity, Streams, Events, and Canonicalization

**Status:** Working Draft
**Date:** 2026-04-12
**Based on:** Codebase audit of 136 crates + edgerun Core Protocol v0 spec (3295 lines)

---

## Abstract

The Protocol Core implements the foundational layer of the edgerun Core Protocol v0: identity binding, single-writer streams, event envelopes, command envelopes, immutable objects, and deterministic canonicalization. It maps to spec sections 1-6, 14-17, and Appendices A-E.

---

## System Components

### `edgerun-proto` — Generated Protobuf Bindings

**Status:** ✅ Functional (generated)
**Tests:** None in this crate (validation happens in consumer crates)

#### Purpose
Auto-generated Rust types from 7 proto packages via `buf generate` → `protoc-gen-prost`. This is the single source of truth for all protocol message types across the entire workspace.

#### Proto Coverage

| Proto Package | Rust Module | Key Messages |
|--------------|-------------|-------------|
| `edgerun.v0.common` | `edgerun_proto::edgerun::v0::common` | `IdentityRef`, `NodeRef`, `StreamRef`, `EventRef`, `HeadRef`, `ObjectRef`, `Digest`, `Signature`, `TimeWindow`, enums (`IdentityKind`, `ObjectKind`, `AssuranceClass`, `TransportClass`, `StorageClass`, etc.) |
| `edgerun.v0.identity` | `edgerun_proto::edgerun::v0::identity` | `IdentityRecord` (key algorithm, public key, supersedes linkage) |
| `edgerun.v0.trust` | `edgerun_proto::edgerun::v0::trust` | `CapabilityDescriptor`, `CapabilityConstraint`, `CapabilityGrant`, `DelegationRecord`, `RevocationRecord`, `AssuranceClaim`, `AssuranceRequirement`, `ScopeDescriptor`, `ConstraintSet` |
| `edgerun.v0.stream` | `edgerun_proto::edgerun::v0::stream` | `EventEnvelope`, `NodeGenesisPayload`, `CommandEnvelope`, `CommandResultPayload`, `ActionLifecyclePayload`, `SecretPutPayload`, `SecretDeletePayload`, `CollectionCreatedPayload`, `CollectionDeletedPayload` |
| `edgerun.v0.object` | `edgerun_proto::edgerun::v0::object` | `LogicalObjectDescriptor`, `StoredRepresentationHeader`, `ChunkManifest`, `ChunkEntry` |
| `edgerun.v0.access` | `edgerun_proto::edgerun::v0::access` | `SnapshotDescriptor`, `QueryRequest`, `QueryResultFragment`, `FederatedAggregateDescriptor` |
| `edgerun.v0.network` | `edgerun_proto::edgerun::v0::network` | `ReachabilityHint`, `RouteAdvertisement`, `SessionHello`, `SessionAccept`, `RelayEnvelope` |

#### Generation Pipeline
```
proto/edgerun/v0/*.proto
    ↓ buf generate (buf.gen.yaml)
    ↓ protoc-gen-prost (Rust prost generator)
crates/edgerun-proto/src/gen/*.rs
```

#### Known Issues
- No tests in this crate (validation is deferred to consumer crates)
- Generated code uses `prost` which requires `std` (not `no_std` compatible)

---

### `edgerun-capabilities` — Capability Model Abstraction

**Status:** ⚠️ Partial (structural validation only, no session lifecycle)
**Tests:** ~50+ passing (874 lines of test code, 130 lines of impl)

#### Purpose
Provides the `CapabilityProvider` trait, capability descriptor builders, and structural validation for `CapabilityDescriptor` and `CapabilityGrant` messages.

#### Public API

| Item | Type | Description |
|------|------|-------------|
| `CapabilityProvider` | trait | Single method: `fn descriptor(&self) -> CapabilityDescriptor` |
| `CapabilityError` | enum | 4 variants: `InvalidRequest`, `PermissionDenied`, `Unsupported`, `Provider(String)` |
| `capability_descriptor()` | fn | Builder for `CapabilityDescriptor` proto (sets version=1) |
| `validate_descriptor()` | fn | 5-check validation: name, instance_id, modalities, event_kinds, operations |
| `validate_grant()` | fn | 3-check validation: grantee, selector, operations |
| `constraint()` | fn | Build `CapabilityConstraint` with just a kind |
| `constraint_with_uint()` | fn | Build constraint with kind + uint value |
| `constraint_with_scope()` | fn | Build constraint with scope string |

#### Re-exported Proto Types
All types from `edgerun_proto::edgerun::v0::capability`: `CapabilityAccessClass`, `CapabilityConstraint`, `CapabilityConstraintKind`, `CapabilityDescriptor`, `CapabilityEventKind`, `CapabilityGrant`, `CapabilityInvocation`, `CapabilityModality`, `CapabilityOperation`, `CapabilityRequest`, `CapabilityResult`, `CapabilityRevocation`, `CapabilityRole`, `CapabilitySelector`.

#### Spec Mapping

| Spec Section | Coverage |
|---|---|
| §3.14 Capability | Re-exports all types; provides builders |
| §7 Trust & Delegation | `validate_grant` enforces structural rules |
| §14.5 CapabilityDescriptor | Builder creates per-spec; validator enforces required fields |
| §18 Validation State Machines | Partial — structural only, no state machine transitions |

#### Known Issues
1. **No session lifecycle** — `CapabilityProvider` only has `descriptor()`. No `open_session`, `invoke`, `close_session`.
2. **No policy engine** — Grant evaluation and constraint enforcement are in `edgerun-capability-policy`.
3. **No revocation processing** — `CapabilityRevocation` is re-exported but has no validator.
4. **`'static str` error variants** — `InvalidRequest`, `PermissionDenied`, `Unsupported` use `&'static str`, not `String`. Dynamic error messages impossible except via `Provider(String)`.
5. **Empty `capability_id`** — Builder always produces empty `capability_id`. Caller has no way to set it.
6. **Not `no_std`** — Uses `std::error::Error`.

---

### `edgerun-storage` — Reference Storage Implementation

**Status:** ⚠️ Partial (functional with known bugs)
**Tests:** ~30+ inline tests

#### Purpose
Implements the storage profile from spec sections 6.1-6.3 and 19.9: append-only event log, encrypted blob store, and rebuildable indexes.

#### Architecture

```
NodeStore (facade)
  ├── Event Log: {data_root}/events/{stream_id_hex}.log (length-prefixed protobuf)
  ├── BlobStore: {data_root}/blobs/{prefix}/{blob_id}.blob (AES-256-GCM ciphertext)
  └── FileIndex: {data_root}/indexes/*.bin (in-memory HashMaps, full-rewrite persist)
```

#### Key Types

| Type | Description |
|------|-------------|
| `NodeStore` | High-level facade coordinating event log, blobs, and indexes |
| `BlobStore` | AES-256-GCM encrypted blob storage on filesystem |
| `BlobKeySource` | Software (HKDF from private key) or HardwareSealed (TPM/YubiKey) |
| `FileIndex` | In-memory HashMaps persisted to `.bin` files (interior mutability via `RefCell`) |
| `CredentialStore` | Standalone credential management with blob encryption |
| `ControllerSet` | Projects authoritative controller set from event history |
| `StorageError` | 5-variant enum: `Io`, `Encode`, `Decode`, `Encryption`, `Decryption` |

#### Public API (NodeStore)

| Method | Description |
|--------|-------------|
| `open(config) -> Result<Self>` | Creates directories, opens FileIndex and BlobStore |
| `append_event(envelope) -> Result<u64>` | Appends to `.log` file, returns byte offset |
| `get_event(stream_id, seq) -> Result<Option<EventEnvelope>>` | Reads from `.log` via FileIndex offset |
| `get_head(stream_id) -> Result<Option<(i64, Vec<u8>)>>` | Returns (seq, hash) from index |
| `put_blob(plaintext, recipients) -> Result<String>` | Encrypts and stores, returns blob_id (SHA-256 hex) |
| `get_blob(blob_id) -> Result<Option<BlobEntry>>` | Loads ciphertext + nonce |
| `put_object(content, kind, recipients) -> Result<ObjectRef>` | Stores as blob, returns ObjectRef |
| `get_object(object_ref) -> Result<Option<ObjectResult>>` | Loads object content |
| `produce_snapshot(signer, view_type, completeness) -> Result<SnapshotDescriptor>` | Creates and signs snapshot |
| `consume_snapshot(descriptor, trusted) -> Result<String>` | Validates and accepts snapshot |
| `record_command_outcome(node, cmd_id, cmd_hash, seq) -> Result<CommandReplayResult>` | Replay cache: New or Duplicate |
| `rebuild_indexes() -> Result<usize>` | Replays all `.log` files, repopulates indexes |
| `integrity_check() -> Result<bool>` | Validates index file formats |
| `project_controller_set(initial, up_to_seq) -> Result<ControllerSet>` | Projects controllers from event history |
| `put_credential(ns, name, secret, desc) -> Result<()>` | Encrypts and stores secret |
| `get_credential(ns, name) -> Result<Option<Vec<u8>>>` | Decrypts and returns secret |
| `process_fetch_queue() -> Result<usize>` | Processes pending fetch entries |

#### Spec Compliance

| Spec Requirement | Status | Notes |
|---|---|---|
| §6.1 Event log as source of truth | ✅ | Length-prefixed protobuf `.log` files |
| §6.1 Encrypted blobs on filesystem | ✅ | AES-256-GCM, two-level directory |
| §6.1 Indexes rebuildable | ✅ | `rebuild_indexes()` replays `.log` files |
| §6.2 Every blob encrypted at rest | ✅ | All writes go through AES-GCM |
| §6.2 Every blob names ≥1 recipient | ❌ **VIOLATION** | Code calls `store(plaintext, &[])` (empty recipients) |
| §6.2 No plaintext blob path | ✅ | No plaintext persistence |
| §19.9 `put_event`/`get_event` | ✅ | Implemented |
| §19.9 `get_head` | ✅ | Implemented |
| §19.9 `compare_and_set_head` | ❌ **MISSING** | `set_head()` is unconditional, no CAS |
| §19.10 Replay keyed by `command_hash` | ✅ | `CommandReplayResult::Duplicate` |
| §19.9 Atomic stream append | ❌ **VIOLATION** | `put_event` + `set_head` are two separate ops, not atomic |

#### Known Issues (Critical)

1. **`events.bin` validator mismatch** — `validate_events_log` expects `event_id` field that `put_event` doesn't write. `integrity_check` will incorrectly flag healthy files as corrupted.
2. **Blob recipients optional in code, mandatory in spec** — §6.2 requires ≥1 recipient; code allows `&[]`.
3. **No atomic CAS for head updates** — Spec requires `compare_and_set_head`; implementation does separate `put_event` + `set_head`.
4. **Object descriptors not persisted** — `LogicalObjectDescriptor` created in `put_object()` but dropped (`_descriptor`).
5. **`integrity_check_and_rebuild` is a stub** — Returns `Ok(0)` without rebuilding. Should call `self.rebuild_indexes()`.
6. **Full-write persistence** — Every mutation calls `save()` which rewrites ALL `.bin` files. O(N) per write.
7. **No atomic file writes** — `fs::write` directly; crash mid-write = corrupted `.bin`.
8. **Fetch queue priority underflow** — Priority is `i64`, decremented on retry with no floor.
9. **`FileIndex` not `Send`** — `RefCell` prevents multi-threaded access.
10. **Stale doc comments** — References to "SQLite" remain from a previous migration to file-based indexes.
11. **`events` HashMap is dead** — Written to file but never populated in the in-memory map.
12. **`CredentialStore` not composed in `NodeStore`** — Methods are duplicated instead.

---

## Canonicalization & Cryptographic Inputs

### Protocol v0 Requirements (Spec §17)

| Algorithm | Implementation |
|---|---|
| Hash: SHA-256 | `edgerun-crypto` / `edgerun-core::sha256` |
| Signature: ECDSA P-256 with SHA-256 | `edgerun-tpm` (raw TPM commands via `/dev/tpmrm0`) / `edgerun-hardware-signing::MeshSigner` |
| Protobuf: prost semantics | `prost` crate, deterministic encoding |

### Domain Separation Tags (Spec §17.9)

All cryptographic inputs use ASCII domain tags:
- `edgerun:v0:hash:event-envelope`
- `edgerun:v0:hash:command-envelope`
- `edgerun:v0:hash:delegation-record`
- `edgerun:v0:hash:revocation-record`
- `edgerun:v0:hash:snapshot-descriptor`
- `edgerun:v0:sig:event-envelope`
- `edgerun:v0:sig:command-envelope`
- `edgerun:v0:object`
- `edgerun:v0:representation-bytes`

These constants are defined in `edgerun-core` and consumed by `edgerun-storage` for snapshot signing.

### Record Hash Derivation (Spec §17.10)

```
record_hash = SHA-256(hash_domain_tag || 0x00 || protobuf_encode(signable_form))
```

Applied to: EventEnvelope, CommandEnvelope, DelegationRecord, RevocationRecord, SnapshotDescriptor, SessionHello, SessionAccept.

---

## Validation State Machines (Spec §18)

### Stream Append Validation (§18.4)

Implementation status:
- Structural check: ✅ (via `edgerun-storage` event log format)
- Crypto check: ⚠️ (signature verification exists but not integrated into stream append)
- Position check: ⚠️ (seq/hash tracking exists in FileIndex but not validated on append)
- Family check: ❌ (no event-family rule dispatch)
- Atomic append: ❌ (two separate operations)

### Command Acceptance Validation (§18.6)

Implementation status:
- Ingress screening: ❌
- Structural check: ⚠️ (via `validate_grant` for delegation chains)
- Crypto check: ❌ (signature verification not integrated)
- Replay check: ✅ (`record_command_outcome` with `command_hash` key)
- Authority check: ❌ (delegation chain validation not implemented)
- Decision record: ✅ (`CommandResultPayload` proto type exists)

### Delegation Chain Validation (§18.5)

Implementation status:
- Direct authority check: ❌
- Chain parse: ✅ (proto types exist)
- Chain crypto check: ❌ (signature verification not implemented)
- Chain continuity check: ❌
- Chain attenuation check: ❌
- Chain revocation check: ❌
- Root trust check: ❌

### Snapshot Acceptance (§18.8)

Implementation status:
- Descriptor check: ✅ (`consume_snapshot` validates structure)
- Producer trust check: ✅ (`trusted_producers` allowlist)
- Base check: ⚠️ (partial — "always accept stale snapshots" in v0)
- Payload fetch: ❌ (payload fetching not integrated)
- Delta request: ❌

---

## Dependency Graph

```
edgerun-proto (generated prost)
    ↑
    ├── edgerun-capabilities (capability validation)
    ├── edgerun-storage (consumes proto types for events, objects, snapshots)
    ├── edgerun-core (re-exports + utilities)
    └── all other crates (indirectly)

edgerun-crypto (workspace)
    ↑
    ├── edgerun-storage (AES-GCM for blob encryption)
    └── edgerun-hardware-signing (ECDSA signing)

edgerun-hardware-signing
    ↑
    └── edgerun-storage (snapshot signing via MeshSigner trait)
```

---

## Test-Derived Behavioral Specification

### edgerun-capabilities

| Test | Input | Expected Behavior | Spec Rule |
|------|-------|-------------------|-----------|
| `test_validate_descriptor_empty_name` | descriptor with empty provider_name | Returns `InvalidRequest` | §14.5: required fields |
| `test_validate_descriptor_whitespace_name` | descriptor with "   " provider_name | Returns `InvalidRequest` | §14.5: name must be non-empty |
| `test_validate_descriptor_empty_instance` | descriptor with empty provider_instance_id | Returns `InvalidRequest` | §14.5: required fields |
| `test_validate_descriptor_no_modalities` | descriptor with empty modalities | Returns `InvalidRequest` | §14.5: at least one modality |
| `test_validate_descriptor_no_events` | descriptor with empty event_kinds | Returns `InvalidRequest` | §14.5: at least one event kind |
| `test_validate_descriptor_no_operations` | descriptor with empty operations | Returns `InvalidRequest` | §14.5: at least one operation |
| `test_validate_descriptor_valid` | fully populated descriptor | Returns `Ok(())` | §14.5: valid descriptor |
| `test_validate_grant_no_grantee` | grant with no grantee | Returns `InvalidRequest` | §14.9: grantee required |
| `test_validate_grant_no_selector` | grant with no selector | Returns `InvalidRequest` | §14.9: selector required |
| `test_validate_grant_no_operations` | grant with empty operations | Returns `InvalidRequest` | §14.9: operations required |
| `test_validate_grant_valid` | fully populated grant | Returns `Ok(())` | §14.9: valid grant |
| `test_constraint_builders` | various constraint kinds | Correct proto fields set | §14.7: ConstraintSet |

### edgerun-storage

| Test | Input | Expected Behavior | Spec Rule |
|------|-------|-------------------|-----------|
| `test_append_and_get_event` | EventEnvelope appended | Returns identical envelope | §19.9: put_event/get_event |
| `test_get_head_returns_latest` | Multiple events appended | Returns latest seq+hash | §4: head tracking |
| `test_blob_store_encrypt_decrypt` | Plaintext stored | Decrypted matches original | §6.2: blob confidentiality |
| `test_blob_two_level_directory` | Blob stored | Path is `{prefix}/{id}.blob` | §6.1: storage layout |
| `test_put_and_get_object` | Content stored as object | Returns content via ObjectRef | §6: object model |
| `test_replay_cache_duplicate` | Same command_hash recorded twice | Returns `Duplicate` | §19.10: replay resistance |
| `test_replay_cache_different_hash` | Different command_hash | Returns `New` | §5.1: replay key is command_hash |
| `test_produce_and_consume_snapshot` | Snapshot produced and consumed | Returns `accepted_trusted` | §10.6: snapshot flow |
| `test_credential_put_get_delete` | Secret stored and retrieved | Decrypted matches original | §14.16: SecretPutPayload |
| `test_credential_rotate` | Credential rotated | New value returned, old gone | §6: immutable objects |
| `test_controller_set_projection` | Controller change events recorded | Correct final controller set | §10.10: control change |
| `test_rebuild_indexes_repopulates_heads` | Indexes cleared, rebuilt | Heads restored from `.log` | §6.3: rebuildability |
| `test_fetch_queue_priority` | Items enqueued with priorities | Higher priority dequeued first | §8: access model |
| `test_list_stream_heads` | Multiple streams with events | All heads returned | §4: stream model |
| `test_list_unreachable_peers` | Peers with unreachable status | Only unreachable returned | §9: networking model |

---

## Outstanding Work

### Critical (spec violations)
1. **Implement `compare_and_set_head`** — Atomic CAS for stream head updates (§19.9)
2. **Require blob recipients** — Enforce ≥1 recipient on all `store()` calls (§6.2)
3. **Fix `events.bin` validator** — Align `validate_events_log` with `put_event` format
4. **Make stream append atomic** — Combine `put_event` + `set_head` into single atomic operation

### High Priority
5. **Implement `integrity_check_and_rebuild`** — Call `self.rebuild_indexes()` instead of returning stub
6. **Persist `LogicalObjectDescriptor`** — Store descriptors, don't drop them
7. **Implement delegation chain validation** — Signature verification, continuity, attenuation checks (§18.5)
8. **Implement command acceptance validation** — Full §18.6 state machine
9. **Add atomic file writes** — Write-to-temp + rename for `.bin` files

### Medium Priority
10. **Implement event-family rule dispatch** — §18.4 family check
11. **Implement snapshot delta request** — §18.8 delta handling
12. **Implement ingress screening** — §18.3 cheap-first screening order
13. **Add minimum priority floor to fetch queue** — Prevent infinite priority decrement

### Low Priority
14. **Clean up dead code** — Remove unused `events` HashMap, stale SQLite comments
15. **Compose `CredentialStore` in `NodeStore`** — Eliminate method duplication
16. **Add `validate_invocation`, `validate_request`, `validate_result`** — Missing proto validators
17. **Add constraint builders for duration and rate_limit** — Missing `CapabilityConstraint` builders
