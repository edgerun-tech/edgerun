# Percent/Form Local Crate Deletion Queue

This queue crosses out the local compatibility crates:

- `crates/utility/edgerun-percent-encoding`
- `crates/utility/edgerun-form-urlencoded`

Do not touch `crates/utility/edgerun-percent-encoding-upstream`. It remains the
upstream-shaped `percent-encoding` compatibility crate for OpenTelemetry and any
other caller that needs that API shape.

## Canonical WAT

Useful portable behavior is owned by:

- `standards/build/wasm/codec-primitives/percent-url-form.wat`
- `standards/components/manifests/codec-primitives/percent-url-form.toml`
- `standards/runners/percent-url-form-smoke.js`

Exports:

- `percent_decode_strict`: strict `%XX` decode; malformed escapes are rejected.
- `percent_encode_component`: component/path/query percent encoding by mode.
- `form_urlencoded_next_pair`: scan the next nonempty `&`-separated form pair
  and return key/value spans, including key-only and empty-value cases.
- `uri_scan_path_query`: split URI path, query, and fragment spans.

Plus-as-space is caller policy. The WAT form scanner returns raw spans, so a
caller that is specifically processing `application/x-www-form-urlencoded` must
apply `+ -> space` before strict percent decode.

## Deleted Local Metadata

The Rust source for both local crates had already been removed before this
queue. This pass removed the remaining stale crate metadata:

- `crates/utility/edgerun-form-urlencoded/Cargo.toml`
- `crates/utility/edgerun-form-urlencoded/Cargo.lock`
- `crates/utility/edgerun-form-urlencoded/README.md`
- `crates/utility/edgerun-percent-encoding/Cargo.toml`
- `crates/utility/edgerun-percent-encoding/Cargo.lock`
- `crates/utility/edgerun-percent-encoding/README.md`

The empty crate directories may remain only as untracked filesystem directories;
they are no longer crate metadata.

## Remaining References

Workspace root references are only for the upstream-shaped compatibility crate:

- `Cargo.toml`: workspace member `crates/utility/edgerun-percent-encoding-upstream`
- `Cargo.toml`: workspace dependency `percent-encoding = { path = "crates/utility/edgerun-percent-encoding-upstream" }`

Those are intentionally retained.

Standards docs still mention the deleted local crates as historical parity
sources and deletion evidence. Those references are documentation, not live Rust
callers.

`standards/runners/rust-parity-misc-codecs.js` no longer compiles a Rust oracle
for these deleted crates. Its percent/form section now records fixed WAT
expectations plus a deleted-oracle note, while dedicated smoke and composition
runners keep executable coverage.

Runtime-style WAT evidence already exists in:

- `crates/edgerun-sdk/src/runtime.rs`
  - `form_urlencoded_unit_locates_and_decodes_values`
  - `form_urlencoded_locate`
  - `form_urlencoded_decode`

These are not imports of the deleted crates.

## Replacement Map

| Deleted Rust surface | WAT replacement |
| --- | --- |
| `edgerun_percent_encoding::percent_decode` | `percent_decode_strict`, with explicit form plus-as-space only when applicable |
| `edgerun_percent_encoding::percent_decode_str` | `percent_decode_strict` plus caller-owned UTF-8 policy |
| `edgerun_percent_encoding::percent_encode_byte` | `percent_encode_component` |
| `edgerun_percent_encoding::percent_encode` | `percent_encode_component` |
| `edgerun_percent_encoding::utf8_percent_encode` | `percent_encode_component` plus caller-owned UTF-8 input validation |
| `edgerun_percent_encoding::AsciiSet` | no WAT object; encode mode/policy is the boundary |
| `edgerun_form_urlencoded::parse` | `form_urlencoded_next_pair` plus strict field decode policy |
| `edgerun_form_urlencoded::byte_serialize` | `percent_encode_component` |
| `edgerun_form_urlencoded::Serializer` | caller-owned buffer assembly using `percent_encode_component` |

## Blockers

No blocker remains for crossing out the two local crates.
