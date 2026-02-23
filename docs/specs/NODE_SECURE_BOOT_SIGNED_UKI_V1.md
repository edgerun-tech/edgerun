# Node Secure Boot Signed UKI V1

## Goal
- Produce a reproducible node boot artifact flow that builds a statically linked `edgerun-node-manager` PID 1 initramfs payload, composes a UKI, signs it with an EdgeRun certificate rooted in a YubiKey-managed key, and verifies Secure Boot properties.
- Enforce runtime checks for Secure Boot enabled state and locked kernel command line semantics.
- Ensure PID 1 startup path is `crates/edgerun-node-manager` and defaults to `https://api.edgerun.tech` for bonding/node init operations.
- Ensure `edgerun-node-manager` creates and manages a local Solana validator instance and uses its RPC endpoint for acceptance, registration metadata, and submission lifecycle operations.

## Non-goals
- No in-repo Linux kernel source import.
- No automated platform-specific firmware key enrollment tooling beyond documented command-level integration points.
- No bypass mode that disables secure-boot/cmdline checks in production path.

## Security and constraints
- `edgerun-node-manager` must build statically for node boot usage (`x86_64-unknown-linux-musl`).
- UKI must be signed and signature-verifiable (`sbverify`) before acceptance.
- Runtime must fail closed if Secure Boot variable indicates disabled mode.
- Runtime must fail closed when required locked-cmdline markers are absent.
- Default API base for boot path is `https://api.edgerun.tech`.
- Local validator default RPC is `http://127.0.0.1:8899` and must be health-checked before node lifecycle API calls.
- Build artifacts must stay under `out/`.

## Acceptance criteria
- A script builds static node-manager and validates static linkage.
- A script composes and signs UKI using a YubiKey-backed certificate flow and verifies signature.
- Node manager checks Secure Boot and locked cmdline when running as PID 1.
- Node manager includes API-driven bootstrap sequence aimed at bonding/node-init lifecycle.
- Node manager starts/restarts `solana-test-validator` locally and waits for JSON-RPC health before bootstrap/heartbeat calls.
- Required repository checks pass for changed Rust crate(s).

## Rollout and rollback
- Rollout: add scripts, hardening checks, and runtime bootstrap behavior; validate with local build/check/test where possible.
- Rollback: remove new scripts and revert node-manager secure boot bootstrap path to previous behavior.
