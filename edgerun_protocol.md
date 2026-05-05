
# edgerun Core Protocol v0
Single-file specification

Status: working draft
Scope: semantics, abstract record families, rkyv wire bytes, validation, conformance, and reference implementation layout

---

## Table of contents

0. Conformance and notation
1. Axioms
2. Scope and model
3. Core definitions 
4. Stream model
5. Event and command model
6. Object and storage model
7. Trust and delegation model
8. Access model
9. Networking model
10. Interaction flows
11. Versioning
12. Security model
13. Non-goals
14. Abstract record families
15. Cross-family validation rules
16. Internal wire readiness
17. Rkyv archive bytes and cryptographic inputs
18. Validation state machines
19. Conformance suite and reference skeleton
Appendix A. Global invariants
Appendix B. Canonical message families
Appendix C. Core state machines
Appendix D. Term normalization
Appendix E. Native rkyv record layout
Appendix F. Reference repository layout

---

## 0. Conformance and notation

The keywords **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

The protocol defines semantics first and encoding later. A conforming implementation may vary in storage backend, transport binding, batching, caching, retry strategy, or UI, but it **MUST** preserve the same meanings, authority boundaries, validation rules, and durability semantics.

Normalization decisions:

- A stream has exactly one writer for its entire lifetime.
- An event in a stream is always authored by that stream’s writer. If an event needs to mention some other principal, that principal appears in payload or metadata, not as the stream writer.
- `seq` is the strictly increasing per-stream sequence number. The term “nonce” is not used for stream ordering.
- The core model is authored streams, immutable objects, snapshots, and query-based access. Replication exists for durability and access, not as universal mirroring.
- A command is not authoritative until the target node validates it and records the outcome in its own stream.

---

## 1. Axioms

1. A node is any active protocol participant capable of storing data, establishing sessions, enforcing local policy, or writing one or more streams.
2. An identity is a cryptographic principal. Identities and nodes are distinct.
3. Every stream has exactly one writer.
4. Durable truth is append-only. Mutable application state is derived.
5. Transport is not identity.
6. Reachability is not trust.
7. Authority is explicit, local, bounded, and verifiable.
8. Replication is selective and policy-bound.
9. Objects are immutable. Changes create new objects.
10. Snapshots are derived and useful, but never the root of truth.
11. Persistent state changes for a subject become authoritative only when committed by that subject’s own writer in that subject’s own stream.
12. The system is local-first and partition-tolerant.

---

## 2. Scope and model

The system is a distributed, identity-based, append-only information fabric.

It consists of:
- identities that hold authority
- nodes that operate in the protocol
- single-writer streams that record durable history
- immutable objects that carry data
- snapshots and indexes that accelerate access
- queries that provide bounded access without requiring full replication

The protocol does **not** require:
- global consensus
- universal full replication
- a central coordinator
- one transport
- one storage backend

Global total ordering is not a goal. Single-writer local ordering is.

The system favors:
- access over possession
- verifiability over ambient trust
- immutable history over shared mutable state
- receiver-driven retrieval over sender-driven guaranteed delivery

---

## 3. Core definitions

### 3.1 Node

A **node** is an execution, storage, networking, or policy-enforcing participant.

A node is typically a device, service instance, or execution environment, but is not limited to physical hardware.

A node is not defined by its network location.

### 3.2 Identity

An **identity** is a cryptographic principal defined by a public identifier and proof of control over a corresponding private key.

Identities may represent:
- a user
- a node
- an agent
- a service

A node may possess multiple identities. An identity may control multiple nodes.

### 3.3 Node identity

A **node identity** is the identity a node uses to authenticate sessions and write its own streams.

### 3.4 User identity

A **user identity** is a long-lived principal that delegates authority and controls nodes.

### 3.5 Agent

An **agent** is a principal operating under delegated authority.

### 3.6 Stream

A **stream** is a strictly ordered, append-only sequence of immutable events with one fixed writer.

### 3.7 Node stream

A **node stream** is a stream written only by a node and representing that node’s authoritative account of its own observations, decisions, and actions.

### 3.8 Event

An **event** is an immutable stream entry committed by the stream writer.

### 3.9 Command

A **command** is a signed external request directed at a node. A command is not authoritative until the target node validates it and records the outcome in its own stream.

### 3.10 Logical object

A **logical object** is an immutable data unit with stable identity.

### 3.11 Stored representation

A **stored representation** is a concrete storage form of a logical object, possibly encrypted, chunked, compressed, or access-packaged.

### 3.12 Snapshot

A **snapshot** is a derived immutable object representing a view at one or more stream heads or checkpoints.

### 3.13 View

A **view** is a derived perspective over streams and objects, such as a recent timeline, controller set, search index, or current topology.

### 3.14 Capability

A **capability** is a bounded permission over a defined scope, validity window, and constraint set.

### 3.15 Delegation

A **delegation** is a signed transfer of bounded authority from one principal to another.

### 3.16 Controller

A **controller** is an identity authorized to influence a node.

### 3.17 Assurance claim

An **assurance claim** is signed evidence about security posture, such as hardware-backed keys or attested runtime.

### 3.18 Locator

A **locator** is a transport-specific means of attempting contact.

### 3.19 Reachability hint

A **reachability hint** is advisory routing or contact information about how a node may currently be reached.

### 3.20 Route

A **route** is a local plan for reaching a target identity.

### 3.21 Head

A **head** is the latest valid event in a stream.

### 3.22 Checkpoint

A **checkpoint** is a stable reference to a stream position or set of stream positions.

### 3.23 Storage tier

A **storage tier** is a policy class such as hot, warm, cold, or archive.

---

## 4. Stream model

Every stream has:
- a stable `stream_id`
- a fixed `writer_identity`
- exactly one genesis event at `seq = 0`

Events in a stream are ordered by strictly increasing `seq`. No gaps are allowed.

Every event after genesis carries `prev_hash`, which **MUST** equal the canonical hash of the immediately preceding event.

The event hash commits to the canonical event representation. At minimum that representation includes:
- stream_id
- seq
- prev_hash
- event_type
- event_version
- timestamp fields
- payload reference if any
- required metadata

Every event **MUST** be signed by the stream’s writer identity.

A stream is valid only if:
- genesis exists and is unique
- sequence numbers are contiguous
- previous-hash linkage is correct
- canonical hashes verify
- signatures verify
- the writer identity remains consistent with genesis

Streams do not fork in v0. If two different events appear at the same sequence number for the same stream, the stream is invalid beyond the divergence point until resolved out of band.

State is derived by replaying stream events. The stream is authoritative for its subject. Derived state is not.

---

## 5. Event and command model

Commands are external signed requests. Events are authoritative internal records.

A command includes:
- command identifier
- target node identity
- issuer identity
- command type
- payload or payload reference
- issuance time
- optional expiry
- delegation chain if any
- issuer signature

A node receiving a command **MUST** verify:
- target binding
- issuer signature
- delegation chain validity if present
- scope and capability constraints
- replay resistance
- freshness and timing policy
- any required assurance conditions

Delivery alone has no effect on authoritative node state.

The receiving node makes a local decision: reject or commit.

If rejected, the node records a `command_rejected` event in its own stream.

If committed, the node records a `command_committed` event in its own stream.

For long-running actions, the node **MAY** later record `action_started`, `action_completed`, or `action_failed`.

The command sender **MAY** independently record `command_sent` in its own stream. This produces deliberate double-entry history: the sender records issuance, the receiver records acceptance or rejection.

A node's authoritative state changes only through events the node itself appends to its own stream.

### 5.1 Replay resistance

Replay detection is keyed by `command_hash` (SHA-256 of the canonical signable command envelope), which is globally unique by construction. The `command_id` field is an application-level idempotency hint only and MUST NOT be used as the replay key.

- Same `command_hash` => `DUPLICATE` (already processed)
- Different `command_hash` => distinct command, regardless of `command_id` value

---

## 6. Object and storage model

Streams carry small, ordered, signed facts. Objects carry data.

A logical object is immutable and has stable identity derived from canonical logical content or an equivalently stable immutable definition.

A stored representation is the storable form of an object. Stored representations **MAY** differ by:
- encryption wrapper
- chunking
- compression
- access packaging
- storage tier

The logical object identity must stay stable even if multiple stored representations exist.

Objects may represent:
- event payloads
- attachments
- snapshots
- manifests
- indexes
- commands
- proofs
- derived views

Encryption applies to stored representations, not to stream semantics. Confidentiality must not depend on trusting the transport, the storage host, or the disk.

Large objects **MAY** be chunked. Chunking requires a manifest or equivalent immutable description that preserves integrity and reconstruction order.

Storage is tiered:
- **hot**: optimized for latency and immediate working state
- **warm**: optimized for common durable access
- **cold**: optimized for economical durable retention
- **archive**: optimized for long-term durability and low-frequency retrieval

Data temperature influences placement, caching, promotion, and eviction. Temperature does not affect identity or verifiability.

Replication is selective and policy-bound. Access is not equivalent to possession. A node may access data through local possession, snapshots, queries, or retrieval on demand.

### 6.1 Reference storage profile

For the reference implementation profile, the recommended storage split is:

- **event log and immutable protocol records** as the recoverable source of truth
- **encrypted blob bytes on the filesystem**
- **file-based binary indexes** for caches, replay state, fetch queues, and derived local materializations

