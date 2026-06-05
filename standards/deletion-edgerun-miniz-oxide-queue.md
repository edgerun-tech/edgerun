# edgerun-miniz-oxide deletion queue

`crates/utility/edgerun-miniz-oxide` is compatibility bulk. The durable behavior
worth keeping is DEFLATE/zlib/gzip parsing, bounded inflate, stored-block emit,
and checksum proof. Rust stream state, `Vec` facades, miniz status enums, serde,
and compression-level compatibility are not reusable portable behavior.

Cargo health is not an acceptance gate for this deletion queue.

## Current WAT coverage

- `gzip-member.wat` (`300064`): gzip member header/trailer scan/emit, CRC32,
  ISIZE, payload offsets.
- `zlib-wrapper.wat` (`300065`): zlib CMF/FLG scan/emit, Adler32 trailer,
  payload offsets.
- `deflate-stored.wat` (`300066`): raw stored-block emit and validation.
- `deflate-inflate.wat` (`300067`): bounded raw inflate for stored, fixed
  Huffman, and dynamic Huffman blocks, output-cap enforcement, CRC32/Adler32
  record output.
- Composition runners prove gzip/zlib wrappers over raw DEFLATE with checksum
  and round-trip checks.

## Deleted immediately

These files were Rust API glue or behavior already covered by WAT stored/zlib
wrappers:

- `src/deflate/stored.rs`: stored-block compressor path; replaced by
  `deflate-stored.wat`.
- `src/deflate/zlib.rs`: zlib header helper; replaced by `zlib-wrapper.wat`.
- `src/deflate/stream.rs`: streaming Rust facade; not portable wire behavior.
- `src/inflate/stream.rs`: streaming Rust facade; not portable wire behavior.
- `src/inflate/output_buffer.rs`: internal Rust output adapter; not portable
  wire behavior.
- `src/serde/mod.rs`: empty serde feature stub.
- `src/shared.rs`: Adler/constants compatibility surface; Adler behavior is
  already a WAT primitive and zlib composition proof point.

Local module exports were removed from:

- `src/lib.rs`
- `src/deflate/mod.rs`
- `src/inflate/mod.rs`

## Deleted in final pass

The remaining crate source and metadata were deleted after the dynamic-Huffman
WAT gate cleared:

- `src/deflate/buffer.rs`
- `src/deflate/core.rs`
- `src/deflate/mod.rs`
- `src/inflate/core.rs`
- `src/inflate/mod.rs`
- `src/lib.rs`
- `Cargo.toml`
- `Readme.md`
- `LICENSE`
- `LICENSE-APACHE.md`
- `LICENSE-MIT.md`
- `LICENSE-ZLIB.md`

The root workspace member, workspace dependency, and crates.io patch entries for
`miniz_oxide` were removed.

## Adapter work still needed

The useful product callers remain, but they now need owner-local host-side WASM
invocation boundaries instead of Rust compression APIs:

- `crates/authority/edgerun-vfs/src/packet.rs`
  - `compress_object`: route to `deflate-stored.wat` stored raw encode.
  - `unsealed_payload_to_object`: route to `deflate-inflate.wat` for stored raw
    bytes and keep plaintext length/object hash verification.
- `crates/protocol/edgerun-protocols/src/http/http1/compression.rs`
  - `compress_gzip`: route to `gzip-member.wat` + `deflate-stored.wat`.
  - `compress_deflate`: route to `zlib-wrapper.wat` + `deflate-stored.wat`.
- `crates/edgerun-oci/src/registry/tar_push.rs`
  - `gzip_bytes`: route to `gzip-member.wat` + `deflate-stored.wat`.

- `crates/protocol/edgerun-protocols/src/http/http1/compression.rs`
- `crates/apps/edgerun-tor-bench/src/tor.rs`
- `crates/edgerun-oci/src/tar_compression.rs`
- `crates/edgerun-oci/src/registry/tar_push.rs`
- `crates/edgerun-codelyzer/src/bin/apk_api_calls.rs`
- `crates/authority/edgerun-vfs/src/packet.rs`

These routes should not promise compression-ratio parity. The useful invariant
is valid bytes with correct wrapper metadata, deterministic limits, and checksum
proof.

## Final deletion status

Complete:

1. Fixed Huffman landed in `deflate-inflate.wat`.
2. Dynamic Huffman landed in `deflate-inflate.wat`.
3. `crates/utility/edgerun-encoding/src/compression.rs` is deleted.
4. Remaining `edgerun-miniz-oxide` source and metadata are deleted.
5. Root workspace/dependency/patch entries for `miniz_oxide` are removed.

Not complete:

- Product callers still need real WAT invocation adapters. They intentionally
  remain visible in scans until that owner-local routing work lands.

## Evidence commands

Use these after each deletion pass:

```bash
rg -n "miniz_oxide::|edgerun_encoding::compression" crates standards
rg -n "edgerun-encoding = .*compression|edgerun-encoding/compression|dep:miniz_oxide|miniz_oxide =" Cargo.toml crates --glob 'Cargo.toml'
find crates/utility/edgerun-miniz-oxide/src -type f -printf '%p\n' | sort
```
