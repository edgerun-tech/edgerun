# WAT Encoder/Decoder Roadmap

This roadmap ranks the first EdgeRun encoder/decoder primitives to convert into
small deterministic WAT modules. The goal is not to make a second internal wire
format. Edgerun internal records remain rkyv-only; these modules cover external
standards, byte codecs, protocol scanners, and conformance components.

## Ground Rules

- Keep modules deterministic: no clocks, entropy, syscalls, threads, host
  callbacks, or allocation-dependent behavior.
- Keep outputs offset-based when possible. Parsers should return spans into the
  caller-provided input rather than copying payloads.
- Start with scanners, validators, and header/framing encoders before full
  object models.
- Use existing Rust tests and known vectors as the source of truth.
- Avoid converting cryptographic signing, TLS AEAD, full X.509, and policy
  decisions until byte-format kernels are proven.

## ABI Shape

Use the existing standard-module convention:

```text
proto_abi_version() -> i32
proto_standard_id() -> i32
memory
```

For byte functions, prefer fixed-buffer exports:

```text
function(input_ptr, input_len, out_ptr, out_cap) -> u64
```

Pack `u64` returns as:

```text
low32  = status
high32 = bytes_consumed | bytes_written | scalar result
```

Common status values:

```text
0 ok
1 input_short
2 output_short
3 invalid
4 overflow
5 truncated
6 too_long
```

For structured parser results, write fixed-width records to `out_ptr`. Use
little-endian fields inside result records unless a standard explicitly requires
the returned bytes to preserve wire order.

One compatibility issue needs cleanup before generating new modules:
`standards/abi/standard-module-v1.md` says `proto_abi_version` is `2`, while
`standards/generators/sha512-family-wat.py` emits `1`.

## Phase 1: Core Byte Primitives

These should be converted first because every protocol parser can reuse them.

| Rank | Primitive | Source | Initial WAT exports |
| --- | --- | --- | --- |
| 1 | BE/LE byteorder | `crates/utility/edgerun-encoding/src/byteorder.rs` | `read_u16_be`, `read_u24_be`, `read_u32_be`, `read_u64_be`, LE variants, checked writes |
| 2 | LEB128 varint | `crates/utility/edgerun-encoding/src/varint.rs` | `varint_encode_u64`, `varint_decode_u64` |
| 3 | QUIC varint | `crates/utility/edgerun-encoding/src/quic_varint.rs` | `quic_varint_encode_u64`, `quic_varint_decode_u64` |
| 4 | CRC32/IEEE | `crates/utility/edgerun-encoding/src/crc32.rs` | `crc32`, `crc32_update` |
| 5 | Adler32 | deleted `crates/utility/edgerun-adler2`; behavior is WAT-owned | `adler32`, `adler32_update` |
| 6 | Strict hex | `crates/utility/edgerun-encoding/src/hex.rs` | `hex_encode_lower`, `hex_decode_strict` |
| 7 | Base64url no-pad | `crates/utility/edgerun-encoding/src/base64.rs` | `base64url_nopad_encode`, `base64url_nopad_decode` |

Known vectors already covered by Rust tests include:

- LEB128: `0 -> 00`, `127 -> 7f`, `128 -> 80 01`, `300 -> ac 02`.
- QUIC varint: `63 -> 3f`, `64 -> 40 40`, `16383 -> 7f ff`.
- CRC32: `crc32("hello") == 0x3610a686`.
- Adler32: empty input is `1`, `Wikipedia -> 0x11e60398`.
- Base64: `foo -> Zm9v`, base64url no-pad `foob -> Zm9vYg`.

Verification commands used before source retirement:

```bash
cargo test -p edgerun-encoding
cargo test -p edgerun-encoding --no-default-features
```

`edgerun-adler2` is now deleted. Adler behavior is verified through
`encoding-core-smoke.js` and composition runners, not Cargo.

## Phase 2: HTTP And WebSocket Scanners

Do not convert `HttpRequest`, `HttpResponse`, `HeaderMap`, or URI construction
first. Convert span-based scanners and validators.

Initial HTTP exports:

