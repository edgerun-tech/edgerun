# WAT Codec Replacement Audit

This audit covers the current WAT codec primitive batch under
`standards/build/wasm/codec-primitives`. It is a deletion/replacement plan only.
No Rust source should be deleted until a WAT host adapter and Rust-backed parity
tests are landed for the exact call site being replaced.

## Summary

Best first replacement targets:

1. HTTP/2 frame header encode/decode in
   `crates/protocol/edgerun-protocols/src/http/http2/frame/mod.rs`.
2. HTTP/3 frame header encode/decode in
   `crates/protocol/edgerun-protocols/src/http/http3/frame.rs`.
3. WebSocket frame prefix, payload length, mask, and server frame-header helpers
   in `crates/protocol/edgerun-protocols/src/websocket.rs`.
4. HPACK/QPACK prefix/string/Huffman helper paths, not full dynamic-table
   header decoding.
5. DNS fixed header/count preflight in
   `crates/protocol/edgerun-protocols/src/dns/message.rs` and
   `crates/protocol/edgerun-protocols/src/dns/limits.rs`.

Keep Rust as canonical for now where code owns allocation-heavy object models,
semantic validation, crypto, key handling, policy, or permissive compatibility
behavior.

## Safe WAT Adapter Candidates

### HTTP/2 Frame Headers

- Rust: `crates/protocol/edgerun-protocols/src/http/http2/frame/mod.rs`
- Functions/types: `Frame::to_bytes`, `Frame::from_bytes`, `FrameType::from_u8`
- WAT: `http2-frame.wat`
- Evidence: `Frame::to_bytes` writes a fixed 9-byte header plus payload; `Frame::from_bytes`
  reads length/type/flags/stream id, checks `max_frame_size`, checks payload
  availability, and returns consumed length.
- Active call sites:
  - `crates/utility/edgerun-http-client/src/http/http2/connection.rs`
  - `crates/utility/edgerun-http-client/src/http/http2/client.rs`
  - HTTP/2 conformance tests under `crates/utility/edgerun-http-client/src/http/http2/*conformance.rs`
- Rationale: this is a clean byte-record boundary with no dynamic state and no
  security policy. The WAT module already reports payload length, type class,
  flags, reserved stream bit, cleared stream id, header length, and total length.
- Recommendation: first adapter patch should keep the Rust `Frame` type and use
  WAT only to parse/write the fixed frame header. Keep payload-specific
  `validate_semantics` and typed frame modules in Rust.
- Deletion candidate after adapter parity: duplicated manual header
  read/write lines inside `Frame::to_bytes` and `Frame::from_bytes`, not the
  `Frame` struct or payload-specific frame files.

### HTTP/3 Frame Headers

- Rust: `crates/protocol/edgerun-protocols/src/http/http3/frame.rs`
- Functions/types: `Http3Frame::to_bytes`, `Http3Frame::from_bytes`,
  `Http3FrameType::from_u64`
- WAT: `http3-frame.wat`
- Evidence: frame header handling is only two QUIC varints: frame type and
  payload length. Rust then switches on frame type and parses typed payloads.
- Active call sites:
  - `crates/utility/edgerun-http-client/src/http/http3/connection.rs`
  - HTTP/3 QUIC tests under `crates/utility/edgerun-http-client/src/http/http3/quic/tests.rs`
- Rationale: header scan is deterministic and already covered by Rust parity.
  Typed payload construction remains Rust-owned.
- Recommendation: add `Http3FrameHeader` adapter first. Use WAT to identify
  `frame_type`, `payload_len`, and `header_len`, then keep Rust payload parsing.
- Deletion candidate after adapter parity: duplicated QUIC-varint frame header
  scan/write in `Http3Frame::{from_bytes,to_bytes}`. Do not delete typed
  `Settings`, `Goaway`, `PushPromise`, or stream validation logic.

### WebSocket Frame Helpers

- Rust: `crates/protocol/edgerun-protocols/src/websocket.rs`
- Functions/types:
  - `decode_frame_prefix`
  - `WebSocketFramePrefix::extended_len_bytes`
  - `WebSocketFramePrefix::require_client_mask`
  - `decode_payload_len`
  - `encode_server_frame`
  - `encode_server_binary`
  - `encode_server_control`
- WAT: `ws-frame.wat`
- Active call sites:
  - `crates/protocol/edgerun-work/src/std_runtime/websocket_channel.rs`
- Rationale: frame prefix, extended length, mask application, and server header
  writing are pure byte operations. WAT is already stricter than the current Rust
  helper on non-minimal extended lengths and 64-bit high-bit payload lengths,
  which is desirable for protocol correctness.
