# EdgeRun Agent Guide

This repository builds EdgeRun: user-owned internet infrastructure for identity,
apps, storage, routing, compute, publishing, and proof-backed payments.

EdgeRun is not a token-first crypto product and not a generic cloud clone. It is
usage-first infrastructure where users and developers can run, publish, host,
sell, cache, relay, store, compute, and settle through verifiable work. Payments
follow proof of useful work.

## Engineering Standard

The goal is not to merely get tests green. The goal is to produce code that is
small, coherent, defensible, and good enough to present publicly and depend on
daily with a clear conscience.

Work by these rules:

- Always read the existing code before writing. Inspect the relevant modules,
  tests, call sites, feature flags, and existing patterns first.
- Do not claim behavior, compatibility, performance, security, or correctness
  without proof. Run the relevant checks, inspect the output, and state exactly
  what was verified.
- Do not add external dependencies. Use the standard library, existing workspace
  crates, or small local code that fits the existing design. If a dependency is
  truly unavoidable, stop and get explicit approval before changing manifests.
- Design portable logic as `no_std + alloc` first so it can run in browser WASM.
  Native `std` code is thin OS glue, not the home for protocol, policy, wire,
  storage, routing, or app semantics.
- Move data across boundaries through `edgerun-wire` records and rkyv bytes.
  Do not invent side-channel structs or ad hoc serialization for browser/native
  crossings.
- Prefer deleting, consolidating, and simplifying code over adding new layers.
  Remove dead paths, duplicate models, stale compatibility shims, and redundant
  abstractions whenever it is safe and in scope.
- Keep one canonical implementation for each concept. Do not create parallel
  app/package/protocol/wire/policy models just to make a local task easier.
- Treat tests as evidence, not the finish line. A change that passes tests but
  leaves unclear ownership, duplicated logic, hidden protocol drift, or fragile
  behavior is not done.
- Preserve auditability. Important behavior should have a verifier, a test, a
  proof trail, or a clearly documented reason it cannot yet be proven.
- When uncertain, narrow the change and prove the smaller claim instead of
  expanding scope.

## Product Direction

Keep this public framing:

> Own your identity and data. Run apps from verified network storage. Cache
> locally when you want. Publish from your own node. Enforce your own policy.
> Developers sell directly. Infrastructure gets paid for useful work.

Prefer these terms in user-facing UX:

- "Run" instead of "Install".
- "Verify & cache" instead of "Install".
- "Cached" instead of "Installed".
- "Remove cache" instead of "Uninstall".
- "Network app" instead of "catalog app".
- "Trust Container" for the sealed local profile/root.
- "Proof dashboard" for Trust Manager.
- "Publisher policy" or "app policy" instead of "DRM".
- "Import/sync source" for Gmail, Drive, GitHub, mailboxes, folders, and other
  legacy services.
- "Publish from your node" or "identity-routed publishing" for user-hosted
  services.
- "Admission policy" for the user-owned policy gate.
- "Node instance" for a role-specific WASM/native node with identity, policy,
  budget, and route scope.

Do not present users as only app consumers. The architecture should make it
natural for a user to become a publisher, host, storage provider, relay, compute
provider, data source, app seller, or service endpoint.

## Core Product Model

Every user starts with a browser node. Treat the browser node as a first-class
local node instance that can:

- hold and unlock a sealed Trust Container;
- sign through password or passkey;
- verify signed app packages and content hashes;
- run apps from network storage;
- cache verified package bytes locally;
- request and grant capabilities;
- record local proof/audit events;
- eventually earn from browser-appropriate useful work;
- publish identity-routed services when enabled by policy.

The intended user flow is:

```text
create/unlock identity
-> browser node starts
-> choose EdgeRun DAO admission or user-owned admission
-> define node/admission policy
-> connect contacts and old data sources
-> run app from network storage
-> optionally cache verified bytes
-> publish or sync what the user chooses
-> inspect proof in Trust Manager
-> earn/spend through usage
```

