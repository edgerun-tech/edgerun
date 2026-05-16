# EdgeRun Browser Core Workspace

This is the extraction staging workspace for the browser-first coherent slice.
It intentionally builds only the surfaces needed for:

- rkyv wire records;
- browser app package verification and first-run projection;
- runtime package/cache/app-run records;
- memory/browser storage capability providers;
- session-bound `CapabilityEnvelope` storage object invocation.
- a byte-oriented browser host boundary that accepts and returns rkyv records.
- raw WASM-callable host entry points with explicit input/output buffer
  ownership.
- a dependency-free browser JS adapter that loads the WASM host, copies rkyv
  bytes through the raw ABI, frees owned output buffers, and stores package,
  object, and host-visible record bytes in IndexedDB.
- a smoke harness that reads deterministic rkyv fixture bytes from Rust WASM
  exports and drives the host adapter through first-run, grant, storage
  binding, session, and storage invocation.
- a real browser first-run byte adapter that passes package bytes and a
  `BrowserAppFirstRunInput` rkyv record into Rust, receives a
  `BrowserAppFirstRunProjection` rkyv bundle, and records that bundle through
  the host without JS parsing package or runtime records.

Five Rust surfaces have now been physically lifted/pruned into this workspace:

- `edgerun-browser-wire` preserves the `edgerun_wire` crate API while giving the
  staged workspace its own rkyv-only wire boundary package.
- `edgerun-browser-runtime` replaces the broad `edgerun-node/node-core`
  dependency for this slice and keeps only browser runtime state, storage
  provider traits, first-run records, grants, sessions, storage envelopes, and
  audit chain verification.
- `edgerun-browser-authoring` replaces the broad `edgerun-sdk` dependency for
  package authoring, package verification, developer signatures, and first-run
  projection records. It now emits the browser first-run input/projection and
  package retrieval wire records used by the JS bridge.
- `edgerun-browser-work` preserves the `edgerun_work` crate API for the staged
  admission, ordered channel envelope, recipient channel proof, capability
  envelope, storage availability/retrieval proof, signing preimage, verifier,
  and BLAKE3 hash subset needed by browser storage invocation.
- `edgerun-browser-host` is the first browser host boundary. It keeps runtime
  state in Rust, accepts `SdkWireRecord` bytes for first-run projection bundles,
  first-run parts, grants, storage binding, sessions, and capability requests,
  and returns
  `BrowserHostResultRecord` bytes plus proof hashes. It also exports the raw
  host handle, allocation/free, first-run, grant, storage binding, session, and
  storage invocation entry points used by a browser WASM host.

The `browser/host-adapter.mjs` module is the first browser JS adapter around
`edgerun-browser-host`. It is intentionally a byte bridge: callers provide
already-serialized rkyv records, the adapter allocates/copies them into WASM,
copies returned `BrowserHostResultRecord` bytes back out, frees WASM-owned
buffers, and offers an IndexedDB byte store for packages, objects, and rkyv
records. Do not copy the broader protocol crate until a caller proves each
record is part of the browser-first path.

The `browser/first-run.mjs` module is the first real browser first-run adapter.
It copies package manifest bytes, app graph bytes, developer signature bytes,
and a serialized `BrowserAppFirstRunInput` record into Rust. Rust verifies the
package and returns one serialized `BrowserAppFirstRunProjection` bundle, which
the host records directly via `recordFirstRunProjection`. JS does not decode or
rebuild app/package/runtime records.

The `browser/package-retrieval.mjs` module is the browser retrieval adapter. It
copies retrieved package bytes, package key, retrieval cost, and request time
into Rust when the browser does not already have retrieval evidence. Rust
verifies the developer signature and package hashes, signs and verifies a
`WorkRequest`, admits it through a signed `WorkAdmission`, and returns a
serialized `BrowserPackageRetrievalEvidenceRecord` plus the browser admission
hash. It then builds the serialized `BrowserPackageRetrievalRecord` from the
same verified package bytes and admitted evidence. JS stores those opaque rkyv
records and does not parse them.

Admission is boundary-local. Browser package retrieval crosses the browser node
boundary and must use a browser admission node. Host capability/storage
invocation crosses the host boundary and must use a host admission node. The
browser admission hash that proves package retrieval is not the host admission
hash that opens or spends a host capability session.

Each boundary must have at least one admission node and one relay node. Node
inbound and outbound traffic crosses relay nodes regardless of node type. A
relay node is controlled by exactly one admission node; one admission node may
control many relay nodes; relays under the same admission node may connect
directly to each other. Relays only forward traffic approved by their controlling
admission node, except that they may forward an external admission node's
request to the controlling admission node for work admission.

