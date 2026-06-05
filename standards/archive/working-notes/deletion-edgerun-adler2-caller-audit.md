# edgerun-adler2 caller audit

This audit records retirement of `crates/utility/edgerun-adler2`. The useful
portable behavior is Adler-32 checksum calculation, represented by
`encoding-core.wat`. The crate source, metadata, root workspace member, and
root `adler2` patch entry are deleted.

## Current status

- No `crates/utility/edgerun-adler2` files remain.
- No live Rust caller should import `adler2`, `edgerun_adler2`, or `Adler32`.
- `edgerun-miniz-oxide`, the former direct dependent, is also deleted.
- Remaining references are intentional standards documentation, roadmap notes,
  and checksum terminology.

## Current after-scan

Required after-scan:

```bash
rg -n "edgerun_adler2|edgerun-adler2|\badler2\b|Adler32" crates Cargo.toml standards --glob '*.rs' --glob 'Cargo.toml' --glob '*.md'
```

Current result:

- no hits in `crates` Rust source;
- no hits in the root `Cargo.toml`;
- remaining hits are standards documentation, roadmap entries, checksum
  terminology, and this deletion audit.

## Historical pre-deletion counts

Commands run from the repository root:

```bash
rg -l "edgerun-adler2|edgerun_adler2|\badler2\b|Adler32|adler32" . --glob '!target/**' | wc -l
# 25

rg -l "edgerun-adler2|edgerun_adler2|\badler2\b|Adler32|adler32" crates/utility/edgerun-adler2 --glob '!target/**' | wc -l
# 5

rg -l "edgerun-adler2|edgerun_adler2|\badler2\b|Adler32|adler32" crates/utility/edgerun-miniz-oxide --glob '!target/**' | wc -l
# 8

rg -l "edgerun-adler2|edgerun_adler2|\badler2\b|Adler32|adler32" standards --glob '!target/**' | wc -l
# 10

rg -l "edgerun-adler2|\badler2\b" Cargo.toml Cargo.lock crates/utility/edgerun-miniz-oxide/Cargo.toml --glob '!target/**' | wc -l
# 3

rg -l "adler2::|use adler2|Adler32::from_checksum" crates --glob '*.rs' --glob '!target/**' | sort
# crates/utility/edgerun-adler2/src/lib.rs
# crates/utility/edgerun-miniz-oxide/src/shared.rs
```

Crate-local Rust source size:

```text
287 crates/utility/edgerun-adler2/src/lib.rs
155 crates/utility/edgerun-adler2/src/algo.rs
442 total
```

## Delete crate-local API

These files are crate-local implementation, docs, or release metadata. They do
not need to survive once the crate is tombstoned:

- `crates/utility/edgerun-adler2/src/lib.rs`
- `crates/utility/edgerun-adler2/src/algo.rs`
- `crates/utility/edgerun-adler2/CHANGELOG.md`
- `crates/utility/edgerun-adler2/README.md`
- `crates/utility/edgerun-adler2/Cargo.toml`

Useful behavior currently exposed by this API:

- `Adler32::new()` starts at checksum `0x00000001`.
- `Adler32::from_checksum(u32)` resumes from existing `a` and `b` lanes.
- `Hasher::write` incrementally updates checksum state.
- `adler32_slice(&[u8]) -> u32` calculates one-shot checksums.
- `adler32<R: BufRead>` is `std` reader glue and is not portable codec logic.

## Replace with WAT or in-runtime checksum

Canonical portable checksum behavior is already in:

- `standards/build/wasm/codec-primitives/encoding-core.wat`
- `standards/runners/encoding-core-smoke.js`
- `standards/runners/codec-parity-smoke.js`
- `standards/runners/rust-parity-encoding.js`
- `standards/corpus/codec-primitives/encoding-core.json`
- `standards/components/manifests/codec-primitives/encoding-core.toml`
- `standards/build/wasm/codec-primitives/README.md`

The replacement policy is:

- use `encoding-core.wat::adler32` for one-shot checksum calculation;
- add a tiny resumed-state WAT helper only if a runtime needs to continue from
  a prior checksum outside a larger deflate/inflate state machine;
- keep byte streaming and host readers in the runtime layer, not in a checksum
  crate;
- keep `rust-parity-encoding.js` as the deletion proof runner and let it report
  `rust_source_deleted` once the Rust oracle is intentionally unavailable.

Existing proof references:

- empty input is `0x00000001`;
- `"Wikipedia"` is `0x11e60398`;
- `encoding-core.wat` also carries CRC32 and integer primitive coverage, so
  Adler deletion should not split a new checksum-only primitive unless a runtime
  forces it.

## Miniz oxide WAT prerequisite

`crates/utility/edgerun-miniz-oxide` was the only real Rust dependent. It has
now been retired in the compression WAT lane, and this Adler batch provides the
checksum primitive that lane needs:

- `encoding-core.wat::adler32_update(seed, ptr, len)`

Former files with Adler-related references:

- `crates/utility/edgerun-miniz-oxide/Cargo.toml`
- `crates/utility/edgerun-miniz-oxide/Readme.md`
- `crates/utility/edgerun-miniz-oxide/src/shared.rs`
- `crates/utility/edgerun-miniz-oxide/src/lib.rs`
- `crates/utility/edgerun-miniz-oxide/src/deflate/core.rs`
- `crates/utility/edgerun-miniz-oxide/src/inflate/core.rs`
- `crates/utility/edgerun-miniz-oxide/src/inflate/mod.rs`
- `crates/utility/edgerun-miniz-oxide/src/inflate/stream.rs`

Direct external use of the `adler2` crate was isolated to:

- `crates/utility/edgerun-miniz-oxide/src/shared.rs`

The `shared.rs` path currently bridges miniz update calls with
`adler2::Adler32::from_checksum(adler)`. That path was deleted with miniz and
replaced conceptually by the WAT checksum update primitive instead of another
Rust Adler implementation.

The former miniz Adler fields were part of zlib stream validation and are now
represented by wrapper/inflate WAT records and composition runners:

- `check_adler32`
- `z_adler32`
- `adler32()`
- `adler32_header()`
- `TINFLStatus::Adler32Mismatch`
- `MZ_ADLER32_INIT`

## Manifest cleanup

Manifest and lock references removed or made obsolete by Adler2 retirement:

- `Cargo.toml`
  - workspace member: `crates/utility/edgerun-adler2`
  - patch entry: `adler2 = { path = "crates/utility/edgerun-adler2" }`
- deleted `crates/utility/edgerun-miniz-oxide/Cargo.toml`
  - feature edge: `rustc-dep-of-std = ["adler2/rustc-dep-of-std"]`
  - dependency: `adler2 = { path = "../edgerun-adler2", default-features = false }`
- `Cargo.lock`
  - package entry for `adler2`
  - dependency reference from `miniz_oxide`

The root workspace member and root `adler2` patch entry were removed in the
same deletion pass. Miniz was already deleted, so no miniz manifest cleanup
remains.

## Other references

Project standards already mention Adler2 as replaceable or covered:

- `standards/wat-codec-deletion-audit.md`
- `standards/wat-codec-replacement-audit.md`
- `standards/wat-encoder-decoder-roadmap.md`

These should be updated only if the roadmap needs to mark Adler2 as retired.