On first run of a network app, the UX should ask:

```text
This app runs from EdgeRun network storage.
Your browser node will retrieve signed package bytes, verify hashes, and run locally.
Retrieval cost is deterministic from the package size and policy schedule.
Would you like to cache verified bytes locally to avoid repeated retrieval payments?

[Run once] [Verify & cache] [Cancel]
```

Apps are signed, content-addressed packages stored on the network. Catalogs are
discovery only. Authority comes from package hash, manifest hash, developer
identity/signature, release id, app policy hash, and local verification/proof
events.

## Node And Admission Model

Do not model "the node" as one global singleton. Model nodes as role instances:

```text
identity + role + policy + budget + route scope + runtime target
```

Examples:

```text
admission:family
admission:business
relay:private-devices
relay:public-paid
storage:home-nas
storage:vps-cache
compute:browser-local
compute:native-gpu
publishing:personal-site
```

All workloads enter the network through an admission node. The browser signs
intent and submits a `WorkRequest`; an admission node returns a signed
`WorkAdmission` that defines whether the work may enter the network, the route
or channel it may use, the admitted budget, the policy hash, and validity.

Default path:

```text
user/browser node
-> EdgeRun DAO admission node
-> assigned relay/channel
-> worker/recipient/storage through relay
```

User-owned path:

```text
user/browser node
-> user-owned admission node
-> user-approved relay/storage/compute nodes
-> worker/recipient/storage through relay
```

Do not bypass admission by sending authoritative user work directly to storage,
compute, relays, apps, services, or workers. Payload layers may prepare bytes,
hashes, sealed objects, manifests, packets, and local verification state, but
network authority comes from the admission chain.

Admission policy can be inherited:

```text
personal admission
-> family admission policy
-> organization admission policy
-> DAO admission policy
```

Each admission node signs its own admission and commits to the policy hash it
enforced. Trust Manager should expose policy sources and proof refs when
available.

## Workspace Map

The workspace is a large Rust 2024 monorepo. `Cargo.toml` is the source of
truth for members. Several excluded crates/devices still exist but are not
workspace members.

Important families:

- `crates/protocol/*`: core protocol surfaces.
  - `edgerun-work`: admitted work, routing, channels, proofs, receipts,
    settlement, storage payloads, capability packets, and std/native runtime
    adapters.
  - `edgerun-wire`: rkyv-only wire boundary and shared ABI records.
  - `edgerun-core`: lower-level protocol types, validation, commands, and
    crypto-facing helpers.
  - `edgerun-protocols`: transport-independent parsers, encoders, and protocol
    state machines for HTTP, TLS, DNS, SMTP, IMAP, WebSocket, TFTP, block, USB,
    Wi-Fi, and other compatibility protocols.
- `crates/utility/*`: EdgeRun-owned compatibility crates and shared utilities.
  Many mirror external crate APIs under local ownership. Avoid casually adding
  new third-party dependencies when a local compatibility crate already exists.
- `crates/utility/edgerun-ui-core`: shared Rust UI kit. This is the effective
  `edgerun-ui` crate in this repo.
- `crates/node/*`: node orchestration, daemon runtime, machine inventory, and
  SDL/native UI integration.
- `crates/authority/*`: storage, VFS, and virtual disk authority/storage
  surfaces.
- `crates/capability/*`: capability and remote capability request/response
  structures.
- `crates/identity/*`: hardware-backed identities such as TPM and YubiKey.
- `crates/linux-adapters/*`: Linux device/network adapters.
- `crates/apps/*`: app/service crates such as email, OAuth, exchange API, and
  secret service.
- `crates/edgerun-codex/*`: Codex-derived agent/client/TUI/web/native UI
  surfaces adapted into the workspace.
- `crates/edgerun-sdk`, `edgerun-unit`, and `edgerun-unit-macros`: deterministic
  app/unit authoring path.
