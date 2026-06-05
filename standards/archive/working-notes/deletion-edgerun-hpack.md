# edgerun-hpack retirement map

`crates/utility/edgerun-hpack` is a no-std HPACK compatibility crate whose
useful byte-level behavior has already been pulled into codec WAT primitives.
The remaining Rust value is mostly API shape, header-table state, and HTTP/2
runtime policy. Those should not become a second full HPACK crate in WAT.

## Source scope

Read for this retirement pass:

- `crates/utility/edgerun-hpack/src/lib.rs`
- `crates/utility/edgerun-hpack/src/decoder.rs`
- `crates/utility/edgerun-hpack/src/encoder.rs`
- `crates/utility/edgerun-hpack/src/huffman.rs`

Current Rust size:

- `lib.rs`: 756 lines
- `decoder.rs`: 1650 lines
- `encoder.rs`: 486 lines
- `huffman.rs`: 702 lines

## WAT ownership

The portable codec ownership now lives in these WAT modules:

- `standards/build/wasm/codec-primitives/http-prefix-int.wat`
  - `hpack_prefix_int_decode`
  - `hpack_prefix_int_encode`
  - `qpack_prefix_int_decode`
  - `qpack_prefix_int_encode`
- `standards/build/wasm/codec-primitives/hpack-string.wat`
  - `hpack_string_scan`
  - `hpack_string_decode`
- `standards/build/wasm/codec-primitives/hpack-huffman.wat`
  - `hpack_huffman_decode`
  - `hpack_huffman_validate`
- `standards/build/wasm/codec-primitives/hpack-header-block.wat`
  - `hpack_header_instruction_decode`
  - `hpack_header_block_scan`
- `standards/build/wasm/codec-primitives/hpack-table-core.wat`
  - `hpack_table_entry_size`
  - `hpack_table_insert_plan`
  - `hpack_table_resize_plan`
- Related QPACK modules that share the same prefix/string/Huffman substrate:
  - `qpack-encoder-stream.wat`
  - `qpack-decoder-stream.wat`

## Behavior map

| Rust behavior | Source | Replacement / decision |
| --- | --- | --- |
| HPACK prefix integer decode, including flag extraction, consumed length, invalid prefix, truncated input, and excessive continuation status | `decoder.rs::decode_integer` | `http-prefix-int.wat::hpack_prefix_int_decode` |
| HPACK prefix integer encode with caller-supplied leading bits | `encoder.rs::encode_integer_into`, `encode_integer` | `http-prefix-int.wat::hpack_prefix_int_encode` |
| Raw HPACK string scan: Huffman flag, prefix length, payload offset, payload length, total consumed | `decoder.rs::decode_string` first half | `hpack-string.wat::hpack_string_scan` |
| HPACK string decode for raw literals | `decoder.rs::decode_string` raw path | `hpack-string.wat::hpack_string_decode` |
| HPACK string decode for Huffman literals | `decoder.rs::decode_string` Huffman path plus `huffman.rs::HuffmanDecoder::decode` | `hpack-string.wat::hpack_string_decode` composes with `hpack-huffman.wat::hpack_huffman_decode` |
| Huffman validation: EOS in string, invalid padding, padding too large | `huffman.rs::HuffmanDecoder::decode` | `hpack-huffman.wat::hpack_huffman_validate` and decode status |
| Huffman encode | `huffman.rs::encode` | Do not preserve as a deletion blocker. The Rust HPACK encoder never uses Huffman encoding; it emits raw string literals only. If outbound compression becomes valuable, add a focused WAT encoder later with RFC vectors. |
| Field representation classification: indexed, literal with incremental indexing, table size update, literal never indexed, literal without indexing | `decoder.rs::FieldRepresentation::new` | `hpack-header-block.wat::hpack_header_instruction_decode` |
| Literal header structural decode: indexed name or literal name, literal value spans, consumed length | `decoder.rs::decode_literal` | `hpack-header-block.wat::hpack_header_instruction_decode` and `hpack_header_block_scan` |
| Header-block scan loop over complete HPACK block | `decoder.rs::decode_with_cb` byte loop | `hpack-header-block.wat::hpack_header_block_scan` |
| Indexed-header and indexed-name table resolution | `decoder.rs::decode_indexed`, `decode_literal`, `lib.rs::HeaderTable::get_from_table` | HTTP/2 runtime/table policy. WAT should emit table indexes and representation kind, not resolve header objects. |
| Static table contents and 1-based merged static/dynamic address space | `lib.rs::STATIC_TABLE`, `HeaderTable` | HTTP/2 runtime/table policy. Keep a single runtime table definition near the HTTP/2 adapter or shared protocol code; do not duplicate it in WAT. |
| Dynamic table entry size, newest-retained-first eviction planning, resize planning, oversized-entry handling | `lib.rs::DynamicTable`, `decoder.rs::update_max_dynamic_size` | `hpack-table-core.wat`. Actual table storage, lookup, mutation, and SETTINGS enforcement remain HTTP/2 runtime policy. |
| Encoder table lookup strategy: exact match first, then name-only match; literal indexed insertion for unknown full header; indexed emit on second use | `encoder.rs::Encoder`, `HeaderTable::find_header` | HTTP/2 runtime/table policy. WAT can encode representation bytes, but selection strategy must stay with the runtime that owns table state and SETTINGS limits. |
| Raw string literal emission, no Huffman on encode | `encoder.rs::encode_string_literal` | `http-prefix-int.wat::hpack_prefix_int_encode` plus caller copy of bytes. This does not require a standalone Rust crate. |
| Rust API conveniences: `Decoder`, `Encoder`, `Writer`, `Cow` callback API, owned `Vec` decode API, test-only helpers | all source files | Delete with crate retirement. Callers should consume WAT records through the HTTP/2 adapter, not through this compatibility API. |

