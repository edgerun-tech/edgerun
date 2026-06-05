# edgerun-percent-encoding Retirement

`crates/utility/edgerun-percent-encoding` is retired with the
`edgerun-form-urlencoded` batch. The useful portable byte behavior is covered by
`standards/build/wasm/codec-primitives/percent-url-form.wat`; the remaining Rust
crate is an allocation/iterator compatibility API.

This note applies only to the local crate named `edgerun-percent-encoding`.
`crates/utility/edgerun-percent-encoding-upstream` was retired later after
OpenTelemetry/W3C baggage percent encoding was extracted to
`percent-url-form.wat::percent_encode_baggage`.

## Rust Behavior

The local crate exposes:

- `percent_encode_byte`: unconditional uppercase `%XX` emission for one byte.
- `percent_encode` / `utf8_percent_encode`: iterator APIs controlled by an
  `AsciiSet`.
- `AsciiSet`: a 128-bit ASCII mask with `add`, `remove`, `union`, and
  `complement` helpers.
- `CONTROLS` and `NON_ALPHANUMERIC` masks.
- `percent_decode` / `percent_decode_str`: permissive byte decode that consumes
  valid `%HH` escapes and preserves malformed `%` bytes.
- `decode_utf8` / `decode_utf8_lossy`: allocation-facing UTF-8 conversion
  helpers.

The only direct workspace consumer of this local crate is
`edgerun-form-urlencoded`, which uses `percent_decode` and
`percent_encode_byte`. Generic upstream-style `AsciiSet` usage lives in
`edgerun-percent-encoding-upstream`, which is now deleted after WAT extraction.

## WAT Ownership

`percent-url-form.wat` owns the portable behavior that remains useful for codec
pipelines:

- `percent_decode_strict`: decodes `%HH` with uppercase or lowercase hex and
  rejects truncated or malformed escapes.
- `percent_encode_component`: emits uppercase `%XX` and supports named encode
  modes:
  - mode `0`: component mode, preserving RFC 3986 unreserved bytes only.
  - mode `1`: path mode, also preserving `/`.
  - mode `2`: query mode, also preserving `/`, `?`, `&`, and `=`.
- `form_urlencoded_next_pair`: scans `application/x-www-form-urlencoded` pairs,
  returns key/value byte spans, skips empty `&` segments, and rejects malformed
  percent escapes.
- `uri_scan_path_query`: separates path, query, and fragment byte spans for URI
  composition.

The WAT primitive intentionally does not preserve the Rust iterator API, Cow
borrowing choices, lossy UTF-8 helpers, or configurable `AsciiSet` object model.
Those are host/API conveniences rather than durable codec behavior.

## Strictness Policy

This retirement adopts strict WAT behavior as canonical:

- Malformed percent escapes such as `%`, `%G0`, and `%0G` are rejected rather
  than preserved.
- Generic percent decode preserves literal `+`; form callers must apply
  form-specific plus-as-space handling at the form policy layer.
- Encoding always emits uppercase hex.

The old permissive Rust behavior is not suitable for canonical URI, manifest,
package, route, policy, cache-key, or proof paths because malformed escapes can
create alternate byte spellings. Legacy import or display paths that need
permissive decoding should use an explicitly named compatibility boundary, not
this retired local crate.

## Caller Decision

Same-batch retirement is justified:

- `rg` shows the local crate is directly used by `edgerun-form-urlencoded`.
- OpenTelemetry's upstream-style baggage policy is now covered by
  `percent-url-form.wat::percent_encode_baggage`; the `AsciiSet` API is not
  preserved.
- WAT already covers the form-urlencoded batch's required percent byte behavior.

After this batch, `edgerun-form-urlencoded` should consume the WAT
`percent-url-form` scanner/encoder pipeline or be deleted as Rust source.
