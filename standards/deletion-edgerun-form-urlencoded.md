# edgerun-form-urlencoded retirement map

`crates/utility/edgerun-form-urlencoded` is a no-std compatibility crate for
`application/x-www-form-urlencoded` parsing and serialization. The useful
portable byte behavior now belongs to `percent-url-form.wat`; the remaining
Rust value is API compatibility, allocation, lossy UTF-8 conversion, and legacy
permissive malformed-percent handling.

## Source scope

Read for this retirement pass:

- `crates/utility/edgerun-form-urlencoded/src/lib.rs`
- `standards/build/wasm/codec-primitives/percent-url-form.wat`
- `standards/runners/percent-url-form-smoke.js`
- `standards/runners/codec-composition-http1-query.js`
- `standards/wat-codec-strictness-policy.md`

## WAT ownership

The portable codec ownership lives in:

- `standards/build/wasm/codec-primitives/percent-url-form.wat`
  - `percent_decode_strict`
  - `percent_encode_component`
  - `form_urlencoded_next_pair`
  - `uri_scan_path_query`

The WAT module owns canonical strict byte scanning and encoding. It does not own
Rust allocation APIs, iterator shapes, `Cow` lifetimes, lossy UTF-8 conversion,
or compatibility parsing for malformed percent escapes.

## Behavior map

| Rust behavior | Source | Replacement / decision |
| --- | --- | --- |
| Iterate nonempty `&`-separated form pairs | `parse`, `Parse::next` | `percent-url-form.wat::form_urlencoded_next_pair` skips empty `&` segments and returns key/value spans plus the next offset. |
| Split each pair at the first `=` and treat missing value as empty | `Parse::next` | `form_urlencoded_next_pair` returns key span, value span, and zero value length for key-only pairs. |
| Replace `+` with space before form decoding | `decode`, `replace_plus` | Keep as explicit form policy in the WAT/runtime pipeline. `percent_decode_strict` intentionally preserves `+`; form callers must apply plus-as-space only for form fields. |
| Percent-decode field bytes | `decode` through `edgerun_percent_encoding::percent_decode` | `percent_decode_strict` for canonical/proof paths after the form field span is selected. |
| Preserve malformed `%` sequences as literal bytes | `percent_decode` dependency behavior observed by this crate | Do not preserve for canonical paths. WAT rejects malformed escapes with status `3`; permissive preservation is legacy import/display behavior only. |
| Decode bytes to UTF-8 lossily | `decode_utf8_lossy` | Delete as codec behavior. A runtime/UI importer may apply lossy display conversion after strict byte validation, but proof and policy paths should operate on bytes or validated UTF-8. |
| `ParseIntoOwned` owned string iterator | `Parse::into_owned`, `ParseIntoOwned` | Delete as Rust API convenience. WAT returns spans/status; ownership belongs to the caller/runtime adapter. |
| Form byte serialization with unescaped `* - . 0-9 A-Z _ a-z` and space as `+` | `byte_serialize`, `byte_serialized_unchanged` | Partially covered by `percent_encode_component` for strict percent encoding. Space-as-`+` is form-specific serialization policy and should be an explicit adapter step, not generic percent encoding. |
| Uppercase percent hex | `byte_serialize` via `percent_encode_byte` | `percent_encode_component` emits uppercase hex. |
| `Serializer` appending `&`, `=`, pairs, key-only items, suffix start position, and clear/finish behavior | `Serializer`, `append_pair`, `append_key_only`, `append_separator_if_needed` | Delete as Rust API/target glue. The portable behavior is separator placement and key/value component encoding; runtime adapters can build output buffers from WAT encode calls. |
| `EncodingOverride` callback | `encode`, `EncodingOverride` | Delete. It is caller-supplied Rust API policy, not a canonical wire codec. |
| URI path/query/fragment scan for HTTP composition | not in Rust form crate; adjacent useful behavior | `uri_scan_path_query` plus `codec-composition-http1-query.js` proves composition with `http1-lines.wat`. |

## Strictness decision

The canonical WAT path is intentionally stricter than the Rust compatibility
crate:

- Generic percent decoding preserves literal `+`; form decoding applies
  plus-as-space as a named form policy.
- Malformed percent escapes such as `%`, `%G0`, and `%0G` are rejected by WAT.
  The Rust compatibility path preserves them.
- WAT returns byte spans and byte output. It does not apply lossy UTF-8
  replacement.
- WAT scan/encode functions return explicit statuses for malformed input and
  insufficient output capacity.

This matches `standards/wat-codec-strictness-policy.md`: strict WAT codecs are
the default for canonical validation, proof preimages, policy hashes, package
manifests, admission decisions, settlement evidence, and cache keys. Permissive
Rust behavior is acceptable only at explicitly named import, diagnostics, or
legacy compatibility boundaries.

## Deletion readiness

Ready to delete from `edgerun-form-urlencoded` once this batch retires the
crate:

- `parse` / `Parse` / `ParseIntoOwned`: replaced by WAT pair scanning plus
  caller-owned decoding.
- `replace_plus`: keep only as named form policy in the adapter, not generic
  percent behavior.
- `decode_utf8_lossy`: delete with the Rust API; it is not portable canonical
  codec behavior.
- `byte_serialize`: replace with WAT strict component encoding plus explicit
  form-space policy if serialization is still needed.
- `Serializer` and `Target`: delete as Rust allocation/API compatibility.
- `EncodingOverride`: delete as noncanonical caller policy.

The companion local crate `crates/utility/edgerun-percent-encoding` can be
retired in the same family after its direct callers are audited.
`edgerun-percent-encoding-upstream` was deleted in a later batch after
OpenTelemetry/W3C baggage percent encoding was extracted to
`percent-url-form.wat::percent_encode_baggage`.

## Proofs to keep attached

Existing runner coverage relevant to this retirement:

- `standards/runners/percent-url-form-smoke.js`
  - strict percent decode success
  - malformed percent rejection
  - output-capacity errors
  - strict percent component encoding
  - path and query encode modes
  - form pair scanning for normal, empty-value, and key-only pairs
  - malformed-percent form rejection
  - URI path/query/fragment scanning
- `standards/runners/codec-composition-http1-query.js`
  - `http1-lines.wat -> percent-url-form.wat`
  - HTTP request target scan
  - query pair spans
  - strict percent decode of a query value

Do not use `cargo check` as the acceptance criterion for the source deletion
pass. The point of the batch is to retire the old Rust compatibility API after
the useful byte behavior is proven in WAT.