## Runtime policy that should not become codec WAT

The following behaviors are important, but they belong in the HTTP/2 runtime or
an adapter that owns stream/session state:

- HPACK static table lookup and the merged 1-based static/dynamic index space.
- Dynamic table storage, mutation, header lookup, and SETTINGS-driven maximum
  table size enforcement. WAT owns only entry-size and eviction arithmetic.
- The choice to encode a header as indexed, indexed-name literal, or new-name
  literal.
- Decoder callback ownership and allocation policy.
- Header semantic validation, pseudo-header ordering, duplicate restrictions,
  connection-header rejection, and HTTP request/response policy.
- Bridging from scanned literal spans into owned header records.

The WAT modules should stay structural: classify bytes, decode prefix integers,
surface spans, validate Huffman strings, and report consumed lengths/statuses.

## Caller surface

Direct crate callers found in this pass:

- `crates/protocol/edgerun-protocols`
  - `src/http/http2/hpack.rs` re-exports `edgerun_hpack::{Decoder, Encoder}`.
  - HTTP/2 server request/response/data paths construct `Decoder` and
    `Encoder`.
- `crates/node/edgerun-node`
  - HTTP/2 client/server paths use the `edgerun-protocols` HPACK re-export and
    also map `edgerun_hpack::DecoderError` into `Http2Error`.
- `crates/utility/edgerun-http-client`
  - Mirrors the HTTP/2 client/server paths and error mapping.
- `crates/utility/edgerun-encoding`
  - Feature-gated re-export of `Decoder`, `DecoderError`, `Encoder`, and
    `HuffmanDecoder`.

Replacement should start in `edgerun-protocols/src/http/http2/hpack.rs`: keep
that module as the runtime owner of table state and make it consume WAT scanner
records. Then the node and HTTP-client crates can continue depending on the
protocol surface while the compatibility crate disappears.

## Deletion readiness

Ready to delete from `edgerun-hpack`:

- `decoder.rs` byte parsing and Huffman/string decode behavior are covered by
  WAT scanners.
- `encoder.rs` prefix integer and raw literal byte emission are covered by WAT
  prefix encode plus direct payload copy.
- `huffman.rs` decode/validate behavior is covered by WAT. The unused Huffman
  encoder can be deleted or reintroduced later as a focused WAT primitive.
- `lib.rs` table arithmetic is covered by WAT. Runtime table storage and
  index resolution belong in the HTTP/2 adapter, not this retired
  compatibility crate.

Keep or recreate outside this crate:

- A runtime HPACK table-state adapter with static table, dynamic table, and
  SETTINGS-bound max-size policy.
- An HTTP/2 error mapping from WAT status codes and runtime table errors.
- A caller-facing header-list builder for existing HTTP/2 code until those
  call sites are converted to WAT records directly.

## Proofs to keep attached

Existing runner coverage relevant to this retirement:

- `standards/runners/http-prefix-int-smoke.js`
- `standards/runners/hpack-string-smoke.js`
- `standards/runners/hpack-huffman-smoke.js`
- `standards/runners/hpack-header-block-smoke.js`
- `standards/runners/hpack-table-core-smoke.js`
- `standards/runners/codec-composition-http2-hpack-huffman.js`
- `standards/runners/codec-composition-http2-hpack.js`
- `standards/runners/qpack-encoder-stream-smoke.js`
- `standards/runners/qpack-decoder-stream-smoke.js`

For the adapter batch, resolve static and dynamic table indexes in the HTTP/2
runtime over the WAT header-block records.
