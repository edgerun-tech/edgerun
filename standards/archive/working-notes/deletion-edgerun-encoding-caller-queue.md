# edgerun encoding caller cleanup queue

This queue is for agents removing live callers of retired Rust codec APIs after
their useful behavior has moved to WAT. Do not rebuild these APIs as Rust
compatibility shims:

- `edgerun_encoding::base64`
- `edgerun_encoding::percent`
- `edgerun_encoding::varint`
- `edgerun_encoding::quic_varint`
- `edgerun_encoding::chunked`
- `edgerun_encoding::crc32`
- `edgerun_form_urlencoded`
- `edgerun_percent_encoding`
- `adler2`

Status: `crates/utility/edgerun-encoding` is now fully crossed out. The crate
shell, root workspace member, and root workspace dependency are deleted. Any
remaining `edgerun_encoding::*` import is a caller-demolition target, not an
API to preserve.

The goal is caller demolition: move each runtime surface to a WAT adapter,
domain-specific byte pipeline, or direct deletion of the feature surface that
only existed to keep the Rust compatibility crate alive.

## Ground truth

Current WAT primitives:

| Behavior | WAT module | Exports |
| --- | --- | --- |
| base64url no-pad, hex | `standards/build/wasm/codec-primitives/encoding-text.wat` | `base64url_nopad_encode`, `base64url_nopad_decode`, `hex_encode_lower`, `hex_decode_strict` |
| varint, QUIC varint, CRC32, Adler32 | `standards/build/wasm/codec-primitives/encoding-core.wat` | `varint_encode_u64`, `varint_decode_u64`, `quic_varint_encode_u64`, `quic_varint_decode_u64`, `crc32`, `adler32`, `adler32_update` |
| percent/form/query | `standards/build/wasm/codec-primitives/percent-url-form.wat` | `percent_decode_strict`, `percent_encode_component`, `form_urlencoded_next_pair`, `uri_scan_path_query` |
| HTTP/1 chunk body scanning | `standards/build/wasm/codec-primitives/http1-chunk-stream.wat` | `http1_chunk_next`, `http1_chunk_scan_body` |
| HTTP body framing | `standards/build/wasm/codec-primitives/http1-body.wat` | `http_parse_content_length`, `http_has_transfer_token`, `http_classify_body_framing` |
| WebSocket frame bytes | `standards/build/wasm/codec-primitives/ws-frame.wat` | `ws_decode_prefix`, `ws_decode_payload_len`, `ws_apply_mask_in_place`, `ws_parse_header`, `ws_write_frame_header`, `ws_parse_close_payload`, `ws_write_close_payload`, `ws_write_server_frame_header` |
| PEM base64 text extraction | `standards/build/wasm/codec-primitives/pem-rfc7468.wat` | `pem_scan`, `pem_compact_base64` |

Standard base64 is now covered by `encoding-base64.wat`; callers below still
need owner-side routing and proof runners. Do not hide this with a new Rust
base64 shim.

Primary proof runners:

```bash
node standards/runners/encoding-text-smoke.js
node standards/runners/encoding-core-smoke.js
node standards/runners/percent-url-form-smoke.js
node standards/runners/http1-chunk-stream-smoke.js
node standards/runners/ws-frame-smoke.js
node standards/runners/rust-parity-encoding.js
node standards/runners/rust-parity-misc-codecs.js
```

Cargo is not the acceptance gate for this queue. The retired Rust modules are
already gone or tombstoned in places, so old import paths are expected to fail
until callers are removed.

## Packet 1: OAuth and ACME base64url plus percent

First files:

- `crates/apps/edgerun-oauth/src/lib.rs`
- `crates/apps/edgerun-oauth/src/client.rs`
- `crates/apps/edgerun-oauth/src/oauth_client.rs`
- `crates/apps/edgerun-oauth/src/types.rs`
- `crates/apps/edgerun-oauth/src/jwt.rs`
- `crates/protocol/edgerun-protocols/src/oauth/pkce.rs`
- `crates/protocol/edgerun-protocols/src/acme/challenge_material.rs`
- `crates/protocol/edgerun-protocols/src/acme/types.rs`

Retired calls:

- `edgerun_encoding::base64::base64url_nopad_encode`
- `edgerun_encoding::base64::base64url_decode`
- `edgerun_encoding::{base64url_decode, base64url_nopad_encode}`
- `edgerun_encoding::percent::url_encode_pair`
- `edgerun_encoding::percent::percent_encode`

