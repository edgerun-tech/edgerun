# EdgeRun product and agent guidance

This repository is building EdgeRun: user-owned internet infrastructure for identity, apps, storage, routing, compute, and payments.

EdgeRun is not a token-first crypto project and not a generic cloud clone. It is usage-first infrastructure where users and developers can run, publish, host, sell, cache, relay, store, compute, and settle through verifiable work. Payments follow proof of useful work.

## Product thesis

EdgeRun gives users freedom and accountability at the same time.

Freedom:

- Users own their identity, data, local app cache, contacts, keys, and proof history.
- Developers can publish and sell apps without Apple, Google, Meta, Stripe, Cloudflare, or AWS as required gatekeepers.
- Anyone can run useful nodes and earn from bandwidth, storage, relay, compute, hosting, and app distribution.
- Apps and sites are content-addressed and can be served by the network.

Accountability:

- Identities, packages, policies, routes, requests, admissions, proofs, and receipts are signed or hash-addressed.
- Nodes get paid only for verifiable work.
- App access, caching, payments, and capabilities are governed by explicit content-addressed policies.
- Users see what they are signing and can inspect proof trails.

The public framing should be:

> Own your identity and data. Run apps from verified network storage. Cache locally when you want. Developers sell directly. Infrastructure gets paid for useful work.

Avoid presenting the product as merely “decentralized cloud” or “DRM”. Internally, policy can enforce app licensing and accountability, but externally the better language is verifiable licensing, publisher-defined access policy, user-owned execution, and proof-backed payments.

## User model

Every user starts with a browser node.

The browser node should be treated as a first-class local node that can:

- hold a sealed Trust Container;
- unlock/sign through password or passkey;
- verify signed app packages and hashes;
- run apps from network storage;
- optionally cache verified packages locally;
- request and grant capabilities;
- record local audit/proof events;
- eventually earn from useful work when resource sharing is enabled.

The user flow should feel simple:

```text
Create/unlock identity
→ browser node starts
→ run app from network storage
→ optional local cache
→ inspect proof in Trust Manager
→ earn/spend through usage
```

On first run of a network app, users should see a clear prompt:

```text
This app runs from EdgeRun network storage.
Your browser node will retrieve signed package bytes, verify hashes, and run locally.
Retrieval is usually very cheap and may pay storage/CDN nodes.
Would you like to cache verified bytes locally to avoid repeated retrieval payments?

[Run once] [Verify & cache] [Cancel]
```

Use “run” as the primary user action. “Install” is the wrong mental model. Local state is a verified cache, not ownership of a copied app from a centralized store.

## Developer model

Developers publish from the CLI.

The intended developer flow is:

```text
Register developer identity
→ pay deposit/status/hosting budget
→ package app/site with the SDK
→ sign release
→ publish package objects to network storage
→ publish catalog entry/policy
→ users run by hash or cache locally
→ developer and infra providers earn from usage
```

The SDK is the developer-facing business adapter. It should handle or expose:

- developer identity;
- app/site packaging;
- release signing;
- package/content hashing;
- app policy references;
- payments/admission integration;
- publish/deploy helpers;
- verification helpers for websites using EdgeRun directly.

The current SDK browser-authoring path already produces artifacts such as:

- `app.edapp` — app manifest;
- `app.eapp` — app graph/package;
- `developer.esig` — developer signature.

Future work should keep these artifacts aligned with the frontend App Store and network storage model.

## App distribution model

Apps are not primarily installed. Apps are signed, content-addressed packages stored on the network.

Catalogs are discovery only. The catalog must not be treated as the authority. The authority is:

- package hash;
- manifest hash;
- developer identity/signature;
- release id;
- app policy hash;
- local verification/proof event.

The frontend `AppDefinition` should be treated as a UI projection, not source of truth. Source of truth is the signed SDK package and its content-addressed artifacts.

Users can choose:

- run from network storage each time;
- verify and cache locally;
- run cached copy after hash verification;
- pay per run/retrieval/cache/license according to the app policy.

App access policy modes currently used in the frontend are:

```text
free-run
paid-run
paid-cache
license-required
```

Do not add new protocol types just to represent app-store semantics. Prefer content-addressed policies referenced by `policy_hash`, combined with existing work/request/admission/proof/receipt primitives.

## Trust Container and Trust Manager

The Trust Container is the user’s local sealed identity/data root.

It contains, or should be understood as containing:

- owner identity;
- owner encryption identity;
- browser node identity;
- contacts;
- profile events;
- app secrets/OAuth tokens;
- sealed messages/data;
- local preferences;
- local proof/audit context.

Identity app should be the friendly view: profile, browser node, passkey status, public contact card, and local sealed state.

Trust Manager should be the proof dashboard: identity, browser node, cached packages, capability grants, routes, profile events, runtime events, authority refs, and proof refs. Prefer deriving Trust Manager rows from real stores instead of hardcoded mock data.