This means:

- the index is **not** the authoritative semantic source of truth
- the index exists for performance and local queryability
- the index **SHOULD** be rebuildable from the event log plus locally present encrypted blobs

The protocol does **not** require any specific indexing engine, but a conforming reference profile using binary append-only file indexes and filesystem storage for encrypted blob bodies is preferred over specialized database engines unless local requirements clearly justify them.

### 6.2 Blob confidentiality invariants

For the reference storage profile:

- every persisted blob **MUST** be encrypted at rest
- every persisted blob **MUST** name at least one recipient
- no plaintext blob persistence path is part of the reference profile

Recipient metadata may be stored alongside blob headers, access packages, or equivalent local index material, but confidentiality of blob content must not depend on trusting the local disk, storage host, or relay.

### 6.3 Rebuildability and local indexes

Reference indexes such as:

- current stream heads
- accepted/rejected/deferred status
- object availability
- representation/chunk presence
- replay cache state
- pending fetch and query work

are local performance structures. They **MAY** be deleted and rebuilt. Loss of such indexes does not invalidate already stored records or blobs.

---

## 7. Trust and delegation model

Authority is explicit and local. There is no global trust root.

Every node evaluates authority using:
- its own genesis state
- locally available trust records
- verified delegation chains
- local policy
- locally known revocation and assurance state

For interoperability, every rule in this section falls into exactly one bucket:
- **deterministic core rule** — every conforming implementation MUST produce the same outcome for the same semantic inputs and local state
- **bounded local policy rule** — implementations MAY configure thresholds or allow-lists, but MUST expose the decision boundary as explicit local policy input
- **implementation-defined rule** — allowed only where the outcome is advisory and never silently changes authority, integrity, or identity semantics

A capability defines:
- allowed actions
- scope
- constraints
- validity bounds
- whether onward delegation is allowed

Delegation transfers bounded authority from issuer to recipient. A delegation chain must be cryptographically verifiable, rooted in an authority the node recognizes, and obey attenuation. Child delegations must not expand the privilege of their parent.

Some capabilities may be non-delegable.

A node’s durable controller set is authoritative only when that node records it in its own stream. External claims about node control are inputs, not authoritative control state, until the node accepts and commits them.

Ephemeral authority and installed authority are distinct. A node may honor a one-time delegated command without installing the delegate as a persistent controller.

Revocation is explicit but not magically global. A revocation becomes effective for a node when that node receives it, verifies it, and treats it as authoritative under local policy.

Revocation is not retroactive by default. Past committed events remain history.

### 7.1 Assurance normalization

Assurance evaluation is a **deterministic core rule** once the local policy inputs are fixed.

`AssuranceClass` ordering in v0 is:

```text
ASSURANCE_CLASS_UNSPECIFIED < ASSURANCE_CLASS_SOFTWARE < ASSURANCE_CLASS_HARDWARE_BACKED < ASSURANCE_CLASS_ATTESTED_RUNTIME
```

Rules:
- a claim with an absent or expired validity window MUST NOT satisfy a positive assurance requirement
- if `max_evidence_age` is present, implementations MUST compare it against `issued_at`
- if `acceptable_attesters` is non-empty, the claim attester MUST be a member of that set
- multiple claims MAY exist for the same subject; a requirement is satisfied if at least one accepted claim satisfies the requested class, age, and attester constraints
- conflicting claims do not cancel each other by default; revocation or local policy must do that explicitly

### 7.2 Revocation normalization

Revocation processing is a mix of **deterministic core rules** and **bounded local policy rules**.

Rules:
- a revocation with no target is structurally invalid
- a revocation with `effective_at` in the future is not yet active; implementations SHOULD treat it as known-but-not-yet-effective rather than as an active revocation
- if `effective_at` is absent, the revocation becomes active when accepted locally
- revocation target matching is exact for the target family and identifier carried by the record
- revocation does not rewrite past committed history; it only affects future authority evaluation and future trust decisions unless a higher-level application policy explicitly says otherwise
- if `replacement_id` is present, it is advisory linkage only unless the accepting policy explicitly grants replacement semantics
- `scope_override` narrows the accepted effect of a revocation; it MUST NOT expand the set of things revoked beyond the record's declared target family

### 7.3 Installed authority vs ephemeral authority

This distinction is a **deterministic core rule**.

Rules:
- direct controller installation changes durable node control state only through the node's own committed stream events
- delegated authority may authorize a one-time command without granting durable controller installation
- implementations MUST NOT infer durable control from successful delegated command execution alone
- external control assertions remain inputs until the subject node commits the resulting control state

---

## 8. Access model

Access is distinct from replication and distinct from possession.

A node may gain useful state through:
- local replay
- snapshot consumption
- remote query
- object retrieval
- federated aggregation

The protocol is access-oriented, not sync-oriented.

The authoritative roots of truth are valid streams and valid immutable objects they reference. Snapshots, indexes, caches, and query results are derived artifacts.

A query is a bounded request for information. Queries **SHOULD** be constrained by one or more of:
- target node or node set
- stream or view
- time range
- checkpoint
- object class
- result count
- cost limit

A node **MUST** authorize a query before answering it. Query answers may be full, partial, redacted, denied, or metadata-only.

Federated queries are normal. Each responder returns its own fragment from its own local knowledge. Aggregation produces a derived answer, not a new root truth.

Live access and historical access are distinct. Live access favors freshness from currently reachable nodes. Historical access favors completeness from stored streams, snapshots, and archived objects.

Caches and snapshots are valid access tools, but never automatically authoritative over underlying streams.

Access responsibility is local:
- if a node wants an event, object, snapshot, or query result, it is that node's responsibility to ask for it
- no sender is required to retransmit by default
- no receiver is entitled to completeness merely because some peer once advertised, relayed, or mentioned an artifact
- missing data is a normal condition and is handled by later query, later fetch, or continued partial operation

### 8.1 Query proof normalization

Proof interpretation is split as follows:
- proof structure and family matching are **deterministic core rules**
- responder allow-lists, proof size limits, and optional enrichment are **bounded local policy rules**
- scoring or ranking of multiple acceptable advisory answers is **implementation-defined** unless a stricter local policy object says otherwise

Rules:
- a `QueryResultFragment` is advisory by default and MUST NOT become authoritative solely by being well-formed or signed
- a `QueryResultFragment` without any backing references, proof objects, or bundled result object is not a sufficient positive answer for object existence or view derivation
- a responder MAY return `DENIED`, `PARTIAL`, or `METADATA_ONLY`; those are valid protocol outcomes and not protocol failures
- a proof bundle MUST identify its payload family explicitly
- a proof bundle whose `source_query_id` does not match the active local query context for which it is being evaluated MUST be rejected for that query context
- a proof bundle whose declared payload object is not locally available for validation SHOULD be deferred rather than rejected when the receiver's local policy expects local backing before use
- `SnapshotSetProof`, `EventSetProof`, `ObjectAssertionProof`, `AggregateSummaryProof`, and `TrustPolicyProof` are advisory by default in v0 unless a stricter validator promotes them under explicit local policy
- a `SnapshotSetProof` or `EventSetProof` with an empty asserted set is structurally invalid
- an `ObjectAssertionProof` without an `object_ref` is structurally invalid
- an `AggregateSummaryProof` with overlapping included and excluded responders is structurally invalid
- a `TrustPolicyProof` MUST carry at least one of `policy_object` or `assignments_object`
- if a request specifies `required_proof_classes`, a positive answer SHOULD carry enough references or proof objects to satisfy those classes, otherwise the receiver SHOULD treat the result as advisory or partial rather than authoritative
- a `FederatedAggregateDescriptor` is a derived summary only; it MUST NOT silently override the semantics of its input fragments

### 8.2 Promotion rules for derived access artifacts

Promotion from advisory data into local durable derived state is a **bounded local policy rule**, but the following invariants are mandatory:
- snapshots, query results, proof bundles, and aggregates MAY be cached
- none of them become stream authority or controller authority by being cached
- local durable derived views MUST retain enough linkage to the streams, checkpoints, snapshots, or proof objects they came from
- proof artifacts accepted in v0 without full local backing remain advisory cached evidence and MUST NOT by themselves install trust, controller state, or stream authority
- route advertisements MAY be cached as network hints, but remote route-policy artifacts MUST NOT become authoritative route-selection state merely by being received
- implementations SHOULD distinguish at least:
  - advisory accepted
  - usable with local backing
  - promoted derived state
  - authoritative subject-local state

### 8.3 Route trust and selection normalization

Route trust evaluation in v0 is intentionally narrow.

Classification:
- structural validity of route-policy objects is a **deterministic core rule**
- acceptance of remotely supplied route-policy artifacts is a **bounded local policy rule**
- route scoring inputs beyond the normalized fields below are **implementation-defined advisory inputs**

Rules:
- `RouteTrustAssignments` and `AggregateTrustPolicy` are local policy inputs by default, not remote authority
- an implementation SHOULD only accept remotely supplied route-policy artifacts from a current controller or configured trust root unless a stricter local policy explicitly allows more sources
- `RouteSelectionPolicy` MUST apply hard constraints before scoring:
  - minimum quality
  - maximum cost
  - required active session, if present
- a route failing a hard constraint is ineligible
- v0 normalized route score is the integer sum of:
  - assignment trust score, default `0`
  - reachability quality hint, default `0`
  - minus any local cost penalty, default `0`
