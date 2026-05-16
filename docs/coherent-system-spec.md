# EdgeRun Coherent System Spec

This spec consolidates the concepts already present in this workspace into a
single buildable system. It is intentionally narrower than the current repo:
one authority model, one wire boundary, one runtime/app model, and explicit
adapter edges for everything else.

## Problem Statement

The workspace contains useful concepts, but they are mixed across protocol,
runtime, SDK, marketplace, storage, UI, node, hardware, and compatibility
crates. Some concepts are canonical, some are projections, and some are
transitional. The system becomes buildable only if each concept has one owner
and one place in the execution flow.

The target product is:

```text
user-owned identity
  -> browser node
  -> admission policy
  -> verified network app package
  -> capability/storage/network bindings
  -> admitted work over routes and channels
  -> proof-backed receipts and audit events
  -> optional settlement
```

## Concept Inventory

### Canonical Concepts

These are core concepts that should survive into the minimal workspace.

| Concept | Current source | Canonical role |
| --- | --- | --- |
| Wire boundary | `edgerun-wire` | rkyv-only ABI records for browser/native/node/app movement. |
| User profile / Trust Container | `edgerun-wire`, `edgerun-work::trust_container_role` | Sealed owner profile and private authority boundary. |
| Node identity | `edgerun-work::identity`, hardware signing crates | Public-key identity for browser, admission, relay, storage, compute, app, and capability nodes. |
| Node role instance | `edgerun-work::protocol`, docs | `identity + role + policy + budget + route scope + runtime target`. |
| Work request | `edgerun-work::WorkRequest` | Signed user/app intent asking to cross an authority boundary. |
| Work admission | `edgerun-work::WorkAdmission` | Signed admission decision committing route, channel, budget, policy hash, validity, and request hash. |
| Route commitment | `edgerun-work::route_*`, `admitted_route` | Hash-bound path selected by admission. |
| Channel endpoint/envelope/proof | `edgerun-work::channel`, `work_channel`, `transport_channel` | Ordered packet movement and evidence over memory, WebSocket, TCP, QUIC, browser host, etc. |
| Capability envelope | `edgerun-work::capability_packet` | Generic request/invoke/event/close packet for devices, storage, network, render, input, object, and app capabilities. |
| Runtime capability grant | `edgerun-wire::RuntimeCapabilityGrant` | User/profile/app/release scoped grant. |
| Storage binding | `edgerun-wire::RuntimeStorageBinding` | Runtime binding from app namespace to provider capability. |
| Network binding | `edgerun-wire::RuntimeNetworkBinding` | Runtime binding from app route/fetch/socket scope to provider capability. |
| Capability session | `edgerun-wire::RuntimeCapabilitySession` | Short-lived live use of a grant bound to admission and route commitment. |
| Runtime event | `edgerun-wire::RuntimeEvent`, `edgerun-storage` | Append-only audit/projection event, optionally bound to admitted work evidence. |
| App manifest/package | `edgerun-wire::AppManifestRecord`, `edgerun-browser-authoring` staged from `edgerun-sdk::browser_authoring`, `package` | Signed content-addressed runnable app record. |
| Runtime app projection | `edgerun-wire::RuntimeAppInstall` | Verified runnable app metadata; despite the old name it is not install authority. |
| SDK unit | `edgerun-sdk`, `edgerun-unit` | Deterministic WASM unit with rkyv manifest and exact dependency hashes. |
| Composition/segment/chain | `edgerun-sdk` | Deterministic compute and distributed proof units. |
| Receipt/proof | `edgerun-work::WorkReceipt`, delivery/transit/storage proofs | Evidence for useful work and settlement. |
| Settlement | `edgerun-work::settlement`, `batch_settlement`, wallet/exchange crates | Optional payment layer consuming valid receipts and proofs. |

### Adapter Concepts

These concepts should exist, but only as adapters around the canonical model.

| Concept | Current source | Adapter role |
| --- | --- | --- |
| Browser host | `edgerun-node-ui-web`, `edgerun-ui-core` | Starts browser node, renders UI scene, stores local cache, bridges bytes. |
| Native node host | `edgerun-node`, `edgerun-node-ui-native` | OS glue for storage, sockets, processes, hardware, and UI. |
| Storage providers | `edgerun-storage`, `edgerun-vfs`, `edgerun-virtual-disk`, `edgerun-work::storage_adapter` | Back capability operations with memory, browser, native, virtual disk, or remote object stores. |
| External protocols | `edgerun-protocols`, HTTP/DNS/SMTP/IMAP/TFTP/TLS/etc. | Parse or encode external protocol bytes; never create EdgeRun authority. |
| Hardware identity | `edgerun-hardware-signing`, `edgerun-tpm`, `edgerun-yubikey` | Signing providers for node/user identity. |
| Device adapters | linux adapter crates, remote capability adapters | Bind camera, input, display, audio, Wi-Fi, Bluetooth, USB, PCI, sysfs, netif to capability envelopes. |
| OCI/virtio/platform/network driver | platform crates | Future native runtime substrate; not required for the first browser slice. |
| UI scene system | `edgerun-ui-core` | Projection of verified state and user intent into command buffers. |
| Codex crates | `crates/edgerun-codex/*` | Developer/agent tooling and UI surfaces, not base EdgeRun protocol. |
| Marketplace | `edgerun-sdk::marketplace*`, docs | Discovery, publisher metadata, checkout UX; authority remains package hash, policy, grants, and admitted work. |
| Wallet/exchange | wallet/exchange crates | Payment applications and settlement adapters. |
| Machine report | `edgerun-machine-report` | Native node inventory/projection input. |

### Transitional Or Legacy Concepts

These should not be lifted into the minimal workspace until their useful pieces
are migrated behind canonical records.

| Concept | Current source | Treatment |
| --- | --- | --- |
| Legacy core protocol records | `edgerun-core`, parts of `edgerun-protocols::core_protocol` | Mine for useful validators, then migrate to wire/work/storage/domain crates. |
| Mesh frames as authority | `edgerun-mesh`, legacy node command streams | Keep as transport/evidence only; cross-node authority is admitted work. |
| Install terminology | `RuntimeAppInstall`, marketplace docs/code | Keep type name for ABI compatibility, but product language is run/cache. |
| Store-only storage payment | storage and settlement experiments | Not payable until retrieval or availability evidence is included. |
| Unchecked receipt settlement | `edgerun-work::settlement` | Generic receipts only; relay payments require delivery and transit evidence. |
| Marketplace-specific protocol authority | marketplace docs/code | Do not lift as core. Catalogs discover packages; packages and admissions authorize work. |

## Canonical System Model

### Identities

Every actor is a node identity:

```text
user owner key
browser node
admission node
relay node
storage node
compute node
app node
capability node
publisher node
notary node
```

The node id is the public key used by the current verifier. Roles are attached
to identities by admission policy and runtime projection; a role does not invent
a second identity scheme.

### Role Instance

A runnable node instance is:

```text
NodeInstance {
  node_id,
  owner_id,
  role,
  runtime_target,
  policy_hash,
  budget,
  route_scope,
  status
}
```

Initial roles:

- `browser`: first local node and user intent signer.
- `admission`: verifies signed requests and signs admissions.
- `relay`: forwards admitted channel packets, keeps bounded transit proof state,
  and stops relaying when its proof custody path is not accepted.
- `storage`: stores and retrieves content-addressed objects.
- `capability`: exposes a device, API, app, object store, render target, or input.
- `notary`: seals or opens bytes under admitted notary work and signs plaintext
  access reports.
- `publisher`: signs app releases and identity-routed publication.
- `wallet`: records payment requests, receipts, and settlement state.

### Authority Flow

No work crosses a node, relay, storage, capability, execution, or settlement
boundary without this chain:

```text
Runtime intent
  -> signed WorkRequest
  -> signed WorkAdmission
  -> ChannelEnvelope packets on admitted route
  -> CapabilityEnvelope / storage / compute / app payload
  -> delivery, transit, execution, storage, or receipt proof
  -> RuntimeEvent audit projection
```

Runtime events are not authority by themselves. They are local evidence and UI
projection inputs. A runtime event can become strong evidence only when its
payload binds to the admitted work chain.

VFS, storage, and notary are separate roles in this chain. VFS builds and
verifies content-addressed object records. Storage persists and retrieves object
or shard bytes. Notary performs seal/unseal work and signs reports when
plaintext is opened. None of those roles replaces admission or relay.

Admission is boundary-local, not global. A browser node uses its own admission
node for browser-bound work such as package retrieval, first-run verification,
and run-from-network authority. A host node uses its own admission node for
host-bound work such as filesystem, storage-provider, device, native process, or
host capability invocation. A browser admission hash and a host admission hash
are different claims and must not be substituted for each other.

Every boundary has at least one admission node and at least one relay node. Node
inbound and outbound traffic crosses relay nodes regardless of node type;
storage, host, browser, compute, publisher, and capability nodes do not accept
authoritative work directly from arbitrary peers. Relay nodes are controlled by
admission nodes. A relay node has exactly one controlling admission node, while
one admission node may control many relay nodes. Relay nodes under the same
controlling admission node may connect directly to each other and relay only
traffic approved by that admission node. The only exception is external
admission contact: when an external admission node needs work admitted by this
boundary's controlling admission node, the relay may forward that admission
request to its controller.

Boundary authority is independent of machine, process, browser tab, or network
location. Nodes running on the same machine do not gain authority over each
other's boundaries. A single browser tab can host many independent boundaries,
each with its own boundary id, admission node set, relay node set, policy, and
budget. A single boundary can also span multiple machines when those machines
participate through that boundary's relay and admission topology. Local
in-process or loopback communication is only a transport optimization; it does
not bypass relay admission, boundary identity, or policy.

### Wire Boundary

All browser/native/node/app crossings use concrete rkyv records from
`edgerun-wire` or wire-compatible generated records from `edgerun-work`.

Rules:

- Do not add JSON, ad hoc bincode, or stringly side-channel records for internal
  EdgeRun movement.
- ABI constants are append-only commitments.
- New app/runtime/capability records belong in `edgerun-wire` unless they are
  work-authority records owned by `edgerun-work`.

### App Model

An app is not installed. It is verified, run, and optionally cached.

```text
AppPackage {
  app_id,
  release_id,
  manifest_hash,
  package_hash,
  developer_id,
  signatures,
  artifacts,
  declared_routes,
  storage_namespaces,
  provided_capabilities,
  required_capabilities,
  app_policy_hash
}
```

First run flow:

```text
retrieve package bytes
  -> verify content hash, manifest hash, developer signature, release id,
     app policy hash
  -> show deterministic retrieval cost
  -> user chooses Run once / Verify & cache / Cancel
  -> create RuntimeAppInstall projection if verified
  -> request grants only when the app asks for capabilities
```

The current SDK can project a verified package into `RuntimeAppInstall`,
`AppRunPromptDecisionRecord`, optional `PackageCacheRecord`, and corresponding
`RuntimeEvent` records. The node runtime accepts that same first-run projection:
Run once and Verify & cache make the verified app runnable, while Cancel records
the user decision without granting app runtime authority.

The app manifest declares possible routes, storage namespaces, and capabilities.
It does not grant itself authority.

### Capability Model

All local and remote resources use the same capability envelope:

```text
CapabilityEnvelope {
  session_id,
  invocation_id,
  capability_id,
  source_node_id,
  target_node_id,
  kind,
  operation,
  content_type,
  sequence,
  timestamp,
  payload_hash,
  payload
}
```

Examples:

- object storage: `object.get`, `object.put`, `object.delete`
- camera/microphone/speaker/display/input: stream and event operations
- network: scoped fetch, socket, HTTP route, node message
- app-to-app: routed app message
- Trust Container: sign, verify, seal, unseal, decrypt

Capability access requires:

```text
RuntimeCapabilityGrant
  -> RuntimeStorageBinding or RuntimeNetworkBinding when applicable
  -> WorkAdmission when crossing a node/capability boundary
  -> RuntimeCapabilitySession
  -> CapabilityEnvelope packets
```

### Storage Model

Storage is a capability, not a special side path.

Storage layers:

- content-addressed package/object bytes
- local browser cache or native cache
- event log and audit stream
- VFS/tree manifest/virtual disk adapters
- remote storage provider behind admitted capability work

`edgerun-vfs` is the canonical VFS packet layer. It defines:

- `VfsObjectPacket`: chunked bytes bound by object id, payload hash, packet id,
  offset, index, and total object length.
- `VfsFileRef`: path, object id, object length, and file hash.
- `VfsTreeManifest`: deterministic sorted file refs with a root hash.
- `VfsObjectTransformRef`: a verifiable mapping from plaintext object id to
  transport object id after compression/sealing.
- `VfsObjectSealRequest` and `VfsObjectUnsealRequest`: requests prepared for a
  sealing role, with AAD derived from plaintext object id, length, and
  compression kind.

VFS does not own admission, routing, payment, relay, or notary policy. It
prepares and verifies records that can be carried as work payloads. If VFS bytes
cross a boundary, they must be inside admitted `edgerun-work` packets and routed
through relays.

The canonical VFS flow is:

```text
plaintext file bytes
  -> VfsFileRef + VfsObjectPacket[] for unsealed content
  -> optional VfsObjectSealRequest
  -> admitted notary seal work
  -> NotarySealResponse(sealed_envelope)
  -> VfsObjectTransformRef + VfsObjectPacket[] for sealed transport bytes
  -> admitted storage store/retrieve work
  -> VfsObjectUnsealRequest from retrieved packets + transform ref
  -> admitted notary unseal work
  -> NotaryUnsealResponse + signed NotaryDeliveryReport
  -> unsealed VFS entry only after file ref and transform hashes verify
```

Storage nodes should store transport object bytes, not become VFS path
authorities. A storage node can prove it stored or retrieved the object bytes it
was admitted to handle; the VFS layer proves those bytes materialize the
declared file or tree.

VFS work should follow the same bounded proof custody rule as relay work. VFS
hashes the object packets, file refs, transform refs, and tree manifests it
produces or verifies. When VFS performs admitted packing work for another role,
the payable claim is the VFS worker's own claim over the packed root, input
object refs, output object/tree refs, admission hash, and policy hash. The
packed bytes can then be notarized and stored through admitted notary and
storage work. VFS does not need storage authority to be paid for packing; it
needs an admitted work claim whose hashes can be recomputed from the VFS records.

Payable storage must prove more than "I accepted bytes." Minimum production
evidence is retrieval proof or availability proof bound to the object hash,
provider identity, admission hash, and policy hash.

### Notary Model

`edgerun-work::NotaryRole` owns sealing and opening work. It accepts only
`DEPARTMENT_NOTARY` with `WORK_TYPE_NOTARY_SEAL` or
`WORK_TYPE_NOTARY_UNSEAL`. Requests and responses are `NotaryPayload` records
carried in `NetworkMessage.payload`.

Notary seal:

```text
NotarySealRequest {
  aad,
  plaintext_hash,
  plaintext
}
  -> NotarySealResponse {
       notary,
       requester,
       aad_hash,
       plaintext_hash,
       sealed_hash,
       sealed_envelope
     }
```

Notary unseal:

```text
NotaryUnsealRequest {
  aad,
  sealed_hash,
  sealed_envelope
}
  -> NotaryUnsealResponse {
       plaintext_hash,
       plaintext,
       delivery_report
     }
```

The delivery report is the durable proof that plaintext was opened. It is signed
by the notary and commits to notary identity, requester, request message id,
request payload hash, relay node, sequence, AAD hash, sealed hash, plaintext
hash, and open time. VFS should consume this report as access evidence when
materializing sealed objects.

Notary is not the Trust Container. Trust Container decrypts recipient-sealed
capability/message payloads for the owner. Notary seals and unseals work payloads
under admitted notary policy and emits a signed opening report. A design that
uses Trust Container for user-private local data can still use notary for
policy-bound shared, escrowed, or audited opening.

Notary work should also produce a hash-bound proof trail. For seal work, the
notary proof binds the admitted request, requester, notary node id, AAD hash,
plaintext hash, sealed hash, and response payload hash. For unseal work, the
notary proof additionally binds the sealed hash, plaintext hash, delivery report
hash, and plaintext access time. The notary signs the report because opening or
sealing is the notary's work product, not because the notary grants payment.

Notary settlement must require the admission signature and the notary signature.
The admission signature proves the notary was authorized to do that seal/unseal
work under a policy and budget. The notary signature proves the specific
seal/unseal result came from that notary node. Settlement then verifies both
against the request/admission hashes and the notarized payload hashes.

### Admission Model

Admission is the control plane.

Inputs:

- signed `WorkRequest`
- user/profile/app/release identity
- requested role, department, work type, budget, validity
- policy source and policy hash
- available relay/storage/capability nodes

Output:

- signed `WorkAdmission`
- route commitment
- assigned channel
- relay path
- admitted budget
- validity window
- admission node identity

Admission can be DAO-provided or user-owned. The UI must show which source was
used and the policy hash it enforced.

### Proof And Settlement Model

Proofs are role-specific:

- channel proof: ordered packet hash and route hash
- transit proof: relay handled packet hash in a hash chain
- delivery proof: recipient accepted payload under policy
- storage proof: retrieval or availability evidence
- VFS proof: object/file/transform/tree hashes materialized or packed under
  admitted work
- notary proof: signed seal/unseal report bound to admitted request and payload
  hashes
- execution proof: deterministic unit/composition/segment report
- receipt: worker claim bound to request and admission

Settlement consumes proofs. It does not create authority. Every payable role
claim must bind to a signed admission, request hash, worker identity, policy
hash, input hash, output hash, and role-specific verifier. Relay settlement must
require receiver delivery proof, transit hash, forwarded packet hash, and
admission/policy binding. VFS settlement must recompute the object/file/tree or
packed proof root. Notary settlement must verify the admission signature and the
notary signature over the seal/unseal report. Batch settlement must preflight
all receipts before mutating ledger state.

### Relay Proof Custody Model

A relay is a dumb admitted packet forwarder, not a storage authority. It should
hash each forwarded packet into a relay transit chain, but it must not retain an
unbounded proof log. The relay keeps only bounded working state:

- controlling admission node id and admission/session hash
- route/channel binding
- current sequence window
- previous transit hash and current/final transit hash
- packet or forwarded packet hashes for the active window
- recipient delivery proof references when available

When the active window reaches a policy limit, or before the relay drops local
proof state, the relay externalizes the proof material:

```text
relay forwards admitted packet
  -> relay computes forwarded packet hash and transit hash
  -> relay accumulates a bounded proof window
  -> VFS packs the proof fragments as content-addressed objects/manifests
  -> relay verifies the packed root matches its local proof window
  -> relay signs a batch claim over the packed proof root
  -> admitted notary work notarizes the relay-signed bundle root
  -> admitted storage work stores the notarized bundle bytes
  -> settlement later retrieves the bundle and verifies edgerun-work proofs
```

The relay signature is a batch claim, not a declaration that payment is owed. It
should commit to the relay node id, controlling admission node id, admission or
session hash, route/channel hash, sequence range, previous transit hash, final
transit hash, packed proof root, and validity window.

The notary does not make relay work valid. It signs a narrow custody/access
claim that the relay-signed bundle root was notarized under admitted notary
policy. VFS does not make relay work valid either; it only packs and verifies
content-addressed records. Storage persists the notarized bundle bytes. Payment
authority remains in settlement verification of the `edgerun-work` chain:
admission binding, route/channel binding, forwarded packet hashes, transit hash
continuity, recipient delivery proof, and policy.

