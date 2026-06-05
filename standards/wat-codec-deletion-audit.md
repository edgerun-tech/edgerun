# WAT Codec Deletion Audit

Scope: audit Rust deletion candidates after current codec-primitive WAT parity.
This file is an audit only. It does not approve deleting Rust code directly.

Evidence reviewed:

- WAT modules and manifests under `standards/build/wasm/codec-primitives/` and
  `standards/components/manifests/codec-primitives/`.
- Rust parity runners:
  `standards/runners/rust-parity-encoding.js`,
  `standards/runners/rust-parity-http-ws.js`,
  `standards/runners/rust-parity-der-tls.js`, and
  `standards/runners/rust-parity-misc-codecs.js`.
- Local call sites found with `rg` across `crates/`, excluding no source edits.

## Summary

The best deletion candidates are leaf scanner/encoder helpers whose WAT modules
return fixed records or bytes and have Rust-backed parity. Do not delete
canonical Rust object models, security verification paths, dynamic table state,
or permissive parsers until call sites are explicitly moved to WAT and matching
composition tests exist.

The current WAT parity proves useful kernel behavior. It does not yet prove that
larger Rust flows can delete their canonical Rust implementations.

## Replaceable Scanner/Encoder Candidates

These are the first candidates to replace at call sites or delete after all Rust
callers are migrated to the WAT ABI.