- if multiple eligible routes tie, implementations SHOULD break ties in this order:
  1. highest score
  2. preferred advertiser
  3. preferred next hop
  4. earliest advertisement timestamp
  5. lexicographically smallest next-hop node identifier
- no remotely supplied route-policy artifact becomes controller authority, trust-root authority, or stream authority merely by being accepted

---

## 9. Networking model

The protocol is identity-first and transport-agnostic.

A node is addressed by identity, not by IP address, BLE address, or any other locator.

Locators are transport-specific contact methods. Reachability hints are advisory statements about how a node may currently be contacted.

A node may support multiple transports simultaneously. No node is required to reduce its behavior to the weakest transport available in the environment.

Sessions authenticate identities, not merely sockets. A session may be direct or relayed, and may migrate between transports if continuity can be preserved safely.

Routing is local. A route is a local plan for reaching a target identity using known locators, relays, cost information, and policy.

Direct and relayed paths are both normal. Relay use is not a failure mode. A relay may forward encrypted traffic without learning plaintext.

The protocol distinguishes:
- **control-plane traffic**
- **data-plane traffic**
- **discovery-plane traffic** (optional narrower subset, e.g. BLE advertisement)

Recommended interpretation:
- the **control plane** carries small, cheap, lossy-tolerant metadata such as event announcements, head advertisements, want/have hints, query requests, denials, and routing hints
- the **data plane** carries larger or more expensive bodies such as full event envelopes, snapshots, encrypted payload objects, manifests, and chunk data
- the control plane is allowed to be opportunistic and incomplete; the data plane is usually receiver-driven

The protocol is intentionally compatible with transports where message loss is normal, including short-range discovery and opportunistic transports such as BLE.

No transport binding is required to provide reliable retransmission by default. Reliability, if desired, is built by later re-query, later fetch, or local retry policy rather than being a semantic requirement of every transmission.

Connectivity never implies trust or authority. A relay, bridge, or nearby node does not gain control by virtue of carrying traffic.

### 9.1 Receiver-driven transport pattern

The preferred transport pattern in v0 is:

1. **announce**
2. **express interest**
3. **fetch body**
4. **query later if still missing**

This is a transport pattern, not a new source of authority.

Interpretation:
- an **announcement** is a cheap hint that some event, head, snapshot, object, or route artifact exists
- a **want/have** exchange is a cheap expression of receiver interest or duplicate state
- a **fetch** transfers the full envelope, object, snapshot, or chunk body
- a later **query** repairs missed opportunities or fills gaps

Bindings SHOULD prefer:
- small control-plane announcements first
- full bodies only when receiver interest is shown
- object and payload fetch by reference where possible
- later repair by query rather than mandatory sender retransmission

Bindings MAY realize this pattern using:
- existing protocol messages
- transport-native framing
- discovery beacons
- mailbox or relay queues

But they MUST preserve the same semantics:
- announcement is not acceptance
- fetch is not commitment
- delivery is not authority
- missing a first announcement is normal and repaired later by explicit request

---

## 10. Interaction flows

The normative flows are:

### 10.1 Node bootstrap

A node establishes its identity, writes genesis at `seq = 0`, defines initial control basis, and begins its stream.

### 10.2 Discovery and session establishment

Nodes discover each other, authenticate identities, establish secure communication, and optionally upgrade from weaker or local discovery transports to stronger data transports.

This flow is optional for one-shot or opportunistic exchange. A transport binding MAY support direct message publication and later receiver-driven retrieval without first establishing a long-lived session.

Recommended cheap-first flow:
- announce current head or candidate event cheaply
- let the receiver answer `have`, `want`, or silence
- transfer the full body only when wanted
- allow later query if the first opportunity is missed

### 10.3 Delegation issuance and use

A principal creates a bounded delegation, signs it, and a delegate later presents it with a request. The receiving node verifies the full chain under local policy.

### 10.4 Command issuance and commitment

An issuer signs a command. The sender may record issuance. The receiver validates it and either rejects or commits it. Only the receiver’s stream authoritatively records the receiver’s decision.

### 10.5 Long-running action execution

A committed command may be followed by explicit started, completed, or failed events.

### 10.6 Snapshot production and consumption

A producer derives a view at known heads or checkpoints, signs the snapshot, and publishes it. A consumer verifies it and may request deltas beyond its base.

### 10.7 Query and federated query

A requester issues bounded queries to one or more responders. Responders return local fragments. The requester aggregates them if desired.

Queries are the normal repair path for missed deliveries. A node that did not receive an event, object, or snapshot when first announced MAY later ask for it explicitly. No prior sender commitment to retransmit is implied.

### 10.8 Object publication, replication, and retrieval

A producer creates a logical object, stores one or more encrypted representations, replicates selectively, and later requesters retrieve and verify those objects by reference.

Bindings SHOULD prefer:
- event or snapshot body first only when small enough
- otherwise body by reference
- payload object transfer only when the receiver asks
- chunk transfer only when the receiver asks

### 10.9 Cold access, promotion, and eviction

Nodes fetch colder objects only when needed, may promote them to hotter tiers on demand, and may evict them later while retaining enough metadata to locate and verify them again.

### 10.10 Persistent control change and control transfer

A node accepts a control change only through a validated command and only becomes authoritative once it records the resulting controller state in its own stream.

### 10.11 Revocation processing

Nodes receive, verify, and apply revocations according to local knowledge and policy.

### 10.12 Reconnection after partition

Nodes refresh heads, snapshots, deltas, and trust updates without any multi-writer stream merge.

### 10.13 Store-and-forward

Deferred delivery is allowed and affects timing only, not authorship or commitment semantics.

Store-and-forward is best-effort. Carriage by an intermediary does not imply eventual delivery, completeness, fairness, or persistent retention.

---

## 11. Versioning

The protocol must evolve without semantic drift.

Versioning is required at multiple layers:
- core protocol semantics
- stream or event envelope families
- event types
- object schemas
- command envelopes
- query families
- transport bindings

A stream’s genesis anchors the interpretation basis for that stream.

Unknown optional fields may be ignored only if doing so does not weaken authority, identity, or integrity semantics. Unknown required fields or unknown critical types must cause rejection.

Rkyv archive rules and native record shape are versioned semantics. A change in the archived signable bytes is a breaking change for object identity, event hashing, and signatures.

Mixed-version operation is allowed only where both parties can preserve the same meaning safely.

Feature negotiation is not a substitute for versioning. Features negotiate optional behavior. Versions define meaning.

### 11.1 Version handling classes

For interoperability, every family should classify fields and enum values as:
- **critical to authority/integrity**
- **critical to semantic interpretation**
- **advisory/optional**

Rules:
- unknown fields MUST NOT participate in rkyv signable bytes unless introduced by the negotiated record version
- unknown values in authority-critical or semantic-critical enums MUST cause rejection before cryptographic acceptance
- unknown advisory enum values MAY be preserved at the wire layer but MUST NOT silently change authority, integrity, or identity semantics
- mixed-version operation is safe only when both sides agree on the rkyv archive and validation meaning of the families being exchanged

---

## 12. Security model

The protocol is designed to provide:
- strong identity binding to keys
- explicit and bounded authority
- tamper-evident stream history
- object-level confidentiality independent of storage host
- transport-independent security semantics
- auditability of accepted authority use

The protocol assumes:
- untrusted networks
- untrusted relays
- possibly malicious storage providers
- intermittent connectivity
- possible stale or conflicting trust knowledge
- possible software-key compromise

The primary security boundary is the principal and its local policy enforcement.

The protocol does not guarantee:
- honesty of compromised nodes
- perfect metadata privacy
- instant global revocation
- permanent availability
- correctness of all derived views
- safe arbitrary remote compute on untrusted hosts
- immunity to bad local policy
- retransmission until receipt
- sender-side responsibility for making every receiver whole
- that any node will obtain all events or payloads unless it explicitly asks and validates them

A delivered command is not an authority event. Only the receiving node’s committed stream entry is.

Confidentiality protects payload content, not necessarily timing, size, or topology metadata.

Availability is an operational property created by replication, repair, and economics, not a guarantee of the core protocol itself.

---

## 13. Non-goals

The core protocol does not provide:
- global consensus
- one universal chain
- one total order over all events

It does not require:
- universal full sync
- a shared mutable global database
- a mandatory central coordinator
- reliable transport by default
- sender-side push of all possibly relevant data

It does not grant authority from:
- network topology
- transport adjacency
- physical proximity

It does not guarantee:
- permanent availability
- perfect metadata privacy
- globally optimal routing
- complete dissemination to all interested peers
- that a node will receive everything it cares about without asking for it

It does not claim that arbitrary remote compute is safe on untrusted nodes by default.

It does not require secure hardware, though it can represent stronger assurance when present.

It does not bind itself permanently to one transport, one backend, or one serialization.

It does not retroactively erase history. Corrections and revocations are new facts.

It does not define the economic layer in v0.

---

## 14. Abstract record families

### 14.1 Record conventions

Field classes:
- **required**
- **optional**
- **repeated**
- **oneof**

These abstract record families define:
- field names
- field meaning
- required/optional status
- validation constraints

They do **not** define:
- byte encoding
- rkyv archive layout
- exact integer sizes
- exact hash byte lengths
- exact enum numeric values

Those belong to the encoding layer.