- Recommendation: first replace `decode_frame_prefix` and `decode_payload_len`
  behind an adapter. Then replace server frame header construction while keeping
  payload allocation/write and HTTP upgrade handling in Rust.
- Deletion candidate after adapter parity: Rust frame-prefix/length/header byte
  helpers only.

### HPACK/QPACK Prefix And String Helpers

- Rust:
  - `crates/utility/edgerun-hpack/src/decoder.rs`
  - `crates/utility/edgerun-hpack/src/huffman.rs`
  - `crates/protocol/edgerun-protocols/src/http/http3/qpack/prefix_int.rs`
  - `crates/protocol/edgerun-protocols/src/http/http3/qpack/prefix_string/decode.rs`
- Functions:
  - HPACK private `decode_integer`
  - HPACK private `decode_string`
  - `HuffmanDecoder::decode`
  - QPACK `prefix_int::decode` / `prefix_int::encode`
  - QPACK prefix-string decode helpers
- WAT: `http-prefix-int.wat`, `hpack-huffman.wat`, `hpack-string.wat`,
  `qpack-string.wat`
- Rationale: prefix integers and prefix strings are compact byte transforms and
  are isolated from table mutation. WAT parity exists for representative cases.
- Recommendation: expose or wrap these as local helper adapters first. Keep the
  HPACK `Decoder` and QPACK decoder state machines in Rust.
- Deletion candidate after adapter parity: duplicate prefix/string/Huffman decode
  routines. Do not delete dynamic table, static table, or header-block state
  logic.

### DNS Fixed Header And Bounds

- Rust:
  - `crates/protocol/edgerun-protocols/src/dns/message.rs`
  - `crates/protocol/edgerun-protocols/src/dns/limits.rs`
- Functions/types:
  - `DnsHeader::to_wire`
  - `DnsHeader::from_wire`
  - `dns_section_counts`
  - `validate_dns_wire_bounds`
- WAT: `dns-message-header.wat`
- Rationale: the 12-byte DNS header and count bounds are fixed-format. This is
  a good adapter candidate because it can run before full DNS parsing.
- Recommendation: add a WAT-backed preflight adapter for count extraction and
  bounds validation. Keep `DnsMessage` and record parsing in Rust.
- Deletion candidate after adapter parity: duplicate fixed-header read/write and
  count extraction code.

### JSON Scalar Helpers

- Rust: `crates/utility/edgerun-json/src/parse.rs`
- Functions: `parse_u64_fast`, `parse_i64_fast`, string/number scanner portions
  inside `Parser`
- WAT: `json-scalar.wat`
- Rationale: strict integer parsing and string escape validation are byte-local
  and parity-covered for representative cases.
- Recommendation: use WAT first as a conformance oracle in tests, then as an
  optional adapter for proof/canonical paths. Avoid replacing full JSON parsing
  yet.
- Deletion candidate after adapter parity: duplicated scalar validation helpers,
  not the parser object model.

## Conditional Candidates Due To Strictness Mismatch

### Encoding Text

- Rust: `crates/utility/edgerun-encoding/src/base64.rs`,
  `crates/utility/edgerun-encoding/src/hex.rs`
- WAT: `encoding-text.wat`
- Mismatch: WAT base64url no-pad rejects noncanonical unused tail bits. Rust
  currently accepts examples like `AB` and `AAB`.
- Recommendation: use WAT strict decoding for canonical/proof paths. Either add
  a Rust strict mode or add a WAT permissive mode before replacing permissive
  decode call sites.

### PEM RFC7468

- Rust: `crates/utility/edgerun-crypto/src/pem_rfc7468.rs`
- WAT: `pem-rfc7468.wat`
- Mismatch: WAT rejects lowercase labels; Rust accepts lowercase labels.
- Recommendation: decide policy before replacement. For certificate/key material
  in canonical proof paths, strict uppercase labels are defensible. For import
  compatibility, keep Rust or add a permissive WAT export.

### TLS Record Header

- Rust: `crates/protocol/edgerun-protocols/src/tls/record.rs`
- Functions/types: `TlsRecord::to_bytes`, `TlsRecord::from_bytes`
- WAT: `tls-frame.wat`
- Mismatch: WAT rejects unknown content types and versions outside
  `0x0300..0x0304`; Rust accepts unknown content type `0x13` and version
  `0x0305` in the parity probe.
- Recommendation: replacement is appropriate only at policy boundaries that want
  strict record framing. Keep Rust behavior in compatibility or future-version
  tolerant paths.

### DNS Name

- Rust:
  - `crates/protocol/edgerun-protocols/src/dns/name.rs`
  - `crates/protocol/edgerun-protocols/src/dns/record.rs`