A custody acknowledgement is replay-safe only when it is verified against the
actual bundle context. The verifier must recompute the relay transit bundle
root, match request hash, admission hash, and bundle root, check that the
acknowledged window covers the bundle's one-based hop range, require a nonzero
packed proof root, and reject expired acknowledgements. Relay-local sequence
counters are still part of each hop's transit hash, but they are not a global
sequence across independent relays. Runtime code should construct custody acks
from the completed relay bundle with the canonical `edgerun-work` constructor
instead of hand-filling request, admission, root, and hop-window fields.
Settlement-level custody verification must also compare the signed
`packed_proof_root` and `custody_kind` against the root and custody level the
caller expected. Otherwise an attacker could replay a real custody signature for
the same bundle but a different packed root, or satisfy a stored/notarized
custody requirement with a weaker packed-only acknowledgement.
The expected packed root and required custody kind should be carried as a single
custody requirement object and reused by both single-ack and chain verification
paths. The requirement itself must reject empty expected roots and unknown
custody kinds before evaluating acknowledgements. The requirement also has a
domain-separated hash so audit logs, policy records, challenge evidence, and
settlement references can point at the exact expected packed root and custody
level without repeating the whole object. It is wire-encoded so runtimes,
settlement, and proof dashboards can exchange the exact same requirement.
The custodian role must match the custody kind: packed custody can be signed by
storage or verifier roles, notarized custody by notary roles, and stored custody
by storage roles.
When a policy requires multiple custody steps, settlement should verify a
custody chain rather than a single acknowledgement. The chain verifier checks
that every acknowledgement targets the same relay bundle and packed proof root,
that custody kinds contiguously progress from packed to notarized to stored
without skipping steps, and that the final acknowledgement reaches the required
custody kind.

Relays are self-serving participants. If VFS, notary, storage, or settlement
participants do not handle proof custody correctly, a relay should apply
backpressure or stop accepting more relay work for that admission/session. This
keeps the role dumb and bounded while making correct cooperation the profitable
path for every role.

### Logic Flaws To Resolve

The proof-custody model is coherent only if these flaws are explicitly closed
before implementation:

1. Generic receipt settlement is too weak for payable roles.
   A signed `WorkReceipt` plus signed admission proves authorization and a claim,
   not useful work. Production settlement must require role-specific evidence
   for relay, VFS, notary, storage, compute, and capability work. Generic
   unchecked settlement is test/dev-only or restricted to non-payable local
   accounting.

2. VFS is currently a packet/materialization layer, not yet a payable node role.
   If VFS packing is payable, the protocol needs an explicit VFS role or
   capability work type, canonical VFS work input/output hashes, and a verifier
   that recomputes object packet ids, file refs, transform refs, tree roots, and
   packed proof roots. Without that, VFS should remain a library used by storage,
   notary, or local runtime code.

3. Notary seal work lacks the same signed report shape as unseal work.
   Existing notary unseal emits a signed plaintext-access report. Seal work also
   needs a signed seal report or a generic signed notary work report that binds
   request hash, admission hash, requester, notary node id, AAD hash, plaintext
   hash, sealed hash, response hash, and time. Admission alone cannot prove the
   notary produced the sealed output.

4. Proof custody can become recursively payable work.
   Relay proof bundles are carried by relay, packed by VFS, notarized by notary,
   and stored by storage; each of those operations may itself need proof. The
   system must define a checkpoint rule: proof-custody traffic is admitted under
   a bounded session, each role keeps local state until the next custody ack, and
   settlement accepts checkpoint roots rather than requiring real-time settlement
   of the custody traffic itself.

5. Packed proof roots are not enough if settlement cannot inspect the bundle.
   Settlement must receive either the full proof bundle, Merkle inclusion proofs
   for the claimed entries, or an admitted notary opening report for sealed
   bundles. Public settlement metadata should contain hashes, identities,
   admission/session refs, sequence ranges, and signatures; private payload bytes
   can remain sealed behind policy.

6. Batch claims can double-count individual receipts.
   A relay or VFS batch claim must define whether it replaces individual receipts
   or indexes them. The ledger must track either receipt hashes or non-overlapping
   worker/admission/sequence ranges so the same work cannot be paid once through
   an individual receipt and again through a bundle root.

7. Admission must bind usefulness, not only permission.
   Otherwise cooperating roles can admit and settle useless self-referential
   packing/notary/storage loops. Admission policy must bind the requester, input
   root, expected role, output shape, budget, route, validity, and consumer or
   later verifier that makes the work useful.

8. Relay control is an invariant but not a standalone proof object yet.
   The settlement path needs evidence that the relay was controlled by the
   admission node for the claimed session and that the route/channel was assigned
   by that controller. A relay controlled by one admission node must not settle
   work admitted by another controller unless the external-admission forwarding
   exception is explicitly present in the route proof.

9. Bounded-memory roles need loss and retry rules.
   A relay, VFS, or notary must not drop local proof state until it has a custody
   acknowledgement whose hash can be verified later. If VFS/notary/storage fails
   to acknowledge, the worker retries within the window, switches to an admitted
   alternate, or stops accepting new work for that session.

10. Time and validity must be verified at settlement.
    Notary report time, relay sequence windows, VFS pack time, and storage
    retrieval/availability evidence must fit inside admission validity or an
    explicitly admitted challenge/finalization window.

11. Role refusal is useful backpressure but can become denial of service.
    Admission should expose relay/VFS/notary/storage health and assign alternates
    when possible. A role that accepted a work session may refuse new work if its
    proof path is broken, but policy should define what happens to already
    accepted in-flight work.

12. Proof bundles must avoid leaking payloads unnecessarily.
    Settlement usually needs hashes, signatures, route/admission refs, sequence
    ranges, and recipient/storage/notary reports, not raw user payloads. Bundle
    design should split public verifiable metadata from sealed private payload
    refs.

### Low-Overhead Resolution

Do not solve the flaws by making every packet individually settled or by adding
a bespoke proof protocol per role. Use one small settlement-facing claim shape
and role-specific verifiers.

The common claim is:

```text
RoleWorkClaim {
  worker_node_id,
  worker_role,
  controlling_admission_node_id,
  request_hash,
  admission_hash,
  policy_hash,
  work_kind,
  input_root,
  output_root,
  sequence_start,
  sequence_end,
  units_used,
  total_claim,
  evidence_root,
  valid_until_unix_ms,
  worker_signature
}
```

`WorkReceipt` can remain the compact payment receipt, but production settlement
must pair it with a `RoleWorkClaim` or a role-specific evidence object that
hashes to the receipt input/output. This keeps the hot path small: each worker
keeps bounded local state, signs one batch claim, and submits one evidence root
per window instead of one settlement transaction per packet or operation.

Use role-specific verifiers behind the same interface:

```text
verify_relay_claim(claim, evidence)
verify_vfs_claim(claim, evidence)
verify_storage_claim(claim, evidence)
verify_notary_claim(claim, evidence)
verify_compute_claim(claim, evidence)
verify_capability_claim(claim, evidence)
```

Each verifier recomputes `input_root`, `output_root`, and `evidence_root` from
that role's canonical records. Settlement only needs to know that the verifier
accepted the claim under the signed admission and that the claim has not already
been paid.

Add a `verifier` role, but keep it optional on the happy path. A verifier node
does not create work authority and does not replace settlement. It performs
admitted verification work for expensive or disputed claims and signs a compact
`VerificationReport`:

```text
VerificationReport {
  verifier_node_id,
  verified_worker_node_id,
  verified_worker_role,
  claim_hash,
  evidence_root,
  verifier_policy_hash,
  result,
  checked_at_unix_ms,
  verifier_signature
}
```

Settlement may accept cheap claims by recomputing locally, and may require one
or more verifier reports for expensive, private, sampled, challenged, or
cross-boundary claims. Verifier work is itself admitted and payable only for the
verification it performed; it never makes an invalid claim valid.

Same-actor roles are allowed, but role claims stay separate. The same owner may
run relay, VFS, notary, storage, compute, verifier, and admission nodes on the
same machine or with the same public-key node id and different roles. That saves
transport and coordination overhead, but it does not merge authority. Settlement
still verifies each role's claim against its role verifier. Policy can discount
or disallow self-verification where independence matters.

Resolve each flaw with the minimum mechanism:

| Flaw | Low-overhead solution |
| --- | --- |
| Generic receipts too weak | Keep `WorkReceipt`, but require `RoleWorkClaim` plus verifier for payable production work. |
| VFS not a payable role | Either keep VFS as a library, or add a VFS role/work kind whose verifier recomputes object/file/transform/tree roots. |
| Unsigned notary seal output | Add one generic signed notary work report used for both seal and unseal. |
| Recursive custody | Admit proof-custody sessions with checkpoint roots; custody work is batched, not settled per hop. |
| Bundle inspection | Store public metadata roots in claims; reveal full bundle, inclusion proof, or notary opening only when policy requires. |
| Double counting | Ledger tracks `(worker, admission_hash, work_kind, sequence range)` plus receipt/claim hash. Ranges must not overlap. |
| Useless self-work | Admission must include expected consumer/verifier and output shape; settlement rejects claims with no admitted consumer. |
| Relay control proof | Admission signs relay assignment/control in the claim preimage or route proof. |
| Loss/retry | Workers drop local state only after a signed custody ack for the evidence root. |
| Time validity | Claims and reports carry checked/produced time and must fit admission or challenge windows. |
| Refusal/DoS | Admission assigns alternates and defines in-flight completion/refund rules. |
| Payload leakage | Claims expose hashes and refs; payload bytes stay sealed unless verification requires opening. |

Storage and compute fit the same pattern:

- Storage claim input is object/store/retrieve request root; output is
  retrieval or availability evidence root. Storage is not payable for accepting
  bytes alone.
- Compute claim input is program/unit/composition input root; output is result
  root plus deterministic execution, transcript, or verifier report. Compute is
  not payable for merely starting a process.

This keeps overhead bounded:

- one admission per work window/session, not per byte;
- one role claim per batch/window;
- one VFS/notary/storage custody path per evidence root;
- verifier reports only for expensive, sampled, private, or challenged claims;
- same-machine same-owner roles can use local transport while preserving
  separate role evidence and settlement checks.

### Current Strengths And Limits

Current implemented strengths:

- Role claims are signed by the worker and bind worker identity, role, controlling
  admission node, request hash, admission hash, policy hash, work kind, input
  root, output root, sequence range, evidence root, cost, and validity.
- Settlement rejects role claims whose receipt does not match the claim input,
  output, units, total claim, request, admission, worker, or final sequence.
- Settlement rejects claims outside admission/claim validity, claims over
  admission budget, duplicate claim hashes, and overlapping sequence ranges for
  the same worker, admission, and work kind.
- Authority-domain substitution is rejected because the claim admission hash,
  controlling admission node id, receipt admission hash, and signed admission
  must all agree.
- Verifier reports are signed by verifier-role nodes only, and a verifier report
  does not bypass settlement's own claim/receipt/admission checks.
- Notary work reports are signed by notary-role nodes only and bind seal/unseal
  work output to request/admission hashes and payload hashes.
- Relay transit bundles provide a bounded, wire-encoded proof root for a packet
  moving through multiple relays. Verification checks the request/admission
  context, controlling admission node, admitted relay path, hop order, endpoint
  continuity, packet hash continuity, per-hop transit hashes, bundle root, and a
  maximum hop count.
- Relay transit bundle construction is also bounded and protocol-level. The
  builder rejects empty or oversized paths, wrong-order relay hops, endpoint
  discontinuity, packet substitution, incomplete paths, and missing final
  delivery proof hashes before it can produce the bundle root used by settlement.
- Live `RelayRole` forwarding results carry the source, destination, channel,
  route, packet, input, previous-transit, and transit hashes needed to append
  directly into the bundle builder. This avoids a separate ad hoc evidence
  reconstruction path in relay runtime code.
- Relay proof custody acknowledgements provide the next handoff object after a
  bundle is complete. Storage, notary, or verifier nodes can sign that a relay
  bundle root has been packed under a custody root for a bounded bundle hop
  window. Verification binds the acknowledgement to the request hash, admission
  hash, recomputed bundle root, one-based bundle hop range, custodian role,
  custody-kind/role match, nonzero packed proof root, custody kind, signature,
  and expiry. Settlement can verify this custody handoff independently against
  an expected packed proof root and required custody kind, but the
  acknowledgement is evidence of custody only; it does not make invalid relay
  work valid and does not replace settlement verification of the bundle. A
  canonical constructor now creates this acknowledgement from the bundle and
  rejects wrong roles, wrong role/kind pairs, empty packed roots, and tampered
  bundle roots before runtime code can publish the handoff.
- Relay custody requirements are explicit verifier inputs. They reject empty
  expected roots and unknown custody kinds before a single acknowledgement or
  custody chain is accepted, and they have stable hashes for audit, policy, and
  challenge references. They are wire-encoded and roundtrip without changing the
  requirement hash.
- Std runtime settlement exposes the same custody requirement, single
  acknowledgement, and chain verification entry points as the portable core, so
  native services do not need a parallel custody verifier. It also exposes the
  admitted multi-relay settlement path and the custody-gated relay settlement
  path through the thread-safe runtime ledger.