| Candidate | Rust path/functions | WAT module/exports | Local call-site notes | Audit classification |
| --- | --- | --- | --- | --- |
| Byte-order fixed reads and writes | `crates/utility/edgerun-encoding/src/byteorder.rs`: `read_u16_be`, `read_u24_be`, `read_u32_be`, `read_u64_be`, `read_u16_le`, `read_u32_le`, `write_u16_be`, `write_u24_be`, `write_u32_be`, `push_u16_be`, `push_u24_be`, `push_u32_be`, `push_u64_be` | `encoding-core.wat`: `read_u16_be`, `read_u24_be`, `read_u32_be`, plus related writes covered by module intent | Used broadly by DNS, TLS, HTTP, WebSocket, SSH, and utility code. Replace only inside WAT-backed component execution paths first; keep Rust helpers while native no-std crates call them directly. | Replaceable scanner/encoder, but not file-level delete yet. |
| LEB128 varint | `crates/utility/edgerun-encoding/src/varint.rs`: `encode_varint`, `decode_varint_slice`, `decode_varint_iter` | `encoding-core.wat`: `varint_encode_u64`, `varint_decode_u64` | No high-volume non-test call sites surfaced in the broad search. `decode_varint_iter` is iterator glue not represented by WAT. | Replace `encode_varint`/`decode_varint_slice` first; keep or re-evaluate iterator glue separately. |
| QUIC varint | `crates/utility/edgerun-encoding/src/quic_varint.rs`: `encode_varint`, `decode_varint`, `encode_varint_vec`; `crates/protocol/edgerun-protocols/src/http/http3/varint.rs`: `quic_encode_varint`, `quic_decode_varint`, `quic_decode_varint_at` | `encoding-core.wat`: `quic_varint_encode_u64`, `quic_varint_decode_u64`; `http3-frame.wat` also composes frame headers | Used heavily by HTTP/3 connection and crypto-frame code in `crates/utility/edgerun-http-client/src/http/http3/*` and mirrored node HTTP/3 code. `*_at` is caller-position glue. | Replaceable scanner/encoder; migrate HTTP/3 frame header paths before raw connection code. |
| CRC32 and Adler32 | deleted `crates/utility/edgerun-encoding/src/crc32.rs`; deleted `crates/utility/edgerun-adler2` | `encoding-core.wat`: `crc32`, `adler32`, `adler32_update` | Low-risk pure checksums now WAT-owned. Remaining checksum references are caller adapter or documentation work, not reasons to restore Rust crates. | Crossed out. |
| HTTP header validation and token scan | `crates/protocol/edgerun-protocols/src/http/header.rs`: `is_tchar`, `HeaderName::new`, `HeaderValue::new`, `header_value_has_token` | `http1-scan.wat`: `http_validate_header_name`, `http_validate_header_value`, `http_value_has_token` | Used by HTTP maps and upgrade paths, including `crates/utility/edgerun-http-client/src/http/http1/upgrade.rs` and node mirrors. | Replace scanner internals only; keep `HeaderName`, `HeaderValue`, and `HeaderMap` as canonical Rust models. |
| HTTP/1 line and header-span scanners | `crates/protocol/edgerun-protocols/src/http/message.rs`: `HttpRequest::from_http`, `HttpResponse::from_http`; `method.rs`: `Method::from_str`; `status.rs`: `StatusCode::new` | `http1-lines.wat`: `http_parse_request_line`, `http_parse_status_line`, `http_next_header` | Full request/response objects allocate strings/maps and remain Rust-owned. | Replaceable scanner front end; not a deletion target for object parsers. |
| HTTP body framing classifiers | `crates/protocol/edgerun-protocols/src/http/chunked.rs`: `has_chunked_transfer_coding`; `crates/utility/edgerun-encoding/src/chunked.rs`: chunk step logic | `http1-body.wat`: `http_parse_content_length`, `http_has_transfer_token`, `http_classify_body_framing`; `http1-scan.wat`: `http_parse_chunk_step` | Full chunk body decode and trailer map construction allocate and parse headers. | Replace scanner/classifier helpers; keep full decode/trailer logic until composition tests exist. |
| WebSocket frame prefix, length, mask, and server header | `crates/protocol/edgerun-protocols/src/websocket.rs`: `decode_frame_prefix`, `decode_payload_len`, `decode_client_message` mask loop, `encode_server_frame`, `encode_server_binary`, `encode_server_control` | `ws-frame.wat`: `ws_decode_prefix`, `ws_decode_payload_len`, `ws_apply_mask_in_place`, `ws_write_server_frame_header` | Runtime call sites: `crates/protocol/edgerun-work/src/std_runtime/websocket_channel.rs`, `crates/node/edgerun-node/src/services/work_websocket.rs`, and `crates/apps/edgerun-pocketbase/src/main.rs`. | Replaceable scanner/encoder, but see strictness mismatch for reserved opcodes and non-minimal lengths. |
| HTTP/2 fixed frame header | `crates/protocol/edgerun-protocols/src/http/http2/frame/mod.rs`: `FrameType::from_u8`, `Frame::to_bytes`, header portion of `Frame::from_bytes` | `http2-frame.wat`: `http2_frame_type_classify`, `http2_frame_header_decode`, `http2_frame_header_encode` | Used by HTTP/2 connection and conformance code in `crates/utility/edgerun-http-client/src/http/http2/*` and node mirrors. | Strong replaceable candidate for header-only parse/encode; keep `Frame` and semantic validators. |
| HTTP/3 fixed frame header | `crates/protocol/edgerun-protocols/src/http/http3/frame.rs`: `Http3FrameType::from_u64`, header part of `Http3Frame::to_bytes`, header part of `Http3Frame::from_bytes` | `http3-frame.wat`: `http3_frame_type_classify`, `http3_frame_header_decode`, `http3_frame_header_encode` | Used in HTTP/3 connection parsing in `crates/utility/edgerun-http-client/src/http/http3/connection.rs` and node mirrors. | Strong replaceable candidate for frame header only; keep typed frame enum and payload semantics. |
| HPACK/QPACK prefix integers | `crates/utility/edgerun-hpack/src/encoder.rs`: `encode_integer_into`, `encode_integer`; `crates/utility/edgerun-hpack/src/decoder.rs`: private `decode_integer`; `crates/protocol/edgerun-protocols/src/http/http3/qpack/prefix_int.rs`: prefix integer helpers | `http-prefix-int.wat`: `hpack_prefix_int_decode`, `hpack_prefix_int_encode`, `qpack_prefix_int_decode`, `qpack_prefix_int_encode` | Dynamic-table encoder/decoder still consume these helpers. | Replaceable leaf encoder/decoder; do not delete dynamic table logic. |
| HPACK Huffman and string scalar decode | `crates/utility/edgerun-hpack/src/huffman.rs`: `HuffmanDecoder::decode`, `encode`; `crates/utility/edgerun-hpack/src/decoder.rs`: private `decode_string`; QPACK string decode under `crates/protocol/edgerun-protocols/src/http/http3/qpack/prefix_string/` | `hpack-huffman.wat`, `hpack-string.wat`, `qpack-string.wat` | Full HPACK/QPACK header blocks need dynamic table and field representation context. | Replaceable string kernels after composition tests. |
| DER TLV and basic ASN.1 spans | `crates/utility/edgerun-crypto/src/der/length.rs`, `tag.rs`, `header.rs`; `der/asn1/integer.rs`, `bit_string.rs`, `octet_string.rs`, `null.rs`, `sequence.rs`; `const_oid/parser.rs`, `const_oid/encoder.rs`, `der/asn1/oid.rs` | `der-tlv.wat`, `der-asn1-basic.wat`, `der-oid.wat` | Certificate parser and DER traits consume more context than spans. | Replaceable scanner/encoder kernels; keep DER trait API and certificate parser. |
| TLS vector and ALPN spans | `crates/protocol/edgerun-protocols/src/tls/handshake.rs`: vector construction/parsing helpers inside `ClientHelloBuilder` and parser helpers; `tls/tls_alpn.rs` | `tls-vector.wat`: `tls_vector_u8_decode`, `tls_vector_u16_decode`, `tls_vector_u24_decode`, `tls_extension_next`, `tls_alpn_next` | Used by ClientHello and TLS handshake flows. | Replaceable scanner kernels only. |
| TLS DNS name normalization/matching | `crates/protocol/edgerun-protocols/src/tls/name_match.rs`: `normalize_tls_dns_name`, `tls_dns_name_matches` | `tls-name.wat`: `tls_dns_name_normalize`, `tls_dns_name_matches` | Re-exported by node and HTTP client TLS modules; used with certificate hostname checks. | Replaceable if cert-path tests prove identical policy. |
| DNS fixed header counts/bounds | `crates/protocol/edgerun-protocols/src/dns/limits.rs`: `dns_section_counts`, `validate_dns_wire_bounds`; `dns/message.rs`: `DnsHeader::{to_wire,from_wire}` | `dns-message-header.wat`: `dns_header_flags_classify`, `dns_header_decode`, `dns_header_encode` | Good pre-parse guard for DNS message readers. | Replaceable scanner/encoder. |
| JSON scalar scan and integer bounds | `crates/utility/edgerun-json/src/parse.rs`: `parse_u64_fast`, `parse_i64_fast`, string/number scanner internals; `src/api.rs`: `parse_json_tape`; `src/tape.rs`: `TapeValue::{as_i64,as_u64,as_f64}` | `json-scalar.wat`, `json-tape.wat` | Many callers consume `JsonTape` and `JsonValue`: codelyzer, node services, Codex API, PocketBase, OAuth, exchange. | Replace scalar validation first; keep Rust tape/value API. |
| Percent component encode | `crates/utility/edgerun-encoding/src/percent.rs`: `percent_encode`, `percent_encode_path_segments`, `percent_encode_colon_pair`; `crates/utility/edgerun-form-urlencoded/src/lib.rs`: `byte_serialize`, serializer append helpers | `percent-url-form.wat`: `percent_encode_component`, `uri_scan_path_query` | OCI registry client uses path/digest-specific wrappers in `crates/edgerun-oci/src/registry/*`. | Component encode is replaceable where strict RFC3986 component rules match caller policy. |

