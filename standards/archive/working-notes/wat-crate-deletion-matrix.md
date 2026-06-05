# WAT crate deletion matrix

This matrix organizes the Rust compatibility cleanup by crate. Cargo health is
not an acceptance gate for this branch: once the useful portable behavior is
covered by WAT, the Rust source can be deleted or tombstoned even while old
callers still point at the removed API. Caller cleanup is tracked as follow-up
work, not as a blocker for source deletion.

Use this status language:

- `delete now`: WAT covers the useful behavior and remaining Rust value is API
  compatibility or host glue.
- `delete after one small WAT extraction`: most behavior is covered, but one
  obvious compact primitive is still missing.
- `split crate`: delete covered modules now, keep or extract the remaining
  unrelated modules crate-by-crate.
- `not this family`: do not delete in the codec-WAT pass.

## Delete now

| Crate | Covered behavior | WAT owner | Remaining Rust value | Action |
| --- | --- | --- | --- | --- |
| `crates/utility/edgerun-percent-encoding` | Strict percent decode, component encode, malformed escape rejection | `percent-url-form.wat`: `percent_decode_strict`, `percent_encode_component` | `AsciiSet`, iterators, `Cow`, permissive malformed-percent preservation, lossy UTF-8 | Delete local crate. The later upstream-shaped crate deletion extracts baggage policy and rejects the compatibility API. |
| `crates/utility/edgerun-form-urlencoded` | Form pair scanning, key-only pairs, empty values, strict percent validation, URI path/query/fragment scan | `percent-url-form.wat`: `form_urlencoded_next_pair`, `uri_scan_path_query`, `percent_decode_strict`, `percent_encode_component` | Owned iterators, `Serializer`, `EncodingOverride`, lossy UTF-8, permissive legacy behavior | Delete crate source and metadata. Form plus-as-space is adapter policy, not a crate. |
| `crates/utility/edgerun-hpack` | Prefix integers, HPACK strings, Huffman decode/validate, header-block structural scan, table arithmetic | `http-prefix-int.wat`, `hpack-string.wat`, `hpack-huffman.wat`, `hpack-header-block.wat`, `hpack-table-core.wat` | `Decoder`/`Encoder` API, static/dynamic table runtime state, callback ownership | Delete compatibility crate. Runtime state belongs in `edgerun-protocols/src/http/http2/hpack.rs` as a WAT-record adapter, not in a revived crate. |
| `crates/utility/edgerun-json` | JSON scalar scan, token tape, value-kind helpers, object field lookup, compact emission, TOML scan, YAML scan | `json-scalar.wat`, `json-tape.wat`, `json-value-core.wat`, `json-emit.wat`, `toml-scan.wat`, `yaml-scan.wat` | `JsonValue`, `Map`, macros, derive-compatible traits, IO/error glue, permissive parser conveniences | Delete/tombstone crate. Callers should move to WAT field projection, fixed emitters, raw spans, or rkyv records. |
| `crates/utility/edgerun-json-derive` | Derive output only exists to feed `edgerun-json` traits | same JSON WAT family | proc-macro compatibility for `ToJson` / `FromJson` | Delete with `edgerun-json`; do not rebuild derive over a dynamic JSON object model. |

## Crossed out: `edgerun-adler2`

`crates/utility/edgerun-adler2` is deleted. The root workspace member and
`[patch.crates-io]` override are gone. Its useful behavior, Adler-32 one-shot
and resumed update, is owned by `encoding-core.wat` exports `adler32` and
`adler32_update`. The removed Rust value was only `Adler32` struct shape,
`Hasher`, BufRead glue, and vectorized implementation details.

## Crossed out: `edgerun-time`

`crates/utility/edgerun-time` is deleted. The root workspace member and root
workspace dependency are gone. Its useful RFC3339 behavior is owned by
`time-rfc3339.wat`; the removed crate code was a reexport over deleted
`edgerun-encoding` RFC3339 helpers, host clock glue, and chrono-shaped
compatibility scaffolding. Remaining `edgerun_time::*` imports are
caller-demolition targets, not a reason to restore the crate.

## Crossed out: `edgerun-httpdate`

`crates/utility/edgerun-httpdate` is deleted. HTTP date behavior is owned by
`http-date.wat`: IMF-fixdate parse, obsolete RFC850 parse with two-digit year
mapping, obsolete asctime parse, weekday/date validation, Unix seconds
conversion for years `1970..=9999`, and exact 29-byte canonical IMF-fixdate
emission. The removed Rust value was `SystemTime`, `Display`, `FromStr`,
comparison, and `std::error` API scaffolding.

