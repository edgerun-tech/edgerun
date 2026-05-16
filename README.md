# EdgeRun

EdgeRun is user-owned internet infrastructure for identity, apps, storage,
routing, compute, publishing, and proof-backed payments.

## Public Snapshot

This repository is being made public as a preservation and collaboration
snapshot. It is the result of a personal attempt to build infrastructure that
people can own and run for themselves: identity, apps, storage, routing,
compute, publishing, and proof-backed payments under user policy.

Development has been constrained by available Codex credits, and I cannot
currently afford to buy more. Rather than leave the work private, this snapshot
is public so the design, code, tests, and unfinished paths can be inspected,
used, continued, or challenged.

Contact: `kensservices@gmail.com`.

This is not a production release. It contains working protocol pieces,
experimental app/runtime surfaces, local deployment templates, firmware notes,
and research code. Review the code and run the relevant checks before depending
on any part of it.

## Why This Matters

Most internet infrastructure asks users to trade ownership for convenience:
identity is delegated, apps are rented, storage is opaque, publishing depends on
platform permission, and payment is disconnected from verifiable work. EdgeRun
is an attempt to reverse that default. The user should be able to hold identity,
run and cache verified apps, define admission policy, publish from their own
node, pay only for useful work, and inspect the evidence behind every claim.

The significance of this repository is not that every subsystem is finished.
The significant part is the shape of the system: admission before authority,
hash-bound work, user-owned policy, portable wire records, proof-backed
settlement, and UI surfaces that treat the browser as a real node instead of a
thin client. Even incomplete, the repo records a concrete path toward internet
infrastructure that can be run by individuals, families, communities, and small
operators without making them passive tenants of someone else's platform.

The core idea is simple: users submit signed work requests, admission nodes
decide whether that work is allowed, workers perform useful work, and settlement
pays only when the proof chain matches the admitted work.

EdgeRun is not a token-first crypto product and not a generic cloud clone. It is
usage-first infrastructure where users and developers can run, publish, host,
sell, cache, relay, store, compute, verify, and settle through verifiable work.

## What The System Does

EdgeRun turns infrastructure work into signed, checkable records.

Notable repository areas:

- admitted work, route, proof, receipt, verifier, notary, and settlement logic
  in `crates/protocol/edgerun-work`;
- rkyv-only browser/native/app/node wire records in
  `crates/protocol/edgerun-wire`;
- a shared Rust UI scene system in `crates/utility/edgerun-ui-core`;
- storage, VFS, virtual disk, capability, OAuth, email, secret service, OCI,
  terminal, compositor, wallet, exchange, and node runtime experiments;
- local compatibility crates under `crates/utility/*` to keep the project less
  dependent on third-party package availability;
- device and firmware bring-up notes under `devices/` and `firmware/`;
- public example deployment templates under `deploy/server/`.

### `edgerun-work`

`crates/protocol/edgerun-work` is the core protocol/economic crate. It models
signed user work requests, admission decisions, route commitments, ordered
channels, relay transit evidence, delivery proofs, storage payloads, capability
packets, worker claims, receipts, verifier/notary reports, custody evidence,
and settlement checks.

Its central rule is that work should not become authoritative or payable just
because a process performed it. Work has to be admitted, budgeted, hash-bound,
signed, routed through the admitted path, and backed by the role-specific proof
that settlement expects. This crate is where the project tries to make "pay for
useful work" something checkable rather than rhetorical.

### `edgerun-ui-core`

`crates/utility/edgerun-ui-core` is the shared Rust UI scene system. Rust builds
`GpuScene` command buffers for shell, workspace, app surfaces, capability
prompts, proof/audit rows, identity state, package/app cards, and setup flows;
browser or native hosts render those buffers and send normalized input back.

The point is to keep app metadata, trust state, capability grants, proof
surfaces, and layout semantics in one portable UI core instead of duplicating
them across JavaScript, SDL, WebGL, and native host glue. It is the beginning of
an EdgeRun shell where users can run network apps, verify and cache packages,
inspect proofs, revoke grants, and manage node roles from the same model.

