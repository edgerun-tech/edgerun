# EdgeRun product and agent guidance

This repository is building EdgeRun: user-owned internet infrastructure for identity, apps, storage, routing, compute, publishing, and payments.

EdgeRun is not a token-first crypto project and not a generic cloud clone. It is usage-first infrastructure where users and developers can run, publish, host, sell, cache, relay, store, compute, and settle through verifiable work. Payments follow proof of useful work.

## Product thesis

EdgeRun gives users freedom and accountability at the same time.

Freedom:

- Users own their identity, data, local app cache, contacts, keys, and proof history.
- Users are not only consumers. With identity routing and built-in internet protocols, every user can become a publisher, host, app seller, website owner, data source, or service provider.
- Developers can publish and sell apps without Apple, Google, Meta, Stripe, Cloudflare, or AWS as required gatekeepers.
- Anyone can run useful nodes and earn from bandwidth, storage, relay, compute, hosting, and app distribution.
- Apps and sites are content-addressed and can be served by the network.

Accountability:

- Identities, packages, policies, routes, requests, admissions, proofs, and receipts are signed or hash-addressed.
- Nodes get paid only for verifiable work.
- App access, caching, payments, and capabilities are governed by explicit content-addressed policies.
- Users see what they are signing and can inspect proof trails.

The public framing should be:

> Own your identity and data. Run apps from verified network storage. Cache locally when you want. Publish from your own node. Developers sell directly. Infrastructure gets paid for useful work.

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
- eventually earn from useful work when resource sharing is enabled;
- publish identity-routed services when the user enables them.

The user flow should feel simple:

```text
Create/unlock identity
→ browser node starts
→ define node policy
→ connect contacts and old data sources
→ run app from network storage
→ optional local cache
→ publish or sync what the user chooses
→ inspect proof in Trust Manager
→ earn/spend through usage
```

On first run of a network app, users should see a clear prompt:

```text
This app runs from EdgeRun network storage.
Your browser node will retrieve signed package bytes, verify hashes, and run locally.
Retrieval cost is deterministic from the package size and policy schedule.
Would you like to cache verified bytes locally to avoid repeated retrieval payments?

[Run once] [Verify & cache] [Cancel]
```

Use “run” as the primary user action. “Install” is the wrong mental model. Local state is a verified cache, not ownership of a copied app from a centralized store.

## Users as publishers

A user-owned node can be more than a browser runtime. EdgeRun nodes already aim to include identity routing and common internet protocol capability such as HTTP, TLS, ACME, SSH, iPXE, TFTP, and related service/provisioning protocols.

That means a user can become their own publisher:

```text
user identity
→ node policy
→ signed route / endpoint advertisement
→ content-addressed site/app/file/API/boot image
→ HTTP/TLS/ACME or other protocol exposure
→ proof/audit trail
→ optional payment/settlement
```

Examples:

- publish a personal website;
- host an app or app catalog entry;
- expose a private API to approved contacts/agents;
- serve files or package objects from EdgeRun storage;
- publish boot/provisioning artifacts such as iPXE/TFTP flows;
- expose an agent service under explicit policy;
- let contacts message or call directly by identity.

Do not design UX where users are only consumers of apps. The product direction is that users can start as consumers, then become publishers/providers by enabling node policy and route/service capabilities.

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

## Data import and personal timeline model

Third-party connections are migration sources, not permanent homes.

Gmail, other mailboxes, Google Drive, GitHub, photos, contacts, calendars, local folders, browser cache, and other sources should be presented as import/sync connectors into user-owned EdgeRun storage and the Trust Container.

The intended direction:

```text
connect old services
→ import/sync selected data into EdgeRun storage
→ build private indexes/timelines/graphs
→ grant specialized agents scoped access
→ slowly stop depending on the old services
```

This enables app-driven visualization of a person’s life. Apps and agents can visualize timelines, relationships, finances, health records, documents, projects, messages, calls, and media, but only through explicit capability grants.

Specialized AI agents should be scoped by assignment and policy, for example:

- finance agent: invoices, receipts, accounts, tax documents;
- health agent: health documents, wearable data, food logs, habits;
- memory agent: timeline, notes, messages, photos, contacts;
- code agent: repos, issues, docs, release history;
- travel agent: flights, calendars, locations, documents;
- publishing agent: sites, apps, DNS/ACME, release metadata.