```text
http_find_crlf(input_ptr, input_len, start) -> u64
http_find_double_crlf(input_ptr, input_len, start) -> u64
http_validate_header_name(ptr, len) -> i32
http_validate_header_value(ptr, len) -> i32
http_value_has_token(value_ptr, value_len, token_ptr, token_len) -> i32
http_parse_request_line(ptr, len, out_ptr) -> i32
http_parse_status_line(ptr, len, out_ptr) -> i32
http_next_header(ptr, len, start, out_ptr) -> i32
http_parse_chunk_step(ptr, len, start, out_ptr) -> i32
```

Source files:

- `crates/protocol/edgerun-protocols/src/http/header.rs`
- `crates/protocol/edgerun-protocols/src/http/message.rs`
- `crates/protocol/edgerun-protocols/src/http/chunked.rs`
- `crates/utility/edgerun-encoding/src/chunked.rs`

Initial WebSocket exports:

```text
ws_decode_prefix(ptr, len, out_ptr) -> i32
ws_decode_payload_len(ptr, len, payload_len_code, max_len, out_ptr) -> i32
ws_apply_mask_in_place(payload_ptr, payload_len, mask_u32) -> i32
ws_write_server_frame_header(opcode, payload_len, out_ptr, out_cap) -> u64
```

Source file:

- `crates/protocol/edgerun-protocols/src/websocket.rs`

Verification commands used for this slice:

```bash
cargo test -p edgerun-encoding chunked
cargo test -p edgerun-protocols --features http,websocket chunked
cargo test -p edgerun-protocols --features http,websocket websocket
```

## Phase 3: TLS And DER Framing

Start with DER and TLS headers only. Defer TLS AEAD, certificate generation,
signature verification, and full `ClientHelloBuilder`.

Initial DER exports:

```text
der_len_decode(in_ptr, in_len) -> u64
der_len_encode(value, out_ptr, out_cap) -> u64
der_tag_decode(in_ptr, in_len) -> u64
der_header_decode(in_ptr, in_len) -> u64
```

Source files:

- `crates/utility/edgerun-crypto/src/der/length.rs`
- `crates/utility/edgerun-crypto/src/der/tag.rs`
- `crates/utility/edgerun-crypto/src/der/header.rs`

Initial TLS exports:

```text
tls_record_header_decode(in_ptr, in_len) -> u64
tls_record_header_encode(content_type, version, fragment_len, out_ptr, out_cap) -> u64
tls_handshake_header_decode(in_ptr, in_len) -> u64
tls_handshake_header_encode(handshake_type, body_len, out_ptr, out_cap) -> u64
tls_vector_u16_decode(in_ptr, in_len, out_ptr) -> i32
tls_vector_u24_decode(in_ptr, in_len, out_ptr) -> i32
```

Source files:

- `crates/protocol/edgerun-protocols/src/tls/record.rs`
- `crates/protocol/edgerun-protocols/src/tls/handshake.rs`
- `crates/protocol/edgerun-protocols/src/tls/session_ticket.rs`

## Phase 4: JSON, TOML, YAML Text Tapes

Do not convert full `JsonValue`, `Map`, or pretty serializers first. Convert
text scanners and token tapes.

Use a fixed-width 20-byte token:

```text
kind: u32
start: u32
end: u32
parent: u32
flags: u32
```

Initial exports:

```text
json_parse_tape(input_ptr, input_len, token_ptr, token_cap, scratch_ptr, scratch_len) -> u64
json_scan_string(input_ptr, input_len, offset) -> u64
json_scan_number(input_ptr, input_len, offset) -> u64
json_escape_string(input_ptr, input_len, out_ptr, out_cap) -> u64
toml_scan_line(input_ptr, input_len, offset, out_ptr) -> i32
toml_scan_scalar(input_ptr, input_len, out_ptr) -> i32
yaml_scan_line(input_ptr, input_len, offset, out_ptr) -> i32
```

Source files:

- `crates/utility/edgerun-json/src/parse.rs`
- `crates/utility/edgerun-json/src/tape.rs`
- `crates/utility/edgerun-json/src/util.rs`
- `crates/utility/edgerun-json/src/toml_parse.rs`
- `crates/utility/edgerun-json/src/yaml_parse.rs`

Verification commands used for this slice:

```bash
cargo test -p edgerun-json
cargo test -p edgerun-json --features toml,yaml
cargo test -p edgerun-json --no-default-features
```

## First Implementation Batch

The first concrete batch should be small enough to prove end to end:

1. `encoding-core.wat`: byteorder, LEB128 varint, QUIC varint, CRC32, Adler32.
2. `encoding-text.wat`: strict hex and base64url no-pad.
3. `http1-scan.wat`: CRLF scanners, header validators, token matcher, chunk step.
4. `ws-frame.wat`: WebSocket prefix, payload length, mask, server header writer.
5. `der-tlv.wat`: DER length, tag, and header.
6. `tls-frame.wat`: TLS record and handshake headers.
7. `json-tape.wat`: JSON token tape, string scan, number scan.

Each module should get:

- a generator or checked-in WAT source,
- a manifest under `standards/components/manifests/`,
- a runner under `standards/runners/`,
- corpus cases under `standards/corpus/`,
- a Rust parity test or runner that compares WAT output to the existing crate.

## Current Batch Closeout

The first checked-in WAT batch now exists under
`standards/build/wasm/codec-primitives/`, with smoke runners under
`standards/runners/`.

Current hardening status:

1. `encoding-core.wat`: edge-vector smoke coverage for short reads, LEB128
   truncation/overflow, `u64::MAX`, QUIC thresholds, CRC32, and Adler32.
2. `encoding-text.wat`: strict hex plus canonical base64url no-pad; nonzero
   unused tail bits are rejected.
3. `http1-scan.wat`: header names follow HTTP token characters; header values
   are limited to HTAB, SP, and visible ASCII.
4. `ws-frame.wat`: rejects reserved opcodes, fragmented or oversized control
   frames, non-minimal extended lengths, and high-bit 64-bit lengths.
5. `der-tlv.wat`: DER length/tag/header smoke coverage exists, but needs wider
   boundary tests before larger DER objects depend on it.
6. `tls-frame.wat`: validates record content type, legacy version range, and
   16 KiB record fragment limit.
7. `json-tape.wat`: covers common malformed structures, invalid escapes,
   invalid unicode escape shape, trailing junk, and too-small token output.

Current WAT smoke command:

```bash
node standards/runners/encoding-core-smoke.js
node standards/runners/encoding-text-smoke.js
node standards/runners/http1-scan-smoke.js
node standards/runners/ws-frame-smoke.js
node standards/runners/der-tlv-smoke.js
node standards/runners/tls-frame-smoke.js
node standards/runners/json-tape-smoke.js
```

Before treating this batch as production-ready, add parity runners against the
Rust crates named in each phase. The smoke tests prove local WAT behavior, not
full equivalence with the Rust implementations.

## Second Implementation Batch

These modules build directly on the first batch and avoid full object models,
allocation-heavy serializers, or crypto policy:

1. `http1-lines.wat`: `http_parse_request_line`,
   `http_parse_status_line`, `http_next_header`.
   - Parity sources: `crates/protocol/edgerun-protocols/src/http/message.rs`,
     `method.rs`, `status.rs`, and `header.rs`.
   - Tests: request/status lines, missing spaces, bad versions, invalid header
     names/values, and missing CRLF.
2. `http1-body.wat`: `http_parse_content_length`,
   `http_has_transfer_token`, `http_classify_body_framing`.
   - Parity sources: `crates/protocol/edgerun-protocols/src/http/chunked.rs`
     and `message.rs`.
   - Tests: duplicate content length, overflow, mixed-case chunked transfer
     tokens, and conflicting transfer/content-length inputs.
3. `http2-frame.wat`: `http2_frame_header_decode`,
   `http2_frame_header_encode`, `http2_frame_type_classify`.
   - Parity source:
     `crates/protocol/edgerun-protocols/src/http/http2/frame/mod.rs`.
   - Tests: DATA, HEADERS, SETTINGS, PING, reserved stream bit handling, max
     frame size rejection, short header, and incomplete payload.
4. `http-prefix-int.wat`: HPACK/QPACK prefix integer encode/decode.
   - Parity sources: `crates/utility/edgerun-hpack/src/decoder.rs` and
     `crates/protocol/edgerun-protocols/src/http/http3/qpack/prefix_int.rs`.
   - Tests: prefix sizes 5/7/8, continuation bytes, truncation, overlong
     encodings, and flag-bit preservation.