## Keep Canonical Rust

These Rust surfaces are canonical models, state machines, or no-std API shapes.
They should stay even if their internals later call WAT kernels.

- `crates/protocol/edgerun-protocols/src/http/header.rs`:
  `HeaderName`, `HeaderValue`, `HeaderMap`.
- `crates/protocol/edgerun-protocols/src/http/message.rs`:
  `HttpRequest`, `HttpResponse`, `HttpRequest::from_http`,
  `HttpResponse::from_http`, and `to_http_bytes`.
- `crates/protocol/edgerun-protocols/src/http/uri.rs`: `Uri`, `Scheme`,
  `Uri::parse`, `Uri::request_target`.
- `crates/protocol/edgerun-protocols/src/http/http2/frame/mod.rs`: `Frame`,
  frame-specific structs, and `Frame::validate_semantics`.
- `crates/protocol/edgerun-protocols/src/http/http3/frame.rs`: `Http3Frame`
  enum and payload-specific parse/serialize branches.
- `crates/utility/edgerun-hpack/src/lib.rs`,
  `crates/utility/edgerun-hpack/src/encoder.rs`,
  `crates/utility/edgerun-hpack/src/decoder.rs`: dynamic table, static table,
  full `Encoder`, and full `Decoder`.
- `crates/utility/edgerun-json/src/api.rs`, `parse.rs`, `tape.rs`,
  `toml_parse.rs`, `toml_api.rs`: `JsonValue`, `JsonTape`, `TapeValue`,
  `from_toml_str`, and TOML object parsing.