### 14.2 Common scalar and reference types

#### Identifier
Opaque stable identifier.

Used for:
- node_id
- stream_id
- identity_id
- command_id
- delegation_id
- revocation_id
- object_id
- representation_id

Constraint:
- stable within its semantic domain
- bytewise comparable
- never reused for different semantic objects

#### Version
Version marker for a schema family or object family.

#### Timestamp
Absolute time marker.

#### Duration
Positive time interval.

#### Digest
Cryptographic hash output.

#### Signature
Cryptographic signature over the domain-separated hash of rkyv signable bytes.

#### Bytes
Opaque byte sequence.

#### Text
Canonical text value.

#### Bool
Boolean.

#### Enum
Closed symbolic set for a given schema family version.

#### IdentityRef
- `identity_id` — required
- `identity_kind` — optional
- `key_hint` — optional

#### NodeRef
- `node_id` — required

#### StreamRef
- `stream_id` — required

#### EventRef
- `stream_id` — required
- `seq` — required
- `event_hash` — required

#### HeadRef
- `stream_id` — required
- `seq` — required
- `event_hash` — required

#### CheckpointRef
- `checkpoint_id` — optional
- `heads` — repeated HeadRef, required unless `checkpoint_id` resolves externally

#### ObjectRef
- `object_id` — required
- `object_kind` — optional

#### RepresentationRef
- `representation_id` — required
- `object_id` — required

#### CommandRef
- `command_id` — required
- `command_hash` — required

#### DelegationRef
- `delegation_id` — required
- `delegation_hash` — required

#### RevocationRef
- `revocation_id` — required
- `revocation_hash` — required

#### SnapshotRef
- `snapshot_id` — required
- `object_id` — optional

### 14.3 Rkyv archive rules for structured records

All structured records in this section follow these rules unless the family says otherwise:

- Native Rust protocol records are the source of truth for internal bytes.
- Internal hashing, signing, storage, transport payloads, caches, and local bridges MUST use rkyv-family archive bytes for the concrete protocol type at the boundary.
- Signable bytes are produced from the record with its `signature` field absent.
- Full bytes preserve the stored or transported record, including signature when present.
- Repeated fields preserve order exactly as supplied.
- Core records MUST NOT use arbitrary key-value maps for authority-bearing semantics.
- Unknown required fields invalidate the record.
- Unknown optional fields may be ignored only if doing so cannot weaken identity, integrity, or authority semantics.
- Legacy wire-schema bridges, alternate canonical encoders, and compatibility adapters are not part of the protocol.

### 14.4 IdentityRecord

Fields:
- `record_version` — required
- `identity_id` — required
- `identity_kind` — required
- `key_algorithm` — required
- `public_key` — required
- `created_at` — optional
- `supersedes_identity` — optional IdentityRef
- `assurance_claim_objects` — repeated optional
- `metadata_object` — optional ObjectRef
- `signature` — optional

### 14.5 CapabilityDescriptor

Fields:
- `capability_version` — required
- `capability_kind` — required
- `actions` — repeated required
- `scope` — required ScopeDescriptor
- `constraints` — optional ConstraintSet
- `delegation_policy` — required
- `minimum_assurance` — optional AssuranceRequirement
- `capability_metadata` — optional ObjectRef

### 14.6 ScopeDescriptor

Fields:
- `scope_version` — required
- `scope_kind` — required
- `target_nodes` — repeated optional
- `target_streams` — repeated optional
- `target_object_kinds` — repeated optional
- `target_view_types` — repeated optional
- `target_domains` — repeated optional
- `time_bounds` — optional TimeWindow
- `scope_metadata` — optional ObjectRef

### 14.7 ConstraintSet

Fields:
- `constraint_version` — required
- `not_before` — optional Timestamp
- `expires_at` — optional Timestamp
- `max_uses` — optional positive integer
- `rate_limit` — optional RateLimit
- `requires_local_session` — optional Bool
- `requires_user_presence` — optional Bool
- `requires_transport_classes` — repeated optional
- `requires_location_classes` — repeated optional
- `export_policy` — optional
- `execution_class_limits` — repeated optional
- `storage_class_limits` — repeated optional
- `constraint_metadata` — optional ObjectRef

### 14.8 AssuranceRequirement

Fields:
- `assurance_version` — required
- `required_class` — required
- `acceptable_attesters` — repeated optional
- `max_evidence_age` — optional Duration
- `assurance_metadata` — optional ObjectRef

### 14.9 DelegationRecord

Fields:
- `record_version` — required
- `delegation_id` — required
- `issuer` — required IdentityRef
- `recipient` — required IdentityRef
- `issued_at` — required Timestamp
- `not_before` — optional Timestamp
- `expires_at` — optional Timestamp
- `capability` — required CapabilityDescriptor
- `parent_delegation` — optional DelegationRef
- `revocation_authorities` — repeated optional
- `delegation_metadata` — optional ObjectRef
- `signature` — required

### 14.10 RevocationRecord

Fields:
- `record_version` — required
- `revocation_id` — required
- `issuer` — required IdentityRef
- `issued_at` — required Timestamp
- `effective_at` — optional Timestamp
- `revocation_kind` — required
- one target among:
  - `target_delegation`
  - `target_identity`
  - `target_node`
  - `target_object`
- `scope_override` — optional ScopeDescriptor
- `reason_code` — optional
- `replacement_ref` — optional
- `revocation_metadata` — optional ObjectRef
- `signature` — required

### 14.11 EventEnvelope

Fields:
- `envelope_version` — required
- `stream_id` — required
- `seq` — required
- `prev_event_hash` — required for `seq > 0`, absent for `seq = 0`
- `event_type` — required
- `event_version` — required
- `recorded_at` — required Timestamp
- `effective_at` — optional Timestamp
- `payload_object` — optional ObjectRef
- `related_events` — repeated optional
- `related_commands` — repeated optional
- `related_objects` — repeated optional
- `related_delegations` — repeated optional
- `related_revocations` — repeated optional
- `event_metadata` — optional ObjectRef
- `signature` — required

Derived value:
- `event_hash` = canonical hash of signable form

### 14.12 NodeGenesisPayload

Fields:
- `payload_version` — required
- `node_id` — required
- `primary_node_identity` — required IdentityRef
- `initial_controllers` — repeated required
- `initial_policy_object` — optional ObjectRef
- `bootstrap_records` — repeated optional
- `assurance_claims` — repeated optional
- `node_roles` — repeated optional
- `genesis_metadata` — optional ObjectRef

### 14.13 CommandEnvelope

Fields:
- `envelope_version` — required
- `command_id` — required
- `target_node` — required NodeRef
- `issuer` — required IdentityRef
- `command_type` — required
- `command_version` — required
- `issued_at` — required Timestamp
- `not_before` — optional Timestamp
- `expires_at` — optional Timestamp
- `idempotency_key` — optional Identifier
- one payload:
  - `payload_object` — optional ObjectRef
  - `inline_payload` — optional Bytes
- `delegation_chain` — repeated optional
- `requested_assurance` — optional AssuranceRequirement
- `command_metadata` — optional ObjectRef
- `signature` — required

Derived value:
- `command_hash` = canonical hash of signable form

### 14.14 CommandResultPayload

Fields:
- `payload_version` — required
- `command` — required CommandRef
- `issuer` — required IdentityRef
- `decision` — required
- `decision_basis` — optional ObjectRef
- `reason_code` — optional
- `effect_summary_object` — optional ObjectRef
- `result_object` — optional ObjectRef

### 14.15 ActionLifecyclePayload

Fields:
- `payload_version` — required
- `origin_command` — required CommandRef
- `action_instance_id` — required Identifier
- `status` — required
- `result_object` — optional ObjectRef
- `error_object` — optional ObjectRef
- `progress_object` — optional ObjectRef
- `action_metadata` — optional ObjectRef

### 14.16 SecretPutPayload

Records that a secret was created, stored, or rotated. The actual secret value
is NOT in this payload — only metadata. The encrypted secret lives in the
BlobStore; the blob_id is referenced here.

Fields:
- `payload_version` — required
- `namespace` — required string (credential namespace, e.g. "default", "login")
- `key` — required string (stable key identifier derived from label + attributes)
- `label` — required string (human-readable label)
- `attributes` — optional map<string, string> (key-value pairs for search)
- `secret_blob_id` — required string (hex blob ID of the encrypted secret)

### 14.17 SecretDeletePayload

Records that a secret was deleted. Only metadata is recorded.

Fields:
- `payload_version` — required
- `namespace` — required string
- `key` — required string
- `label` — required string
- `reason` — optional string (reason for deletion)

### 14.18 CollectionCreatedPayload

Records that a secret collection was created.

Fields:
- `payload_version` — required
- `collection_name` — required string (also used as namespace)
- `label` — required string (human-readable display label)

### 14.19 CollectionDeletedPayload

Records that a secret collection was deleted.

Fields:
- `payload_version` — required
- `collection_name` — required string
- `items_removed` — required uint32 (count of items in the collection)

### 14.20 LogicalObjectDescriptor

Fields:
- `descriptor_version` — required
- `object_id` — required
- `object_kind` — required
- `object_schema_version` — required
- `canonicalization_id` — required
- `canonical_digest` — required Digest
- `canonical_size` — required
- `created_at` — optional Timestamp
- `producer` — optional IdentityRef
- `describes_object` — optional ObjectRef
- `object_metadata` — optional ObjectRef