- Functions: `validate_name`, `normalize_name`, `decode_domain_name`,
  `encode_domain_name`
- WAT: `dns-name.wat`
- Mismatch/gap: WAT scans uncompressed wire names and rejects compression
  pointers. Full DNS message parsing needs compressed-name traversal.
- Recommendation: use WAT for uncompressed names, query construction, zone-like
  validation, and preflight. Do not replace compressed DNS message parsing yet.

### Percent/Form/URI

- Rust:
  - `crates/utility/edgerun-encoding/src/percent.rs`
  - `crates/utility/edgerun-form-urlencoded/src/lib.rs`
  - `crates/protocol/edgerun-protocols/src/http/uri.rs`
- WAT: `percent-url-form.wat`
- Mismatch: WAT strict percent decode rejects malformed `%` and preserves `+`;
  Rust form parsing is permissive and decodes `+` to space.
- Recommendation: use WAT for canonical URL component/proof paths and path/query
  scanning. Keep permissive form decode behavior unless a compatibility change
  is intentional.

### TOML Scanner

- Rust: `crates/utility/edgerun-json/src/toml_parse.rs`
- WAT: `toml-scan.wat`
- Mismatch: Rust falls back to strings for malformed scalar-looking values; WAT
  rejects malformed scalar cases.
- Recommendation: WAT can be used for strict lint/preflight. Do not replace the
  TOML parser until strictness policy is changed.

## Not Ready Because Scanner-Only

### HTTP/1

- Rust:
  - `crates/protocol/edgerun-protocols/src/http/message.rs`
  - `crates/protocol/edgerun-protocols/src/http/header.rs`
  - `crates/protocol/edgerun-protocols/src/http/chunked.rs`
  - `crates/protocol/edgerun-protocols/src/http/http1/chunked.rs`
- WAT: `http1-scan.wat`, `http1-lines.wat`, `http1-body.wat`
- Rationale: WAT currently provides scanners/classifiers, not a full request or
  response parser, body decoder, chunk state machine, or semantic message model.
- Recommendation: use WAT for header validation, request/status line preflight,
  and body framing classification. Do not delete full HTTP/1 Rust parsing yet.

### TLS ClientHello And Certificate List

- Rust:
  - `crates/protocol/edgerun-protocols/src/tls/server/client_hello.rs`
  - `crates/protocol/edgerun-protocols/src/tls/certificate.rs`
  - `crates/utility/edgerun-http-client/src/tls/async_tls.rs`
- WAT: `tls-clienthello.wat`, `tls-certificate-list.wat`, `tls-vector.wat`
- Rationale: WAT scans spans for ClientHello fields, SNI, ALPN, certificate
  entries, and extensions. Rust builds typed vectors and maps cipher suites,
  groups, key shares, signature algorithms, and later handshake behavior.
- Recommendation: use WAT as a preflight/span scanner in TLS and QUIC handshake
  entry points. Keep typed extraction and handshake policy in Rust.

### JSON Tape

- Rust: `crates/utility/edgerun-json/src/parse.rs`,
  `crates/utility/edgerun-json/src/tape.rs`
- WAT: `json-tape.wat`
- Rationale: WAT emits a compact token tape but does not replace `JsonValue`,
  borrowed values, object indexes, schema helpers, or conversion APIs.
- Recommendation: keep Rust parser. Use WAT as a canonical scanner/conformance
  layer for browser/WASM-facing proof paths.

### DER Basic ASN.1

- Rust:
  - `crates/utility/edgerun-crypto/src/der/asn1/*.rs`
  - `crates/utility/edgerun-crypto/src/der/reader/*.rs`
  - `crates/utility/edgerun-crypto/src/der/writer/*.rs`
- WAT: `der-tlv.wat`, `der-oid.wat`, `der-asn1-basic.wat`
- Rationale: WAT covers TLV/header, OID arcs, INTEGER, BIT STRING, OCTET STRING,
  NULL, and SEQUENCE child iteration. The Rust DER crate is a generic typed DER
  reader/writer with trait-based decode/encode and many ASN.1 types.
- Recommendation: use WAT for certificate/SPKI scanner preflight and selected
  primitive readers. Do not delete generic DER traits or typed ASN.1 models.

## Not Ready Because Security/Crypto/Policy

- `crates/protocol/edgerun-protocols/src/tls/record.rs`: keep `RecordCipher`,
  AEAD nonce construction, encryption/decryption, sequence handling, and tag
  checks in Rust.
- `crates/protocol/edgerun-protocols/src/tls/server/client_hello.rs`: keep
  cipher suite selection, named group mapping, key share selection, and
  signature algorithm policy in Rust.