Important frontend stores/surfaces:

- `useAuth` / `authStore`: real Trust Container, identity, passkey, browser node, contacts, app secrets, profile event log.
- `browserAppInstallStore`: currently named as install state, but semantically this is verified package cache state.
- `runtimeEventLogStore`: runtime audit/proof events such as package verification.
- `localCapabilityGrantsStore`: local app capability grants.
- `TrustManagerSurface`: should show real projected state.
- `AppStore`: should present run/cache network apps, package proofs, developer identity, policy, and local cache state.

## Node responsibilities

Users should understand the node types.

Browser node:

- default node every user gets;
- signs local actions;
- verifies app packages;
- runs network apps;
- caches verified package bytes;
- records runtime/proof events;
- can eventually earn from browser-appropriate work.

Storage/CDN node:

- stores content-addressed app/site/package/model objects;
- serves retrievals and ranges;
- earns from retrieval/storage receipts.

Relay node:

- moves ordered encrypted messages/work packets;
- should not need to understand payload content;
- earns only from admitted, ordered, proof-backed delivery.

Compute node:

- runs deterministic work;
- should eventually prove input/program/output relation;
- useful for AI agents and paid jobs.

Admission node:

- checks signed user request, balance/funding, policy, route plan, budget, and validity window;
- admits work into the network.

Settlement rail:

- pays proof-backed receipts;
- may use smart contracts/cross-chain settlement;
- should keep on-chain logic minimal and hash/proof-oriented.

## Existing protocol primitives

Do not add new protocol objects unless they create a new cryptographic or economic guarantee.

Prefer reusing:

```text
WorkRequest        = signed user intent
WorkAdmission      = funded/policy-approved authorization
RouteAdvertisement = reachability/capability announcement
NetworkMessage     = signed payload transfer
ChannelEnvelope    = ordered transport context
ChannelProof       = recipient acceptance/proof
WorkReceipt        = payable work claim
policy_hash        = content-addressed policy commitment
SettlementLedger   = local settlement model
```

For app sales, website hosting, package delivery, CDN retrieval, user-to-user payments, and passkey payments, use existing primitives where possible:

```text
User action
→ WorkRequest
→ policy_hash
→ WorkAdmission
→ execution/delivery/retrieval
→ ChannelProof or typed proof
→ WorkReceipt
→ settlement
```

## Important edgerun-work invariants

Keep these invariants intact:

1. A node identity is valid only if `node_id == derive_node_id(public_key, role)`.
2. Admission must verify the signed `WorkRequest` before trusting user/request fields.
3. Admission commits to user, request hash, budget, route/channel, policy hash, validity, and admission node.
4. Route signatures prove reachability/state; new work should use available routes.
5. Ordered channels verify packet hash, route hash, sequence, and previous message hash.
6. Relays hash each packet they handle and commit transit work into a hash chain.
7. Recipient delivery proof should be policy-bound: the recipient signs acceptance of an ordered message under a specific content-addressed policy hash.
8. Relay payment should require receiver delivery proof, transit hash, forwarded packet hash, and admission/policy binding.
9. Storage proofs must eventually include retrieval or availability proof; store-only receipts are not enough for final production payment.
10. Generic unchecked receipt settlement must not be used for relay payments.
11. Batch settlement must be atomic: preflight the whole batch before mutating ledger state.
12. Do not prune paid receipt/admission tracking until an admission is finalized and its challenge window has closed.
13. Core protocol code should remain `no_std + alloc`. Sockets, threads, filesystem, and locks belong in `std_runtime`.

## `edgerun-work` module map

Important modules in `crates/protocol/edgerun-work`:

- `protocol.rs`: durable wire/economic objects such as `NodeIdentity`, `NetworkMessage`, `WorkRequest`, `WorkAdmission`, `WorkReceipt`, `WorkPacket`, and acknowledgements.
- `identity.rs`: `derive_node_id`, `verify_node_identity`, `node_identity_from_key`.
- `signing.rs`: signed-object preimages and `sign_*` / `verify_*` helpers.
- `codec.rs`: rkyv packet encoding/decoding and packet hash helpers. Do not put signing or identity helpers back in `codec.rs`.
- `preimage.rs`: `PreimageBuilder` for signed preimages and `HashBuilder` for domain-separated BLAKE3 commitments.
- `request_auth.rs`: signed user `WorkRequest` verification.
- `route_auth.rs` / `route_plan.rs`: signed route advertisements, snapshots, roots, and route selection.
- `channel_order.rs`: ordered channel state and ordered message hashes.
- `recipient_policy.rs`: recipient-defined content-addressed messaging policy.
- `delivery_proof.rs` / `transit_proof.rs`: recipient delivery proofs, policy-bound proofs, and relay transit commitments.
- `relay_role.rs`: relay forwarding, transit receipts, and finalized relay delivery receipts.
- `storage_payload.rs` / `typed_storage_role.rs`: typed object store/retrieve payloads and typed storage role.
- `erasure_storage.rs`: XOR 2+1 proof erasure model. This is a proof/test scheme, not final production erasure coding.
- `settlement.rs` / `batch_settlement.rs`: local settlement model, typed relay delivery settlement, unchecked non-relay receipt settlement, and batch settlement.
- `std_runtime/*`: std-only runtime code such as TCP, threads, admission runtime, client runtime, and synchronized wrappers.