Each agent must have a clear capability scope and should appear in Trust Manager.

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
- Storage/data-source apps: should present external services as import/sync sources into EdgeRun storage.

## Node responsibilities

Users should understand the node types.

Browser node:

- default node every user gets;
- signs local actions;
- verifies app packages;
- runs network apps;
- caches verified package bytes;
- records runtime/proof events;
- can eventually earn from browser-appropriate work;
- can become a publisher/service endpoint when enabled by policy.

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

Publishing/service node:

- exposes user-approved services over identity-routed endpoints and standard protocols;
- can serve websites, APIs, package objects, boot artifacts, or agent endpoints;
- must obey user policy and emit auditable events.

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

For app sales, website hosting, package delivery, CDN retrieval, user-to-user payments, passkey payments, and user publishing, use existing primitives where possible:

```text
User action
→ WorkRequest
→ policy_hash
→ WorkAdmission
→ execution/delivery/retrieval/publication
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
- “Import/sync source” instead of treating Google/GitHub/mail providers as long-term homes.
- “Publish from your node” or “identity-routed publishing” for user-hosted services.

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
- AI agent business activity and payments;
- user-published services and direct identity-routed commerce.

The token/settlement rail exists to clear value between users, publishers, and infrastructure providers. The product should prove utility first: users leave a browser node running, it does useful work, earns, and the user can spend earnings inside the ecosystem.

## UX principles

- Make complex infrastructure feel like normal web actions.
- Show clear signing/payment/policy summaries before asking for passkey approval.
- Always expose hashes/proofs for advanced users.
- Do not fake metrics. If unknown, say unknown. If preview, label preview. If measured, link evidence.
- Keep platform apps coherent: Help teaches, Identity owns, App Store runs/caches, Trust Manager proves, Storage shows content, Finances shows payments/receipts.
- Make old services feel like import bridges into EdgeRun, not final destinations.
- Make publishing feel like a natural next step for any user node, not only professional developers.

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
9. Add user publishing UX: publish site/app/API/file from identity-routed node policy.
10. Add data-source sync UX: Gmail/Drive/GitHub/local imports into EdgeRun storage and personal timeline.

Avoid:

- adding marketplace-specific protocol types before necessary;
- creating duplicate app/package models;
- treating catalogs as authority;
- hiding app policies or payment terms;
- weakening proof paths for performance without preserving packet/content hashes;
- presenting browser nodes as guaranteed high-availability infrastructure;
- using unchecked receipt settlement for relay receipts;
- moving std-only functionality into core protocol modules;
- silently changing protocol hashes without updating golden hash tests;
- presenting users as only consumers when the architecture makes them publishers.

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

## Latest edgerun-work handoff

The broad hash/preimage consolidation pass has been completed across the main protocol modules. These files now use `PreimageBuilder` or `HashBuilder` where appropriate:

```text
signing.rs
request_auth.rs
route_auth.rs
delivery_proof.rs
transit_proof.rs
batch_settlement.rs
route_plan.rs
cost_model.rs
storage_payload.rs
erasure_storage.rs
channel_order.rs
recipient_policy.rs
settlement.rs
std_runtime/admission_v2.rs
std_runtime/relay_client.rs
```

The `codec.rs` compatibility re-exports were intentionally removed. If a build fails with unresolved imports from `crate::codec::{empty_signature, sign_*, verify_*, node_identity_from_key}`, fix the caller to import from `signing.rs` or `identity.rs`; do not add those re-exports back to `codec.rs`.

Current known post-consolidation status:

- Tests were reported green after fixing the `codec`/`signing`/`identity` split.
- `std_runtime/admission_v2.rs` and `std_runtime/relay_client.rs` were patched to use explicit imports and `HashBuilder` domains.
- `protocol.rs` has been split so node-control structs live in `node_control.rs`, while `protocol.rs` remains focused on core work/economic wire objects.
- `node_control.rs` is exported from `lib.rs`.
- Many protocol hash domains changed by design during consolidation. Tests that recompute through canonical helpers should pass; tests with hardcoded old hashes must be updated deliberately.

Immediate next task: add `tests/golden_hashes.rs` using deterministic fixtures from the current green state. First add an ignored printer test that emits candidate constants, then fill those constants into non-ignored assertions. Do not add new protocol features before golden hashes are locked.
