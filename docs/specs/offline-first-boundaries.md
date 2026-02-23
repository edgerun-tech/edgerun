# SPDX-License-Identifier: Apache-2.0

# Offline-First Boundaries: `api.edgerun.tech` as Convenience Only

## Status
- Proposed
- Date: 2026-02-23
- Scope: control-plane boundaries, local shell availability, bootstrap and failover model

## Goal
Define hard architectural boundaries so the system remains operational when `api.edgerun.tech` is unavailable.

## Non-Goals
- Defining full on-chain program ABI changes in this document.
- Defining browser extension workflows.
- Introducing personal data collection or identity enrichment.

## Constraints and Security Requirements
- System correctness must not depend on `edgerun.tech` domain availability.
- Domain-hosted APIs are convenience adapters only.
- Browser local shell access must depend only on local daemon reachability (`127.0.0.1`), not cloud control plane.
- No personal data storage or processing in control-plane records.
- Identity and authorization are based on:
  - wallet public key,
  - device public key (TEE-backed where enforcement policy requires),
  - contract-verified signatures.
- Local browser state must be encrypted with a key derived from wallet signing flow (challenge-sign-KDF), not plaintext storage.
- Local daemon interfaces must bind to loopback and enforce origin/token checks.

## Core Boundaries

### 1. Core System Boundary
Core scheduler/worker/registry logic must be endpoint-agnostic and function with:
- on-chain registry as source of truth,
- configured bootstrap peers and/or local config,
- no hardcoded dependency on `api.edgerun.tech`.

### 2. Convenience Adapter Boundary
`api.edgerun.tech` may provide:
- bootstrap discovery convenience,
- UX aggregation,
- optional operator-hosted control APIs.

It must not provide unique required state that cannot be recovered from on-chain + local config.

### 3. Local Shell Boundary
Browser shell capability is provided by local daemon only.
- Browser -> `localhost` daemon for PTY stream.
- No browser extension requirement.
- Daemon enablement is explicit opt-in (`daemonize`/service install), not automatic during initial CLI install.

## Required Runtime Modes

### Mode A: Local-Only
- Browser can access local shell via localhost daemon.
- Works with no cloud endpoint.

### Mode B: Decentralized Network
- Scheduler/worker participate through on-chain registry + bootstrap peers.
- No hard requirement for `api.edgerun.tech`.

### Mode C: Convenience
- Uses `api.edgerun.tech` for easier onboarding/operations.
- Must gracefully degrade to Mode A/B when unavailable.

## Configuration Model
- No single hardcoded control URL in core runtime paths.
- Endpoint selection order:
  1. explicit operator/user config,
  2. on-chain advertised endpoints,
  3. convenience defaults.
- Failover list (multi-endpoint) required for control-plane adapters.
- Repo-stored, auditable defaults preferred over ad-hoc env-only behavior.

## Data Policy
- Allowed persisted identifiers:
  - wallet public key,
  - device public key,
  - bonding/proof metadata,
  - capability and health metadata.
- Disallowed:
  - personal profile data,
  - user content collection as identity metadata.

## Rollout Plan
1. Add endpoint adapter abstraction and remove hardcoded domain assumptions from runtime paths.
2. Gate convenience endpoint usage behind explicit adapter config.
3. Ensure frontend reads state from one selected control endpoint adapter and optional localhost daemon for shell.
4. Ensure daemonized local shell remains available regardless of cloud endpoint status.
5. Add tests for cloud-down behavior and localhost-only shell behavior.

## Rollback Notes
- Keep compatibility flags during migration:
  - `control_plane_mode = convenience|decentralized|local_only`
- If regressions occur, force `convenience` mode while preserving abstraction layer.

## Acceptance Criteria
- With `api.edgerun.tech` unreachable:
  - local daemon shell remains usable from browser,
  - decentralized scheduler/worker flows still start from configured/on-chain bootstrap,
  - frontend does not hard-fail; reports degraded convenience mode.
- No core crate requires literal `api.edgerun.tech` to compile or run.
- No browser extension requirement for local shell usage.
- Security checks for localhost daemon endpoints are enforced (loopback bind, token/origin validation).
- No personal-data fields are added to registry/control payloads.

## Verification Checklist
- Build:
  - `cargo check --workspace`
- Lint:
  - target crates with strict warnings
- Functional:
  - convenience endpoint down test
  - localhost daemon shell test
  - bootstrap fallback test (on-chain/configured endpoints)
