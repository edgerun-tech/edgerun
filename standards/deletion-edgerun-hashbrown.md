# edgerun-hashbrown deletion

`crates/utility/edgerun-hashbrown` was inspected from source before deletion.

Useful behavior found:

- SwissTable open-addressed hash table internals;
- top-seven-bit hash control tags;
- empty/deleted/full control-byte scans;
- SIMD and word-group table probing;
- triangular probing over power-of-two bucket groups;
- resize, tombstone, allocator-layout, and raw-entry machinery.

Behavior not extracted to WAT:

- generic allocator-backed `HashMap` / `HashSet` implementation;
- Rust collection API compatibility;
- rayon and external trait impls;
- raw table and raw-entry APIs;
- default hasher compatibility.

Reason:

This is collection machinery, not a portable wire/protocol/codec behavior. A
WAT hash table would preserve Rust runtime internals in another runtime instead
of extracting a reusable standard primitive.

Deletion action:

- delete `crates/utility/edgerun-hashbrown`;
- remove root workspace member, workspace dependency, and crates.io patch;
- remove stale lockfile package/dependency entries;
- delete `rkyv` archived-hashbrown compatibility impls;
- replace internal `rkyv` sharing/pooling/validation maps with
  `alloc::collections::BTreeMap` so they no longer depend on the deleted crate.

Remaining `hashbrown-0_17` feature text in `rkyv` is inert compatibility naming.
It no longer enables a dependency.