- `crates/edgerun-compositor`, `edgerun-platform`, `edgerun-virtio`,
  `edgerun-oci`, `edgerun-wallet`, `edgerun-exchange`, `edgerun-network-driver`,
  `edgerun-term`, `edgerun-codelyzer`: platform, runtime, settlement, terminal,
  analysis, and infrastructure support.
- `devices/*`: device firmware/bridge crates; currently excluded from the main
  workspace.

When scanning the repo, prefer `rg`, `rg --files`, and `cargo metadata
--no-deps`. Do not assume every directory under `crates/` is a workspace member;
check the root manifest.

## Rust And Dependency Rules

- The workspace uses Rust edition 2024 and rust-version 1.95.
- Default new reusable logic to `no_std + alloc` so it can run in browser WASM,
  firmware, native, tests, and future runtimes.
- Preserve existing `no_std + alloc` boundaries in protocol and utility crates.
- Route durable cross-runtime data through `edgerun-wire`. If bytes cross
  browser/native, worker/main-thread, node/app, transport, storage, or capability
  boundaries, prefer an explicit wire record over local-only structs or ad hoc
  encodings.
- Put sockets, threads, filesystem access, locks, timers, and other OS features
  behind explicit `std` features or in `std_runtime` modules. `std` should be a
  thin adapter layer over portable core logic.
- Do not add external dependencies without explicit approval. Prefer
  workspace-local utility crates and existing compatibility surfaces. Assume
  this repository already has the crate you need; prove otherwise before asking
  to add anything.
- Keep changes scoped. Do not refactor vendored/compatibility crates unless the
  task is explicitly about them.
- Consolidate before extending. If two modules express the same concept, prefer
  one shared implementation and remove the duplicate path when safe.
- Remove code that is proven dead, obsolete, or superseded by a clearer local
  abstraction. Do not keep compatibility shims unless a real caller still needs
  them.
- Before changing protocol hashes, signed preimages, or wire records, find all
  tests and golden fixtures that lock the old behavior and update them
  deliberately.
- Use structured builders/parsers already present in the codebase instead of ad
  hoc string or byte manipulation.

## edgerun-work Focus

`crates/protocol/edgerun-work` is the highest-risk crate. It is not just message
passing; it models verifiable state transitions for admitted work, routes,
channels, capabilities, typed payloads, proofs, receipts, and settlement.

Primary files:

- `protocol.rs`: durable wire/economic objects such as `NodeIdentity`,
  `NetworkMessage`, `WorkRequest`, `WorkAdmission`, `WorkReceipt`, `WorkPacket`,
  and acknowledgements.
- `node_control.rs`: node-control structs split out from `protocol.rs`.
- `identity.rs`: node identity derivation and verification helpers.
- `signing.rs`: signed-object preimages and `sign_*` / `verify_*` helpers.
- `codec.rs`: rkyv encoding/decoding plus packet bytes and packet hashes.
- `preimage.rs`: `PreimageBuilder` and `HashBuilder`.
- `request_auth.rs`: signed user `WorkRequest` verification.
- `route_binding.rs`, `route_builder.rs`, `route_policy.rs`,
  `admitted_route.rs`: route commitments, route construction, route policy, and
  admission-defined routing.
- `channel_order.rs`, `channel.rs`, `memory_channel.rs`, `work_channel.rs`,
  `transport_channel.rs`, `ws_channel.rs`: ordered channels and transport
  adapters.
- `delivery_proof.rs`, `transit_proof.rs`, `relay_role.rs`: recipient delivery
  proofs, transit commitments, relay receipts, and finalized relay delivery.
- `storage_payload.rs`, `typed_storage_role.rs`, `object_storage_service.rs`,
  `storage_adapter.rs`: typed storage payloads and storage role logic.
- `erasure_storage.rs`: XOR 2+1 proof/test erasure model, not final production
  erasure coding.
