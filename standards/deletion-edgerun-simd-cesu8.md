# Deletion: edgerun-simd-cesu8

Status: deleted after WAT extraction.

`crates/utility/edgerun-simd-cesu8` was a Rust compatibility crate for CESU-8
and Java Modified UTF-8. Its useful portable behavior is now owned by
`standards/build/wasm/codec-primitives/cesu8-mutf8.wat` (`300069`).

Extracted behavior:

- CESU-8 encode from UTF-8 input: ASCII, two-byte, and three-byte BMP UTF-8 are
  copied unchanged; supplementary UTF-8 code points are converted to UTF-16
  surrogate pairs and emitted as two three-byte surrogate sequences.
- MUTF-8 encode from UTF-8 input: same CESU-8 behavior plus U+0000 emitted as
  `c0 80`.
- Strict CESU-8 decode: BMP triples are copied, CESU surrogate pairs are decoded
  back to four-byte UTF-8, literal UTF-8 four-byte sequences are rejected, and
  malformed, truncated, or unpaired surrogate sequences fail closed.
- Strict MUTF-8 decode: same strict CESU-8 behavior plus literal NUL rejection
  and `c0 80` decoding to U+0000.

Deleted Rust value:

- `Cow` borrowed/owned API shape.
- lossy decode convenience wrappers.
- SIMD and word dispatch scaffolding.
- iterator/display/docs/benchmark/image compatibility material.
- `DecodingError` wrapper and Rust trait surface.

Verification:

```bash
node standards/runners/cesu8-mutf8-smoke.js
```

The runner compiles through `wat2wasm`, validates with `wasm-validate`, and
covers ASCII/BMP passthrough, supplementary encode/decode, MUTF-8 null handling,
literal four-byte UTF-8 rejection under strict CESU-8 decode, malformed,
truncated, and unpaired surrogate rejection, and output-capacity failure.

Intentional caller fallout:

- `crates/utility/edgerun-jni`
- `crates/utility/edgerun-jni-macros`

These callers should use the WAT primitive or a caller-owned host adapter in the
JNI string lane. Do not restore a Rust `simd_cesu8` crate or compatibility API.
