# WAT HPACK/QPACK HTTP Runtime Adapter Queue

This queue turns `standards/deletion-edgerun-hpack.md` into executable agent
work. The goal is to retire `crates/utility/edgerun-hpack` from HTTP runtime
callers without recreating a Rust compatibility crate.

Do not move dynamic table state, HTTP validation, stream/session state, SETTINGS
policy, or header semantic checks into WAT. WAT owns compact byte kernels and
structural scanners. The HTTP runtime owns connection state and turns scanned
records into header lists.

## Current public blocker

The deleted crate surface is gone. `edgerun-protocols` now has a
protocol-owned adapter skeleton:

- `crates/protocol/edgerun-protocols/src/http/http2/hpack.rs` defines
  `HpackContext`, `HpackDecodeError`, `HpackEncodeError`, and
  `HpackHeaderList`.
- `HpackContext::decode_header_block` still returns `NotYetRouted` because
  `edgerun-protocols` has no in-crate WebAssembly invocation surface.
- `HpackContext` now exposes the protocol-owned WAT record boundary:
  `HpackWatStatus`, `HpackWatInstructionKind`,
  `HpackWatHeaderInstruction::from_wat_record`, and
  `HpackContext::decode_wat_records`.
- `decode_wat_records` maps `hpack-header-block.wat` records into runtime
  headers for static indexed fields and raw literal strings. Huffman literals
  are rejected until the host supplies decoded string bytes from
  `hpack-string.wat` / `hpack-huffman.wat`.
- `encode_header_block` now has a deterministic direct HPACK path for the
  current server response shape: static `:status: 200` plus never-indexed raw
  literals for the other headers. It does not use dynamic-table insertion or
  Huffman encoding.
- `crates/protocol/edgerun-protocols/src/http/http2/server/headers.rs`,
  `server/response.rs`, and `server/data.rs` now take `HpackContext` instead
  of the old `Decoder` / `Encoder` types.
- `crates/utility/edgerun-http-client/src/http/http2/mod.rs` and
  `client.rs` now reexport/use `HpackContext` and map adapter errors to the
  local `Http2Error::HpackDecode` / `HpackEncode` variants.
- `crates/node/edgerun-node/src/http/http2/mod.rs` and `client.rs` now
  reexport/use `HpackContext` the same way.

Remaining public/downstream blockers:

- `crates/protocol/edgerun-protocols/src/http/http2/server/tests.rs` no longer
  uses the retired `Decoder` / `Encoder` fixtures. Tests now pass literal HPACK
  header-block bytes into `HpackContext` and expect `COMPRESSION_ERROR` while
  decode returns `NotYetRouted`.
- `crates/utility/edgerun-http-client/src/http/server.rs` now stores one
  `HpackContext` for its HTTP/2 server adapter copy, maps decode failures to
  connection-level `COMPRESSION_ERROR`, can use the deterministic response
  encoder for the current 200 response shape, and applies client
  `SETTINGS_HEADER_TABLE_SIZE` through `HpackContext::set_max_table_size`.
- `crates/node/edgerun-node/src/http/server.rs` now stores one `HpackContext`
  for its HTTP/2 server adapter copy, maps decode failures to connection-level
  `COMPRESSION_ERROR`, can use the deterministic response encoder for the
  current 200 response shape, and applies client
  `SETTINGS_HEADER_TABLE_SIZE` through `HpackContext::set_max_table_size`.
- The HTTP/2 client mirrors in `crates/utility/edgerun-http-client` and
  `crates/node/edgerun-node` apply remote `SETTINGS_HEADER_TABLE_SIZE` to their
  connection `HpackContext`. The stale deleted-crate `decoder` field path is
  gone.
- `standards/runners/hpack-response-encoder-proof.js` proves the direct 200
  response encoder byte sequence against `http-prefix-int.wat` prefix emission
  and `hpack-string.wat` raw string records.

Next replacement target: add the actual host WebAssembly invocation surface for
`hpack-header-block.wat`, `hpack-string.wat`, and `hpack-huffman.wat`, then have
`decode_header_block` call that host adapter instead of returning
`NotYetRouted`.

## WAT replacement map

Use these modules and exports for byte work:

| Byte/runtime need | WAT module/export | Runtime responsibility |
| --- | --- | --- |
| HPACK prefixed integer decode/encode | `http-prefix-int.wat::hpack_prefix_int_decode`, `hpack_prefix_int_encode` | map status to HTTP/2 `COMPRESSION_ERROR`; pass representation-specific prefix widths and leading flags |
| QPACK prefixed integer decode/encode | `http-prefix-int.wat::qpack_prefix_int_decode`, `qpack_prefix_int_encode` | keep QPACK stream buffering, required insert count, and HTTP/3 error mapping outside WAT |
| HPACK string span/decode | `hpack-string.wat::hpack_string_scan`, `hpack_string_decode` | own output allocation, UTF-8/header validation, and header list construction |
| HPACK/QPACK Huffman validation/decode | `hpack-huffman.wat::hpack_huffman_validate`, `hpack_huffman_decode` | decide whether invalid strings become HTTP/2 `COMPRESSION_ERROR` or QPACK decoder errors |
| HPACK header-block structural scan | `hpack-header-block.wat::hpack_header_instruction_decode`, `hpack_header_block_scan` | resolve static/dynamic indexes, mutate HPACK table, apply table-size update limits |
| HPACK table arithmetic | `hpack-table-core.wat::hpack_table_entry_size`, `hpack_table_insert_plan`, `hpack_table_resize_plan` | store entries, newest-first eviction, SETTINGS max-size enforcement, lookup strategy |
| QPACK prefix strings | `qpack-string.wat::qpack_string_scan`, `qpack_string_decode` | own output allocation and header validation |
| QPACK encoder stream scan | `qpack-encoder-stream.wat::qpack_encoder_instruction_decode`, `qpack_encoder_stream_scan` | mutate decoder dynamic table, emit insert count increment, retain incomplete stream bytes |
| QPACK decoder stream scan | `qpack-decoder-stream.wat::qpack_decoder_instruction_decode`, `qpack_decoder_stream_scan` | update known received count, unblock/forget streams, handle stream cancellation |

## Runtime state that must remain outside WAT

- HPACK static table contents and the merged 1-based static/dynamic index space.
- HPACK dynamic table storage, lookup, insertion, eviction, and SETTINGS-bound
  max table size.
- HPACK encoder representation choice: exact indexed hit, name-only literal,
  incremental indexing, never-indexed, or raw literal.
- HPACK decoder connection context shared across HEADERS and CONTINUATION
  sequences.
- HTTP/2 pseudo-header ordering, duplicate checks, connection-header rejection,
  lowercase header-name checks, request/response validation, and stream state.
- QPACK dynamic table storage, absolute/relative/post-base index resolution,
  required insert count, base calculation, blocked stream accounting, known
  received count, and encoder/decoder unidirectional stream state.
- HTTP/3 stream type handling, critical stream lifetime, settings negotiation,
  and QPACK error mapping.

## Exact HPACK HTTP/2 call sites

Protocol server:

- `crates/protocol/edgerun-protocols/src/http/http2/server/headers.rs`,
  `response.rs`, and `data.rs` now use `HpackContext` instead of the retired
  HPACK crate names.
- Decode calls intentionally return adapter `NotYetRouted` until the runtime can
  invoke WAT modules. Encode calls now produce deterministic direct HPACK bytes
  for the existing server 200 response path.
- `crates/protocol/edgerun-protocols/src/http/http2/server/tests.rs` uses
  `HpackContext` and literal header-block bytes. The surviving tests prove
  HTTP/2 frame/state behavior before decode, and decode still returns explicit
  `COMPRESSION_ERROR` until WAT invocation lands.

HTTP client crate:

- `crates/utility/edgerun-http-client/src/http/server.rs` now imports
  `HpackContext`, constructs `HpackContext::new()`, calls
  `decode_header_block`, and can use the protocol adapter's deterministic
  response encoder for the current 200 response shape.
- `crates/utility/edgerun-http-client/src/http/http2/mod.rs` and
  `client.rs` already reexport/use the protocol adapter names.

Node crate mirror:

- `crates/node/edgerun-node/src/http/server.rs` now imports `HpackContext`,
  constructs `HpackContext::new()`, calls `decode_header_block`, and returns
  `COMPRESSION_ERROR` for decode until host WAT invocation lands.
- `crates/node/edgerun-node/src/http/http2/mod.rs` and `client.rs` already
  reexport/use the protocol adapter names.

## Exact QPACK HTTP/3 targets

QPACK is not part of `edgerun-hpack`, but it uses the same WAT byte kernels and
should follow after the HTTP/2 adapter is in place:

- `crates/protocol/edgerun-protocols/src/http/http3/qpack/prefix_int.rs:20-76`
  duplicates QPACK prefixed integer encode/decode.
- `crates/protocol/edgerun-protocols/src/http/http3/qpack/prefix_string/decode.rs`
  owns Huffman/raw string decode through a Rust bit-window table.
- `crates/protocol/edgerun-protocols/src/http/http3/qpack/block.rs:85-201`
  decodes/encodes the field section prefix.
- `block.rs:66-82`, `:210-258`, `:282-389` classify and decode QPACK field
  line representations and literal strings.
- `crates/protocol/edgerun-protocols/src/http/http3/qpack/stream.rs:58-71`,
  `:95-215`, and `:263-323` classify and decode QPACK encoder/decoder stream
  instructions.