WAT replacement:

- `encoding-text.wat::base64url_nopad_encode`
- `encoding-text.wat::base64url_nopad_decode`
- `percent-url-form.wat::percent_encode_component`
- `percent-url-form.wat::form_urlencoded_next_pair` when query/form field
  splitting is required

Deletion strategy:

1. Start with ACME and protocol PKCE because they are narrow base64url no-pad
   users with deterministic byte inputs and outputs.
2. Replace direct imports with a small domain adapter that invokes the WAT
   exports and returns the existing domain error type. The adapter belongs next
   to the ACME/OAuth protocol surface or a shared WAT runtime bridge, not inside
   `edgerun-encoding`.
3. Update JWT decode paths to reject noncanonical base64url tails according to
   `standards/wat-codec-strictness-policy.md`.
4. Replace OAuth URL/query assembly with explicit component encoding through
   `percent_encode_component`; keep `+` handling only for form semantics, never
   for generic percent paths.
5. Delete the `pub use edgerun_encoding::base64::*` compatibility export from
   `edgerun-oauth` after all internal callers are migrated.

Acceptance evidence:

- Add/extend a WAT adapter proof for PKCE challenge generation, ACME JWK thumbprint
  material, JWT segment decode, and OAuth query encoding.
- Run `encoding-text-smoke.js`, `percent-url-form-smoke.js`,
  `rust-parity-encoding.js`, and `rust-parity-misc-codecs.js`.

## Packet 2: DNS DoH and TSIG base64

First files:

- `crates/protocol/edgerun-protocols/src/dns/doh.rs`
- `crates/protocol/edgerun-protocols/src/dns/tsig.rs`
- follow-up adjacent DNS byte work: `crates/protocol/edgerun-protocols/src/dns/record.rs`

Retired calls:

- `edgerun_encoding::base64::base64url_decode`
- `edgerun_encoding::base64::standard_decode`

WAT replacement:

- DoH GET `dns=` parameter: `encoding-text.wat::base64url_nopad_decode`
- TSIG shared secret text: blocked until standard-base64 decode exists in WAT,
  or until a DNS/TSIG-specific WAT helper owns padded standard-base64 decode

Deletion strategy:

1. Migrate `doh.rs` first. DoH base64url decode is covered now and should become
   a small WAT adapter around strict no-pad base64url bytes.
2. Leave TSIG as a separate packet or add standard-base64 decode to
   `encoding-text.wat` before touching `tsig.rs`.
3. Do not weaken TSIG key parsing by accepting base64url or permissive padded
   variants unless a DNS standard requires it. TSIG shared secrets are key
   material, so canonical decode must fail closed.
4. After DoH decode no longer imports `edgerun_encoding`, keep DNS message name
   and section work pointed at the DNS WAT modules rather than backfilling Rust
   helpers.

Acceptance evidence:

- DoH: WAT proof for canonical base64url query decode and rejection of invalid
  text.
- TSIG: wait for a standard-base64 WAT runner case before deleting the Rust
  caller.

## Packet 3: WebSocket accept and base64

Status: `crates/utility/edgerun-tungstenite` is deleted. Useful portable
behavior is WAT-owned by `ws-accept.wat` and `ws-frame.wat`; remaining
workspace references to `edgerun-tungstenite` are caller demolition targets, not
compatibility APIs to restore.

First files:

- `crates/protocol/edgerun-protocols/src/websocket.rs`
- `crates/utility/edgerun-http-client/src/http/http1/upgrade.rs`
- `crates/node/edgerun-node/src/http/http1/upgrade.rs`

Retired calls:

- `edgerun_encoding::base64::standard_encode`
- `edgerun_encoding::base64::encode_u64_base64`

WAT replacement:

- WebSocket frame byte work: `ws-frame.wat::ws_decode_prefix`,
  `ws_decode_payload_len`, `ws_apply_mask_in_place`,
  `ws_parse_header`, `ws_write_frame_header`, `ws_parse_close_payload`,
  `ws_write_close_payload`, `ws_write_server_frame_header`
- WebSocket accept text: `ws-accept.wat::ws_accept_key`

Deletion strategy:

1. Do not preserve the `encode_u64_base64` path. The two HTTP upgrade files
   currently compute `Sec-WebSocket-Accept` from `DefaultHasher`; that is not
   RFC 6455. Delete or replace it with the canonical SHA-1 plus standard-base64
   accept pipeline.
2. Move frame parsing and frame header writing to `ws-frame.wat` independently
   of the handshake accept string.
3. Retire duplicate WebSocket accept implementations after one canonical adapter
   covers `edgerun-protocols`, node HTTP upgrade, HTTP client upgrade, and
   the deleted tungstenite compatibility fallout.

Acceptance evidence:

- Existing `ws-frame-smoke.js` for frame byte behavior.
- Add a WebSocket accept proof for the RFC example key:
  `dGhlIHNhbXBsZSBub25jZQ==` -> `s3pPLMBiTxaQ9kYGzzhZRbK+xOo=`.

## Packet 4: Terminal and SSH base64

First files:

- `crates/edgerun-sdk/src/ssh_support.rs`
- `crates/edgerun-term/edgerun-term-core/src/terminal.rs`

Retired calls:

- `edgerun_encoding::base64::standard_decode`
- `edgerun_encoding::base64::standard_encode`

WAT replacement:

- SSH authorized-key body decode: blocked until standard-base64 decode exists in
  WAT, or until SSH key parsing owns a WAT standard-base64 decode helper
- Terminal OSC payload encode/decode: blocked until standard-base64
  encode/decode exists in WAT

Deletion strategy:

1. Treat this as UI/runtime payload cleanup, not protocol proof work.
2. Add standard-base64 encode/decode WAT exports once, then point SSH and
   terminal code at the same adapter.
3. Keep SSH key structure parsing in `ssh_support.rs`; only the base64 text
   body decode belongs to the WAT codec.
4. Keep terminal OSC command semantics in terminal code; only payload text
   encode/decode belongs to the WAT codec.

Acceptance evidence:

- SSH key fixture round trip through WAT decode.
- Terminal OSC copy/paste payload round trip through WAT encode/decode.

## Packet 5: Storage event log varint

First file:

- `crates/authority/edgerun-storage/src/core/event_log.rs`

Retired call:

- `edgerun_encoding::varint::encode_varint`

WAT replacement:

- `encoding-core.wat::varint_encode_u64`
- later read path, if present: `encoding-core.wat::varint_decode_u64`

Deletion strategy:

1. Replace the event frame length prefix with a WAT varint adapter.
2. Keep the record body unchanged: it remains `edgerun-wire` event envelope
   bytes.
3. Add a small proof that the same event bytes produce the same length prefix
   for boundary lengths: `0`, `1`, `127`, `128`, `300`, and `u32::MAX` where
   allocation is not required.
4. Do not introduce a storage-local varint implementation.

Acceptance evidence:

- `encoding-core-smoke.js`
- an event-log-specific WAT adapter proof comparing prefix bytes to the current
  fixture expectations

## Packet 6: HTTP chunked body

First file:

- `crates/protocol/edgerun-protocols/src/http/chunked.rs`

Retired calls:

- `edgerun_encoding::chunked::decode_chunked`
- `edgerun_encoding::chunked::ChunkedError`

WAT replacement:

- `http1-chunk-stream.wat::http1_chunk_scan_body`
- `http1-chunk-stream.wat::http1_chunk_next`
- optional framing precheck through
  `http1-body.wat::http_classify_body_framing`

Deletion strategy:

1. Replace `decode_chunked` with a loop over `http1_chunk_next`, copying payload
   slices into the existing body buffer and collecting trailer spans after the
   zero chunk.
2. Keep trailer header parsing in `chunked.rs` for now; WAT should only find
   chunk boundaries and status.
3. Map WAT status codes directly to `ChunkedError` without importing the
   retired Rust error type.
4. Delete the `edgerun_encoding::chunked` import and retire the old
   `map_chunked_error` bridge.

Acceptance evidence:

- `http1-chunk-stream-smoke.js`
- existing HTTP chunked Rust tests only after the adapter exists; do not use
  Cargo as the first proof for this queue

## Packet 7: QUIC and HTTP/3 varints

First files:

- `crates/protocol/edgerun-protocols/src/http/http3/varint.rs`
- `crates/protocol/edgerun-protocols/src/quic/packet.rs`
- `crates/protocol/edgerun-protocols/src/quic/frame/varint.rs`

Retired calls:

- `edgerun_encoding::quic_varint::encode_varint`
- `edgerun_encoding::quic_varint::decode_varint`
- `edgerun_encoding::prefix_varint::{encode_prefix_varint, decode_prefix_varint}`

WAT replacement:

- QUIC varint: `encoding-core.wat::quic_varint_encode_u64`,
  `encoding-core.wat::quic_varint_decode_u64`
- QPACK/HPACK prefix integers: `http-prefix-int.wat` exports, not
  `encoding-core.wat`

Deletion strategy:

1. Split QUIC varint cleanup from QPACK prefix-int cleanup.
2. Replace HTTP/3 and QUIC packet varints with `encoding-core.wat` first.
3. Move QPACK prefix integer use to the HPACK/QPACK WAT queue, because prefix
   width and continuation semantics belong with header compression codecs.

Acceptance evidence:

- `encoding-core-smoke.js`
- HTTP/3 frame and QPACK composition runners after the adapter is wired

## Packet 8: CRC32 and Adler32

First files:

- `crates/protocol/edgerun-protocols/src/tuya.rs`
- deleted `crates/utility/edgerun-miniz-oxide/src/shared.rs`

Retired calls:

- `edgerun_encoding::crc32`
- `adler2::Adler32::from_checksum`

WAT replacement:

- CRC32: `encoding-core.wat::crc32`
- Adler one-shot: `encoding-core.wat::adler32`
- Adler resumed checksum: `encoding-core.wat::adler32_update`

Deletion strategy:

1. Tuya CRC32 is a small direct target. Replace the checksum call with the WAT
   CRC32 adapter and keep packet framing in `tuya.rs`.
2. Miniz must be handled as a compression-codec packet, not an Adler leaf
   packet. Replace only the shared checksum update once deflate/inflate runtime
   ownership is clear.
3. Do not delete zlib Adler state fields until the miniz WAT extraction has an
   inflate/deflate proof.

Acceptance evidence:

- `encoding-core-smoke.js`
- existing Adler parity cases in `rust-parity-encoding.js` while the Rust oracle
  still exists; after deletion, convert that runner to report an intentional
  deleted Rust oracle.

## Packet 9: Direct form/percent compatibility crates

First scan targets:

- `crates/utility/edgerun-form-urlencoded`
- `crates/utility/edgerun-percent-encoding`
- any crate-local `edgerun_form_urlencoded` or `edgerun_percent_encoding` import
  found by `rg`

Retired calls:

- `edgerun_form_urlencoded::parse`
- `edgerun_form_urlencoded::Serializer`
- `edgerun_percent_encoding::{percent_decode, percent_encode_byte}`
- permissive malformed-percent preservation

WAT replacement:

- `percent-url-form.wat::form_urlencoded_next_pair`
- `percent-url-form.wat::percent_decode_strict`
- `percent-url-form.wat::percent_encode_component`

Deletion strategy:

1. Keep `crates/utility/edgerun-percent-encoding-upstream` deleted. Its concrete
   baggage policy is extracted to `percent-url-form.wat::percent_encode_baggage`;
   do not restore upstream-shaped compatibility.
2. Delete or tombstone local form/percent crate source after no runtime caller
   imports them.
3. For form semantics, explicitly apply plus-as-space before strict percent
   decode. For generic URI semantics, preserve literal `+`.
4. Never use permissive malformed-percent preservation for signed, routed,
   cached, or policy-checked bytes.

Acceptance evidence:

- `percent-url-form-smoke.js`
- `codec-composition-http1-query.js`
- `rust-parity-misc-codecs.js`, updated to identify intended strictness
  divergence where Rust accepted malformed legacy encodings

## Dispatch order

Recommended first five packets:

1. OAuth/ACME base64url plus percent.
2. Storage event log varint.
3. HTTP chunked body.
4. DNS DoH base64url, leaving TSIG blocked on standard-base64 WAT.
5. WebSocket frame byte migration, leaving accept-string deletion blocked on
   standard-base64 WAT.

These packets remove real caller pressure without hiding the standard-base64
gap. Standard-base64 encode/decode should be the next WAT extraction if agents
need to finish DNS TSIG, WebSocket accept, terminal OSC payloads, SSH keys,
SMTP/IMAP auth, DKIM, PEM decode, Codex API payloads, cert helpers, or
tungstenite compatibility.