- Custody-gated relay preflight and settlement return both the final custody
  acknowledgement hash and the custody requirement hash, giving audit and
  challenge code a compact reference to the evidence accepted and the policy
  enforced. The custody-gated settlement result is wire-encoded so runtime
  services and proof dashboards can exchange the same audit object. Settlement
  result hashes, custody check hashes, and custody-gated settlement result
  hashes give audit/challenge records stable references to what was paid and
  why.
- Relay custody chains verify ordered, contiguous progression over the same
  packed proof root. This lets settlement require, for example, packed ->
  notarized -> stored custody without accepting an unrelated stored signature,
  a skipped intermediate step, or a downgraded final step.
- Multi-relay relay-hop settlement now requires a signed relay receipt to match
  a verified admitted transit bundle. Settlement checks the signed admission,
  assigned relay path, admission route commitment, bundle context, hop identity,
  receipt input hash, transit output hash, sequence, final recipient delivery
  proof, recipient policy, budget, and duplicate receipt state.
- Public claim/report records contain hashes and references, not raw payload
  bytes.
- Same-owner roles can use local transport, but settlement still checks each role
  as a separate signed role claim.

Current implemented adversarial tests cover:

- replaying a worker claim through a different admission domain;
- using the wrong controlling admission node for relay-like work;
- overlapping sequence ranges under the same worker/admission/work kind;
- allowing the same sequence range under a different admission or work kind;
- zero-unit claims, backwards ranges, expiry, and budget exhaustion;
- colluding verifier reports that approve a receipt/claim mismatch;
- verifier/notary reports signed by the wrong role;
- multi-relay proof bundles with cross-domain substitution, wrong relay paths,
  tampered packet hashes, broken endpoint continuity, forged summary roots, and
  oversized paths;
- relay bundle builder misuse, including wrong hop order, packet substitution,
  incomplete path, missing delivery proof hash, and oversized path;
- live relay forwarding output appended into a transit bundle builder and then
  verified as a relay transit bundle;
- relay proof custody acknowledgements with wrong custodian role, invalid
  custody kind, custody-kind/role mismatch, empty packed proof root, invalid
  sequence range, signature tampering, wire roundtrip, replay against a
  different bundle root, wrong bundle hop window, constructor misuse, wrong
  expected packed proof root, wrong required custody kind, settlement-level
  custody verifier rejection, and expiry;
- relay custody requirements with empty expected packed roots or unknown
  required custody kinds, plus deterministic requirement hashes that change when
  expected root or required custody kind changes and remain stable after wire
  roundtrip;
- relay custody chains with empty chains, wrong packed proof root, reordered,
  skipped, or downgraded custody kinds, and missing required final custody kind;
- relay-hop settlement through a multi-relay bundle, including wrong admitted
  path, receipt/input mismatch, and forged bundle root rejection;
- multi-relay route-hash substitution where the bundle is internally valid but
  no longer matches the admission route commitment;
- multi-relay settlement with a tampered or wrong-relay recipient delivery
  proof;
- public claim/report byte records not embedding private payload bytes.

Current limits:

- `RoleWorkClaim` proves the shared settlement envelope. Full role-specific
  evidence verifiers for VFS, storage retrieval/availability, compute execution,
  and capability invocation still need to be implemented.
- Verifier reports are defined and signed, but settlement does not yet have a
  policy path that requires verifier reports for expensive, private, sampled, or
  challenged work.
- Multi-relay proof bundles now have a concrete hash-bound evidence record and a
  relay-hop settlement path with recipient delivery proof and signed route
  commitment integration. A bounded protocol builder now constructs the same
  bundle shape that settlement verifies, and live relay forwarding output can
  append into that builder. Relay proof custody acknowledgements now give
  VFS/storage/notary/verifier custody a signed handoff object, and settlement
  can verify that handoff without making it payment authority. The remaining
  relay work is carrying builder state and custody acknowledgements across real
  multi-hop runtime sessions.
- Cross-authority-domain forwarding is modeled by admission/relay invariants,
  but still needs explicit external-admission forwarding evidence in route proofs.
- VFS is still mostly a packet/materialization library. Making VFS packing
  payable requires a VFS role/work kind and a verifier that recomputes packet,
  file, transform, tree, and packed proof roots.
- Storage is still not production-payable for "accepted bytes"; the payable path
  must require retrieval or availability evidence.
- Compute claims currently have the envelope only; deterministic unit execution,
  transcripts, or verifier reports must define the useful compute output root.
- Privacy tests prove public proof envelopes do not carry raw payload bytes; they
  do not prove traffic metadata privacy, timing privacy, or protection from
  inference across colluding relays/storage/verifiers.
- DoS protection is currently economic and structural: bounded ranges, budgets,
  validity, duplicate/range rejection, and role refusal. Network-level rate
  limits, admission alternates, backoff, challenge windows, and in-flight refund
  policy still need implementation.

## Buildable Vertical Slice

The first complete system should be a browser-first local network app run path.

### Slice Goal

Build a user-visible loop that proves:

```text
create/unlock profile
  -> start browser node
  -> select admission policy
  -> retrieve verified app package
  -> run once or verify and cache
  -> grant one storage capability
  -> open admitted capability session
  -> write/read one object
  -> record proof/audit events
  -> show Trust Manager rows from real records
```

No marketplace, wallet settlement, native hardware, OCI, or broad protocol
support is required for this slice.

### Required Records

Use existing records where possible:

- `UserProfileEnvelope`, `UserProfileBody`
- `RuntimeAppInstall`
- `RuntimeCapabilityGrant`
- `RuntimeStorageBinding`
- `RuntimeCapabilitySession`
- `RuntimeEvent`
- `AppManifestRecord`
- `WorkRequest`
- `WorkAdmission`
- `ChannelEndpoint`
- `ChannelEnvelope`
- `CapabilityEnvelope`
- `WorkReceipt` only for measured storage/retrieval once evidence exists

These records are now part of `edgerun-wire` and should be used by the first
slice:

- `NodeInstanceRecord` in `edgerun-wire`
- `AdmissionPolicyRecord` in `edgerun-wire`
- `PackageCacheRecord` in `edgerun-wire`
- `AppRunPromptDecisionRecord` in `edgerun-wire`

### First Runtime State Machine

```text
Locked
  -> ProfileOpen
  -> BrowserNodeStarted
  -> AdmissionSelected
  -> PackageRetrieved
  -> PackageVerified
  -> RunPromptShown
  -> AppRunnable
  -> CapabilityRequested
  -> GrantApproved
  -> AdmissionRequested
  -> SessionOpen
  -> CapabilityInvoked
  -> AuditRecorded
```

Every transition emits `RuntimeEvent`. Events after `AdmissionRequested` include
the relevant request hash, admission hash, route commitment, or capability
session id in the payload.