- `recipient_policy.rs`: recipient-defined content-addressed messaging policy.
- `settlement.rs`, `batch_settlement.rs`: local settlement model, typed relay
  delivery settlement, unchecked non-relay receipt settlement, and atomic batch
  settlement.
- `capability_packet.rs`, `capability_role.rs`, `trust_container_role.rs`:
  capability envelopes, capability execution, and Trust Container capability
  behavior.
- `std_runtime/*`: std-only admission, TCP, WebSocket, threading, storage daemon,
  process, and settlement runtime code.

Important invariants:

1. A node identity is valid only if the claimed node id matches the authority
   key. Current code treats `node_id == public_key`; older notes may mention
   role-derived ids. Follow the code and tests in `identity.rs`.
2. Admission must verify the signed `WorkRequest` before trusting user/request
   fields.
3. Admission commits to user, request hash, admitted route/channel/path, budget,
   policy hash, validity, and admission node.
4. Admission is the policy and route gate for work entering the network.
5. Route signatures and admission-defined route commitments prove reachability
   and state for new work.
6. Ordered channels verify packet hash, route hash, sequence, and previous
   message hash.
7. Relays hash each packet they handle and commit transit work into a hash
   chain.
8. Recipient delivery proof should be policy-bound: the recipient signs
   acceptance under a content-addressed policy hash where the flow requires it.
9. Relay payment should require receiver delivery proof, transit hash, forwarded
   packet hash, and admission/policy binding.
10. Storage proofs must include retrieval or availability evidence before final
    production payment. Store-only receipts are not enough.
11. Generic unchecked receipt settlement must not be used for relay payments.
12. Batch settlement must be atomic: preflight the whole batch before mutating
    ledger state.
13. Do not prune paid receipt/admission tracking until an admission is finalized
    and its challenge window has closed.
14. Core protocol code must remain `no_std + alloc`; std-only functionality
    belongs in `std_runtime`.

Import boundaries:

```rust
use crate::identity::{node_identity_from_key, verify_node_identity};
use crate::signing::{empty_signature, sign_work_receipt, verify_work_receipt};
use crate::codec::{blake3_hash, packet_bytes, packet_hash};
```

Do not reintroduce old compatibility re-exports from `codec.rs` for signing or
identity helpers. If a build fails with imports such as
`crate::codec::{empty_signature, sign_*, verify_*, node_identity_from_key}`, fix
the caller to import from `signing.rs` or `identity.rs`.

For protocol/economic hashes:

- Use `PreimageBuilder` for bytes that are signed.
- Use `HashBuilder` for BLAKE3 commitments and Merkle-like hashes.
- Every protocol/economic hash needs an explicit domain string.
- If a domain changes, update the golden tests intentionally and explain the
  compatibility impact.

Golden hash status:

- `tests/golden_hashes.rs` exists and locks deterministic fixture hashes.
- When changing any signed preimage, packet hash, route hash, receipt id,
  erasure hash, manifest hash, policy hash, or settlement root, run and update
  the golden fixture deliberately.
- Do not add new protocol features while golden hashes are failing.

Required checks for `edgerun-work` changes:

```bash
cargo test --manifest-path crates/protocol/edgerun-work/Cargo.toml
cargo test --manifest-path crates/protocol/edgerun-work/Cargo.toml --no-default-features
cargo build --manifest-path crates/protocol/edgerun-work/Cargo.toml --target wasm32-unknown-unknown --release --no-default-features
```

Run the size script when touching WASM-facing code:

```bash
crates/protocol/edgerun-work/scripts/wasm-size.sh
```

## edgerun-wire Focus

`crates/protocol/edgerun-wire` is the rkyv-only wire protocol boundary.

Rules:

- `WIRE_PROTOCOL` is `"rkyv"`.
- Legacy structural wire APIs were intentionally removed. Do not add a second
  wire format for convenience.
