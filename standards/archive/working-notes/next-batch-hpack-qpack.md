# Next Batch: HPACK And QPACK

## Recommendation

HPACK/QPACK should be in the next WAT batch, but the batch should target header
block and instruction scanners, not full dynamic-table replacement.

The current WAT modules already cover the leaf kernels:

- `http-prefix-int.wat`: HPACK/QPACK prefixed integer encode/decode.
- `hpack-huffman.wat`: HPACK Huffman decode and validation.
- `hpack-string.wat`: HPACK string literal scan/decode.
- `qpack-string.wat`: QPACK prefix-string scan/decode.

The next valuable layer is a set of structural scanners that compose those
kernels into header-block and stream-instruction records. That gives useful
replacement pressure on repeated Rust byte parsing while keeping static table
lookup, dynamic table mutation, header validation, stream state, and HTTP
semantics in Rust.

## Candidate WAT Modules

1. `hpack-header-block.wat`
   - Scan a complete HPACK header block into field records.
   - Classify representations: indexed, literal with incremental indexing,
     size update, literal never indexed, literal without indexing.
   - Decode representation-local integers and string spans.
   - Emit offsets, consumed length, indexing mode, table index, and literal
     name/value spans.
   - Do not resolve static/dynamic table entries.
   - Do not mutate dynamic table state.

2. `qpack-header-block.wat`
   - Scan QPACK encoded field section prefix and field line records.
   - Cover indexed, indexed post-base, literal with name reference, literal
     with post-base name reference, and literal literal-name fields.
   - Emit required insert count, sign, delta base, field kind, table selector,
     index, string spans, and consumed length.
   - Do not calculate dynamic table availability beyond structural integer
     validity.
   - Do not resolve static/dynamic table entries.

3. `qpack-encoder-stream.wat`
   - Scan encoder stream instructions: dynamic table size update, insert with
     name reference, insert without name reference, and duplicate.
   - Emit instruction kind, table selector, index/capacity, literal string
     spans, and consumed length.
   - Return a partial/truncated status for incomplete instructions so Rust can
     retain buffering policy.

4. `qpack-decoder-stream.wat`
   - Scan decoder stream instructions: section acknowledgement, stream
     cancellation, and insert count increment.
   - Emit instruction kind, stream id or increment, and consumed length.
   - This is a small module, but it is a clean QPACK parity surface.

5. Optional: `hpack-header-block-encode.wat`
   - Encode only stateless field representation prefixes and raw string
     literals.
   - Useful after scanner parity is proven.
   - Keep the HPACK encoder's table-selection strategy in Rust.

## Rust Parity Targets

Primary HPACK parity targets:

- `crates/utility/edgerun-hpack/src/decoder.rs`
  - `decode_integer`
  - `decode_string`
  - `FieldRepresentation::new`
  - `Decoder::decode_with_cb`
  - `Decoder::decode_literal`
  - `Decoder::update_max_dynamic_size`
- `crates/utility/edgerun-hpack/src/encoder.rs`
  - `encode_integer_into`
  - raw string literal emission
  - indexed/literal representation prefix emission
- `crates/utility/edgerun-hpack/src/huffman.rs`
  - `HuffmanDecoder::decode`
  - `encode`

Primary QPACK parity targets:

- `crates/protocol/edgerun-protocols/src/http/http3/qpack/prefix_int.rs`
  - `decode`
  - `encode`
- `crates/protocol/edgerun-protocols/src/http/http3/qpack/prefix_string/decode.rs`
  - Huffman/raw string decode behavior.
- `crates/protocol/edgerun-protocols/src/http/http3/qpack/block.rs`
  - `HeaderPrefix::{decode, encode, get}`
  - `Indexed::{decode, encode}`
  - `IndexedWithPostBase::{decode, encode}`
  - `LiteralWithNameRef::{decode, encode}`
  - `LiteralWithPostBaseNameRef::{decode, encode}`
  - `Literal::{decode, encode}`
- `crates/protocol/edgerun-protocols/src/http/http3/qpack/stream.rs`
  - `EncoderInstruction::decode`
  - `InsertWithNameRef::{decode, encode}`
  - `InsertWithoutNameRef::{decode, encode}`
  - `Duplicate::{decode, encode}`
  - `DynamicTableSizeUpdate::{decode, encode}`
  - `DecoderInstruction::decode`
  - `HeaderAck::{decode, encode}`
  - `StreamCancel::{decode, encode}`
  - `InsertCountIncrement::{decode, encode}`

Do not target these for WAT replacement in this batch:

- `crates/utility/edgerun-hpack/src/lib.rs`: static and dynamic table ownership.
- HPACK `Decoder` and `Encoder` table state and header selection strategy.
- `crates/protocol/edgerun-protocols/src/http/http3/qpack/dynamic.rs`.
- `crates/protocol/edgerun-protocols/src/http/http3/qpack/decoder.rs` dynamic
  table resolution and blocking behavior.