### 14.17 StoredRepresentationHeader

Fields:
- `header_version` — required
- `representation_id` — required
- `object` — required ObjectRef
- `representation_digest` — required Digest
- `plaintext_size` — optional
- `stored_size` — required
- `encryption_scheme` — optional
- `compression_scheme` — optional
- `chunking_mode` — required
- `chunk_manifest_object` — optional ObjectRef
- `access_package_object` — optional ObjectRef
- `created_at` — optional Timestamp
- `representation_metadata` — optional ObjectRef

### 14.18 ChunkManifest

Fields:
- `manifest_version` — required
- `object` — required ObjectRef
- `representation` — optional RepresentationRef
- `chunk_count` — required
- `total_stored_size` — required
- `chunk_entries` — repeated required
- `manifest_metadata` — optional ObjectRef

### 14.19 SnapshotDescriptor

Fields:
- `descriptor_version` — required
- `snapshot_id` — required
- `view_type` — required
- `view_version` — required
- `producer` — required IdentityRef
- `produced_at` — required Timestamp
- `base_heads` — repeated optional
- `base_checkpoints` — repeated optional
- `scope` — required ScopeDescriptor
- `completeness` — required
- `payload_object` — required ObjectRef
- `supersedes` — optional SnapshotRef
- `snapshot_metadata` — optional ObjectRef
- `signature` — required

### 14.20 QueryRequest

Fields:
- `request_version` — required
- `query_id` — required
- `requester` — required IdentityRef
- `target_scope` — required ScopeDescriptor
- `query_class` — required
- `time_window` — optional TimeWindow
- `checkpoint_base` — optional CheckpointRef
- `result_limit` — optional positive integer
- `cost_limit` — optional
- `required_proof_classes` — repeated optional
- `query_payload_object` — optional ObjectRef
- `signature` — optional

### 14.21 QueryResultFragment

Fields:
- `fragment_version` — required
- `query_id` — required
- `responder` — required IdentityRef
- `answered_at` — required Timestamp
- `completeness` — required
- `snapshot_refs` — repeated optional
- `event_refs` — repeated optional
- `object_refs` — repeated optional
- `inline_objects` / bundled result — optional by schema family
- `proof_objects` — repeated optional
- `omission_reason` — optional
- `result_metadata` — optional ObjectRef
- `signature` — optional

### 14.22 FederatedAggregateDescriptor

Fields:
- `descriptor_version` — required
- `aggregate_id` — required
- `source_query_id` — required
- `aggregator` — required IdentityRef
- `aggregated_at` — required Timestamp
- `input_fragments` — repeated required
- `aggregation_policy_object` — optional ObjectRef
- `payload_object` — required ObjectRef
- `signature` — optional

### 14.23 ReachabilityHint

Fields:
- `hint_version` — required
- `subject_node` — required NodeRef
- `transport_class` — required
- `locator_payload` — required
- `directness` — required
- `valid_after` — optional Timestamp
- `valid_until` — optional Timestamp
- `cost_hint` — optional
- `quality_hint` — optional
- `issuer` — optional IdentityRef
- `signature` — optional

### 14.24 RouteAdvertisement

Fields:
- `advertisement_version` — required
- `target_node` — required NodeRef
- `advertiser` — required IdentityRef
- `next_hop_node` — optional NodeRef
- `reachability` — repeated required
- `metric_hint` — optional ObjectRef
- `advertised_at` — required Timestamp
- `expires_at` — optional Timestamp
- `route_metadata` — optional ObjectRef
- `signature` — optional

### 14.25 SessionHello

Fields:
- `message_version` — required
- `initiator` — required IdentityRef
- `target_node` — optional NodeRef
- `supported_transport_features` — repeated optional
- `supported_protocol_versions` — repeated required
- `session_nonce` — required Bytes
- `initiator_locators` — repeated optional
- `hello_metadata` — optional ObjectRef
- `signature` — required

### 14.26 SessionAccept

Fields:
- `message_version` — required
- `responder` — required IdentityRef
- `echoed_session_nonce` — required Bytes
- `selected_protocol_version` — required
- `selected_transport_features` — repeated optional
- `responder_locators` — repeated optional
- `accept_metadata` — optional ObjectRef
- `signature` — required

### 14.27 RelayEnvelope

Fields:
- `envelope_version` — required
- `relay_message_id` — required
- `original_sender` — required IdentityRef
- `intended_recipient_node` — required NodeRef
- `relay_chain` — repeated optional
- `payload_kind` — required
- one payload:
  - `payload_object` — optional ObjectRef
  - `inline_payload` — optional Bytes
- `store_until` — optional Timestamp
- `relay_metadata` — optional ObjectRef
- `signature` — optional

---

## 15. Cross-family validation rules

These rules tie the schemas together.

- An `EventEnvelope` is valid only in the context of its stream’s writer identity and stream history.
- A `CommandEnvelope` is valid only if the target node accepts the issuer and delegation chain under local policy.
- A `DelegationRecord` is valid only if each parent link attenuates rather than expands authority.
- A `RevocationRecord` becomes effective only when received and accepted under local policy.
- A `LogicalObjectDescriptor` defines semantic identity. A `StoredRepresentationHeader` never replaces it.
- A `SnapshotDescriptor` is valid only as a derived object over the declared base heads or checkpoints.
- A `QueryResultFragment` is a responder-local answer. It is not automatically authoritative without backing references.
- A `ReachabilityHint` or `RouteAdvertisement` is advisory. It must never silently alter trust or authority.

---

## 16. Internal wire readiness

The internal wire protocol is rkyv only. These protocol families are stable enough to archive as native records at rkyv boundaries without changing the architectural model:

- IdentityRecord
- CapabilityDescriptor
- ScopeDescriptor
- ConstraintSet
- DelegationRecord
- RevocationRecord
- EventEnvelope
- NodeGenesisPayload
- CommandEnvelope
- CommandResultPayload
- ActionLifecyclePayload
- SecretPutPayload
- SecretDeletePayload
- CollectionCreatedPayload
- CollectionDeletedPayload
- LogicalObjectDescriptor
- StoredRepresentationHeader
- ChunkManifest
- SnapshotDescriptor
- QueryRequest
- QueryResultFragment
- FederatedAggregateDescriptor
- ReachabilityHint
- RouteAdvertisement
- SessionHello
- SessionAccept
- RelayEnvelope

External service protocols keep their standards-defined encodings. HTTP, DNS, TLS, QUIC, HPACK, QPACK, DHCP, and NFC are not Edgerun internal wire protocols just because Edgerun services speak them at their external boundaries.

Removed legacy byte paths MUST NOT be reintroduced as schema bridges, generated compatibility code, or alternate canonicalization paths. If a caller still depends on a removed path, migrate that caller to archive the concrete protocol type at the rkyv boundary or fail loudly.

---

## 17. Rkyv archive bytes and cryptographic inputs

### 17.1 Goals

The rkyv boundary makes these things unambiguous:
- what exactly is hashed
- what exactly is signed
- what exactly defines logical object identity
- what exactly defines stored representation identity
- how implementations reach the same bytes before hashing

This section is normative. Internal protocol bytes for hashing, signing, storage, transport payloads, caches, and local bridges are rkyv-family archive bytes of concrete native protocol records. There is no alternate Edgerun internal wire protocol.

### 17.2 Required algorithms in v0

Core v0 requires:
- hash / digest: **SHA-256**
- signature: **ECDSA P-256 with SHA-256**
- internal wire: **rkyv** archives of native protocol records

### 17.3 Identifier classes

#### Content-derived identifiers
Deterministic from rkyv signable or object bytes.

Examples:
- `event_hash`
- `command_hash`
- `delegation_hash`
- `revocation_hash`
- `object_id`
- `representation_digest`
- `representation_id` in the simple v0 profile

#### Opaque stable identifiers
Stable identifiers chosen by actor or implementation and not derived from full content.

Examples:
- `node_id`
- `stream_id`
- `command_id`
- `delegation_id`
- `revocation_id`
- `snapshot_id`
- `checkpoint_id`
- `aggregate_id`
- `action_instance_id`

### 17.4 Recommended identity identifier derivation

Recommended:

```text
identity_id = SHA256(
  "edgerun:v0:id:identity" || 0x00 ||
  rkyv_archive(IdentityRecord_signable_or_key_form)
)
```

Recommended, not universally mandatory, but once chosen for a deployment it should remain stable.

### 17.5 Rkyv archive encoding v0

The canonical internal bytes for a protocol record are the rkyv-family archive bytes of the concrete native type at the boundary. Implementations MUST use the protocol-native record shape for that family and version.

Used for:
- hashing
- signing
- object identity for structured logical objects
- event log record bodies
- internal transport payloads
- caches and local bridge payloads

### 17.6 Deterministic archive requirement

A conforming implementation MUST produce identical signable bytes for the same semantic record, family, and version. The archived byte shape is cryptographic semantics.

To preserve determinism:
- native record shape is versioned and cryptographically significant
- authority-bearing fields MUST NOT be reordered or reinterpreted within a version
- removed fields MUST fail at migration boundaries rather than being hidden by compatibility shims
- new semantics require a new record or family version when old readers cannot preserve meaning
- enum values use the native protocol enum mapping for the family version