5. `http3-frame.wat`: `http3_frame_header_decode`,
   `http3_frame_header_encode`, `http3_frame_type_classify`.
   - Parity sources:
     `crates/protocol/edgerun-protocols/src/http/http3/frame.rs` and
     `http3/varint.rs`.
   - Tests: DATA, HEADERS, SETTINGS, reserved/unknown types, declared length
     beyond input, and QUIC varint width thresholds.
6. `tls-vector.wat`: `tls_vector_u8_decode`, `tls_vector_u16_decode`,
   `tls_vector_u24_decode`, `tls_extension_next`, `tls_alpn_next`.
   - Parity sources: `crates/protocol/edgerun-protocols/src/tls/handshake.rs`
     and `tls/tls_alpn.rs`.
   - Tests: SNI, ALPN, supported versions, extension-list length mismatch,
     truncated vectors, and zero-length ALPN rejection.
7. `tls-name.wat`: `tls_dns_name_normalize`, `tls_dns_name_matches`.
   - Parity source:
     `crates/protocol/edgerun-protocols/src/tls/name_match.rs`.
   - Tests: case folding, trailing dot normalization, wildcard matching, IP
     literal rejection, bad label rejection, and multi-label wildcard rejection.
8. `pem-rfc7468.wat`: `pem_find_boundaries`, `pem_validate_label`,
   `pem_compact_base64`, `pem_encoded_len`.
   - Parity source: PEM support in `crates/utility/edgerun-crypto`, plus
     `crates/utility/edgerun-encoding/src/base64.rs`.
   - Tests: certificate label, LF/CRLF, mismatched boundaries, invalid labels,
     headers disallowed, and invalid base64.
9. `json-scalar.wat`: `json_scan_string_strict`,
   `json_scan_number_strict`, `json_unescape_string`, `json_parse_i64`,
   `json_parse_u64`.
   - Parity sources: `crates/utility/edgerun-json/src/parse.rs`,
     `number.rs`, and `util.rs`.
   - Tests: surrogate pairs, lone surrogate rejection, control characters,
     integer min/max, overflow, leading-zero invalid numbers, and exponents.
10. `toml-scan.wat`: `toml_scan_line`, `toml_scan_key_value`,
    `toml_scan_table_header`, `toml_scan_scalar`.
    - Parity source: `crates/utility/edgerun-json/src/toml_parse.rs`.
    - Tests: comments, blank lines, table headers, quoted and literal strings,
      booleans, numeric bases, arrays, and missing delimiter errors.

Suggested parallel split:

- Agent A: `http1-lines.wat` and `http1-body.wat`.
- Agent B: `http2-frame.wat` and `http-prefix-int.wat`.
- Agent C: `http3-frame.wat`.
- Agent D: `tls-vector.wat` and `tls-name.wat`.
- Agent E: `pem-rfc7468.wat`.
- Agent F: `json-scalar.wat` and `toml-scan.wat`.

Second batch status:

- Implemented WAT modules and smoke runners for all six slices.
- Assigned unique `proto_standard_id` values:
  `300008..300017`.
- Consolidated the codec primitive README module-id registry.
- Added TOML component manifests for all 17 codec primitive modules.
- Added JSON corpus seed fixtures under `standards/corpus/codec-primitives/`.
- Added a JS reference parity-foundation runner for selected low-risk
  primitives.
- Smoke runners pass for all current codec primitive modules.

Second batch smoke command:

```bash
for f in $(ls standards/runners/*smoke.js | sort); do
  node "$f"
done
```

Remaining work before production use:

- Replace or supplement the JS reference parity foundation with Rust-backed
  parity runners for each module against the listed source crates.
- Wire corpus fixtures into executable conformance runners.
- Hash compiled WASM artifacts and fill manifest `sha256` fields once the build
  path is finalized.
- Add broader malformed-input and cross-module composition cases before
  production PEM/certificate flows rely on these modules.

## Third Implementation Batch

The next high-value batch extends the primitive stack into header strings,
certificate scaffolding, TLS handshake routing, DNS, and URL/form parsing:

```text
300018 hpack-huffman
300019 hpack-string
300020 qpack-string
300021 der-oid
300022 der-asn1-basic
300023 tls-clienthello
300024 tls-certificate-list
300025 dns-name
300026 dns-message-header
300027 percent-url-form
```

Composition unlocked:

- `http-prefix-int` -> `hpack-huffman` -> `hpack-string` / `qpack-string`.
- `der-tlv` -> `der-oid` / `der-asn1-basic` -> certificate/SPKI scanners.
- `tls-frame` -> `tls-clienthello` -> `tls-vector` / `tls-name`.
- `tls-frame` -> `tls-certificate-list` -> `der-tlv` / `der-asn1-basic`.
- `encoding-core` -> `dns-name` / `dns-message-header`.
- `encoding-text` / `http1-lines` -> `percent-url-form`.

Third batch status:

- Implemented all 10 WAT modules with smoke runners.
- Added TOML component manifests and JSON corpus fixtures for each module.
- Updated the codec primitive README registry and corpus index.
- Smoke runners pass for all current codec primitive modules.
- Added composition runners proving current span boundaries compose across:
  `tls-frame -> tls-clienthello -> tls-vector -> tls-name`,
  `tls-frame -> tls-certificate-list -> der-tlv -> der-asn1-basic -> der-oid`,
  `http2-frame -> hpack-string -> hpack-huffman`, and
  `http3-frame -> qpack-string -> hpack-huffman`.

Third batch composition command:

```bash
for f in standards/runners/codec-composition-*.js; do node "$f"; done
```

Remaining work before production use:

- Add Rust-backed parity runners for HPACK/QPACK strings, DER ASN.1/OID, TLS
  ClientHello/cert-list, DNS names/headers, and percent/form/URI scanning.
- Decide whether DNS compression-pointer following belongs in `dns-name.wat` or
  a separate full DNS-message traversal module.
- Add full HPACK/QPACK header-block scanners after string parity is proven.

## Rust Parity Evidence

Rust-backed parity runners now exist under `standards/runners/`:

```text
rust-parity-encoding.js
rust-parity-http-ws.js
rust-parity-der-tls.js
rust-parity-misc-codecs.js
```

Current parity summary:

- Encoding/text: 72 asserted Rust-oracle parity cases. Known divergence:
  canonical WAT base64url rejects nonzero unused tail bits that current Rust
  decode accepts.
- HTTP/WebSocket: 49 Rust-oracle cases. Covers scanner/framing helpers, not
  full HTTP message parsing, HPACK/QPACK dynamic tables, or WebSocket upgrade
  hashing.
- DER/PEM/TLS: 42 exact matches plus 3 WAT-stricter mismatches: lowercase PEM
  labels, unknown TLS record content type, and future TLS record version.
- DNS/URL/JSON/TOML: 28 cases, 19 exact matches, 9 documented mismatches where
  WAT is stricter or scanner-only: DNS compression pointers, permissive percent
  decode/form decode, and permissive TOML fallback-to-string behavior.

Best first replacement/adaptation targets:

- `encoding-core`: byteorder, LEB128, QUIC varint, CRC32, Adler32.
- `http2-frame` and `http3-frame`: fixed frame header encode/decode/classify.
- `http-prefix-int`, `hpack-huffman`, `hpack-string`, `qpack-string`:
  integer/string helpers before dynamic table logic.
- `ws-frame`: frame prefix, payload length, mask, server frame header helpers.
- `der-tlv`, `der-oid`, `der-asn1-basic`: scanner-level DER surfaces.
- `tls-name`: DNS name normalization and matching.
- `dns-message-header`: fixed 12-byte DNS header guards.
- `json-scalar`: strict scalar validation and integer bounds.

Do not delete these Rust surfaces yet:

- Full HTTP/1 parsing, chunk decoding, and body parsing.
- HPACK/QPACK dynamic-table and full header-block logic.
- WebSocket HTTP upgrade and accept-key hashing.
- TLS crypto, certificate chain validation, signature checks, key handling, and
  policy decisions.
- DNS compressed-name/message traversal.
- Permissive percent/form decode callers unless their policy changes to strict
  canonical decoding.
- Full TOML parsing while Rust still accepts malformed scalar-looking values as
  strings.

## Explicit Non-Goals For The First Pass

- Do not convert Edgerun internal rkyv records into WAT wire formats.
- Do not convert full TLS encryption/decryption first.
- Do not convert Ed25519/P-256/RSA verification first.
- Do not convert full JSON/TOML/YAML object allocation models first.
- Do not convert HTTP URI normalization or full request/response builders first.
- Do not add external dependencies.
