# App Runtime Invariants

This note records the app/runtime model that should guide the WAT app SDK,
object format, storage backend, and host runtime.

## Identity

An app identity is the hash of its canonical WASM bytes:

```text
app_id = hash(canonical_wasm_bytes)
```

The app manifest is embedded in those canonical bytes. Changing the manifest
changes the app identity. This includes capabilities, minimum memory, minimum
storage, storage policy, recursion policy, and release or developer mode.

Debuggability is authority. A release app and a developer app are therefore
different identities even when their source code is otherwise the same:

```text
same code + developer manifest -> developer app_id
same code + release manifest   -> release app_id
```

Release app memory has no supported inspection path. Developer mode may expose
memory, transition buffers, render buffers, storage queues, and event logs, but
those receipts and objects are tied to the developer app identity and cannot
impersonate the release identity. Migration from developer identity to release
identity requires explicit user-signed migration or resealing.

## Resource Grants

App memory and app storage are preallocated resources. The manifest declares
the minimum needed to launch:

```text
memory.min_bytes
storage.min_bytes
memory.static = true
storage.direct_access = false
recursion = false
```

At launch, the user grants concrete memory and storage amounts:

```text
memory_grant_bytes >= manifest.memory.min_bytes
storage_grant_bytes >= manifest.storage.min_bytes
```

The user or kernel may modify grants while the app is running, but only at
runtime-defined transition boundaries. The app cannot grow memory, allocate more
storage, or mutate its resource grant by itself. It may only produce explicit
requests or fail with bounded errors such as memory exhausted, output buffer
full, or storage quota exhausted.

There is no dynamic allocation by the app, no `memory.grow`, no ambient heap
growth, and no recursion. Loops are bounded by fuel. Stack size is statically
bounded by validation.

## Memory

Memory is for active execution and IO workspace:

```text
input buffers
output buffers
render buffers
crypto workspace
transition construction
temporary decode and encode
```

The app receives a fixed arena for its current grant. The SDK should expose
typed regions or slices over that arena rather than a general allocator.

## Storage

Storage is app-owned quota, but storage IO is kernel-owned scheduling. Apps do
not directly read and write arbitrary files or blocks. An app transition emits
storage intents:

```text
append event
store object
store receipt
write checkpoint
update derived index
```

The kernel validates and queues those intents, then commits them on its own
schedule. This prevents apps from hogging device IO and keeps every durable
write explicit.

Storage is for useful durable results:

```text
sealed objects
public objects
append-only event logs
receipts
state checkpoints
sync material
user-approved durable outputs
```

Storage is not a hidden cache, speculative scratch space, temp file area, or
analytics sink. If cache storage exists, it must be separately declared,
bounded, evictable, and non-authoritative.

## Child Apps

Apps may spawn child apps. Child resources are moved out of the parent grant;
they are not shared or borrowed:

```text
parent.memory -= child.memory
parent.storage -= child.storage
child receives independent grants
```

The parent receives a child handle, not access to child internals. The handle
may allow message send, message receive, receipt wait, authority-limited
termination, additional resource transfer, and released-resource reclamation.
It does not allow reading child memory, reading child storage, inspecting child
private state, or mutating child state.

Developer mode does not pierce release identities. A developer parent spawning a
release child still receives only a handle unless the child itself is a
developer identity whose manifest permits inspection.

## Data Requirements

Data carries its own requirements. Privacy is not only an app-level property.
Each object records requirements such as:

```text
durability
confidentiality
portability
integrity
lifetime
visibility
access
```

The primary principals are:

```text
device
app
user
```

Each principal has authority and private data. Object data can be public,
integrity-only, or sealed to any combination of device, app, and user. The WAT
object model should favor a principal-set or envelope-list policy rather than
collapsing confidentiality into a small number of named cases. This keeps these
cases expressible:

```text
device
app
user
device + app
device + user
app + user
device + app + user
layered transfer envelopes
```

Apps should normally manipulate object references, requirements hashes, event
references, receipts, routing intents, and UI structure. Private user data is
not app business by default. Plaintext access is an explicit authority path
through the trusted input, reveal, sealing, and rendering pipeline.

## Execution

An app is a deterministic bounded transition machine:

```text
input message or action
current state root
app logical clock
fixed memory arena
bounded fuel
  -> transition
  -> render output
  -> queued storage intents
  -> emitted messages
  -> receipts
```

Apps do not get ambient filesystem access, raw sockets, DNS, ports, wall-clock
authority, host environment variables, or hidden IO. App IPC and networking are
identity-routed messages through the host relay model.

The default network model has no app-visible IP addresses, DNS names, listen
ports, or raw sockets. An app that accepts inbound connections does so by
requesting a hidden-service identity route. Outbound app messages are addressed
to identities and sealed according to object/message requirements. TLS may be
available as an explicit capability on an identity-routed path, but apps do not
receive raw TLS sockets, exported TLS keys, or authority from TLS alone.

The local runtime smoke path in
`standards/ports/wasm-tooling` treats JavaScript as harness glue
only. Tor cell bytes are built by `tor-cell-codec.wat`, hidden-service
fetch/publish/frame artifacts are built by `tor-library.wat`, and the relay
decodes/hashes WAT-owned hidden-service contact/message state from module
memory.

## Clocks

Each app has its own logical clock. The clock ticks when an accepted transition
commits:

```text
app_clock = previous_app_clock + 1
event_hash = hash(app_id, app_clock, previous_event_hash, transition)
```

Apps do not receive authoritative `now()`. Device time, user time, and external
time may appear as signed receipts, but wall time is evidence, not implicit
authority.

Ordering is scoped:

```text
within one app: strict app_clock order
across apps on one device: device receipts may order events
across devices: causal order through messages and receipts
```

## Append-Only Logs

Canonical storage is append-only hashed event logs plus object storage. State,
indexes, and query results are derived:

```text
events + objects + receipts -> state root
```

The event log is the source of truth. Checkpoints and indexes may accelerate
startup and queries, but they must be replayable from verified events, objects,
and receipts.

## Device Query Backend

ClickHouse or a similar analytical engine may be useful as a device-side
projection backend for fast queries, debugger views, receipt search, sync
planning, and state indexes. It must not be the cryptographic source of truth.

The authority remains:

```text
hash-chained event logs
sealed objects
signatures
seal policies
receipts
```

If a derived SQL projection disagrees with the verified log, the projection is
discarded or rebuilt.