- Keep the crate `no_std` by default.
- Treat `edgerun-wire` as the canonical movement boundary. Browser WASM,
  native hosts, workers, node runtimes, app surfaces, capabilities, storage, and
  transports should exchange explicit wire records rather than private one-off
  structs.
- ABI constants are protocol commitments. Do not renumber statuses, capability
  kinds, runtime event codes, route schemes, app store statuses, or signature
  algorithms casually.
- Wire structs should derive the rkyv traits consistently:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
```

- Prefer append-only evolution for records. If fields must change, update
  version constants/tests and all consumers.
- `edgerun-wire` should not learn app business policy, admission policy,
  settlement policy, UI layout, or transport runtime behavior. It owns stable
  record shapes and ABI constants.

Run for `edgerun-wire` changes:

```bash
cargo test -p edgerun-wire
cargo test -p edgerun-wire --no-default-features
```

Also run affected dependent protocol tests, especially `edgerun-work`, when a
wire record used by work/admission/capability/runtime code changes.

## edgerun-ui-core Focus

There is no workspace member named `edgerun-ui`; use
`crates/utility/edgerun-ui-core` for shared UI work.

Architecture rule: Rust builds `GpuScene` command buffers. Hosts render those
buffers and forward input back into Rust.

Layers:

- EdgeRun Shell owns launcher, trust/status indicators, capability prompts,
  lock flow, notifications, and future app switching.
- Workspace owns tiled app placement. Apps do not float over each other and do
  not own global shell chrome.
- App surfaces render content into assigned workspace bounds using shared UI
  nodes and components.

Host rules:

- JS is a byte bridge only.
- Browser hosts render workspace and shell overlay canvases.
- Native hosts may render a combined scene but should use the same
  `UiShellState` and `UiWorkspace` action flow.
- Hosts must not duplicate app metadata, layout, launching, trust semantics, or
  capability semantics.

Core modules:

- `src/lib.rs`: no-std software UI primitives and public modules.
- `src/gpu.rs`: high-level GPU UI module entry point.
- `src/gpu/scene.rs`: scene buffer types, clipping, fallback text emission, and
  color-scheme remapping.
- `src/gpu/node.rs`: JSX-like immediate-mode UI node tree and rendering
  dispatch.
- `src/gpu/painter.rs`, `src/gpu/paint.rs`: drawing facade and shared paint
  helpers.
- `src/gpu/apps.rs`: scene-build entry points and built-in app surfaces.
- `src/gpu/app_registry.rs`: canonical app ids, launcher ids, metadata, icons,
  and app placement.
- `src/gpu/workspace.rs`: tiled workspace model, app surfaces, tabs, focus, and
  app event routing.
- `src/gpu/shell.rs`: shell state, launcher actions, and overlay drawing.
- `src/gpu/runtime.rs`: hit targets, input events, UI actions, focus, scroll,
  text input, and runtime controls.
- `src/gpu/components.rs`: reusable dashboard/form/domain primitives.
- `src/gpu/icons.rs`, `src/tabler*.rs`: icon mapping and generated icon data.
- `src/initial_setup.rs`: initial setup/onboarding surface.

UI rules:

- Use `EDGERUN_APP_REGISTRY` for app metadata, launcher rows, default workspace
  construction, and app opening. Do not duplicate app ids/titles/icons in hosts.
- App surfaces should receive projected state from real stores where available;
  label demo/preview data honestly.
- Use shared components for identity cards, contact cards, proof/audit rows,
  capability grant rows, route/relay visuals, package/app cards, and
  receipt/payment rows.
- Keep controls stable in size. Text must not overlap or resize fixed controls
  unpredictably.
- Prefer icons for tool actions and concise labels for commands.
- Do not put global shell behavior inside an app surface.
- Do not implement layout in browser JS or SDL host glue when it belongs in
  Rust scene construction.

Feature notes:

- Default feature is `tabler-icons`.
- `std` enables the `gpu` module.
- `gpu-gl` enables native OpenGL support.
- `sdl` depends on `gpu-gl` and `fontdue-text`.
- `fontdue-text` enables font rasterization.
- `tabler-svg-atlas` enables SVG atlas generated assets.

Run for UI changes:

```bash
cargo test -p edgerun-ui-core
cargo check -p edgerun-ui-core --features std
cargo check -p edgerun-ui-core --features fontdue-text
```

For GPU/SDL/OpenGL changes, also build the relevant preview binary, for example:

```bash
cargo build -p edgerun-ui-core --features fontdue-text,tabler-svg-atlas --bin ui-preview-sdl-gl-svg
cargo build -p edgerun-ui-core --features std --bin ui-preview-sdl-shadcn
```

## Protocol Object Review Checklist

For every signed, economic, proof, or wire object, answer these before merging:

1. What does it claim?
2. Who signs it?
3. Which fields are covered by the signature or hash?
4. What hash identifies it?
5. What canonical verifier exists?
6. What prior object does it depend on?
7. What later object consumes it?
8. What makes it payable?
9. What makes it slashable or challengeable?
10. What test proves bad data is rejected?

## Testing Expectations

Never report that something works unless you have run a relevant check or can
point to concrete evidence in the code. If you did not test it, say that
plainly.

For general Rust changes, prefer the narrowest package test first:

```bash
cargo test -p <package>
cargo check -p <package>
```

For workspace-wide manifest or shared dependency changes, expect broader checks:

```bash
cargo check --workspace
cargo test --workspace
```

For protocol/no-std/WASM-facing changes, include no-default-features and WASM
checks for the affected crate. For `edgerun-work`, use the exact commands in
the `edgerun-work` section.

For behavior changes, add or update tests that prove the important rejection and
success paths. Do not rely only on broad "still compiles" evidence when the
change affects protocol validity, payments, security, storage, routing, UI
actions, or user-visible state.

For generated files, document the generator command. Do not hand-edit generated
icon or atlas outputs unless the task is specifically to repair generated output
and the generator cannot run.

## Current Priority Queue

Good next tasks:

1. Keep `edgerun-work/tests/golden_hashes.rs` green and update it deliberately
   when protocol hash domains or preimages change.
2. Add or refine first-run network app prompt: Run once / Verify & cache /
   Cancel.
3. Rename internal install terminology toward cache/run terminology where it
   does not create churn.
4. Wire Trust Manager actions: open Identity, open App Store, revoke grants,
   remove cache.
5. Surface app policy hash and access mode in App Store cards/details.
6. Add developer CLI publish/status/deposit flow around SDK artifacts.
7. Show runtime events and package proofs consistently across App Store, Trust
   Manager, and Identity.
8. Connect browser node earning mode to profile preferences and onboarding.
9. Add typed storage retrieval/availability settlement evidence.
10. Add user publishing UX for site/app/API/file from identity-routed node
    policy.
11. Add data-source sync UX for Gmail/Drive/GitHub/local imports into EdgeRun
    storage and personal timeline.
12. Add user-owned admission UX showing routes, relays, workers, budgets, and
    policy hash.
13. Add node-instance UX for admission/relay/storage/compute instances with
    role, runtime target, policy hash, budget, and owner.

Avoid:

- adding marketplace-specific protocol types before necessary;
- creating duplicate app/package models;
- treating catalogs as authority;
- hiding app policies or payment terms;
- weakening proof paths for performance without preserving packet/content
  hashes;
- presenting browser nodes as guaranteed high-availability infrastructure;
- using unchecked receipt settlement for relay receipts;
- moving std-only functionality into core protocol modules;
- silently changing protocol hashes without updating golden tests;
- presenting users as only consumers;
- bypassing admission by sending authoritative user work directly to worker
  nodes;
- treating admission/relay as fixed backend services instead of user-runnable
  role instances.