- `crates/protocol/edgerun-protocols/src/dns/message.rs`, `record.rs`,
  `zone.rs`, `zone_file.rs`: full DNS message, record, zone, and zone-file
  models.
- `crates/utility/edgerun-crypto/src/der/*`: DER trait API, typed ASN.1
  wrappers, reader/writer abstraction.
- `crates/utility/edgerun-crypto/src/pem_rfc7468.rs`: `Decoder`, `Encoder`,
  `decode`, `decode_vec`, `encode`, `encode_string` until PEM/base64
  composition tests exist.
- `crates/protocol/edgerun-protocols/src/tls/server/client_hello.rs`:
  `ClientHello`, `ClientHello::parse`, `NamedGroup::from_wire`.
- `crates/protocol/edgerun-protocols/src/tls/handshake.rs`:
  `ClientHelloBuilder`, `ServerHello::parse`, and handshake object assembly.
- `crates/protocol/edgerun-protocols/src/tls/certificate.rs`: `Certificate`
  and all certificate parsing, validity, hostname, and signature APIs.

## Std/Runtime Glue

These are not codec deletion candidates. They own IO, buffers, runtime errors,
or service behavior and may call WAT-backed helpers only at their codec
boundaries.

- `crates/protocol/edgerun-work/src/std_runtime/websocket_channel.rs`:
  WebSocket reads/writes, handshake flow, ping/pong handling, max frame policy.
- `crates/node/edgerun-node/src/services/work_websocket.rs`: node service
  WebSocket runtime flow mirroring the work runtime.
- `crates/apps/edgerun-pocketbase/src/main.rs`: application WebSocket and JSON
  handling.
- `crates/utility/edgerun-http-client/src/http/http2/connection.rs`,
  `client.rs`, and node mirrors: stream state, flow control, HPACK state, and
  HTTP semantics.
- `crates/utility/edgerun-http-client/src/http/http3/connection.rs`,
  `crypto_frame.rs`, QUIC modules, and node mirrors: connection state, stream
  routing, TLS/QUIC interaction, and runtime policy.
- `crates/utility/edgerun-http-client/src/tls/async_tls.rs` and
  `crates/node/edgerun-node/src/tls/async_tls.rs`: TLS record IO, transcript,
  key schedule use, certificate handling, and session behavior.
- `crates/edgerun-oci/src/registry/client.rs` and
  `crates/edgerun-oci/src/registry/bundle_push.rs`: registry URL construction
  and HTTP workflow; only percent-encoding leaves are candidates.

## Security-Sensitive Not Replaceable

Do not replace these with WAT codec primitives in the deletion pass.

- `crates/protocol/edgerun-protocols/src/tls/record.rs`: `RecordCipher::new`,
  `RecordCipher::encrypt`, `RecordCipher::decrypt`, nonce construction, AEAD
  AAD construction, sequence number handling.