- `crates/protocol/edgerun-protocols/src/http/http3/qpack/decoder.rs:89-137`
  and `:139-176` own dynamic decode and encoder-stream mutation. Keep this
  state in runtime code; replace only byte instruction parsing with WAT records.
- `crates/protocol/edgerun-protocols/src/http/http3/qpack/encoder.rs:52-89`
  and `:91-109` own dynamic encode and decoder-stream feedback. Keep table and
  blocked-stream policy here.
- `crates/protocol/edgerun-protocols/src/http/http3/qpack/convenience.rs:47-64`,
  `:80-90`, `:133-157` are the public `QpackEncoder`/`QpackDecoder` bridge used
  by HTTP/3 connection code.
- `crates/utility/edgerun-http-client/src/http/http3/connection.rs` and
  `crates/node/edgerun-node/src/http/http3/connection.rs` store `QpackEncoder`
  and `QpackDecoder`, feed encoder streams through `on_encoder_stream`, update
  known received count from decoder streams, and call request/response
  encode/decode helpers. Those connection files should not learn byte parsing.

## Agent work packets

### 1. Replace the protocol HPACK reexport with adapter skeleton

Status: landed as skeleton.

Scope:

- Edit only `crates/protocol/edgerun-protocols/src/http/http2/hpack.rs` and
  direct protocol tests in a later implementation pass.
- Define runtime-owned names such as `HpackContext`, `HpackDecodeError`,
  `HpackEncodeError`, and compact scanned-record structs.
- Do not import `edgerun_hpack`.
- Do not implement dynamic table yet unless the agent also wires packet 2.

Acceptance:

- `rg "pub use edgerun_hpack|edgerun_hpack" crates/protocol/edgerun-protocols/src/http/http2`
  finds no protocol HTTP/2 references.
- The adapter doc comments explicitly state that WAT scans bytes and runtime
  code resolves tables and validates headers.

### 2. Implement HPACK decode context over WAT header-block records

Status: boundary landed, runtime invocation pending.

Scope:

- Replace `decoder.decode(...)` behavior in
  `edgerun-protocols/src/http/http2/server/headers.rs`.
- The adapter must call `hpack_header_block_scan`, `hpack_string_decode`, and
  `hpack_huffman_decode` as needed once a host invocation surface exists.
- Current protocol code has exact record structs/status mapping and
  `decode_wat_records` for static indexed fields and raw literal strings.
- Resolve static table indexes and maintain HPACK dynamic table state in the
  adapter.
- Use `hpack_table_entry_size`, `hpack_table_insert_plan`, and
  `hpack_table_resize_plan` for arithmetic only.

Acceptance:

- Protocol handlers keep calling `HpackContext::decode_header_block`; the
  implementation performs WAT-backed structural scan and table resolution.
- Bad prefix integer, truncated string, invalid Huffman padding, invalid table
  index, oversized dynamic update, and header-list success cases have focused
  tests or runner evidence.
- Update the current protocol server tests that expect `NotYetRouted`
  `COMPRESSION_ERROR` so valid header blocks reach request validation again.

### 3. Implement minimal HPACK response encoder over WAT prefix/string records

Status: deterministic direct response path landed with WAT composition proof;
runtime WAT prefix/string invocation pending.

Scope:

- `response.rs` and the indirect `data.rs` dependency no longer use `Encoder`;
  they call `HpackContext::encode_header_block`.
- Start with deterministic raw string literals and static indexed `:status 200`
  where appropriate.
- Use `hpack_prefix_int_encode`; copy literal bytes directly. Do not add
  Huffman encode as a blocker.
- Keep table selection and insertion policy in the runtime adapter.

Acceptance:

- `response::respond_with_200` keeps calling
  `HpackContext::encode_header_block`.
- Protocol server can emit the existing 200 response header block directly
  without importing `edgerun-hpack`.
- `standards/runners/hpack-response-encoder-proof.js` proves the direct bytes
  for `:status: 200`, `content-type: text/plain`, and `content-length: 2`
  compose from `http-prefix-int.wat` prefixes and `hpack-string.wat` raw string
  records.
- Next pass should replace direct prefix/string emission with host calls to
  `http-prefix-int.wat` and `hpack-string.wat`, or keep the direct path as the
  bootstrap oracle until the host invocation surface exists.

### 4. Collapse downstream HTTP/2 mirrors and direct error conversions

Status: `src/http/http2/*` mirror modules and the two wider
`src/http/server.rs` HTTP/2 server adapters are converted to `HpackContext`.

Scope:

- Keep `edgerun-http-client` and `edgerun-node` routed through the protocol
  adapter context as packets 2-3 land.
- Replace any reintroduced `Decoder::new()`/`Encoder::new()` state in both
  HTTP/2 clients and servers with `HpackContext`.