## Crossed out: `edgerun-utf-8`

`crates/utility/edgerun-utf-8` is deleted. Its useful behavior, strict UTF-8
byte scanning plus lossy repair with U+FFFD, is owned by `utf8-scan.wat`.
`encoding-text.wat` did not cover this behavior; it owns hex/base64url text
encodings. `json-scalar.wat` owns JSON string escaping and Unicode escape
emission, not raw UTF-8 validation. The removed Rust value was `BufRead` glue,
callbacks, borrowed `&str` API shape, display text, and `std::error::Error`
impls.

## Crossed out: `edgerun-simdutf8`

`crates/utility/edgerun-simdutf8` is deleted. Its useful behavior, strict
UTF-8 validation with optional `valid_up_to` / `error_len` detail, is owned by
`utf8-scan.wat`. The removed Rust value was CPU-specific SIMD dispatch,
unsafe public implementation modules, streaming validator traits, borrowed
`&str` / `&mut str` API shape, and Rust diagnostics. The only live dependency
edge was `simd_cesu8`, which has now also been deleted after CESU-8/MUTF-8
extraction.

## Crossed out: `edgerun-simd-cesu8`

`crates/utility/edgerun-simd-cesu8` is deleted. Its useful behavior, strict
CESU-8 and Java Modified UTF-8 encode/decode, is owned by `cesu8-mutf8.wat`.
That primitive covers BMP passthrough, supplementary UTF-8 to surrogate-pair
triples, MUTF-8 NUL encoding and decoding, strict surrogate-pair decode back to
UTF-8, malformed/truncated/unpaired surrogate rejection, and output-capacity
failure. The removed Rust value was `Cow` API shape, lossy wrappers, SIMD/word
dispatch, benchmark/image material, display/error wrappers, and docs.
Remaining JNI callers are deliberate fallout for the JNI string lane; do not
restore a Rust `simd_cesu8` crate.

## Crossed out: `edgerun-itoa`

`crates/utility/edgerun-itoa` is deleted. Its useful behavior, bounded decimal
formatting for `u64`, `i64`, and split-limb `u128`, is owned by
`integer-decimal.wat` (`300073`). The removed Rust value was `Buffer`,
trait-based dispatch, borrowed `&str` API shape, target-width wrappers, and
decimal-pair optimization details. Remaining `itoa` callers are intentional
caller-demolition fallout.

## Crossed out: `edgerun-combine`

`crates/utility/edgerun-combine` is deleted. Its only meaningful live caller was
JNI signature parsing in `crates/utility/edgerun-jni/src/signature.rs`; the rest
of the crate was Rust parser-combinator API scaffolding. No standalone WAT was
added for `combine`. JNI descriptor validation belongs to the JNI signature WAT
lane, and the remaining `combine` import in `signature.rs` is intentional
caller-demolition fallout.

## Crossed out: `edgerun-hashbrown`

`crates/utility/edgerun-hashbrown` is deleted. It was a Rust SwissTable
`HashMap` / `HashSet` implementation, not a portable standards behavior.
No WAT extraction was done. The removed value was collection API compatibility,
raw table internals, SIMD group scanning, allocator layout, rayon impls, and
raw-entry support. `rkyv` no longer depends on the crate; its local
sharing/pooling/validation maps now use `alloc::collections::BTreeMap`, and
the archived-hashbrown compatibility impl module is gone.

## Crossed out: `edgerun-encoding`

`crates/utility/edgerun-encoding` was drained module-by-module and then deleted.
The final empty shell (`Cargo.toml` and `src/lib.rs`) is gone, along with the
root workspace member and root workspace dependency. Remaining
`edgerun_encoding::*` imports in product crates are deliberate caller-demolition
targets against deleted APIs; Cargo health is not a deletion gate on this branch.

## Crossed out: `edgerun-percent-encoding-upstream`

`crates/utility/edgerun-percent-encoding-upstream` is deleted. Its only concrete
live upstream-shaped policy, OpenTelemetry/W3C baggage percent encoding, is now
owned by `percent-url-form.wat::percent_encode_baggage`. The removed value was
`AsciiSet` API compatibility, iterator/display/Cow wrappers, lossy UTF-8
helpers, and permissive malformed percent decode.

## Crossed out: `edgerun-eventsource-stream`