- `crates/protocol/edgerun-protocols/src/tls/certificate.rs`:
  `Certificate::from_der`, `Certificate::from_pem`,
  `Certificate::parse_list`, `Certificate::is_valid_at_unix_secs`,
  `Certificate::matches_hostname`, `Certificate::verify_signature`,
  `verify_certificate_signature_with_issuer`,
  `verify_ecdsa_certificate_signature`, `verify_rsa_certificate_signature`,
  `verify_rsa_pkcs1_sha256`, `verify_rsa_pkcs1_sha384`,
  `verify_rsa_pkcs1_sha512`, `verify_rsa_pss_sha256`,
  `verify_ed25519_certificate_signature`.
- `crates/protocol/edgerun-protocols/src/tls/handshake.rs`:
  `ClientHelloBuilder::build`, PSK binders, key share construction, and
  transcript-sensitive handshake assembly.
- `crates/protocol/edgerun-protocols/src/tls/server/client_hello.rs`:
  `ClientHello::parse` as a security boundary. WAT can scan extension spans,
  but Rust should continue owning accepted semantic policy.
- `crates/protocol/edgerun-protocols/src/dns/tsig.rs`: `TsigSigner`,
  `TsigVerifier`, HMAC-compatible signing and verification.
- `crates/protocol/edgerun-protocols/src/websocket.rs`: `websocket_accept` and
  `encode_upgrade_response` because they include SHA-1/base64 handshake
  authority and HTTP upgrade policy, not just frame bytes.

## Blocked By Strictness Mismatch

These are not drop-in deletion candidates until the stricter WAT behavior is
accepted by policy and all callers/tests are adjusted deliberately.

- `crates/utility/edgerun-encoding/src/base64.rs`:
  `base64url_decode_into`, `base64url_decode`, and
  `base64url_to_standard_decode` accept nonzero unused tail bits in cases where
  `encoding-text.wat` rejects noncanonical no-pad base64url tails.
- `crates/utility/edgerun-encoding/src/hex.rs`: `hex_to_bytes` accepts optional
  `0x`/`0X` prefixes and odd-length strings; `encoding-text.wat`
  `hex_decode_strict` rejects odd-length canonical hex.
- `crates/utility/edgerun-encoding/src/percent.rs`: `percent_decode` treats `+`
  as space and preserves malformed percent escapes; `percent-url-form.wat`
  `percent_decode_strict` preserves `+` and rejects malformed escapes.
- `crates/utility/edgerun-form-urlencoded/src/lib.rs`: `parse` performs
  plus-decoding and percent-decoding with lossy UTF-8 behavior; WAT
  `form_urlencoded_next_pair` returns raw spans and needs explicit strict
  percent-decode composition.
- `crates/protocol/edgerun-protocols/src/dns/record.rs`:
  `decode_domain_name` follows compression pointers; `dns-name.wat`
  intentionally rejects compression pointers.
- `crates/protocol/edgerun-protocols/src/dns/name.rs`: `validate_name` accepts
  `@`, empty zone-origin shorthand, and underscore labels; WAT `dns_name_scan`
  is a wire-name scanner, not a zone-file name policy replacement.
- `crates/utility/edgerun-json/src/toml_parse.rs`: `from_toml_str` and
  `parse_toml_value_simple` fall back to strings for malformed scalar-looking
  values; `toml-scan.wat` is stricter.
- `crates/utility/edgerun-crypto/src/pem_rfc7468.rs`: `decode_label`,
  `Decoder::new`, and `parse_pem` accept lowercase labels that
  `pem-rfc7468.wat` rejects.
- `crates/protocol/edgerun-protocols/src/tls/record.rs`: `TlsRecord::from_bytes`
  accepts unknown content types and future record versions; `tls-frame.wat`
  rejects unknown content type and out-of-policy future versions.
- `crates/protocol/edgerun-protocols/src/websocket.rs`:
  `decode_frame_prefix` and `decode_payload_len` are more permissive than
  `ws-frame.wat` for reserved opcodes, fragmented frames, non-minimal extended
  lengths, high-bit 64-bit lengths, and control-frame limits. Some are rejected
  later by Rust flow; WAT rejects earlier.

## Blocked By Missing Composition Tests

These need end-to-end tests that compose WAT kernels before Rust deletion is
defensible.