Examples of work:

- retrieve a signed app package from network storage;
- verify and cache package bytes locally;
- relay admitted packets between nodes;
- store or retrieve content-addressed data;
- seal, unseal, or notarize data movement;
- run deterministic compute;
- verify another worker's claim;
- publish an identity-routed service from a user-owned node.

The default flow is:

```text
user/browser node
-> signed WorkRequest
-> admission node checks policy, route, budget, and validity
-> signed WorkAdmission
-> relay/channel moves packets
-> worker performs role-specific work
-> worker signs a receipt or role claim
-> verifier and settlement check the proof trail
-> payment or rejection
```

Authority comes from signed requests, signed admissions, packet/content hashes,
route commitments, worker claims, verifier reports, and settlement rules. Relays
move bytes; they do not create authority.

## Why Use It As A User

Use EdgeRun when you want infrastructure that is portable and inspectable rather
than locked to one provider.

As a user, you can:

- own your identity and sealed local Trust Container;
- run apps from verified network storage;
- cache verified bytes locally instead of repeatedly paying retrieval cost;
- choose EdgeRun DAO admission or your own admission node;
- define admission policy, route scope, and budgets;
- publish from your own node;
- submit work to storage, relay, compute, verifier, and notary nodes;
- inspect proof and audit events in a proof dashboard;
- pay for useful work instead of opaque service bundles.

The browser node is a first-class node instance. It can sign intent, verify app
packages, cache data, request capabilities, record local proof events, and
eventually earn from browser-appropriate work when policy allows.

## Why Join As A Worker

Join as a worker when you have useful resources to sell:

- bandwidth as a relay node;
- disk as a storage node;
- CPU or GPU as a compute node;
- availability as a host or publisher;
- verification capacity as a verifier node;
- sealing or witness authority as a notary node.

Workers do not need to be giant cloud providers. A worker can run one role:

```text
relay:public-paid
storage:home-nas
compute:native-gpu
verifier:claims
publishing:personal-site
```

The worker's protection is that payable work is admitted, budgeted, hash-bound,
and signed. A worker should only do work covered by a signed admission, then
produce role-specific evidence. If another participant refuses to handle the
proof path correctly, the worker can apply backpressure or stop accepting more
work for that admission/session.

## Why Trust Any Of It

The trust model is:

```text
do not trust actors; verify signed records, hashes, policies, and settlement rules
```

EdgeRun does not assume every relay, storage node, verifier, or user is honest.
It assumes participants may be self-serving, colluding, faulty, or adversarial,
then makes payment depend on evidence.

Current guarantees in the core work model:

- work must be admitted before it becomes payable;
- admissions bind request hash, user, policy hash, route/channel, budget,
  validity, and admission node;
- worker claims are signed by the worker and bound to a specific admission;
- receipts must match the worker claim, input, output, units, and sequence range;
- duplicate claims and overlapping ranges for the same worker/admission/work
  kind are rejected;
- claims outside admission validity or budget are rejected;
- multi-relay transit bundles can prove a packet followed an admitted relay path
  using bounded hop evidence and per-hop transit hashes;
- relay transit bundles can be constructed with a bounded protocol builder that
  rejects wrong-order hops, wrong packets, incomplete paths, oversized paths, and
  missing final delivery proofs;
- live relay forwarding results carry builder-ready hop evidence, so relay
  runtime code can append actual forwarded packets into the same proof bundle
  settlement verifies;
- relay proof custody acknowledgements let storage, notary, or verifier nodes
  sign that a bundle root has been packed under a custody root for a bounded
  bundle hop window and expiry;
- relay custody acknowledgements can be constructed from the verified bundle
  itself, avoiding duplicated runtime field assembly;
- settlement can verify relay custody handoff evidence without treating custody
  as payment authority, and the handoff must match the expected packed proof
  root and required custody kind;
- custody policy is passed as one requirement object so single-ack and chain
  verification use the same expected root and required level;
- custody requirements reject empty expected roots and unknown custody kinds
  before accepting custody evidence;