The `browser/relay-node.mjs` module is the first JS relay participant. It is not
an authority source and does not verify package semantics. It forwards admission
request envelopes to its single controlling WASM admission node and rejects
requests for any other controller. The WASM admission node remains responsible
for package verification, signed `WorkRequest` handling, signed `WorkAdmission`
creation, and retrieval evidence.

Boundary authority is not machine-local. Browser and host nodes on the same
machine can communicate only through their admitted relay paths, and sharing a
browser tab, process, loopback address, or device does not grant authority over a
boundary. One tab may host many boundaries, and one boundary may span many
machines, as long as traffic still enters through that boundary's relay and
admission topology.

The `browser/smoke-harness.mjs` module is the first executable browser harness.
It reads real package, grant, session, and storage rkyv bytes produced by Rust
authoring/runtime helpers through the `browser-core-slice` WASM exports. It
can also accept caller-provided package bytes for a selected network app. It
builds admitted retrieval evidence, the retrieval record, and user first-run
input through Rust, then drives `browser/package-retrieval.mjs`,
`browser/first-run.mjs`, and `browser/host-adapter.mjs` against a byte store.

The `browser/index.html` page is the first no-bundler browser entry. It loads
`browser/page.mjs`, opens `EdgeRunBrowserByteStore`, runs the browser-core
harness against `browser/dist/browser_core_slice.wasm`, passes the selected
Run once / Verify & cache / Cancel decision into the Rust first-run input
builder, forwards optional selected package bytes to the harness, and surfaces
the returned first-run projection and host result byte lengths without parsing
rkyv in JS.

The `browser/package-source.mjs` module is the selected network app package
source. It maps a selected package key to manifest, graph, and developer
signature byte records in the browser byte store and returns those opaque bytes
to the same `browser/first-run.mjs` path. It also persists admitted retrieval
results by storing the package parts plus optional retrieval, proof, and source
admission evidence bytes. It also stores browser-admission and
browser-work-admission aliases so browser boundary proof can be separated from
future host admission proof. When no retrieval evidence or retrieval record is
provided, the harness asks Rust to build browser-admission-bound evidence and the
retrieval rkyv record before storing them. JS does not parse rkyv records.

The `browser/retrieval-flow.mjs` module owns the browser-side retrieval
assembly. Given retrieved package bytes, a selected package key, and a byte
store, it asks Rust for missing admission-bound retrieval evidence, source
admission hash, and retrieval record bytes, then persists only opaque package and
rkyv record bytes. The returned object also exposes `browserAdmissionHash` as
the canonical browser-boundary name while keeping `sourceAdmissionHash` as a
compatibility alias. The smoke harness calls this adapter instead of carrying
retrieval policy inline.

The `browser/network-retrieval.mjs` module is the first browser node retrieval
entry point. It fetches manifest, graph, and developer-signature bytes from
caller-provided package part URLs, asks Rust to serialize or apply a
`BrowserPackageRetrievalPolicyScheduleRecord` when no explicit retrieval cost is
supplied, and then hands off to `browser/retrieval-flow.mjs` so
admission-bound evidence and package records are still produced by Rust and
stored as opaque bytes. The schedule bytes are also stored with retrieval
evidence, and Rust binds the hash of the validated schedule record bytes into
the signed `WorkAdmission` and `BrowserPackageRetrievalEvidenceRecord`.
The retrieval entry point can also call a supplied browser boundary. In that
path, a JS relay node forwards the browser admission request to its single
controlling WASM admission node. The request carries an explicit boundary id so
multiple browser boundaries can coexist in one tab. When the WASM admission node
returns retrieval evidence, browser admission hash, and signed browser
`WorkAdmission` packet bytes, the browser stores those bytes and skips the
deterministic local evidence fixture.

The next extraction should replace the deterministic local admission fixture
with a concrete signed-`WorkRequest` submission path so the JS relay forwards
opaque admitted work bytes to its controlling WASM admission instead of a local
retrieval request object. Host capability/storage work should get a separate
host relay plus host admission path and must not reuse browser retrieval
admission.

Run:

```bash
./cargo test -p browser-core-slice
./cargo build -p browser-core-slice --target wasm32-unknown-unknown --release --crate-type rlib --crate-type cdylib --out workspaces/browser-core/browser/dist
./cargo tree -p browser-core-slice
node --test workspaces/browser-core/browser/*.test.mjs
```

The smoke crate is also registered in the root workspace while the extraction
uses monorepo path dependencies; the local cargo shim materializes one workspace
root for builds, so root-level invocation is the currently supported check.

Current measured dependency list for the smoke slice is 25 packages. It no
longer includes the monorepo `edgerun-wire`, `edgerun-work`, `edgerun-node`,
`edgerun-sdk`, `edgerun-storage`, device crates, mesh, hardware signing, Linux
adapters, virtual disk, or platform crates.
