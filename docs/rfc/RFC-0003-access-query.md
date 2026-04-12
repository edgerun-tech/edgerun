# RFC-0003: Access, Query & Federation

**Status:** Working Draft
**Date:** 2026-04-12
**Based on:** Protocol spec §8, §10.7, §14.19-14.22

---

## Abstract

The access model implements receiver-driven retrieval, bounded queries, federated aggregation, and snapshot consumption. It maps to protocol spec sections 8 (Access Model) and 14.19-14.22 (schema definitions for snapshots, queries, and federated aggregates).

---

## Key Principles (from Spec §8)

1. **Access ≠ Replication ≠ Possession** — A node may access data without possessing it
2. **Receiver-driven retrieval** — Missing data repaired by later query, not sender retransmission
3. **Bounded queries** — Constrained by target, stream, time range, checkpoint, result count, cost limit
4. **Federated queries** — Each responder returns its own fragment; aggregation produces derived answer
5. **Snapshots are derived** — Never automatically authoritative over underlying streams

---

## Components

### edgerun-storage (Snapshot & Query Support)

**Status:** ⚠️ Partial

**Implemented:**
- `produce_snapshot(signer, view_type, completeness) -> SnapshotDescriptor` — Creates and signs snapshots
- `consume_snapshot(descriptor, trusted_producers) -> Result<String>` — Validates and accepts snapshots
- `list_snapshots()`, `get_snapshot(snapshot_id)` — Snapshot listing/lookup
- `record_command_outcome(...)` — Replay cache with `command_hash` key (§19.10)

**Not Implemented:**
- Query processing (no `QueryRequest`/`QueryResultFragment` handling)
- Federated aggregation (no `FederatedAggregateDescriptor` processing)
- Snapshot delta requests (§18.8)
- Proof bundle validation (§8.1)
- Promotion rules (§8.2)

### Snapshot Acceptance Classes (from §18.8)

| Class | Implementation |
|-------|---------------|
| `accepted_trusted` | ✅ Returned when producer in trusted list |
| `accepted_stale` | ⚠️ Returned when snapshot base is **behind** local head (`base_head.seq < local_seq`); when ahead, returns `Err` not acceptance |
| `accepted_cache_only` | ❌ Not implemented |
| `deferred_missing_payload` | ❌ Not implemented |
| `rejected_incompatible` | ⚠️ Invalid snapshots return `Err(StorageError::Decode(...))`, not the string `"rejected_incompatible"` |

### edgerun-replay

**Status:** ✅ Functional (5 tests)

Deterministic binary replay engine. See [RFC-0014](RFC-0014-analysis-tools.md#edgerun-replay).

---

## Spec Mapping

| Spec Section | Implementation | Status |
|---|---|---|
| §8 Access Model | Storage provides object retrieval | ⚠️ Partial |
| §8.1 Query Proof Normalization | — | ❌ Not implemented |
| §8.2 Promotion Rules | — | ❌ Not implemented |
| §10.7 Query & Federated Query | — | ❌ Not implemented |
| §10.8 Object Publication | `put_object`, `get_object` | ✅ |
| §10.9 Cold Access, Promotion, Eviction | — | ❌ Not implemented |
| §14.19 SnapshotDescriptor | Proto type + produce/consume | ✅ |
| §14.20 QueryRequest | Proto type only | ❌ |
| §14.21 QueryResultFragment | Proto type only | ❌ |
| §14.22 FederatedAggregateDescriptor | Proto type only | ❌ |
| §18.8 Snapshot Acceptance | `consume_snapshot` | ⚠️ Partial |
| §19.10 Replay Cache | `record_command_outcome` | ✅ |

---

## Outstanding Work

1. **Query processing** — `QueryRequest` → `QueryResultFragment` pipeline
2. **Federated aggregation** — Multiple responder fragments → derived answer
3. **Proof bundle validation** — Signature, stream head, event ref, snapshot base proofs
4. **Snapshot delta requests** — Request events between snapshot base and current head
5. **Cold access management** — Fetch queue prioritization, promotion/eviction heuristics
6. **Promotion rules** — Advisory → usable → promoted → authoritative state transitions
