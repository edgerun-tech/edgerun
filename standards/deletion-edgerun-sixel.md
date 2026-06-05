# Deletion: edgerun-sixel

`crates/utility/edgerun-sixel` was deleted after extracting its portable decode
behavior to `standards/build/wasm/codec-primitives/sixel-decode.wat`.

WAT ownership:

- `sixel_decode_rgba(ptr, len, zero_color, grid_size, out, out_cap, meta) -> i64`
- ABI version `2`, standard id `300071`
- `meta[0..12]` records final width, final height, and required RGBA bytes.
- Status `0` decodes into caller-provided RGBA bytes, status `1` rejects empty
  input, status `2` rejects insufficient output capacity, and status `3`
  rejects malformed repeat or color directives.

Covered behavior:

- SIXEL DCS payload scan.
- Sixel chars `?` through `~` mapped to six vertical bits.
- Repeat handling through `!<number><sixel-char>`.
- Color register selection and RGB-percent color definitions.
- `$` carriage return and `-` six-row raster advance.
- Raster dimensions and corrected final dimensions.
- Empty payload, invalid repeat, invalid color, and output-cap failure.

Deleted Rust value:

- Rust-only `SixelImage`, `DcsSettings`, `SixelError`, allocation, display/std
  error impls, and terminal-specific image struct shape.

Intentional fallout:

- `crates/edgerun-term/edgerun-term-core` still imports `edgerun_sixel::*`.
  Those call sites should route through host/WAT RGBA records or an owner-local
  terminal image record, not through a revived Rust compatibility crate.

Verification:

```bash
node standards/runners/sixel-decode-smoke.js
```