### 17.7 Rkyv-normalized record

For any protocol record `M`, its internal byte form is:

```text
wire_bytes = rkyv_archive(normalize(M))
```

Where `normalize(M)` produces the concrete native record instance for the target family and version, with optional fields represented as the native absent value and repeated fields in their semantic order.

### 17.8 Signable form vs full form

For any signed record family:

#### Signable form
The protocol record with `signature` absent.

Used for:
- hashing for signature purposes
- record hash derivation
- stream previous-hash linkage

#### Full form
The protocol record with actual signature present.

Used when the signed record itself is stored, transported, or archived as a logical object.

### 17.9 Domain separation

Every cryptographic input uses an ASCII domain tag followed by a zero byte separator.

General form:

```text
domain_tag || 0x00 || payload_bytes
```

Examples:
- `edgerun:v0:hash:event-envelope`
- `edgerun:v0:hash:command-envelope`
- `edgerun:v0:hash:delegation-record`
- `edgerun:v0:hash:revocation-record`
- `edgerun:v0:hash:snapshot-descriptor`
- `edgerun:v0:hash:logical-object-descriptor`
- `edgerun:v0:hash:stored-representation-header`
- `edgerun:v0:hash:chunk-manifest`
- `edgerun:v0:object`
- `edgerun:v0:representation-bytes`
- `edgerun:v0:chunk-bytes`
- `edgerun:v0:sig:event-envelope`
- `edgerun:v0:sig:command-envelope`
- `edgerun:v0:sig:delegation-record`
- `edgerun:v0:sig:revocation-record`
- `edgerun:v0:sig:snapshot-descriptor`

### 17.10 Record hash derivation

For every signable family:

```text
record_hash = SHA256(
  hash_domain_tag || 0x00 || rkyv_archive(signable_form)
)
```

Applies to at minimum:
- EventEnvelope
- CommandEnvelope
- DelegationRecord
- RevocationRecord
- SnapshotDescriptor
- QueryResultFragment when signed
- RouteAdvertisement when signed
- SessionHello
- SessionAccept
- RelayEnvelope when signed
- IdentityRecord when signed
- AssuranceClaim when signed

### 17.11 Signature derivation

```text
sig_input = sig_domain_tag || 0x00 || record_hash_bytes
signature = ECDSA_P256_SHA256_sign(private_key, sig_input)
```

Verification:

```text
ECDSA_P256_SHA256_verify(public_key, sig_input, signature)
```

Hardware signers that require a fixed-size prehash sign SHA-256 of `sig_input` at the signer boundary, while preserving the same record hash and domain tags.

### 17.12 Family-specific hash meanings

- `event_hash = hash(EventEnvelope_signable_form)`
- `command_hash = hash(CommandEnvelope_signable_form)`
- `delegation_hash = hash(DelegationRecord_signable_form)`
- `revocation_hash = hash(RevocationRecord_signable_form)`

### 17.13 Logical object canonicalization classes

Every logical object declares a `canonicalization_id`. For structured Edgerun objects, the canonicalization id names the rkyv-native family and version.

v0 recognizes:

#### `rkyv-v0:<native_record_name>:<schema_version>`
Canonical bytes are `rkyv_archive(full_form_of_record)`.

#### `raw-bytes-v0`
Canonical bytes are exact raw bytes.

#### `utf8-nfc-v0`
Canonical bytes are UTF-8 after NFC normalization.

### 17.14 Logical object identity derivation

```text
object_id = SHA256(
  "edgerun:v0:object" || 0x00 ||
  utf8(canonicalization_id) || 0x00 ||
  canonical_bytes
)
```

### 17.15 Stored representation digest and identity

```text
representation_digest = SHA256(
  "edgerun:v0:representation-bytes" || 0x00 || stored_representation_bytes
)
```

For the simple v0 profile:
```text
representation_id = representation_digest.value
```

### 17.16 Chunk digest

```text
chunk_digest = SHA256(
  "edgerun:v0:chunk-bytes" || 0x00 || chunk_bytes
)
```

### 17.17 Event log framing

Durable event logs store full event wire bytes, not an alternate event schema. The hosted append-log profile frames each event as:

```text
varint(wire_len) || event_full_wire_bytes
```

The body is the current Edgerun event wire record. Index and stream-head hashes are computed from EventEnvelope signable bytes with the event-envelope hash domain.

### 17.18 Signature field handling in object identity

If a signable record is itself stored as a logical object:
- `record_hash` uses signable form with `signature` absent
- `object_id` uses full form with actual signature present

### 17.19 Native field evolution rule

Because archive bytes are cryptographic input:
- native record layout is versioned semantics
- authority-bearing fields are cryptographically significant
- incompatible changes require a new record or family version
- removed byte paths are migrated explicitly, not hidden behind compatibility adapters

### 17.20 Minimal family map

Signable families:
- `IdentityRecord`
- `AssuranceClaim`
- `DelegationRecord`
- `RevocationRecord`
- `EventEnvelope`
- `CommandEnvelope`
- `SnapshotDescriptor`
- `QueryResultFragment` when signed
- `FederatedAggregateDescriptor` when signed
- `RouteAdvertisement` when signed
- `SessionHello`
- `SessionAccept`
- `RelayEnvelope` when signed

Non-signable but rkyv-identifiable structured object families:
- `LogicalObjectDescriptor`
- `StoredRepresentationHeader`
- `ChunkManifest`
- structured payload messages stored as objects

### 17.21 Conformance requirement

A conforming implementation MUST be able to produce identical rkyv-family signable bytes for the same semantic record, family, and version as any other conforming implementation.

### 17.22 Test vectors

Before implementation is considered stable, the protocol SHOULD publish cross-implementation test vectors for at minimum:
- one EventEnvelope
- one CommandEnvelope
- one DelegationRecord
- one RevocationRecord
- one SnapshotDescriptor
- one raw-bytes-v0 object
- one chunked representation + manifest
- one signed SessionHello

Each test vector should include:
- semantic record content
- rkyv wire hex for signable and full forms where applicable
- record hash hex
- signature input hex
- signature hex
- expected object_id or representation_digest where applicable

---

## 18. Validation state machines

### 18.1 Common validation model

Validation outcomes:
- `ACCEPT`
- `REJECT`
- `DEFER`
- `DUPLICATE`

These are local and deterministic.

A node MUST treat `DEFER` as non-authoritative:
- no execution
- no stream append implying acceptance
- no trust installation
- no control change

`DEFER` is normal in a lossy, receiver-driven system. It often means "ask later", "fetch dependencies later", or "not enough local knowledge yet", not "the sender failed".

### 18.2 Validation layers

1. **Structural validation**
2. **Cryptographic validation**
3. **Referential validation**
4. **Semantic validation**
5. **Policy validation**

A record may be structurally and cryptographically valid while still failing semantic or policy validation.

### 18.3 Ingress screening rule

Not every received byte sequence deserves a durable rejection event.

A node **MAY** silently drop, rate-limit, or locally quarantine traffic that fails before protocol-level command recognition, including:
- malformed framing
- unsupported family
- impossible oneof shape
- missing target binding
- obviously broken signature container
- admission control failure before command parsing

Only requests that enter authoritative command handling are candidates for durable `command_rejected` or `command_committed`.

Ingress screening SHOULD be cheap. Especially on constrained or opportunistic transports, an implementation SHOULD separate:
- cheap control-plane acceptance ("am I willing to hear more about this?")
- deeper protocol acceptance ("do I accept this artifact into validated protocol state?")

Receiving, buffering, or requesting a body is not the same as accepting the represented protocol fact.

Recommended cheap-first screening order:
1. size and framing sanity
2. target relevance
3. duplicate or already-known hash/head check
4. local interest check
5. only then deeper signature, decryption, and authority work if still relevant

### 18.4 Stream append validation

Inputs:
- local stream context
- candidate EventEnvelope
- stream writer identity
- local head
- optional referenced payload/metadata objects

States:
- `UNSEEN`
- `STRUCTURAL_CHECK`
- `CRYPTO_CHECK`
- `POSITION_CHECK`
- `FAMILY_CHECK`
- `APPEND_READY`
- `APPENDED`
- `REJECTED`
- `DEFERRED`

Algorithm summary:
1. Check required fields and genesis/prev-hash shape.
2. Derive writer identity and verify signature.
3. Check stream position:
   - exact duplicate => `DUPLICATE`
   - wrong hash at same seq => `REJECT`
   - missing predecessor => `DEFER`
4. Apply event-family rules.
5. Append atomically with head update.

Envelope validity does not require all referenced payload objects to be immediately available for every mirrored stream, but unresolved references must not be used to make authority decisions.

### 18.5 Delegation chain validation

Question answered:
**Does this issuer currently have the bounded authority required for this requested action on this target, under local knowledge and local policy?**

States:
- `START`
- `DIRECT_AUTHORITY_CHECK`
- `CHAIN_PARSE`
- `CHAIN_CRYPTO_CHECK`
- `CHAIN_CONTINUITY_CHECK`
- `CHAIN_ATTENUATION_CHECK`
- `CHAIN_REVOCATION_CHECK`
- `ROOT_TRUST_CHECK`
- `EFFECTIVE_CAPABILITY_CHECK`
- `AUTHORIZED`
- `UNAUTHORIZED`
- `DEFERRED`

