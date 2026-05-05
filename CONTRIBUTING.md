# Contributing

Edgerun Core is a protocol-first Rust workspace. Before changing core behavior,
preserve the rkyv wire boundary and the protocol invariants documented in
`AGENTS.md` and `README.md`.

## License

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this repository is licensed under `MIT OR Apache-2.0`, without
additional terms or conditions.

Do not add third-party code, generated files, protocol extracts, firmware blobs,
or test corpora unless their license is compatible with this repository and the
source is recorded in the relevant README, NOTICE, or crate metadata.

## Change Expectations

- Keep authoritative state changes behind signed event-log semantics.
- Treat indexes, route hints, query results, snapshots, and caches as derived.
- Mark documentation claims as implemented, protocol/design requirement,
  generated type/catalog material, host-only, bare-target/stubbed, or blocked.
- Prefer package-level checks while editing a focused area.

Useful checks:

```bash
cargo metadata --no-deps --format-version 1
cargo deny check licenses
cargo test -p edgerun-core
cargo test -p edgerun-stream
cargo test -p edgerun-storage
cargo test -p edgerun-node
```