- HTTP/1 message parsing:
  `http1-scan.wat` + `http1-lines.wat` + `http1-body.wat` must be tested
  together against `HttpRequest::from_http`, `HttpResponse::from_http`,
  `HeaderMap`, chunked body handling, duplicate `Content-Length`, and
  transfer/content-length conflict policy.
- WebSocket runtime flow:
  `ws-frame.wat` must be tested with `decode_handshake_request`,
  `encode_upgrade_response`, `decode_client_message`, ping/pong behavior, and
  max-frame policy in `edgerun-work` and node service runtimes.
- HTTP/2:
  `http2-frame.wat` must be tested with `Frame::validate_semantics`,
  frame-specific structs, flow-control handling, continuation ordering, and
  connection preface handling.
- HTTP/3:
  `encoding-core.wat` QUIC varint + `http3-frame.wat` must be tested with
  `Http3Frame` payload semantics, settings entries, push IDs, GOAWAY, stream
  parsing, and QPACK interaction.
- HPACK/QPACK:
  `http-prefix-int.wat` + `hpack-huffman.wat` + `hpack-string.wat` /
  `qpack-string.wat` must be tested through full header block decode/encode,
  static table lookup, dynamic table updates, never-indexed fields, and table
  size updates.
- DER/PEM/certificate:
  `pem-rfc7468.wat` + `encoding-text.wat` base64 + `der-tlv.wat` +
  `der-oid.wat` + `der-asn1-basic.wat` must be tested through certificate list,
  SPKI, SAN DNS names, validity, and signature verification entry points.
- TLS ClientHello:
  `tls-frame.wat` + `tls-vector.wat` + `tls-clienthello.wat` + `tls-name.wat`
  must be tested through `ClientHello::parse`, `ClientHelloBuilder::build`,
  SNI, ALPN, supported versions, key share, PSK identity, and unknown extension
  handling.
- DNS:
  `dns-message-header.wat` + `dns-name.wat` must be tested through full DNS
  query and response parsing, compressed names, section traversal, EDNS0, TSIG
  exclusion, and record-specific RDATA parsing.
- Percent/form/URI:
  `percent-url-form.wat` must be tested with strict percent decode, raw
  form-pair spans, plus decoding, path-segment encode policy, colon-pair encode
  policy, and `Uri::parse`.
- JSON:
  `json-scalar.wat` + `json-tape.wat` must be tested through `JsonTape`,
  `TapeValue`, `JsonValue`, object field lookup, compiled tape keys, and
  application callers that rely on unescaped string or numeric conversion.
- TOML:
  `toml-scan.wat` needs a policy decision and parser-level tests before it can
  replace `from_toml_str` or `toml_api::from_toml_str`.

## Recommended Order

1. Replace leaf fixed-byte helpers in WAT-hosted paths only:
   byteorder reads/writes, LEB128, QUIC varint, CRC32, Adler32.
2. Move fixed frame header decode/encode to WAT where the caller already works
   with spans: HTTP/2, HTTP/3, and HPACK/QPACK prefix integers.
3. Move WebSocket prefix/length/mask/server-header logic after adding tests that
   assert the earlier WAT rejection policy for reserved opcodes, non-minimal
   lengths, high-bit 64-bit lengths, and control-frame limits.
4. Move JSON scalar validation and DNS fixed header guards. These are useful
   front-door validators while Rust keeps the object model.
5. Move HPACK/QPACK Huffman and string kernels after full header-block
   composition tests pass.
6. Move DER TLV/OID/ASN.1 scanner kernels and PEM boundary/compaction only
   after PEM + DER + certificate composition tests pass.
7. Move TLS vector/name/ClientHello scanners only as helpers under the Rust TLS
   policy layer. Do not replace TLS crypto, certificate verification, or
   handshake authority.
8. Defer strictness-mismatch surfaces until product/protocol policy explicitly
   chooses stricter behavior: base64url tails, strict hex, percent/form decode,
   DNS compressed-name handling, permissive TOML fallback, PEM label casing,
   TLS record version/type rejection, and WebSocket early rejection.