- custody requirements have stable hashes for audit, policy, and challenge
  references;
- custody requirements are wire-encoded so runtimes, settlement, and proof
  dashboards can exchange the same policy object;
- std runtime settlement exposes the same custody verification entry points as
  the portable settlement core, the admitted multi-relay settlement path, and a
  custody-gated relay settlement path;
- custody-gated relay preflight and settlement return compact audit references
  for both the final custody acknowledgement and the custody requirement that
  was enforced;
- custody-gated settlement results are wire-encoded so runtime services and
  proof dashboards can exchange the same audit object;
- settlement result, custody check, and custody-gated settlement result hashes
  give audit/challenge records stable references to what was paid and why;
- custody kind is role-bound, so storage, notary, and verifier nodes cannot
  impersonate each other's custody step;
- custody chains can prove ordered, contiguous progression from packed to notarized to
  stored custody over the same packed proof root;
- multi-relay relay-hop settlement requires the receipt to match a verified
  admitted relay transit bundle, the admission route commitment, and a real
  recipient delivery proof;
- wrong-role verifier and notary signatures are rejected;
- a colluding verifier report does not override settlement's own checks;
- public proof envelopes can carry hashes without embedding private payload
  bytes.

So the promise is not "trust this server." The promise is "check the chain of
facts before accepting authority or paying for work."

## Boundaries And Admission

Every authority boundary has its own admission node and relay node. A browser
boundary has browser admission. A host boundary has host admission. Multiple
boundaries can live in one browser tab, and one authority can span multiple
machines.

All authoritative inbound and outbound work crosses relay nodes. Relay nodes are
controlled by admission nodes:

- one relay has exactly one controlling admission node;
- one admission node may control many relays;
- relays under the same admission node may connect directly;
- relays forward only traffic approved by their controlling admission node;
- external admission contact is allowed only so an external admission node can
  request work admission from the controlling admission node.

This keeps local process placement from becoming authority. Running two nodes on
the same machine does not let one bypass the other's boundary policy.

## Current Limits

The architecture is still being converged. The strongest implemented path today
is the `edgerun-work` admission, claim, receipt, verifier report, notary report,
and settlement envelope.

Known limits still being closed:

- production storage availability and retrieval proofs;
- compute result verification and verifier market rules;
- challenge windows, slashing, refunds, and finalization policy;
- concrete pricing rules for denial-of-service resistance;
- privacy guarantees beyond keeping raw payload bytes out of public proof
  envelopes;
- explicit external-admission forwarding evidence in route proofs;
- complete VFS, storage, compute, relay, notary, and verifier role integration.

The project should not claim more than the current proof path can verify. The
goal is to keep tightening the chain until every payable role has clear
evidence, a canonical verifier, clear failure modes, and clear settlement rules.

## Repository Layout

This is a Rust 2024 workspace. The root `Cargo.toml` is the source of truth for
workspace members.

Important areas:

- `crates/protocol/edgerun-work`: admitted work, routes, channels, proofs,
  receipts, role claims, verifier reports, notary reports, and settlement.
- `crates/protocol/edgerun-wire`: rkyv-only wire records for browser/native,
  node/app, capability, runtime, and transport boundaries.
- `crates/authority/*`: storage, VFS, and virtual disk authority surfaces.
- `crates/node/*`: node orchestration and runtime adapters.
- `crates/utility/edgerun-ui-core`: shared Rust UI scene and shell surfaces.
- `docs/coherent-system-spec.md`: current architecture model, strengths,
  limitations, and convergence plan.

## Development Checks

For `edgerun-work` protocol changes, use the repository cargo shim:

```bash
./cargo test --manifest-path crates/protocol/edgerun-work/Cargo.toml
./cargo test --manifest-path crates/protocol/edgerun-work/Cargo.toml --no-default-features
./cargo build --manifest-path crates/protocol/edgerun-work/Cargo.toml --target wasm32-unknown-unknown --release --no-default-features
```

Protocol changes should be small, no-std-compatible where possible, and backed
by tests that prove bad data is rejected.
