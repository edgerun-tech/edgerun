# edgerun-form-urlencoded / edgerun-percent-encoding caller audit

This audit scopes retirement of the local compatibility crates:

- `crates/utility/edgerun-form-urlencoded`
- `crates/utility/edgerun-percent-encoding`

`crates/utility/edgerun-percent-encoding-upstream` was later deleted after its
only concrete live upstream-shaped policy, OpenTelemetry/W3C baggage percent
encoding, was extracted to `percent-url-form.wat::percent_encode_baggage`.

## Verified counts

Commands were run from the repository root.

```text
rg -l "edgerun-form-urlencoded|edgerun_form_urlencoded|form_urlencoded" Cargo.toml crates standards --glob '!target/**' | wc -l
# 15

rg -l "edgerun-percent-encoding|edgerun_percent_encoding" Cargo.toml crates standards --glob '!target/**' | wc -l
# 8

rg -l "percent-encoding|percent_encoding" Cargo.toml crates standards --glob '!target/**' | wc -l
# 13

rg -l "edgerun_form_urlencoded|form_urlencoded" crates --glob '*.rs' | wc -l
# 3

rg -l "edgerun_percent_encoding" crates --glob '*.rs' | wc -l
# 1

rg -l "use percent_encoding|percent_encoding::" crates --glob '*.rs' | wc -l
# 4
```

Rust source size in the local retirement batch:

```text
331 crates/utility/edgerun-form-urlencoded/src/lib.rs
172 crates/utility/edgerun-percent-encoding/src/ascii_set.rs
344 crates/utility/edgerun-percent-encoding/src/lib.rs
```

## Delete crate-local API

These files are local compatibility surfaces and can be retired after the WAT
pipeline is accepted as canonical:

- `crates/utility/edgerun-form-urlencoded/src/lib.rs`
  - `parse`
  - `Parse`
  - `ParseIntoOwned`
  - `byte_serialize`
  - `ByteSerialize`
  - `Serializer`
  - `Target`
  - `EncodingOverride`
  - local plus-as-space decode glue
  - string-backed serializer helpers

- `crates/utility/edgerun-percent-encoding/src/lib.rs`
  - `percent_encode_byte`
  - `percent_encode`
  - `utf8_percent_encode`
  - `PercentEncode`
  - `percent_decode`
  - `percent_decode_str`
  - `PercentDecode`
  - permissive malformed-percent preservation
  - UTF-8 and lossy UTF-8 decode convenience methods

- `crates/utility/edgerun-percent-encoding/src/ascii_set.rs`
  - `AsciiSet`
  - `CONTROLS`
  - `NON_ALPHANUMERIC`
  - set algebra helpers used only by the local percent crate

The only direct local Rust dependency is inside the form crate:

- `crates/utility/edgerun-form-urlencoded/src/lib.rs`
  - `use edgerun_percent_encoding::{percent_decode, percent_encode_byte};`

## Replace with WAT pipeline

Canonical portable behavior is already represented by:

- `standards/build/wasm/codec-primitives/percent-url-form.wat`
- `standards/runners/percent-url-form-smoke.js`
- `standards/runners/codec-composition-http1-query.js`
- `standards/components/manifests/codec-primitives/percent-url-form.toml`
- `standards/corpus/codec-primitives/percent-url-form.json`

The replacement functions are:

- `percent_decode_strict`
- `percent_encode_component`
- `form_urlencoded_next_pair`

Important policy difference:

- Rust `edgerun-percent-encoding` preserves malformed percent escapes.
- Rust `edgerun-form-urlencoded` first maps `+` to space, then uses the
  permissive percent decoder.
- WAT `percent_decode_strict` rejects malformed escapes.
- WAT `form_urlencoded_next_pair` returns raw key/value spans; callers decide
  whether to apply plus-as-space and strict percent decoding.

This is a desirable canonical/proof behavior, but it is not a drop-in
compatibility shim for legacy display/import paths.

Current WAT/standards references:

- `standards/build/wasm/codec-primitives/percent-url-form.wat`
- `standards/components/manifests/codec-primitives/percent-url-form.toml`
- `standards/corpus/codec-primitives/percent-url-form.json`
- `standards/runners/percent-url-form-smoke.js`
- `standards/runners/codec-composition-http1-query.js`
- `standards/runners/rust-parity-misc-codecs.js`
- `standards/wat-codec-strictness-policy.md`
- `standards/wat-codec-deletion-audit.md`
- `standards/wat-codec-replacement-audit.md`
- `standards/next-batch-http1.md`

Runtime-style form behavior already appears in:

- `crates/edgerun-sdk/src/runtime.rs`
  - `form_urlencoded_unit_locates_and_decodes_values`
  - `form_urlencoded_locate`
  - `form_urlencoded_decode`

Those names are not imports of the local crates. They are useful evidence that
form query handling is already moving toward explicit runtime units.

## Leave upstream compatibility

Later batch status:

- `crates/utility/edgerun-percent-encoding-upstream`

The root workspace dependency was deleted:

- `Cargo.toml`
  - deleted workspace member: `crates/utility/edgerun-percent-encoding-upstream`
  - deleted workspace dependency: `percent-encoding`

Concrete upstream callers:

- `crates/utility/edgerun-opentelemetry-sdk-upstream/Cargo.toml`
  - `[dependencies.percent-encoding]`
  - `path = "../edgerun-percent-encoding-upstream"`
  - `optional = true`

- `crates/utility/edgerun-opentelemetry-sdk-upstream/src/propagation/baggage.rs`
  - `use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};`

Upstream crate-local docs/tests also mention `percent_encoding`, but those are
self references and are not a reason to touch it in this batch:

- `crates/utility/edgerun-percent-encoding-upstream/src/lib.rs`
- `crates/utility/edgerun-percent-encoding-upstream/src/ascii_set.rs`
- `crates/utility/edgerun-percent-encoding-upstream/Cargo.toml`

## Manifest cleanup

After deleting or tombstoning the local crates, cleanup should be limited to the
local compatibility crates and their direct dependency edge:

- `crates/utility/edgerun-form-urlencoded/Cargo.toml`
  - remove or tombstone the library target
  - remove `edgerun-percent-encoding = { path = "../edgerun-percent-encoding", features = ["alloc"] }`

- `crates/utility/edgerun-percent-encoding/Cargo.toml`
  - remove or tombstone the library target

- `crates/utility/edgerun-form-urlencoded/Cargo.lock`
  - local crate lockfile can be removed or left as tombstone metadata

- `crates/utility/edgerun-percent-encoding/Cargo.lock`
  - local crate lockfile can be removed or left as tombstone metadata

Root `Cargo.toml` previously listed only the upstream percent crate:

- `crates/utility/edgerun-percent-encoding-upstream`
- `percent-encoding = { path = "crates/utility/edgerun-percent-encoding-upstream" }`

Both entries are now deleted.

## Retirement recommendation

Retire both local crates as a single batch:

1. Keep `edgerun-percent-encoding-upstream` deleted.
2. Add a deletion note for the local form/percent crates.
3. Keep `percent-url-form.wat` as the canonical portable implementation.
4. Update `rust-parity-misc-codecs.js` to report a deleted Rust oracle for this
   local crate pair.
5. Delete or tombstone the local crate source files.
6. Run:

```bash
node standards/runners/percent-url-form-smoke.js
node standards/runners/codec-composition-http1-query.js
node standards/runners/rust-parity-misc-codecs.js
```

Then run the full codec smoke sweep and duplicate standard-id scan.