### First UI Surfaces

The UI should be only these surfaces:

- Unlock/create Trust Container.
- Admission selection: EdgeRun DAO admission or user-owned admission.
- Network app first-run prompt: Run once, Verify & cache, Cancel.
- Capability prompt: show app, release, capability, operation, scope, and
  policy hash.
- App surface: minimal runnable test app that reads/writes one object.
- Trust Manager: profile, package verification, cache state, grants, sessions,
  admission, route, and audit events.

### Success Criteria

The slice is complete when these claims are proven by tests:

- A package with the wrong code hash or manifest hash is rejected.
- A package without a valid developer signature is rejected.
- A declared storage namespace does not grant storage access by itself.
- A storage capability request without a user grant is denied.
- A granted storage capability opens a session only after admission.
- Capability payload hash mismatch is rejected.
- Audit events form a previous-event hash chain.
- Trust Manager rows are derived from real records, not hardcoded demo rows.

## Minimal Workspace Extraction

Create a new workspace that starts with the smallest coherent set. Do not copy
the entire current monorepo.

### Phase 1: Core Crates

Lift:

```text
crates/protocol/edgerun-wire
crates/protocol/edgerun-work
crates/authority/edgerun-storage
crates/edgerun-sdk
crates/edgerun-unit
crates/utility/edgerun-unit-macros
```

Also lift only the local utility crates actually required by those crates after
`cargo metadata` in the new workspace. Prefer deleting feature paths over
lifting broad compatibility stacks.

Measured from `cargo metadata --format-version 1` on the current workspace, the
naive closure for the Phase 1 roots is 46 crates. That is too large for the
first extracted workspace because it pulls transitional protocol, machine
report, device, Linux adapter, and virtual disk paths. Treat that as evidence
that the extraction must start with narrow features and dependency pruning, not
with a direct copy of the current default feature graph.

Keep features narrow:

- `edgerun-wire`: `no_std + alloc` default.
- `edgerun-work`: extract with `default-features = false`; add `std` only for
  local test/native adapters.
- `edgerun-storage`: memory and append-only event log first.
- `edgerun-sdk`: extract with `default-features = false`; enable only
  `browser-authoring` and the smallest runtime projection feature needed for
  package verification.

The current measured closures are:

| Root set | Current closure | Meaning |
| --- | ---: | --- |
| `edgerun-wire` + `edgerun-work` | 34 crates | Current defaults still pull protocol and virtual-disk paths. |
| `edgerun-sdk` | 41 crates | Current default `std` pulls machine-report/protocol/device paths. |
| Phase 1 roots | 46 crates | Too broad for the new workspace without feature pruning. |
| Browser runtime roots | 70 crates | Must be rewritten or lifted surgically, not copied as-is. |
| Staged browser core | 25 crates | Physically lifted/pruned `browser-wire`, `browser-work`, `browser-authoring`, `browser-runtime`, and `browser-host` slice, including signed request/admission, recipient channel proof, storage availability/retrieval proof, first-run input/projection wire bundles, a byte-oriented host boundary, raw WASM-callable host entry points, a dependency-free browser JS byte adapter, a real first-run package projection adapter, and a smoke harness driven by Rust-produced rkyv bytes. |

### Phase 2: Browser Runtime

Lift or rewrite minimally:

```text
crates/node/edgerun-node
crates/node/edgerun-node-ui-web
crates/utility/edgerun-ui-core
```

Keep only:

- profile unlock/create bridge
- browser node state
- app package verification
- cache store
- runtime grants/bindings/sessions
- memory or browser storage provider
- UI scene projection

Do not lift native services, broad external protocol adapters, Codex tooling,
OCI, virtio, linux adapters, hardware adapters, marketplace settlement, or
wallet/exchange in this phase.

### Phase 3: User-Owned Node Roles

Add back, one role at a time:

- native admission node
- relay node
- storage node with retrieval/availability evidence
- publisher node
- wallet/settlement app
- hardware signing providers
- external protocol adapters

Each added role must enter through `WorkRequest -> WorkAdmission -> route ->
proof -> RuntimeEvent`.

## Ownership Rules For New Workspace

| Owner | Owns | Must not own |
| --- | --- | --- |
| `edgerun-wire` | rkyv record shapes and ABI constants | policy decisions, host behavior, settlement logic |
| `edgerun-work` | signed work, admission, routes, channels, proofs, receipts | UI, OS sockets as core semantics, app business policy |
| `edgerun-storage` | local logs, objects, cache indexes, projection stores | network authority or payment claims |
| `edgerun-sdk` | app/unit/package manifests and verification | runtime grants, admission decisions |
| `edgerun-node` | resource binding and runtime orchestration | protocol authority bypassing admission |
| `edgerun-ui-core` | UI projection and intent capture | duplicated app metadata or policy enforcement |

## Decisions

1. Authority is admitted work.
2. Wire is rkyv.
3. Storage and network are capabilities.
4. Apps are verified and run, not installed.
5. Catalogs and marketplaces are discovery only.
6. Runtime events are audit/projection records, not standalone authority.
7. Payment requires proof of useful work.
8. Browser node is the first node, not a demo client.
9. Native/std code is adapter glue over portable core logic.
10. New concepts must either become canonical records or stay local adapters.

## Immediate Implementation Backlog

1. Done: add wire records for node instance, admission policy, package cache,
   and app run prompt decision.
2. Done: implement SDK package verification and first-run prompt projections
   using `RuntimeEvent`.
3. Done: make `edgerun-node` record SDK first-run projections, cache records,
   and app-run decisions through canonical `edgerun-wire` records.
4. Done: replace Trust Manager hardcoded rows with host-supplied projections
   for package cache, grants, sessions, admissions, and audit events.
5. Done: add memory/browser object storage as capability providers behind the
   runtime storage trait.
6. Done: open a local admitted storage capability session and route
   `CapabilityEnvelope` object put through it.
7. Done: add rejection tests for invalid package, missing grant, missing
   admission/session, bad payload hash, and broken audit chain.