Key rules:
- direct authority may satisfy validation without a delegation chain
- every delegation signature must verify
- issuer/recipient continuity must hold across the chain
- no child may expand privilege relative to parent
- revocations are applied from local knowledge
- root issuer must be acceptable under local trust policy
- requested action, scope, timing, and assurance must fit the effective capability

### 18.6 Command acceptance validation

Inputs:
- local node identity
- local control state projection
- local policy
- replay cache / command history
- incoming CommandEnvelope
- optional transport/session metadata
- local assurance state

States:
- `INGRESS`
- `STRUCTURAL_CHECK`
- `CRYPTO_CHECK`
- `TARGET_CHECK`
- `TIME_CHECK`
- `REPLAY_CHECK`
- `AUTHORITY_CHECK`
- `POLICY_CHECK`
- `DECISION_PREPARE`
- `REJECT_RECORD`
- `COMMIT_RECORD`
- `EXECUTION_PENDING`
- `EXECUTING`
- `DONE`

Core rules:
- target node must match local node
- timing bounds must be respected
- same `command_hash` => `DUPLICATE`
- `command_id` is an application-level idempotency hint only and MUST NOT be used as the replay key
- different `command_hash` => distinct command, regardless of `command_id` value
- authority must validate before policy can approve
- node MUST decide before execution
- node MUST NOT execute before `command_committed` is durable
- transport delivery, relay carriage, or successful parsing never count as acceptance

### 18.7 Control transfer validation

Specialized validator for durable node control state changes.

States:
- `START`
- `CURRENT_POLICY_LOAD`
- `AUTHORIZATION_CHECK`
- `TARGET_IDENTITY_CHECK`
- `POST_STATE_SIMULATION`
- `OPTIONAL_PROOF_OF_POSSESSION`
- `CONTROL_CHANGE_COMMIT`
- `DONE`
- `REJECTED`

Key rules:
- load current authoritative controller-set projection
- verify command under current control policy
- validate target controller identity and required assurance
- simulate post-state before commit
- preferred safe transfer:
  1. add new controller
  2. verify possession/functionality
  3. remove old controller
- control change becomes authoritative only when committed in the node’s own stream

### 18.8 Snapshot acceptance and delta application

Inputs:
- SnapshotDescriptor
- optional snapshot payload object
- local trust policy for snapshot producers
- local knowledge of referenced stream heads/checkpoints
- optional local view
- optional delta events after snapshot base

States:
- `DESCRIPTOR_CHECK`
- `PRODUCER_TRUST_CHECK`
- `BASE_CHECK`
- `PAYLOAD_FETCH`
- `PAYLOAD_VALIDATE`
- `SNAPSHOT_ACCEPTED`
- `DELTA_REQUEST`
- `DELTA_VALIDATE`
- `VIEW_UPDATE`
- `DONE`
- `REJECTED`
- `DEFERRED`

Acceptance classes:
- `accepted_trusted`
- `accepted_stale`
- `accepted_cache_only`
- `deferred_missing_payload`
- `rejected_incompatible`

### 18.9 Object retrieval and representation validation

Inputs:
- expected ObjectRef or RepresentationRef
- optional LogicalObjectDescriptor
- optional StoredRepresentationHeader
- optional ChunkManifest
- representation bytes or chunk set
- local authorization/access state

States:
- `RESOLVE_TARGET`
- `FETCH_HEADER`
- `FETCH_MANIFEST`
- `FETCH_CHUNKS_OR_BYTES`
- `VERIFY_REPRESENTATION`
- `ACCESS_CHECK`
- `TRANSFORM_REVERSE`
- `VERIFY_LOGICAL_OBJECT`
- `STORE_OR_PROMOTE`
- `DONE`
- `REJECTED`
- `DEFERRED`

Acceptance levels:
- `representation_valid_only`
- `logical_object_valid`
- `deferred_missing_chunks`
- `deferred_missing_access`
- `rejected_integrity`
- `rejected_canonicalization`

A storage or relay node may succeed at `representation_valid_only` without being authorized to decrypt or realize the logical object.

### 18.10 Determinism rule

Given the same:
- local accepted history
- local trusted identity records
- local revocation knowledge
- local policy
- same candidate input

a conforming implementation **SHOULD** reach the same validation result.

### 18.11 Deterministic vs policy vs advisory outcomes

To reduce ambiguity, implementations MUST classify each decision they make as one of:
- **deterministic core outcome** — rkyv byte validation, signature verification, sequence validation, replay classification, exact target matching, required field presence, required proof-family matching
- **bounded local policy outcome** — trusted roots, acceptable attesters, assurance thresholds, proof payload allow-lists, resource ceilings, revocation authority allow-lists
- **implementation-defined advisory outcome** — route ranking, cache eviction, preferred transport selection, ranking of multiple equally valid advisory answers

Implementation-defined advisory logic MUST NOT silently alter:
- identity binding
- authority acceptance
- object integrity
- stream integrity
- replay semantics

---

## 19. Conformance suite and reference skeleton

### 19.1 Purpose

The conformance suite proves that independent implementations produce the same:
- rkyv signable and full bytes
- hashes
- signatures
- validation outcomes
- state transitions

The reference skeleton separates:
- pure protocol logic
- storage adapters
- transport adapters
- projection/query code

### 19.2 Repo shape

```text
edgerun/
  spec/
    core-protocol-v0.md
    rkyv-wire-v0.md
    validation-machines-v0.md

  crates/protocol/
    edgerun-wire/
    edgerun-core/src/protocol_native/
    edgerun-core/src/wire_stream.rs

  vectors/
    rkyv-wire/
    stream/
    command/
    delegation/
    control/
    snapshot/
    object/
    query/
    network/

  core/
    rkyv-wire/
    crypto/
    ids/
    stream/
    trust/
    command/
    object/
    snapshot/
    query/
    network/

  adapters/
    store/
    transport/
    clock/
    kv/
    object_store/

  tests/
    conformance/
    property/
    integration/
```

### 19.3 Conformance corpus format

One directory per case:

```text
vectors/stream/genesis-basic/
  manifest.json
  semantic_input.json
  signable_wire.hex
  full_wire.hex
  record_hash.hex
  signature_input.hex
  public_key.hex
  signature.hex
  expected.json
```

### 19.4 Required conformance suites

Mandatory categories:

1. Rkyv wire bytes
2. Crypto
3. Stream
4. Delegation
5. Command acceptance
6. Control
7. Snapshot
8. Object
9. Query
10. Network
11. Trust

### 19.4.1 Minimum interoperable profile

A minimal conforming v0 implementation MUST support:
- rkyv-only internal wire bytes
- v0 hash and signature derivation rules
- stream append validation
- delegation chain validation
- command acceptance validation
- control change validation
- snapshot acceptance validation
- object retrieval / representation validation
- query authorization and responder-local fragment handling
- basic session hello / accept handling
- trust validation for assurance claims and revocation records

The following families MAY be implemented as advisory/extension behavior in v0 without being required for the minimum interoperable profile:
- federated aggregate ranking policies
- route trust scoring heuristics
- session migration heuristics across transports
- storage tier promotion/eviction heuristics

### 19.4.2 Required vector focus areas

Before v0 is considered stable, the conformance corpus SHOULD include adversarial vectors for at minimum:
- assurance threshold edges
- unacceptable attesters
- future-effective revocations
- revocation target mismatches
- query fragments lacking backing references
- proof bundles with disallowed payload families
- route advertisements that are advisory but non-authoritative
- stale snapshots with and without enough deltas

### 19.5 Validation outcome contract

Every validator should return one of:
- `ACCEPT`
- `REJECT`
- `DEFER`
- `DUPLICATE`

And a structured reason such as:
- `STRUCTURAL_INVALID`
- `CRYPTO_INVALID`
- `VERSION_UNSUPPORTED`
- `TARGET_MISMATCH`
- `REPLAY_DETECTED`
- `AUTHORITY_DENIED`
- `POLICY_DENIED`
- `MISSING_DEPENDENCY`
- `FORK_CONFLICT`
- `CANONICALIZATION_FAILED`
- `OBJECT_ID_MISMATCH`
- `SNAPSHOT_BASE_CONFLICT`

### 19.6 Pure core interfaces

Reference pure functions:

```text
signable_wire_bytes(record) -> bytes
full_wire_bytes(record) -> bytes

hash_identity_record(record) -> Digest
hash_event_envelope(event) -> Digest
hash_command_envelope(command) -> Digest
hash_delegation_record(delegation) -> Digest
hash_revocation_record(revocation) -> Digest

sign_record(record, private_key) -> Signature
verify_record_signature(record, public_key) -> bool

derive_object_id(canonicalization_id, canonical_bytes) -> bytes
derive_representation_digest(stored_bytes) -> Digest
derive_chunk_digest(chunk_bytes) -> Digest
```

Reference validators:

```text
validate_stream_append(ctx, event) -> ValidationResult
validate_delegation_chain(ctx, issuer, action, scope, chain) -> AuthorityResult
validate_command(ctx, command) -> CommandDecisionResult
validate_control_change(ctx, command) -> ControlDecisionResult
validate_snapshot(ctx, snapshot) -> SnapshotAcceptanceResult
validate_object_retrieval(ctx, target, representation) -> ObjectValidationResult
```

Reference projectors:

```text
project_control_state(events) -> ControlState
project_stream_head(events) -> Head
project_view_from_snapshot_and_deltas(snapshot, deltas) -> View
```