`crates/utility/edgerun-eventsource-stream` is deleted. SSE frame parsing is now
owned by `sse-event-stream.wat::sse_parse_events`: blank-line dispatch,
multi-line `data:` joining, event type override, last-event-id carry/update and
empty clear, NUL-bearing id ignore, retry capture, comment and unknown-field
ignore, optional one leading space after colon, strict UTF-8 rejection, and
bounded record/text output. The removed Rust value was async `Stream` glue,
generic transport error plumbing, `String` object shape, and display/error
compatibility.

## Crossed out: `edgerun-shlex`

`crates/utility/edgerun-shlex` is deleted. Its useful shell-like word split
behavior is owned by `shell-word-scan.wat` (`300070`): ASCII whitespace
separation, single/double quotes, backslash policy, empty quoted words,
unterminated quote rejection, and output-cap failure. The removed Rust value was
`Vec<String>` allocation, iterator/string API shape, and shell quote/join
helpers. Codex shell callers still importing `edgerun_shlex` are intentional
caller-demolition fallout.

## Crossed out: `edgerun-sixel`

`crates/utility/edgerun-sixel` is deleted. Its useful terminal image behavior is
owned by `sixel-decode.wat` (`300071`): DCS payload scanning, `!N<char>` repeat
handling, color register selection, RGB-percent color definitions, `$` and `-`
raster movement, sixel char bit mapping, corrected dimensions, empty/invalid
repeat/invalid color rejection, and output-cap failure. The removed Rust value
was `SixelImage`, `DcsSettings`, `SixelError`, allocation, display/std error
impls, and terminal-facing image struct shape. `edgerun-term-core` imports are
intentional caller-demolition fallout.

## Crossed out: `edgerun-terminal-parser`

`crates/utility/edgerun-terminal-parser` is deleted. Its useful byte-stream
scanner behavior is owned by `terminal-control-scan.wat` (`300112`): printable
spans, C0/DEL execute records, ESC classification, CSI params/final byte, OSC
BEL/ST payload spans, DCS hook/data/end records, incomplete sequence records,
and output-cap failure. The removed Rust value was the `Perform` callback trait,
`Parser` facade, nested `Params` vector API, and renderer-facing convenience
shape. `edgerun-term-core` caller imports are intentional demolition fallout.

### Already deleted or source-retired modules

| Rust module | WAT owner | Action |
| --- | --- | --- |
| `base64.rs` | `encoding-text.wat` for base64url no-pad, `encoding-base64.wat` for standard padded base64 | Keep deleted. Standard-base64 blocker is now closed. |
| `chunked.rs` | `http1-chunk-stream.wat`, `http1-body.wat` | Keep deleted. |
| `crc32.rs` | `encoding-core.wat::crc32` | Keep deleted. |
| `percent.rs` | `percent-url-form.wat` | Keep deleted. |
| `prefix_varint.rs` | `http-prefix-int.wat` | Keep deleted. |
| `quic_varint.rs` | `encoding-core.wat`, `http3-frame.wat` | Keep deleted. |
| `varint.rs` | `encoding-core.wat` | Keep deleted. |
| `hpack.rs` | HPACK WAT family | Keep deleted. |

### Deleted after WAT coverage

| Rust module | Current coverage | Caveat | Action |
| --- | --- | --- | --- |
| `hex.rs` | `encoding-text.wat`, `hex-compat.wat` | Remaining caller-specific formatting policy belongs in the caller or WAT adapter. | Deleted. |
| HPACK reexport in `lib.rs` | HPACK WAT family | Reexport pointed at deleted `edgerun-hpack`. | Deleted. |

### WAT extraction completed before deletion

| Rust module | Useful behavior | Suggested WAT target |
| --- | --- | --- |
| `base32hex.rs` | RFC 4648 extended-hex unpadded base32 for DNSSEC NSEC3 and SDK proof helpers | `encoding-base32hex.wat`: `base32hex_encode`, `base32hex_decode` |
| `quoted_printable.rs` | SMTP MIME quoted-printable body encoding | `mime-quoted-printable.wat`: line-wrapped encode and strict decode/scan if needed |
| `rfc2822.rs` | RFC 2822 UTC date formatting for email | `email-date.wat` or `rfc2822-date.wat` |
| `rfc3339.rs` | UTC timestamp parse/format/canonicalization | `time-rfc3339.wat` |
| `cstring.rs` | C string and C multi-string scanning for hardware adapters | `c-string.wat`: `c_string_scan`, `c_multi_string_next` |
| `string_field.rs` | length-prefixed string/byte fields for SSH and small protocols | `length-field.wat` with u8/u16/u32 variants |
| `frame.rs` | length-prefixed frame helpers, especially DNS TCP u16-be | `length-frame.wat` or fold DNS TCP into DNS WAT family |
| `kv.rs` | semicolon tag-list parsing used by DKIM/DMARC and formatting helpers | `tag-list.wat` for `tag=value;` scan |
| `tlv.rs` | generic TLV map helpers used by YubiKey | `generic-tlv.wat` unless DER TLV can be deliberately reused |