## Import boundaries

Do not import signing or identity helpers from `codec.rs`.

Use:

```rust
use crate::identity::{derive_node_id, node_identity_from_key, verify_node_identity};
use crate::signing::{empty_signature, sign_work_receipt, verify_work_receipt};
use crate::codec::{blake3_hash, packet_bytes, packet_hash};
```

Do not reintroduce compatibility re-exports from `codec.rs`.

For new hash/preimage code:

- Use `PreimageBuilder` for bytes that are signed.
- Use `HashBuilder` for BLAKE3 commitments and Merkle-like hashes.
- Every protocol/economic hash must have an explicit domain string.

## Frontend terminology

Prefer:

- “Run” instead of “Install”.
- “Verify & cache” instead of “Install”.
- “Cached” instead of “Installed”.
- “Remove cache” instead of “Uninstall”.
- “Network app” instead of “catalog app” in user-facing copy.
- “Trust Container” for the sealed local profile/root.
- “Proof dashboard” for Trust Manager.
- “Publisher policy” or “app policy” instead of “DRM” in public copy.

Internal names may still use `install`/`installed` temporarily for compatibility, but new UX and refactors should move toward cache/run terminology.

## Economic model

EdgeRun should earn from usage, not from selling tokens as the core product.

Revenue sources can include:

- app sales cut;
- hosting/status deposits;
- storage/CDN delivery fees;
- relay fees;
- compute job fees;
- marketplace/publisher tooling;
- website hosting and domain/DNS/ACME automation;
- AI agent business activity and payments.

The token/settlement rail exists to clear value between users, publishers, and infrastructure providers. The product should prove utility first: users leave a browser node running, it does useful work, earns, and the user can spend earnings inside the ecosystem.

## UX principles

- Make complex infrastructure feel like normal web actions.
- Show clear signing/payment/policy summaries before asking for passkey approval.
- Always expose hashes/proofs for advanced users.
- Do not fake metrics. If unknown, say unknown. If preview, label preview. If measured, link evidence.
- Keep platform apps coherent: Help teaches, Identity owns, App Store runs/caches, Trust Manager proves, Storage shows content, Finances shows payments/receipts.

## Test expectations

Before and after changes to `edgerun-work`, run:

```bash
cargo test --manifest-path crates/protocol/edgerun-work/Cargo.toml
```

Also run no-std and wasm checks when touching core protocol, encoding, signing, route, settlement, storage, or WASM code:

```bash
cargo test --manifest-path crates/protocol/edgerun-work/Cargo.toml --no-default-features
cargo build --manifest-path crates/protocol/edgerun-work/Cargo.toml --target wasm32-unknown-unknown --release --no-default-features
```

Run the size script when touching WASM-facing code:

```bash
crates/protocol/edgerun-work/scripts/wasm-size.sh
```

For frontend changes, run the frontend type/lint commands used by the project, for example:

```bash
cd frontend
bun run typecheck
bun run lint
```

## What to work on next

Good next tasks:

1. Add first-run app prompt: Run once / Verify & cache / Cancel.
2. Rename internal install terminology to cache terminology.
3. Wire Trust Manager actions: open Identity, open App Store, revoke grants, remove cache.
4. Surface app policy hash and access mode in App Store cards/details.
5. Add developer CLI publish/status/deposit flow around SDK artifacts.
6. Show runtime events and package proofs consistently across App Store, Trust Manager, and Identity.
7. Connect browser node earning mode to profile preferences and onboarding.
8. Add typed storage retrieval/availability settlement evidence.

Avoid:

- adding marketplace-specific protocol types before necessary;
- creating duplicate app/package models;
- treating catalogs as authority;
- hiding app policies or payment terms;
- weakening proof paths for performance without preserving packet/content hashes;
- presenting browser nodes as guaranteed high-availability infrastructure;
- using unchecked receipt settlement for relay receipts;
- moving std-only functionality into core protocol modules;
- silently changing protocol hashes without updating golden hash tests.

## Review checklist for new protocol/economic objects

For every signed/economic/proof object, answer these before merging:

1. What does it claim?
2. Who signs it?
3. Which fields are covered by the signature?
4. What hash identifies it?
5. What canonical verifier exists?
6. What prior object does it depend on?
7. What later object consumes it?
8. What makes it payable?
9. What makes it slashable or challengeable?
10. What test proves bad data is rejected?
