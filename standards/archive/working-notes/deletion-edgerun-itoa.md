# edgerun-itoa deletion

`crates/utility/edgerun-itoa` is deleted after extracting its useful portable
integer decimal formatting behavior to `integer-decimal.wat`.

WAT owner:

- `standards/build/wasm/codec-primitives/integer-decimal.wat`
- standard id `300073`
- runner `standards/runners/integer-decimal-smoke.js`

Covered behavior:

- Unsigned `u64` decimal formatting into a caller-provided byte buffer.
- Signed `i64` decimal formatting, including `i64::MIN`.
- Split-limb unsigned `u128` decimal formatting.
- Output-capacity failure with a negative return code.

The deleted Rust value was `Buffer`, trait-based integer dispatch, borrowed
`&str` return shape, `usize`/`isize` target-width wrappers, and optimized
decimal-pair implementation details. Existing Rust callers that still depend on
the `itoa` crate are deliberate caller-demolition fallout and should not cause
this compatibility crate to be restored.