8. In progress: staged `workspaces/browser-core` with a `browser-core-slice`
   smoke crate that depends on the narrow browser-first feature set and proves
   package verify -> first-run cache decision -> runtime grant/session ->
   storage capability envelope -> audit-chain verification. The slice now also
   builds for `wasm32-unknown-unknown --release` through the EdgeRun cargo shim.
   Physical lifts now include `edgerun-browser-wire`, which preserves the
   `edgerun_wire` crate API for the staged rkyv boundary,
   `edgerun-browser-runtime`, which replaces `edgerun-node/node-core`, and
   `edgerun-browser-authoring`, which replaces the broad `edgerun-sdk`
   dependency for package authoring, verification, developer signatures, and
   first-run projections. The staged browser wire boundary now includes
   `BrowserAppFirstRunInputRecord`, `BrowserAppFirstRunProjectionRecord`,
   `BrowserPackageRetrievalRecord`, and
   `BrowserPackageRetrievalEvidenceRecord`, so browser JS can submit package
   bytes plus one rkyv input record and receive a single rkyv projection bundle
   without parsing app/package/runtime records.
   The staged `edgerun-browser-work` crate now preserves
   the `edgerun_work` API for signed `WorkRequest`, signed `WorkAdmission`,
   ordered channel envelopes, recipient channel proofs, storage
   availability/retrieval proofs, route commitments, capability envelopes, and
   BLAKE3 hashes used by browser storage invocation. The staged
   `edgerun-browser-host` crate now accepts rkyv `SdkWireRecord` bytes for
   first-run projection bundles, first-run parts, grants, storage binding,
   sessions, and storage capability requests, then returns
   `BrowserHostResultRecord` bytes plus proof hashes.
   `BrowserHostStorageInvocationRecord` now carries the storage invocation
   context over the same rkyv boundary. The host crate also exports raw
   WASM-callable functions for host creation/drop, byte allocation/free,
   first-run projection bundles, first-run parts, grants, storage binding,
   sessions, and storage invocation. `browser/host-adapter.mjs` now adds the
   dependency-free browser
   JS adapter around `edgerun-browser-host`: it loads the WASM module, allocates
   and copies rkyv input bytes, copies and frees output buffers, persists
   package/object bytes in browser storage, and keeps host-visible state as
   rkyv records. `browser/first-run.mjs` is the first real first-run browser
   adapter: it copies package manifest bytes, app graph bytes, developer
   signature bytes, and a serialized `BrowserAppFirstRunInputRecord` into Rust,
   receives a serialized `BrowserAppFirstRunProjectionRecord`, persists those
   bytes when a byte store is provided, and records the projection through the
   host without JS decoding rkyv. The JS first-run input path now asks Rust to
   serialize `BrowserAppFirstRunInputRecord` from the selected Run once /
   Verify & cache / Cancel decision, and passes `u64` WASM ABI values as
   `BigInt`. `browser/package-retrieval.mjs` asks Rust to build retrieval
   evidence when the browser does not already have it: Rust verifies the package,
   signs and verifies a `WorkRequest`, admits it through a signed
   `WorkAdmission`, and returns opaque `BrowserPackageRetrievalEvidenceRecord`
   bytes plus the browser admission hash. The same adapter then asks Rust to
   serialize `BrowserPackageRetrievalRecord` from package bytes, package key,
   retrieval cost, browser admission hash, and retrieval/proof evidence bytes
   before returning that opaque rkyv record to JS.
   `browser-core-slice` now exposes deterministic smoke fixture
   package/grant/session/storage rkyv bytes from Rust authoring/runtime helpers,
   and `browser/smoke-harness.mjs` drives those bytes, or caller-provided
   selected package bytes, through the real first-run projection adapter, grant,
   storage binding, session, and storage invocation. The real Node/WASM smoke
   harness now completes against
   `browser/dist/browser_core_slice.wasm` while using the same Rust
   developer-signature verification path as native tests. `browser/index.html` and
   `browser/page.mjs` are the first no-bundler browser entry: they load
   `browser/dist/browser_core_slice.wasm`, open `EdgeRunBrowserByteStore`, pass
   the selected Run once / Verify & cache / Cancel decision into the Rust
   first-run input builder, forward optional selected package bytes to the
   browser-core harness, and surface returned first-run projection and host
   result byte lengths without parsing rkyv in JS. `browser/package-source.mjs`
   now maps a selected network app package key to manifest, graph, and developer
   signature byte records in the browser byte store and returns opaque bytes to
   the same first-run path. It also persists admitted retrieval results by
   storing package parts plus optional retrieval, proof, browser admission, and
   compatibility source-admission evidence bytes. `browser/retrieval-flow.mjs`
   now owns that browser-side
   retrieval assembly: given retrieved package bytes, a selected package key,
   and a byte store, it asks Rust for missing browser-admission-bound evidence,
   browser admission hash, and retrieval record bytes before persisting opaque
   package and rkyv record bytes. The smoke harness calls this adapter instead
   of carrying retrieval policy inline. `browser/network-retrieval.mjs` is the
   first browser node retrieval entry point: it fetches manifest, graph, and
   developer-signature bytes from caller-provided package part URLs, asks Rust
   to serialize or apply a `BrowserPackageRetrievalPolicyScheduleRecord` when no
   explicit retrieval cost is supplied, and then hands off to
   `browser/retrieval-flow.mjs`. The schedule bytes are stored with retrieval
   evidence, and Rust binds the hash of the validated schedule record bytes into
   the signed `WorkAdmission` and `BrowserPackageRetrievalEvidenceRecord`.
   The retrieval entry point can also call a supplied browser boundary adapter.
   In that path, `browser/relay-node.mjs` is a JS relay participant: it forwards
   the browser admission request envelope to its single controlling WASM
   admission node and rejects requests for any other controller. The relay does
   not verify package semantics or create authority; the WASM admission node is
   responsible for package verification, signed `WorkRequest` handling, signed
   `WorkAdmission`, retrieval evidence, and signed browser `WorkAdmission`
   packet bytes. The request carries an explicit boundary id so multiple browser
   boundaries can coexist in the same tab without inheriting authority from each
   other. Host capability/storage work must use a separate host relay plus host
   admission path and host admission hash.
   The EdgeRun cargo shim now
   accepts explicit
   `--crate-type` flags and the build driver honors lib crate-type settings,
   allowing the staged slice to emit both an rlib and a browser-loadable WASM
   cdylib when requested. Current `./cargo tree -p browser-core-slice` reports
   25 packages and no longer includes the monorepo `edgerun-wire`,
   `edgerun-work`, `edgerun-node`, `edgerun-sdk`, storage authority, device,
   mesh, hardware, Linux adapter, virtual-disk, or platform crates. The next
   extraction pass should replace the deterministic local admission fixture with
   a concrete signed-`WorkRequest` submission path so the JS relay forwards
   opaque admitted work bytes to its controlling WASM admission while continuing
   to route all app, runtime, and admitted retrieval records through the same
   Rust rkyv projection path.

## Non-Goals For The First Build

- Production marketplace checkout.
- Token-first economics.
- Native hardware signing requirement.
- Relay payment settlement.
- OCI app runtime.
- Full HTTP/DNS/SMTP/IMAP/TLS support.
- Linux device adapters.
- Codex agent UI integration.
- Cross-device high availability.
- Production erasure coding.
