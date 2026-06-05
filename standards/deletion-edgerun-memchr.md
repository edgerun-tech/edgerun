# edgerun-memchr deletion

`crates/utility/edgerun-memchr` is deleted after extracting its useful portable
byte-search behavior to `byte-search.wat`.

WAT owner:

- `standards/build/wasm/codec-primitives/byte-search.wat`
- standard id `300074`
- runner `standards/runners/byte-search-smoke.js`

Covered behavior:

- `memchr` and `memrchr` over byte slices.
- `memchr2` and `memrchr2` for two possible needle bytes.
- `memchr3` and `memrchr3` for three possible needle bytes.
- `memmem_find` and `memmem_rfind` for simple substring search.
- Empty haystack handling.
- First and last byte matches.
- Repeated needle bytes.
- Overlapping substring search.
- Empty substring conventions: forward search returns `0`, reverse search
  returns the haystack length.

The deleted Rust value was CPU-specific SIMD dispatch, iterator API surfaces,
prefilter/searcher allocation strategy, logging hooks, and crate compatibility
shape. Existing `memchr` imports, especially in `edgerun-combine`, are deliberate
caller-demolition fallout and should not cause this crate to be restored.
