# edgerun-hwvault-primitives

Reusable primitives migrated from `hwvault` for use across edgerun components.

Current modules:

- `session`: bearer session creation, request HMAC signing contract, nonce replay protection, timestamp skew checks.
- `audit`: append-only JSONL policy audit events with recent-event listing.

These primitives are integrated into `edgerun-scheduler` for:

- `POST /v1/session/create`
- optional signed access controls for `/v1/policy/info` and `/v1/policy/audit`
- job-create policy audit event emission
