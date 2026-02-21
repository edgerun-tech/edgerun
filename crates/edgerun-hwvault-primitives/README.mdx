# edgerun-hwvault-primitives

Reusable primitives migrated from `hwvault` for use across edgerun components.

Current modules:

- `session`: bearer session creation, request HMAC signing contract, nonce replay protection, timestamp skew checks.
- `audit`: append-only JSONL policy audit events with recent-event listing.

These primitives are integrated into `edgerun-scheduler` for:

- `POST /v1/session/create`
- `POST /v1/session/rotate`
- `POST /v1/session/invalidate`
- signed access controls for:
  - `GET /v1/policy/info`
  - `GET /v1/policy/audit`
  - `GET /v1/trust/policy/get`
  - `POST /v1/trust/policy/set`
  - `GET /v1/attestation/policy/get`
  - `POST /v1/attestation/policy/set`
- job-create policy audit event emission

Default behavior in scheduler:

- policy session enforcement is enabled by default (`EDGERUN_SCHEDULER_REQUIRE_POLICY_SESSION=true`)
- session bootstrap token is optional (`EDGERUN_SCHEDULER_POLICY_SESSION_BOOTSTRAP_TOKEN`)
- shared multi-instance session file mode is optional (`EDGERUN_SCHEDULER_POLICY_SESSION_SHARED`)