### 19.7 Core/adapters boundary

The core must not know about:
- BLE
- QUIC
- RocksDB
- S3
- filesystem layout
- UI
- daemon lifecycle

The core only knows abstract interfaces:
- IdentityStore
- TrustStore
- StreamStore
- ObjectStore
- SnapshotStore
- Clock
- Randomness
- TransportSessionMetadata

### 19.8 Reference node skeleton

Modules:
- `canonical`
- `crypto`
- `ids`
- `stream`
- `trust`
- `command`
- `object`
- `snapshot`
- `query`
- `network`
- `projections`

### 19.9 Minimal storage interfaces

```text
put_event(stream_id, seq, event, event_hash)
get_event(stream_id, seq) -> event?
get_head(stream_id) -> head?
compare_and_set_head(stream_id, expected_head, new_head) -> bool

put_object_descriptor(object_id, descriptor)
get_object_descriptor(object_id) -> descriptor?

put_representation(representation_id, bytes, header)
get_representation(representation_id) -> bytes/header?

put_snapshot(snapshot_id, descriptor)
get_snapshot(snapshot_id) -> descriptor?
```

Stream append requires atomicity between:
- storing the event
- updating the head

### 19.9.1 Reference storage mapping

Recommended reference mapping:

- immutable records and event-log material -> durable append-only record store
- encrypted blob ciphertext -> filesystem blob store
- local lookup/index state -> binary append-only file indexes

Recommended file-index responsibilities:

- stream head index
- stream seq/hash lookup
- object and representation presence index
- replay cache
- validation result cache
- fetch queue and missing-dependency tracking

Recommended filesystem blob responsibilities:

- store ciphertext bytes only
- store by stable blob identifier or equivalent content-addressed path
- never persist plaintext blob bodies in the reference profile

Implementations may choose another index backend, but they should preserve the same rebuildability and confidentiality invariants.

### 19.10 Replay cache semantics

Recommended replay key space:

```text
command_hash -> (command_id, decision_event_ref)
```

Where `command_hash` is the SHA-256 hash of the canonical command envelope (signable form).

Rules:
- same command_hash => `DUPLICATE`
- `command_id` is an application-level idempotency hint only, not the replay key
- exact committed prior command may return prior acknowledgement
- no re-execution of side effects unless explicitly defined idempotent

### 19.11 First milestone

The first milestone should include:
- one node stream
- rkyv wire-byte engine
- hash/signature engine
- event append validator
- delegation validator
- command validator
- object id / representation validation
- one snapshot descriptor validator
- conformance runner

### 19.12 Definition of done for v0 core

The protocol core can be considered stable when:
- two independent implementations produce identical rkyv signable bytes for the same vectors
- they derive identical hashes and signatures
- all mandatory conformance cases pass
- validators produce identical outcomes for the same local state and inputs
- stream append behavior is atomic and deterministic
- control transfer obeys add-verify-remove safely
- snapshot acceptance classes behave identically across implementations
- object validation distinguishes representation-valid from logical-object-valid

---

## Appendix A. Global invariants

- A stream has exactly one writer.
- A stream’s writer is fixed for the stream lifetime.
- Every stream begins with exactly one genesis event.
- Every non-genesis event links to the immediately previous event by canonical hash.
- Sequence numbers are contiguous and strictly increasing.
- Every event is signed by the stream writer.
- A command is not authoritative until the target node commits its decision in its own stream.
- Persistent changes to node control are authoritative only when committed by that node.
- Delegation chains must attenuate privilege, not expand it.
- Objects are immutable.
- Logical object identity is distinct from stored representation identity.
- Snapshots are derived and never override stream authority.
- Reachability never implies trust.
- Routing never implies authority.
- Revocation is knowledge-scoped and not retroactive by default.

## Appendix B. Canonical message families

- IdentityRecord
- NodeBootstrapRecord / NodeGenesisPayload
- AssuranceClaim
- DelegationRecord
- RevocationRecord
- EventEnvelope
- NodeGenesisEvent
- CommandCommittedEvent
- CommandRejectedEvent
- ActionStartedEvent
- ActionCompletedEvent
- ActionFailedEvent
- CommandSentEvent
- CommandEnvelope
- LogicalObjectDescriptor
- StoredRepresentationHeader
- ChunkManifest
- SnapshotDescriptor
- IndexDescriptor
- ProofDescriptor
- QueryRequest
- QueryResultFragment
- FederatedAggregateDescriptor
- ReachabilityHint
- RouteAdvertisement
- SessionHello
- SessionAccept
- RelayEnvelope

## Appendix C. Core state machines

Node stream lifecycle:
- uninitialized → bootstrapping → active → revoked or retired

Command handling lifecycle:
- created → transmitted → received → validated → rejected or committed → optionally started → completed or failed

Delegation lifecycle:
- issued → active → expired or revoked

Snapshot lifecycle:
- planned → materialized → signed → published → superseded

Object lifecycle:
- created → packaged → stored → replicated → retrieved → promoted or evicted

Session lifecycle:
- discovered → connecting → authenticated → active → migrated or closed

Query lifecycle:
- constructed → authorized by responder → answered with fragment → aggregated or discarded

## Appendix D. Term normalization

Use `seq`, not nonce, for stream ordering.

Use `writer_identity`, not event author, as the stream-level source of authorship.

Use `issuer_identity` for the principal that creates a command or delegation.

Use `controller` only for identities that a node recognizes as able to influence that node.

Use `capability` for bounded authority and `delegation` for signed transfer of that authority.

Use `logical object` for immutable semantic data and `stored representation` for encrypted or chunked storage packaging.

Use `snapshot` for derived immutable view objects and `view` for the logical perspective they represent.

Use `locator` for transport-specific contact information and `identity` for the stable endpoint being addressed.

Use `replication` for durability movement, `access` for obtaining useful information, and `sync` only if later defined as a specific derived flow.

## Appendix E. Native rkyv record layout

The internal protocol records live as native Rust types in `crates/protocol/edgerun-core/src/protocol_native/`. Generated files under `protocol_native/gen/` are generated by the rkyv-native pipeline and are part of the native protocol shape, not a legacy wire-schema bridge.

Current native families include:

- `edgerun.v0.access`
- `edgerun.v0.app`
- `edgerun.v0.appabi`
- `edgerun.v0.capability`
- `edgerun.v0.capability_runtime`
- `edgerun.v0.common`
- `edgerun.v0.identity`
- `edgerun.v0.network`
- `edgerun.v0.object`
- `edgerun.v0.server_resources`
- `edgerun.v0.stream`
- `edgerun.v0.trust`
- `edgerun.v0.ui`
- `edgerun.wallet.v0`

The public `ProtocolRecord` enum identifies records that participate directly in shared hashing and signing helpers, including command envelopes, event envelopes, command results, delegation records, revocation records, identity records, route advertisements, assurance claims, snapshot descriptors, object refs, digests, and signatures.

The wire boundary is exported by `edgerun-wire`; its `WIRE_PROTOCOL` value is `rkyv`. Any additional local framing, such as varint event-log length prefixes, frames rkyv-family protocol bytes and does not define a second protocol.

## Appendix F. Reference repository layout

```text
edgerun/
  spec/
    core-protocol-v0.md
    rkyv-wire-v0.md
    validation-machines-v0.md

  crates/protocol/
    edgerun-wire/
    edgerun-core/src/protocol_native/
    edgerun-core/src/wire_stream.rs

  vectors/
    rkyv-wire/
    stream/
    command/
    delegation/
    control/
    snapshot/
    object/
    query/
    network/

  core/
    rkyv-wire/
    crypto/
    ids/
    stream/
    trust/
    command/
    object/
    snapshot/
    query/
    network/

  adapters/
    store/
    transport/
    clock/
    kv/
    object_store/

  tests/
    conformance/
    property/
    integration/
```

Recommended minimal core interfaces:

```text
signable_wire_bytes(record) -> bytes
full_wire_bytes(record) -> bytes

hash_identity_record(record) -> Digest
hash_event_envelope(event) -> Digest
hash_command_envelope(command) -> Digest
hash_delegation_record(delegation) -> Digest
hash_revocation_record(revocation) -> Digest

sign_record(record, private_key) -> Signature
verify_record_signature(record, public_key) -> bool

derive_object_id(canonicalization_id, canonical_bytes) -> bytes
derive_representation_digest(stored_bytes) -> Digest
derive_chunk_digest(chunk_bytes) -> Digest

validate_stream_append(ctx, event) -> ValidationResult
validate_delegation_chain(ctx, issuer, action, scope, chain) -> AuthorityResult
validate_command(ctx, command) -> CommandDecisionResult
validate_control_change(ctx, command) -> ControlDecisionResult
validate_snapshot(ctx, snapshot) -> SnapshotAcceptanceResult
validate_object_retrieval(ctx, target, representation) -> ObjectValidationResult
```

Definition of done for core v0:
- two independent implementations produce identical rkyv signable bytes for the same vectors
- they derive identical hashes and signatures
- mandatory conformance cases pass
- validators produce identical outcomes for the same local state and inputs
- stream append behavior is atomic and deterministic
- control transfer obeys add-verify-remove safely
- snapshot acceptance classes match across implementations
- object validation distinguishes representation-valid from logical-object-valid

---

End of single-file specification.
