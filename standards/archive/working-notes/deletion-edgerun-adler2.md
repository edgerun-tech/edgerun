# edgerun-adler2 deletion note

`crates/utility/edgerun-adler2` was a local fork of the small `adler2`
checksum crate. It is now deleted. Its useful portable behavior is the
Adler-32 checksum recurrence, owned by WAT. The Rust crate shape around that
recurrence was compatibility glue.

## Deletion status

Deleted:

- `crates/utility/edgerun-adler2/src/lib.rs`
- `crates/utility/edgerun-adler2/src/algo.rs`
- `crates/utility/edgerun-adler2/Cargo.toml`
- `crates/utility/edgerun-adler2/README.md`
- `crates/utility/edgerun-adler2/CHANGELOG.md`
- `crates/utility/edgerun-adler2/RELEASE_PROCESS.md`
- `crates/utility/edgerun-adler2/LICENSE-0BSD`
- `crates/utility/edgerun-adler2/LICENSE-APACHE`
- `crates/utility/edgerun-adler2/LICENSE-MIT`

Manifest cleanup:

- removed root workspace member `crates/utility/edgerun-adler2`;
- removed root `[patch.crates-io]` entry `adler2`.

## Useful behavior

The canonical checksum behavior is:

- modulus: `65521`;
- initial checksum: `0x00000001`, equivalent to `a = 1`, `b = 0`;
- per byte: `a = (a + byte) % 65521`, then `b = (b + a) % 65521`;
- final checksum: `(b << 16) | a`;
- resumable state: `Adler32::from_checksum(sum)` splits `sum` into
  `a = low16(sum)`, `b = high16(sum)`, then continues the same recurrence.

The WAT primitive owns the portable checksum behavior:

- `standards/build/wasm/codec-primitives/encoding-core.wat`
  - `proto_standard_id() == 300001`
  - `adler32(ptr, len) -> u32`
  - `adler32_update(seed, ptr, len) -> u32`

The smoke runner proves the current canonical vectors and resumed-state cases:

- empty input -> `0x00000001`;
- `"hello"` -> `0x062c0215`;
- `"123456789"` -> `0x091e01de`;
- `"Wikipedia"` -> `0x11e60398`.
- `"rust"` resumed with `"acean"` equals one-shot `"rustacean"`;
- split `0xff` buffers resume to the same checksum as one-shot input.

The Rust parity runner also compares the WAT result to the Rust oracle when the
Rust source is still present, and switches to deleted-oracle proof mode after
the Rust codec source is retired.

## Rust behavior that is not codec ownership

The following Rust APIs are not useful portable codec behavior:

- `Adler32` as a Rust struct with private `a: u16` and `b: u16` fields;
- `Default`, `Debug`, `Copy`, and `Clone` trait shape;
- `std::hash::Hasher` integration and architecture-dependent `Hash` examples;
- `adler32_slice(data)` as a convenience wrapper around initial-state checksum;
- `adler32<R: BufRead>(reader)` as `std` host I/O glue;
- vectorized or chunked implementation details in `algo.rs`, including
  `U32X4`, chunk sizing, and deferred modulo optimization.

Those are API and performance conveniences. The protocol commitment is the
checksum recurrence and final 32-bit value.

## Miniz prerequisite closed

The former real local caller was `edgerun-miniz-oxide`, which used Adler-32 for
zlib checksum state:

- `crates/utility/edgerun-miniz-oxide/src/shared.rs`
  - `update_adler32(adler, data)`
  - uses `adler2::Adler32::from_checksum(adler)` so zlib inflate/deflate can
    resume from an existing checksum.

`encoding-core.wat::adler32(ptr, len)` starts from the default initial checksum.
That is enough for `adler32_slice`. `encoding-core.wat::adler32_update(seed,
ptr, len)` replaces `miniz_oxide::shared::update_adler32(adler, data)` without
reprocessing the whole stream:

```text
adler32_update(seed: u32, ptr: i32, len: i32) -> u32
```

The update primitive splits `seed` exactly like Rust `Adler32::from_checksum`,
continues the recurrence over `len` bytes, and returns the packed checksum.
That primitive is now the zlib streaming checksum boundary for the WAT
inflate/deflate implementation. `edgerun-miniz-oxide` has since been deleted.

## Retirement plan

1. Done: add `encoding-core.wat::adler32_update(seed, ptr, len)`.
2. Done: extend `encoding-core-smoke.js` with resume vectors:
   - first `1024` bytes of `0xff` -> `0x79a6fc2e`;
   - resuming over the remaining `1024 * 1024 - 1024` bytes of `0xff`
     -> `0x8e88ef11`.
3. Done: keep `rust-parity-encoding.js` as deleted-oracle proof after source
   removal.
4. Done: delete `src/lib.rs` and `src/algo.rs`.
5. Done: delete crate metadata.
6. Done: use `adler32_update(seed, ptr, len)` as the checksum primitive in the
   compression WAT lane instead of a Rust `adler2` dependency.
