# Profile Encrypted Control Profile Proto V1

## Goal
- Define a lean protobuf contract for user control profiles that boot quickly and can be encrypted at rest.
- Keep profile bootstrap metadata minimal while allowing larger encrypted state and timeline chunks to be fetched on demand.
- Standardize one profile schema usable by node/bootstrap/control-plane components.
- Keep authentication factors separate from storage backends so users can use YubiKey/WebAuthn without Google identity coupling.
- Model capability and control-plane authorization in OIDC scope requirements.

## Non-goals
- No JSON schema fallback.
- No transport protocol definition in this change.
- No key management implementation details beyond envelope fields.
- No implementation of Google Drive or Git sync clients in this change.
- No OIDC provider implementation details or token-exchange flow in this change.

## Security and constraints
- Profile must be `fail-closed`: missing or invalid cryptographic metadata means profile is unusable.
- Keep plaintext bootstrap head minimal to reduce startup latency.
- Encrypted payloads must carry algorithm + key reference metadata.
- Integrity metadata (hash/signature fields) must be present for every encrypted payload section.
- Backward-compatible evolution uses protobuf field reservation discipline.
- Authentication and storage selection are decoupled:
  - auth factors prove who can unlock profile,
  - storage backends only define where encrypted profile blobs are stored/synced.
- Unlock policy metadata must explicitly declare gating behavior for ephemeral session mode versus profile mode.
- Unlock and capability routes must support explicit OIDC scope checks (`required_all`, `required_any`) so policy evaluation can fail closed.

## Acceptance criteria
- A new protobuf file exists at `crates/edgerun-runtime-proto/proto/profile/v1/profile.proto`.
- OIDC scope contract exists at `crates/edgerun-runtime-proto/proto/profile/v1/oidc_scopes.proto`.
- Schema includes:
  - lightweight profile head metadata,
  - encrypted state chunk envelope,
  - encrypted secrets bundle envelope,
  - timeline segment envelope,
  - top-level profile document envelope,
  - storage backend descriptors,
  - WebAuthn credential bindings,
  - unlock policy and capability gates,
  - OIDC scope catalog and principal binding metadata,
  - OIDC scope requirements attached to unlock policy and capability gates.
- `edgerun-runtime-proto` compiles the new proto via `prost_build`.
- Generated types are re-exported by the crate so other components can depend on one canonical profile contract.
- `cargo check --workspace` passes.

## Rollout and rollback
- Rollout: additive proto contract in `edgerun-runtime-proto`; wire consumers incrementally.
- Rollback: remove proto and build wiring in this crate if downstream integration uncovers regressions.