- `crates/protocol/edgerun-protocols/src/tls/certificate.rs` and
  `crates/utility/edgerun-crypto/src/der/*`: keep certificate validation,
  signature verification, key parsing, and typed public-key handling in Rust.
- `crates/protocol/edgerun-protocols/src/websocket.rs`: keep
  `websocket_accept` and `encode_upgrade_response` in Rust because they use
  SHA-1/base64 handshake semantics and HTTP upgrade policy.
- `crates/protocol/edgerun-protocols/src/dns/dnssec.rs` and
  `crates/protocol/edgerun-protocols/src/dns/tsig.rs`: keep DNSSEC/TSIG policy
  and crypto in Rust.
- HPACK/QPACK dynamic table and full header-block decoders: keep table mutation,
  eviction, stream state, and semantic validation in Rust.

## Not Worth Replacing Yet

### Generic Byteorder Helpers

- Rust: `crates/utility/edgerun-encoding/src/byteorder.rs`
- WAT: `encoding-core.wat`
- Rationale: byteorder helpers are tiny inline Rust functions and are used very
  broadly across protocol crates. Replacing every call with a WAT boundary would
  likely be slower and noisier than the source it removes.
- Recommendation: do not delete `byteorder.rs`. Use WAT byteorder as a
  conformance primitive and inside WAT-composed protocol modules.

### CRC32 / Adler32 Libraries

- Rust:
  - deleted `crates/utility/edgerun-encoding/src/crc32.rs`
  - deleted `crates/utility/edgerun-adler2`
  - deleted `crates/utility/edgerun-encoding/src/compression.rs`
  - deleted `crates/utility/edgerun-miniz-oxide`
- WAT: `encoding-core.wat`
- Rationale: CRC32/Adler32 are compact protocol checksum behavior and are now
  owned by WAT. `encoding-core.wat` exports one-shot and resumed Adler update
  behavior for zlib composition.
- Recommendation: keep the Rust crates deleted. Route any remaining checksum
  caller through WAT composition or an owner-local host adapter.

### QUIC/LEB Varints As Generic Rust Helpers

- Rust:
  - `crates/utility/edgerun-encoding/src/varint.rs`
  - `crates/utility/edgerun-encoding/src/quic_varint.rs`
  - `crates/protocol/edgerun-protocols/src/http/http3/varint.rs`
- WAT: `encoding-core.wat`
- Rationale: generic helpers are small and widely composed. Replacement is only
  worth it where a larger WAT protocol adapter already needs varints.
- Recommendation: keep generic Rust helpers. Delete duplicated protocol-local
  header varint scans only after HTTP/3 frame adapter replacement.

## Staged Deletion Plan

### Stage 1: Add A Rust Host Adapter

- Add a small no-std-compatible adapter surface that can call a compiled WAT
  codec module in tests and browser/WASM contexts.
- Required adapter behavior:
  - copy input into module memory;
  - call fixed ABI exports;
  - read fixed output records;
  - map WAT status codes into local Rust errors;
  - expose module/version/id checks.
- Do not route production traffic through it until stage 2 passes.

### Stage 2: Replace Tests Before Runtime Callers

- Add Rust tests that instantiate WAT and compare against current Rust behavior
  for:
  - HTTP/2 frame header encode/decode;
  - HTTP/3 frame header encode/decode;
  - WebSocket frame helpers;
  - DNS fixed header/counts.
- Keep Rust implementation active while tests prove adapter equivalence.

### Stage 3: First Runtime Adapter Patch

- Replace only fixed header decoding in HTTP/2 and HTTP/3 call paths:
  - keep Rust frame structs;
  - keep Rust payload allocation;
  - keep Rust semantic validation;
  - use WAT to compute header record and consumed length.
- Run package checks for `edgerun-protocols` and affected `edgerun-http-client`
  targets.

### Stage 4: Delete Duplicated Byte Logic

- Once adapter-backed tests and runtime paths are green, delete only the
  duplicated fixed-header parsing/writing code that is no longer called.
- Do not delete broad utility crates or typed protocol models.

### Stage 5: Expand To Scanner Preflight

- Add WAT preflight for TLS ClientHello, certificate-list spans, DNS fixed
  header, and JSON scalar validation.
- Keep Rust typed extraction and security decisions.

## Explicit Non-Goals For The Next Patch

- No deletion of `edgerun-encoding::byteorder`.
- No deletion of generic DER reader/writer traits.
- No deletion of TLS crypto, certificate validation, or handshake policy.
- No deletion of HPACK/QPACK dynamic table logic.
- No deletion of full HTTP/1, DNS compressed-name, JSON, or TOML parsers.
