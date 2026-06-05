# edgerun-percent-encoding-upstream deletion

`crates/utility/edgerun-percent-encoding-upstream` was inspected before
deletion.

Useful behavior extracted to WAT:

- W3C/OpenTelemetry baggage percent encoding policy is now owned by
  `percent-url-form.wat::percent_encode_baggage`.
- The policy encodes non-ASCII bytes, C0 controls, DEL, space, `"`, `;`, `,`,
  and `=`, while preserving other ASCII bytes such as `/`, `%`, `+`, `?`, and
  `&`.
- The existing `percent_decode_strict`, `percent_encode_component`,
  `form_urlencoded_next_pair`, and `uri_scan_path_query` exports continue to own
  canonical percent/form/query behavior.

Behavior not extracted to WAT:

- `AsciiSet`, `add`, `remove`, `union`, `complement`, `Iterator`, `Display`,
  `Cow`, and lossy UTF-8 helpers are Rust API compatibility scaffolding.
- Permissive malformed percent decoding is intentionally not preserved for
  canonical, signed, routed, cached, policy, or proof paths. Strict WAT decode
  rejects malformed escapes.

Deletion action:

- delete `crates/utility/edgerun-percent-encoding-upstream`;
- remove root workspace member and dependency `percent-encoding`;
- remove the OpenTelemetry SDK manifest edge to `percent-encoding`.

Evidence:

```bash
node standards/runners/percent-url-form-smoke.js
```

The smoke runner covers `percent_encode_baggage`,
`percent_encode_baggage_controls`, and `percent_encode_baggage_small_cap`.
