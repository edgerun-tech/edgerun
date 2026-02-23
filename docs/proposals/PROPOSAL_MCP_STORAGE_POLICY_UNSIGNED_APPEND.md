# SPDX-License-Identifier: Apache-2.0

# Proposal: MCP storage policy + unsigned append (phase 1)

## Goal
Enable repository-scoped, auditable storage behavior while unblocking immediate event capture with an unsigned append API.

## Scope
- Add repo policy file at `config/mcp_storage_policy.toml`.
- Add MCP method `append_storage`.
- Keep append unsigned for now.
- Keep request/response contract stable so signature enforcement can be enabled by policy later.

## Policy model
Policy is loaded by `edgerun mcp serve` from:
- default: `<repo-root>/config/mcp_storage_policy.toml`
- override: `--policy-file <path>`

Current policy keys:
- `[storage]`
- `default_tier`
- `default_segment`
- `[append]`
- `enabled`
- `require_signature`
- `default_actor`
- `default_stream`
- `default_event_type`
- `default_durability`
- `max_payload_bytes`

## API contract
`append_storage` request:
- `storage_id` (required)
- `payload` (optional, defaults `{}`)
- `actor` (optional)
- `stream` (optional)
- `event_type` (optional)
- `durability` (optional: `buffered|local|durable|checkpointed`)
- `signature` (optional now; required later when policy flips)

`append_storage` response:
- `ok`
- `storage_id`
- `event_hash`
- `offset`
- `durability`

## Migration path to signed append
Phase 1 (this change):
- `require_signature = false`
- append accepted without signature.

Phase 2:
- set `require_signature = true`
- reject missing signature in MCP layer.

Phase 3:
- verify signatures against identity policy and reject invalid signatures.
- make signature verification mandatory in production profiles.

## Rationale
- Provides immediate logging utility.
- Keeps deterministic behavior controlled by repo-committed policy.
- Avoids interface churn when signature verification is added.