- `crates/protocol/edgerun-protocols/src/http/http3/qpack/encoder.rs` dynamic
  table policy.
- HTTP/2 and HTTP/3 stream/session code that consumes decoded headers.

## Composition Tests Needed

The current composition runners prove frame-to-string-to-Huffman flow:

- `http2-frame -> hpack-string -> hpack-huffman`
- `http3-frame -> qpack-string -> hpack-huffman`

The next batch should add these composition tests:

1. `http2-frame -> hpack-header-block -> hpack-string -> hpack-huffman`
   - HEADERS frame payload containing static indexed fields.
   - Literal with incremental indexing and raw name/value strings.
   - Literal never indexed.
   - Dynamic table size update.
   - Truncated string and overlong integer rejection.

2. `http3-frame -> qpack-header-block -> qpack-string -> hpack-huffman`
   - Static-only QPACK field section with required insert count zero.
   - Literal with static name reference.
   - Literal with literal name.
   - Post-base and dynamic references classified as structurally valid but
     requiring Rust table context.
   - Invalid prefix and truncated field rejection.

3. `qpack-encoder-stream -> qpack-string -> hpack-huffman`
   - Insert with static name reference.
   - Insert without name reference.
   - Dynamic table size update.
   - Duplicate.
   - Partial instruction status without consuming the caller's buffered bytes.

4. `qpack-decoder-stream -> http-prefix-int`
   - Header acknowledgement.
   - Stream cancellation.
   - Insert count increment.
   - Overflow and invalid prefix cases.

5. Rust oracle parity runners
   - Add a Rust-backed runner that calls existing HPACK/QPACK tests or a small
     Rust oracle binary for exact status/consumed/value comparisons.
   - Separate scanner parity from semantic decode parity. Scanner parity can
     match field kind, flags, values, spans, and consumed length. Semantic parity
     additionally checks resolved header fields where Rust static-table context
     is available.

## Deletion And Replacement Value

High-value replacement candidates after parity:

- HPACK prefixed integer and string decode duplication in
  `crates/utility/edgerun-hpack/src/decoder.rs`.
- HPACK Huffman decode lookup in `crates/utility/edgerun-hpack/src/huffman.rs`,
  if the WAT table is proven across the RFC corpus and Rust edge tests.
- QPACK prefixed integer helpers in
  `crates/protocol/edgerun-protocols/src/http/http3/qpack/prefix_int.rs`.
- QPACK prefix-string decode helpers in
  `crates/protocol/edgerun-protocols/src/http/http3/qpack/prefix_string/`.
- QPACK field and stream instruction byte classification in
  `crates/protocol/edgerun-protocols/src/http/http3/qpack/block.rs` and
  `stream.rs`.

The deletion value is real but limited to leaf parsing and encoding kernels.
The object models should remain Rust. The adapter should return compact records
that Rust turns into existing HPACK/QPACK objects, preserving table ownership
and HTTP integration.

## Blockers

- Dynamic table state is not a byte codec. HPACK and QPACK table mutation,
  eviction, blocked-stream handling, known received count, base calculation, and
  index resolution must stay in Rust until a separate state-machine proof exists.
- Static table lookup is simple but should remain Rust initially so WAT emits
  table selectors and indexes, not header objects.
- QPACK partial instruction behavior matters. The Rust stream decoder can return
  `Ok(None)` for incomplete instructions without advancing the real buffer. WAT
  needs an explicit partial status and consumed length discipline.
- HPACK and QPACK have different integer limits. HPACK Rust currently limits
  continuation length around 32-bit-scale values, while QPACK allows wider
  values and has overflow tests up to 64-bit boundaries. The WAT ABI must expose
  separate HPACK and QPACK status codes instead of flattening errors.
- Huffman parity needs a broader corpus before deletion. Existing runners prove
  representative values; deletion should wait for all RFC vectors, invalid EOS,
  invalid padding, padding too large, and high-byte symbols.
- The EdgeRun C runtime bridge is not integrated into this repo yet. WAT can be
  authored and tested through Node now, but Rust replacement requires the bridge
  described in `standards/wat-codec-edgerun-c-runtime-bridge.md`.

## Batch Decision

Yes, HPACK/QPACK should be in the next batch, with this scope:

1. Build `hpack-header-block.wat`.
2. Build `qpack-header-block.wat`.
3. Build `qpack-encoder-stream.wat`.
4. Build `qpack-decoder-stream.wat`.
5. Add Rust-oracle parity runners for scanner records and static-only semantic
   decode.
6. Add composition runners from HTTP/2 and HTTP/3 frames into header-block
   scanners and string/Huffman helpers.

Do not include dynamic table replacement in this batch. The immediate goal is to
turn byte parsing into reusable WAT records, prove Rust parity, and prepare a
small bridge-backed replacement patch later.
