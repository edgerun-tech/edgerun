# edgerun-sdk Crate Report

Reviewed crate: `/home/ken/edgerun/crates/edgerun-sdk`

This report is based on three passes over the crate: inventory, authored source/docs/DSL review, and verification through the crate's own commands.

No files were modified during the review.

## Scope Read

- 315 non-build-output files excluding `target` and `target-native`.
- 6,323 generated build-output files under unit `rust/target*` directories, inventoried but not manually decoded.
- Authored Rust reviewed:
  - `src/lib.rs`
  - `src/main.rs`
  - `src/runtime.rs`
  - `src/package.rs`
  - `src/marketplace.rs`
  - `src/browser_authoring.rs`
  - `src/deploy.rs`
  - `src/runtime_api.rs`
  - `src/formats.rs`
- Binary artifacts read by size/hash/type, not semantically decoded:
  - `.wasm`
  - `.so`
  - rkyv `.edm`, `.edr`, `.eseg`, `.echn`, `.eapp`, and related app/marketplace artifacts

## What This Crate Is

`edgerun-sdk` is both a library and a CLI for deterministic Edgerun app construction.

The core contract is not Rust API stability. The core contract is pinned artifacts:

```text
unit.wasm + manifest.edm + api.edm + exact SHA-256 bindings
```

The crate models deterministic "standard modules" as standalone wasm units with owned memory, no imports, required identity exports, and byte-oriented APIs. Compositions copy bytes between unit memories and execute a small DSL-compiled step graph.

## Cargo Shape

`Cargo.toml` defines:

- a library crate, `#![no_std]` by default
- CLI binary `edgerun-sdk`, behind `std`
- optional `browser-authoring` and `runtime-api` features
- default feature `std`
- dependencies mostly on workspace Edgerun crates:
  - `edgerun-crypto`
  - `edgerun-protocols`
  - `edgerun-wire`
  - `edgerun-json`

## Library API

`src/lib.rs` exposes the minimal stable SDK model:

- constants:
  - `SDK_ABI_NAME = "standard-module-v1"`
  - `SDK_ABI_VERSION = 2`
- typed manifest structs:
  - `UnitManifest`
  - `CompositionManifest`
  - `SegmentManifest`
  - `ChainManifest`
- `sha256` and `sha256_hex`, implemented in-crate
- `str_eq`, a simple byte equality helper, not constant-time

`src/runtime_api.rs` builds app-facing runtime records:

- app id, release id, and manifest hash derivation
- HTTP route records
- app install records
- capability declaration records

`src/browser_authoring.rs` builds publishable browser app records:

- derives app id from slug and developer Ed25519 public key
- creates `AppManifestRecord` and `AppGraphRecord`
- computes code hash from artifact records
- signs the app graph with a domain-separated Ed25519 signature
- validates artifact paths to block absolute paths, `..`, empty segments, and reserved names

## CLI Surface

`src/main.rs` dispatches a large command set:

- runtime/composition:
  - `list`
  - `verify`
  - `run-composition`
  - `quote-composition`
  - `preflight-composition`
- distributed execution:
  - `run-segment`
  - `verify-segment-report`
  - `sign-segment-report`
  - `verify-chain-reports`
- packaging:
  - `package-app`
  - `sign-app`
  - `verify-signed-app`
  - catalog writing
- marketplace/product/payment/trust/revocation flows
- capability request/response/sign/seal/storage/profile commands
- deployment helpers over SSH

## Runtime And Verifier

`src/runtime.rs` is the main engine.

Important behavior:

- Discovers unit sources under `units/<id>/rust/Cargo.toml`.
- Parses `[package.metadata.edgerun.unit]`.
- Compiles units to `wasm32-unknown-unknown` for artifact generation.
- Parses wasm directly with its own `WasmCursor` / `WasmSurface` logic.
- Requires:
  - zero imports
  - exported memory
  - `proto_abi_version() -> i32`
  - `proto_standard_id() -> i32`
  - at least one unit function
  - only i32 cost-inferable API signatures
- Generates rkyv records for:
  - `UnitManifest`
  - `UnitApi`
  - `Composition`
  - `Segment`
  - `Chain`
  - reports
  - signatures
- Executes compositions through opcodes:
  - `copy-input`
  - `call`
  - `copy`
  - `publish`
  - `branch`
  - `jump`
  - `write-byte`
  - `capture`
  - `load-u32`
- Supports native `.so` loading for accelerated/local execution with a small ABI shim around exported functions and memory.
- Provides length-only quote/preflight paths using symbolic scalar ranges.

## Packaging

`src/package.rs` handles:

- `build-artifacts`
- unit metadata generation
- packaged browser app generation
- native unit implementation discovery/building
- browser JS/HTML templates for SHA-256 and HMAC composition demos
- benchmark support for wasm/native/system wasmtime paths

The generated browser apps in `dist/` are real examples:

- `sha256-fips180-app`
- `hmac-sha256-rfc2104-composed-app`

They include:

- `app.edapp` JSON app manifests
- rkyv app graphs
- wasm units
- native dylibs where present
- browser runtimes that verify hashes before executing wasm

## Marketplace

`src/marketplace.rs` is the largest file. It implements:

- app signing and signed-app verification
- app store catalog generation
- product issuance/verification
- entitlement issuance/verification
- settlement/payment records
- trust policy and revocation enforcement
- capability request/response flows for signing, sealing, storage
- encrypted user profile creation/opening, including password KDF support
- profile-based capability authorization

This file is broad and operational. It mixes CLI argument parsing, wire record creation, signature verification, local provider behavior, and policy checks.

## Units

The crate currently discovers 34 units.

Main categories:

- crypto:
  - `sha256-fips180`
  - `sha384-fips180`
  - `sha512-fips180`
  - `hmac-sha256-rfc2104`
  - `rfc2104-pad`
  - `edgerun-keygen-v1`
  - `edgerun-sign-v1`
  - `edgerun-verify-v1`
- encoding:
  - `base16-rfc4648`
  - `base32hex-rfc4648`
  - `base64url-rfc4648`
  - `cbor-rfc8949`
  - `form-urlencoded-v1`
  - `json-field-v1`
  - `utf8-rfc3629`
- protocols:
  - `ipv4-rfc791`
  - `udp-rfc768`
  - `tftp-rfc1350`
  - `dns-rfc1035`
  - `http-token-rfc9110`
  - `http-field-rfc9110`
  - `http-response-rfc9110`
  - `websocket-rfc6455`
  - `quic-varint-rfc9000`
  - `ethernet-ipv4-v1`
  - `bluetooth-gatt-v1`
  - `proxy-v1`
- policy/runtime helpers:
  - `byte-tools-v1`
  - `constant-time-eq-v1`
  - `decision-byte-v1`
  - `capability-policy-v1`
  - `capability-session-v1`
  - `capability-provider-v1`

All unit source crates follow the expected pattern:

```rust
#![no_std]
edgerun_unit::no_alloc!();
edgerun_unit::metadata!(...);

#[edgerun_unit::export]
...
```

## Compositions

Editable sources live in `compositions/`.

- `hmac-sha256-rfc2104-composed`
  - pads key
  - handles long-key SHA-256 branch
  - computes inner/outer digest
- `hkdf-extract-sha256-rfc5869`
  - HMAC over salt and IKM
- `hkdf-expand-sha256-rfc5869-l42`
  - fixed two-block `L=42` HKDF expand profile
- `http-auth-preflight-rfc9110`
  - parses an HTTP field line and compares header name case-insensitively
- `auth-decision-private-v1`
  - private decision byte equality
- `hmac-sha256-verify-rfc2104`
  - HMAC then constant-time tag comparison

## Segments And Chains

`segments/` defines distributed execution boundaries:

- public HTTP auth preflight segment
- private auth decision segment
- private HMAC verify segment

`chains/` defines:

- one-segment HTTP preflight chain
- two-segment HTTP preflight plus private decision chain, linking output `0.0 -> 1.0`

## Docs

`docs/` consistently says old magic-header byte formats are removed. Current artifacts are rkyv `SdkWireRecord` variants.

The docs are short but coherent and cover:

- unit manifests
- API manifests
- composition records
- reports
- segments
- signatures
- signer policy
- app graph
- product/payment/settlement/trust/revocation/profile/capability records

## Verification Results

Commands run from `/home/ken/edgerun`:

```bash
cargo test -q -p edgerun-sdk
cargo run -q -p edgerun-sdk -- list
cargo run -q -p edgerun-sdk -- verify
```

Results:

- Tests passed: 60 total.
- `list` loaded all 34 units and printed pinned wasm hashes.
- `verify` passed all unit checks:
  - wasm hash
  - binary manifest
  - binary API
  - wasm validation
  - unit surface policy
  - identity exports
  - wasm surface
  - no shared-memory imports
- `verify` passed all composition checks:
  - composition hash
  - binary composition
  - composition execution
- Native SHA-256 dylib conformance also passed.

## Notable Risks / Design Friction

- The crate has no internal split between CLI layer and domain logic in `src/marketplace.rs`. At 5,195 lines, it is hard to audit policy-sensitive behavior.
- The DSL parser in `src/runtime.rs` is intentionally simple whitespace parsing. That is fine for controlled files, but diagnostics and grammar evolution will get brittle.
- Some binary files in `dist/sha256-fips180-app` still show old magic-like prefixes such as `EENT`, `EPAY`, and `ETRU`, while docs say old compact formats are removed and rkyv is canonical. This may be intentional legacy sample output, but it is worth reconciling.
- Build artifacts under each unit's `rust/target*` directories are present in the crate tree. That adds noise and size, and makes review work harder.
- Native dynamic library loading is necessarily unsafe and platform-specific. The code has bounds checks around mapped memory, but this area deserves focused security review if it is used outside local tooling.
- The app/browser package format is partly rkyv graph plus JSON `app.edapp` for browser loading. That duality is practical, but the trust boundary should stay clearly documented.

## Bottom Line

This crate is a working SDK/CLI artifact factory for deterministic Edgerun units.

The strongest parts are:

- artifact verification model
- hash-pinned composition records
- owned-memory wasm policy
- reproducible DSL-to-rkyv flow

The weakest part is maintainability: `runtime.rs` and especially `marketplace.rs` are doing many jobs in large files, so future changes in policy, payment, profiles, or capability authorization will be harder to review safely.
