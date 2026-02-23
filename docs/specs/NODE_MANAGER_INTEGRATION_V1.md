# Node Manager Integration V1

## Goal
- Introduce an in-repo `edgerun-node-manager` runtime that integrates the existing `/home/ken/src/edged` node-manager concepts into `edgerun`.
- Reuse `edgerun-hwvault-primitives` as the canonical TPM/hardware identity implementation.
- Provide a deterministic manager entrypoint that can:
  - derive hardware-backed node identity,
  - perform challenge/handshake registration against an API endpoint,
  - run a control-plane heartbeat loop with bounded worker concurrency metadata.

## Non-goals
- No full EFI/kernel embedding implementation in this change.
- No complete import of all `edged` init/session/mesh/TUI codepaths.
- No on-chain bonding workflow implementation yet.

## Security and constraints
- Default hardware mode is TPM-required (`TpmRequired`).
- Do not duplicate TPM key/record logic already implemented by `edgerun-hwvault-primitives`.
- Keep registration flow challenge/response based and explicit.
- Keep runtime behavior deterministic and observable through structured logs.
- Node manager configuration (`control_ws_url`, `rpc_url`, worker settings, bonding state) is stored in TPM NVRAM.
- Production flow must not include runtime bypass toggles for TPM enforcement.
- After bonding, manager configuration is immutable.

## Acceptance criteria
- New workspace crate `crates/edgerun-node-manager` exists with a CLI binary.
- CLI supports:
  - `identity` (print node identity/backend),
  - `register` (challenge + handshake against configured API),
  - `run` (periodic control-plane heartbeat loop), with `--control-ws-url` and `--rpc-url` for initial/pre-bond config persistence in TPM NVRAM.
- Manager uses `edgerun-hwvault-primitives::hardware::load_or_create_device_signer`.
- Manager uses existing control-plane schema from `edgerun-types` for heartbeat messages.
- Build/type validation passes for the new crate and workspace checks remain green.

## Rollout and rollback
- Rollout: add crate and workspace member, keep integration additive and isolated.
- Rollback: remove workspace member and crate if integration causes regressions.
