# Proposal: Storage-Engine-Signed Append Pipeline

## Status

Proposal only. No implementation in this change.

## Goal

Make storage the trust anchor for append integrity.  
All committed events are signed by the storage engine at append time.

## Non-goals

- Defining MCP ingestion behavior.
- Implementing producer-side signatures.
- Migrating old historical records in this phase.

## Core Decisions

1. Only storage engine signs committed events.
2. Producers submit unsigned event intents.
3. Unsigned records are never committed.
4. Signature verification is deterministic and queryable.

## Append Flow

1. Producer submits event intent.
2. Storage engine validates ACL/policy.
3. Storage engine canonicalizes payload.
4. Storage engine resolves `prev_event_hash`.
5. Storage engine computes `event_hash`.
6. Storage engine signs the committed record.
7. Storage engine appends atomically.

If signing fails, append fails (no fallback mode).

## Committed Record Envelope

- `storage_id`
- `stream_id`
- `event_id`
- `job_id`
- `run_id`
- `actor`
- `event_type`
- `payload`
- `ts_unix_ms`
- `prev_event_hash`
- `event_hash`
- `storage_key_id`
- `storage_signature`
- `schema_version`

## Key Ownership and Rotation

- Signing keys are owned by storage engine runtime only.
- Assistant/clients must not access private keys.
- Every record includes `storage_key_id`.
- Rotation keeps old key IDs verifiable.

## Query/Verification Requirements

- Queries return enough fields to verify:
  - hash-chain continuity
  - signature validity
  - signing key identity

## Failure Semantics

- Any validation/hash/signature failure rejects append.
- Idempotent retry should preserve semantic event identity.
- No hidden mutation of committed event contents.

## Acceptance Criteria

1. Committed append path produces signed envelopes.
2. Verification API/tool confirms signature and chain integrity.
3. Unsigned or invalidly signed records cannot be committed.

## Open Questions

- Canonical JSON format and field ordering.
- Signature suite choice and key storage backend (TEE/HSM/soft for localnet).
- Backfill strategy for pre-signing-era events.