### Deleted or moved outside codec-WAT

| Rust module | Reason |
| --- | --- |
| `io.rs` and `buf.rs` | Rust host traits/glue, not portable codec ownership. Deleted or moved into owner-local code. |
| `ip.rs` | Covered by `ipv4-net.wat`. |
| `net.rs` | Covered by `host-port.wat`. |
| `compression.rs` | Deleted after `gzip-member.wat`, `zlib-wrapper.wat`, `deflate-stored.wat`, and `deflate-inflate.wat` covered useful behavior. |

## Protocol and app crates that become deletion queues

These are not utility crates, but they are the largest remaining caller groups
for deleted APIs.

| Caller group | Current retired dependency | WAT replacement path | First action |
| --- | --- | --- | --- |
| OAuth / ACME | `edgerun-encoding` base64url and percent, `edgerun-json` for ACME JSON | `encoding-text.wat`, `percent-url-form.wat`, `json-tape.wat`, `json-emit.wat` | Delete direct imports in `edgerun-protocols/src/oauth`, `src/acme`, and `apps/edgerun-oauth`. |
| DNS / TSIG / DNSSEC | base64url, standard base64, base32hex, DNS name/header helpers | `encoding-text.wat`, `encoding-base64.wat`, DNS WAT family; add `encoding-base32hex.wat` | Delete DoH base64url first; extract base32hex before NSEC3 cleanup. |
| WebSocket | standard base64 accept, frame helpers | `ws-accept.wat`, `ws-frame.wat` | Replace all accept-string code with `ws_accept_key`; delete incorrect `encode_u64_base64` path. |
| `edgerun-tungstenite` | WebSocket handshake accept, frame header parse/format, mask transform, close payload shape | `ws-accept.wat`, `ws-frame.wat` | Deleted after `ws-frame` gained masked client header, full header parse, and close payload coverage. Remaining `edgerun-tungstenite` callers are intentional demolition fallout. |
| SSH / terminal | standard base64, authorized-key scanning, length fields | `encoding-base64.wat`, `ssh-authorized-key.wat`; add `length-field.wat` | Delete SSH authorized-key Rust scan/base64 split; keep key structure parsing only if still needed. |
| Email auth / SMTP | standard base64, DKIM body canonicalization, tag-list parsing, quoted-printable | `encoding-base64.wat`, `dkim-body.wat`; add `tag-list.wat`, `mime-quoted-printable.wat` | Delete DKIM body canonicalization Rust first; extract tag-list next. |
| HTTP/2 / HTTP/3 | `edgerun-hpack`, QPACK byte parsers, frame headers | HPACK/QPACK WAT family, `http2-frame.wat`, `http3-frame.wat` | Delete `edgerun-hpack` crate now; build runtime adapter in protocol crate as follow-up. |
| OCI / Codex / apps using dynamic JSON | `edgerun-json`, `edgerun-json-derive` | JSON WAT family and explicit schema/record emitters | Delete derive/model dependency, then project fixed records from token spans. |

## Next extraction targets by crate-deletion leverage

1. `encoding-base32hex.wat`: unlocks DNSSEC NSEC3 and remaining base32hex
   callers.
2. `tag-list.wat`: unlocks DKIM/DMARC tag parsing and removes another
   `edgerun-encoding::kv` reason.
3. `mime-quoted-printable.wat`: unlocks SMTP MIME body encoding.
4. `time-rfc3339.wat` and `rfc2822-date.wat`: keep `edgerun-time` deleted and
   unlock validators and email date formatting without a chrono compatibility
   crate.
5. `c-string.wat`: unlocks Bluetooth, V4L2, ALSA, CEC, TPM/YubiKey style
   hardware-adapter string cleanup.
6. `generic-tlv.wat`: unlocks YubiKey TLV helpers.
7. `length-field.wat` / `length-frame.wat`: unlocks SSH string fields and DNS
   TCP frame helpers.

## Deletion rule for agents

When a crate row says `delete now`, agents may remove Rust source and crate
metadata even if `rg` still finds callers. They should add or update a standards
queue naming those callers instead of preserving compatibility shims. Runtime
ambiguity is worse than a clear missing API during this extraction branch.
