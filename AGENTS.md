# Edgerun Core Agent Notes

The internal wire protocol is rkyv only. Do not add compatibility shims, schema
bridges, generated legacy wire code, or alternate canonicalization paths. If a
caller still depends on a removed byte path, migrate that caller to archive the
concrete protocol type at the rkyv boundary or leave a loud breakpoint.

## What To Read First

1. `README.md`
2. `crates/protocol/edgerun-wire/src/lib.rs`
3. `crates/protocol/edgerun-core/src/protocol_native/mod.rs`
4. `crates/protocol/edgerun-core/src/{crypto.rs,command.rs,validators/*.rs}`
5. `crates/authority/edgerun-stream/src/lib.rs`
6. `crates/authority/edgerun-storage/src/{lib.rs,core,store.rs,fs,event_log.rs,file_index.rs,blobs.rs}`
7. `crates/node/edgerun-node/src/lib.rs`
8. Mesh and capability crates: `edgerun-mesh*`, `edgerun-remote-capability`,
   `edgerun-capabilities`, `edgerun-capability-policy`

## Protocol Invariants To Preserve

- Never treat command delivery as authority. A command matters only after the
  target node validates it and records a `COMMAND_COMMITTED` or
  `COMMAND_REJECTED` event in its stream.
- Never mutate authoritative state outside the event log. Indexes, snapshots,
  route hints, query results, and caches are derived.
- Stream append must be contiguous by `seq`, hash-linked by previous event
  hash, and signed by the fixed writer identity.
- Object identity and stored representation identity are different.
- Delegation must attenuate: child delegations cannot expand parent actions,
  scope, timing, assurance, or constraints.
- Route and query artifacts can be advisory; accepting them does not install
  trust roots, controller authority, or stream authority.

## Wire Protocol Rule

- Use rkyv for hashing, signing, storage, transport payloads, caches, and local
  bridge payloads.
- External service protocols such as HTTP, DNS, TLS, QUIC, HPACK, QPACK, DHCP,
  and NFC keep their own standards-defined encodings. They are not Edgerun
  internal wire protocols.
- Do not reintroduce rkyv files or alternate canonical encoders.
- Do not hide breakage with compatibility adapters. Break loudly, then migrate
  the caller to rkyv.

## Targeted Checks

Prefer package-level checks while editing a specific area:

```bash
cargo test -p edgerun-core
cargo test -p edgerun-stream
cargo test -p edgerun-storage
cargo test -p edgerun-node
```

Bare/unikernel build:

```bash
cargo +nightly build --release -p edgerun-unikernel \
  --target x86_64-unknown-none \
  -Zbuild-std=core,alloc
```