- Delete `impl From<edgerun_hpack::DecoderError> for Http2Error` in both mirror
  crates and map adapter statuses/errors instead.

Acceptance:

- These files no longer mention `edgerun_hpack`, `Decoder`, or `Encoder` from
  the retired HPACK crate:
  `crates/utility/edgerun-http-client/src/http/http2/mod.rs`,
  `crates/utility/edgerun-http-client/src/http/http2/client.rs`,
  `crates/utility/edgerun-http-client/src/http/server.rs`,
  `crates/node/edgerun-node/src/http/http2/mod.rs`,
  `crates/node/edgerun-node/src/http/http2/client.rs`,
  `crates/node/edgerun-node/src/http/server.rs`.

### 5. Remove manifest and compatibility exposure after callers are clean

Scope:

- Remove the `edgerun-hpack` workspace member or tombstone it according to the
  crate-retirement convention active on the branch.
- Remove `hpack = ["dep:edgerun-hpack"]` and direct optional dependencies from
  `edgerun-encoding`, `edgerun-protocols`, `edgerun-http-client`, and
  `edgerun-node`.
- Update `standards/protocols/hpack.toml` and codec manifests so WAT modules are
  the implementation references and Rust sources are historical refs only.

Acceptance:

- `rg "edgerun_hpack|edgerun-hpack" crates Cargo.toml --glob '*.rs' --glob 'Cargo.toml'`
  returns no live Rust or manifest dependency.
- WAT runner evidence from `standards/deletion-edgerun-hpack.md` remains listed
  in the deletion commit.

### 6. Replace QPACK leaf byte parsers with WAT records

Scope:

- Start in `qpack/prefix_int.rs`, `qpack/prefix_string/decode.rs`,
  `qpack/block.rs`, and `qpack/stream.rs`.
- Use `qpack_prefix_int_*`, `qpack_string_*`,
  `qpack_encoder_instruction_decode`, `qpack_encoder_stream_scan`,
  `qpack_decoder_instruction_decode`, and `qpack_decoder_stream_scan`.
- Keep public `QpackEncoder`/`QpackDecoder` shape until HTTP/3 connection code
  is migrated.

Acceptance:

- Field-section prefix and stream-instruction byte parsing no longer duplicates
  the Rust prefix/string/Huffman kernels.
- Existing dynamic table modules still own state and policy.

### 7. Keep HTTP/3 connection state behind QPACK convenience adapter

Scope:

- Update both HTTP/3 connection mirrors only after packet 6.
- Preserve `qpack_encoder_stream_id`, `qpack_decoder_stream_id`, critical stream
  closure checks, request/response helper APIs, and settings interactions.
- The connection layer should call adapter methods, not WAT exports directly.

Acceptance:

- HTTP/3 connection files do not parse QPACK instruction bytes directly.
- QPACK stream data is buffered and consumed according to scanner `consumed`
  lengths; partial records remain in connection/runtime buffers.

## Suggested execution order

1. Packet 1: create the protocol HPACK adapter boundary.
2. Packet 2: move HTTP/2 request/trailer decode onto WAT scanner records.
3. Packet 3: move HTTP/2 response encode onto WAT prefix/string emission.
4. Packet 4: update `edgerun-http-client` and `edgerun-node` mirrors.
5. Packet 5: delete Cargo/manifest exposure to `edgerun-hpack`.
6. Packet 6: replace QPACK leaf parsing with WAT records.
7. Packet 7: clean HTTP/3 connection use of the QPACK adapter.

Do not run packet 5 until packets 1-4 remove every live HTTP/2 caller. Do not
run packets 6-7 as a prerequisite for deleting `edgerun-hpack`; they are related
pipeline cleanup, not blockers for HPACK crate retirement.

## Proof commands for implementation agents

Standards/WAT runners to keep attached to implementation PRs:

```bash
node standards/runners/http-prefix-int-smoke.js
node standards/runners/hpack-string-smoke.js
node standards/runners/hpack-response-encoder-proof.js
node standards/runners/hpack-huffman-smoke.js
node standards/runners/hpack-header-block-smoke.js
node standards/runners/hpack-table-core-smoke.js
node standards/runners/codec-composition-http2-hpack.js
node standards/runners/codec-composition-http2-hpack-huffman.js
node standards/runners/qpack-string-smoke.js
node standards/runners/qpack-encoder-stream-smoke.js
node standards/runners/qpack-decoder-stream-smoke.js
node standards/runners/codec-composition-http3-qpack.js
node standards/runners/codec-composition-http3-qpack-huffman.js
```

Cargo checks are intentionally not part of this standards-only queue. Agents
that implement Rust rewrites should state whether the branch still has known
deleted-source Cargo breakage before claiming build success.
