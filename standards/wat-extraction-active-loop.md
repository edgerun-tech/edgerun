# WAT extraction active loop

Last updated: 2026-06-05.

This file is the live coordination surface for the Rust-to-WAT deletion loop.
Keep it current whenever an agent lands useful behavior, deletes source, or
frees a worker slot.

Cargo health is not an acceptance gate for this branch. The rule is:

1. Extract valuable portable behavior to compact WAT.
2. Prove it with smoke or composition runners.
3. Delete covered Rust source and Rust-only compatibility scaffolding.
4. Route remaining callers through WAT composition or owner-local records.
5. Immediately backfill freed agent slots from the queue below.

Do not port Rust-specific traits, `Vec` facades, iterator adapters, derive
macros, or compatibility object models to WAT. Delete or move those to their
owning caller when they are only host-language glue.

## Active lanes

### WebSocket / Tungstenite

Status:

- `crates/utility/edgerun-tungstenite` is crossed out. Source, crate metadata,
  root workspace member, root workspace dependency, and lock package entry are
  deleted.
- `ws-accept.wat` owns WebSocket accept-key SHA-1 plus standard-base64 behavior.
- `ws-frame.wat` owns frame prefix decode, extended payload length decode,
  full header parse, server/general header formatting, mask XOR, and close
  payload status/reason-byte shape.

Verified WAT coverage:

- `node standards/runners/ws-frame-smoke.js`
- `node standards/runners/ws-accept-smoke.js`

Current gates:

- Remaining `edgerun-tungstenite` references are caller demolition targets.
  Do not restore the Rust compatibility crate or stream/config facades.

Documentation:

- `standards/deletion-edgerun-tungstenite.md`

### Compression and miniz

Primary target:

- Cross out `crates/utility/edgerun-encoding`: done. The useful source behavior,
  final crate shell, root workspace member, and root workspace dependency are
  deleted.
- Cross out `crates/utility/edgerun-miniz-oxide`: crate source and metadata are
  deleted.

Current Rust source left:

- `crates/utility/edgerun-encoding` has no remaining source or metadata files
  worth preserving. Remaining `edgerun_encoding::*` imports are caller
  demolition targets against already-deleted APIs.
- `crates/utility/edgerun-miniz-oxide` has no files left in this checkout.

Verified WAT coverage:

- `gzip-member.wat` (`300064`)
- `zlib-wrapper.wat` (`300065`)
- `deflate-stored.wat` (`300066`)
- `deflate-inflate.wat` (`300067`) for stored-block, fixed-Huffman, and
  dynamic-Huffman inflate
- `compression-zlib-composition-smoke.js`
- `compression-gzip-composition-smoke.js`

Current gates:

- Runtime caller routing for OCI read, HTTP compression, APK ZIP extraction, and tor
  bench. These product paths still need a real host-side WASM invocation
  surface; do not add a Rust miniz compatibility shim.
- VFS has the first owner-local compression adapter boundary:
  `VfsRawDeflateWatAdapter` in
  `crates/authority/edgerun-vfs/src/packet.rs`. Default VFS writes remain
  uncompressed, and raw DEFLATE still fails closed through
  `VfsRawDeflateWatUnavailable` until a host invokes `deflate-stored.wat` and
  `deflate-inflate.wat`.
- OCI gzip write has an owner-local adapter boundary:
  `OciGzipWatAdapter` in
  `crates/edgerun-oci/src/registry/tar_push.rs`. It names the required host
  calls for `encoding-core.wat::crc32`,
  `gzip-member.wat::gzip_member_write_header`,
  `deflate-stored.wat::deflate_stored_encode`, and
  `gzip-member.wat::gzip_member_write_trailer`. The default
  `OciGzipWatUnavailable` path fails closed until a host supplies those calls.

Active or recently assigned agents:

- Dynamic Huffman extension for `deflate-inflate.wat` is done.
- `standards/compression-wat-caller-routing.md` records the remaining adapter
  boundaries.

Documentation:

- `standards/wat-miniz-compression-extraction-queue.md`
- `standards/deletion-edgerun-miniz-oxide-queue.md`
- `standards/compression-wat-caller-routing.md`

### Adler-32

Status:

- `crates/utility/edgerun-adler2` is crossed out. Source, metadata, root
  workspace member, and root `adler2` patch entry are deleted.
- Useful Adler-32 behavior is WAT-owned by `encoding-core.wat`:
  `adler32(ptr, len)` and `adler32_update(seed, ptr, len)`.
- `edgerun-miniz-oxide` is already crossed out, so the old Rust Adler dependency
  path is gone.

Documentation:

- `standards/deletion-edgerun-adler2.md`
- `standards/deletion-edgerun-adler2-caller-audit.md`

### Time

Status:

- `crates/utility/edgerun-time` is crossed out. Source, crate metadata, root
  workspace member, and root workspace dependency are deleted.
- RFC3339 parse, format, and canonicalization are owned by `time-rfc3339.wat`.
- `now_unix_millis` / `now_unix_micros` were host clock glue. Callers should
  own their clock boundary locally.
- The `chrono` module was compatibility scaffolding over UTC seconds, duration
  arithmetic, partial date formatting, and RFC3339. Do not recreate it as a
  shared crate.

Documentation:

- `standards/deletion-edgerun-time.md`

### HTTP date

Status:

- `crates/utility/edgerun-httpdate` is crossed out. Source and crate metadata
  are deleted.
- HTTP wire-date behavior is owned by `http-date.wat` (`300103`):
  IMF-fixdate, obsolete RFC850, obsolete asctime, RFC850 two-digit year mapping,
  weekday/date validation, Unix seconds conversion, and exact 29-byte canonical
  IMF-fixdate formatting.
- The deleted Rust crate only added `SystemTime`, `Display`, `FromStr`,
  comparison, and `std::error` API scaffolding around that behavior.

Documentation:

- `standards/deletion-edgerun-httpdate.md`

### UTF-8

Status:

- `crates/utility/edgerun-utf-8` is crossed out. Source and crate metadata are
  deleted.
- Raw UTF-8 validation and lossy repair are owned by `utf8-scan.wat`
  (`300068`), not by `encoding-text.wat` or `json-scalar.wat`.
- `BufReadDecoder`, callback wrappers, borrowed `&str` return shapes, and Rust
  error/display impls were API scaffolding and were deleted rather than ported.

Documentation:

- `standards/deletion-edgerun-utf-8.md`

### CESU-8 and Java MUTF-8

Status:

- `crates/utility/edgerun-simd-cesu8` is crossed out. Source, crate metadata,
  root workspace member, root workspace dependency, and lockfile package entry
  are deleted.
- Strict CESU-8 and MUTF-8 encode/decode behavior is owned by
  `cesu8-mutf8.wat` (`300069`).
- The deleted Rust crate's `Cow`, lossy wrapper, SIMD dispatch, docs, benchmark,
  image, and error wrapper surfaces were compatibility scaffolding and were not
  ported.

Current caller fallout:

- JNI string helpers in `crates/utility/edgerun-jni` and
  `crates/utility/edgerun-jni-macros` still reference `simd_cesu8`. Leave them
  broken until the JNI string lane routes Java MUTF-8 C-string behavior through
  WAT or caller-owned host adapters.

Verified WAT coverage:

- `cesu8-mutf8.wat`
- `cesu8-mutf8-smoke.js`

Documentation:

- `standards/deletion-edgerun-simd-cesu8.md`

### Shell word scan

Status:

- `crates/utility/edgerun-shlex` is crossed out. Source, crate metadata, root
  workspace member, root workspace dependency, lock package entry, and the Codex
  shell manifest dependency are deleted.
- Shell-like word splitting is owned by `shell-word-scan.wat` (`300070`):
  ASCII whitespace separation, single and double quote handling, backslash
  escaping, empty quoted words, and unterminated quote rejection.
- The deleted Rust crate's quote/join helpers were string allocation API
  scaffolding and were not ported.
- Remaining `edgerun_shlex::*` imports in Codex shell source are deliberate
  caller-demolition targets against deleted APIs.

Documentation:

- `standards/deletion-edgerun-shlex.md`

### Server-Sent Events

Status:

- `crates/utility/edgerun-eventsource-stream` is crossed out. Source, crate
  metadata, root workspace member, root workspace dependency, and lock package
  entry are deleted.
- SSE frame parsing is owned by `sse-event-stream.wat` (`300072`) through
  `sse_parse_events`.
- The WAT parser covers blank-line dispatch, joined multi-line `data:`, event
  type override, last-event-id carry/update/empty clear, NUL-bearing id ignore,
  numeric retry capture, comment and unknown-field ignore, optional one leading
  space after colon, strict UTF-8 rejection, and bounded record/text output.
- Remaining Codex/API imports of `edgerun_eventsource_stream` are
  caller-demolition targets. Do not restore the Rust async `Stream` wrapper.

Documentation:

- `standards/deletion-edgerun-eventsource-stream.md`

Final compression deletion status:

- `crates/utility/edgerun-encoding/src/compression.rs` is deleted.
- `crates/utility/edgerun-encoding/Cargo.toml` and
  `crates/utility/edgerun-encoding/src/lib.rs` are deleted.
- `crates/utility/edgerun-miniz-oxide` source and metadata are deleted and its
  root workspace/dependency entries are removed.
- Remaining direct code references are intentional broken adapter boundaries in
  VFS, OCI gzip read/write, HTTP compression, APK ZIP method 8, and tor bench. They should be
  routed to host-side calls for `gzip-member.wat`, `zlib-wrapper.wat`,
  `deflate-stored.wat`, and `deflate-inflate.wat`, not to a restored Rust
  facade.

### JSON caller demolition

Primary target:

- Keep `crates/utility/edgerun-json` and `crates/utility/edgerun-json-derive`
  deleted.
- Remove callers by replacing dynamic Rust JSON objects with WAT field
  projection, fixed emitters, raw spans, or rkyv records.

Verified WAT coverage:

- `json-scalar.wat`
- `json-tape.wat`
- `json-value-core.wat`
- `json-emit.wat`
- `toml-scan.wat`
- `yaml-scan.wat`

Current queue:

- Codex JSON-RPC and tool schema surfaces.
- Codelyzer TOML/JSON/YAML callers, now scoped in
  `standards/codelyzer-json-toml-caller-routing.md`.

Completed caller slices:

- `crates/edgerun-codex/app-protocol/src/jsonrpc_lite.rs` no longer imports
  `edgerun-json` or derives generic JSON traits. JSON-RPC request,
  notification, response, and error records now use fixed field projection and
  emission for `jsonrpc`, `id`, `method`, `params`, `trace`, `result`, `error`,
  `code`, `message`, and `data`, carrying method payloads as validated raw JSON
  spans until the owning protocol record projects them.
- `crates/edgerun-codex/app-protocol/src/protocol/common.rs` no longer converts
  `JSONRPCRequest` / `JSONRPCNotification` through deleted generic JSON
  values. It dispatches by method and returns explicit
  `JsonrpcProtocolProjectionError` records with direction, method, expected
  params type, expected field set, and raw params span length until each method
  payload has a WAT-backed projector. Its generated response/notification
  helpers also no longer call generic `edgerun_json::to_value` /
  `from_json_value`; they fail closed until fixed emit/project paths land.
  Target scan status for `common.rs`: `85 -> 75`, with remaining refs in the
  embedded test module.
- `crates/apps/edgerun-oauth` token response, credential store, OIDC
  discovery, JWKS, and JWT claim/JWK verifier JSON. It no longer imports or
  depends on `edgerun-json`; the temporary Rust side is an owner-local
  fixed-field scanner/emitter until hosts invoke the JSON WAT modules directly.
- `crates/apps/edgerun-secret-service` metadata JSON is owner-local fixed
  emission/parsing for `label`, `attributes`, and `created_us`; it no longer
  imports or depends on `edgerun-json`.
- `crates/edgerun-oci/src/registry/push_manifest.rs` emits the fixed OCI push
  manifest shape directly.
- `crates/edgerun-oci/src/registry/provenance.rs`,
  `crates/edgerun-oci/src/registry/pull.rs`, and
  `crates/edgerun-oci/src/state.rs` no longer use `edgerun-json`; provenance
  and state now use scoped fixed JSON emitters, and state loading projects only
  `ociVersion`, `id`, `status`, `pid`, `bundle`, and string `annotations`.
- `crates/edgerun-oci/src/registry/client.rs` no longer uses `edgerun-json` for
  registry token responses; it projects only the `token` and `access_token`
  fields used by the client.
- `crates/edgerun-oci/src/hooks.rs` no longer uses `edgerun-json`; hook stdin
  state is emitted directly as the fixed OCI object with scoped string escaping.
- `crates/edgerun-oci/src/clap/cli.rs` no longer uses deleted JSON `Value`
  storage for argument matches.
- OCI CLI JSON output in `features`, `events`, `state`, `ps`, `images`, and
  `inspect` now uses scoped fixed string emitters through
  `crates/edgerun-oci/cli/json.rs`.
- `crates/edgerun-oci/bin/ert.rs` no longer uses `parse_json_tape` for dead
  runtime-state PID cleanup.
- `crates/edgerun-codelyzer/src/codealyzer/workspace_membership.rs` no longer
  uses `edgerun_json::parse_json_tape`; it projects only cargo metadata
  `workspace_members`, package `id`, and package `manifest_path`.
- `crates/edgerun-codelyzer/src/codealyzer/tests.rs` no longer uses
  `edgerun_json::Value` or `parse_json` for compiler-message warning checks.
- `crates/edgerun-codelyzer/src/codealyzer/issue_report.rs` and
  `crates/edgerun-codelyzer/src/codealyzer/errors.rs` no longer use
  `JsonValue`, `Map`, or `ToJson`; they emit their fixed JSON object shapes
  directly.
- `crates/edgerun-codelyzer/src/codealyzer/crate_model.rs` no longer imports
  `edgerun-json` or implements report-model `ToJson` compatibility. Report
  records now have fixed JSON string emitters, and `report.rs` writes
  `report.json` through `CrateReport::to_json_string()`.
- `crates/edgerun-codelyzer/src/codealyzer/dependency_footprint.rs` no longer
  uses `impl_json_struct!` or `edgerun_json::from_json_str` for cargo metadata.
  It projects only the package, target, dependency, workspace member, and
  resolve-edge fields used by the footprint report.

Codelyzer scan status:

- Exact broad scan requested by the loop: `358` -> `329` -> `257` lines. This
  count still includes ordinary `HashMap` and `BTreeMap` names because the
  requested pattern contains plain `Map`.
- Focused deleted-API scan:
  `rg -n "edgerun_json|edgerun-json|JsonValue|ToJson|FromJson|json!|impl_json_struct|from_json"`
  is now limited to `src/mcp_rust_ast.rs` and the manifest dependency.

Next codelyzer packets:

- Replace `mcp_rust_ast.rs` `Value`/`json!` request and response handling.
- Remove `crates/edgerun-codelyzer/Cargo.toml` `edgerun-json` after
  `mcp_rust_ast.rs` is clear.
- `crates/edgerun-oci/cli/update.rs` no longer uses `edgerun-json` for update
  resource JSON input. It projects the accepted OCI Linux resource fields
  directly: `memory.limit`, `memory.reservation`, `memory.swap`, CPU shares,
  quota, period, realtime, cpuset strings, `pids.limit`, `blockIO.weight`, and
  device cgroup `allow`, `type`, `major`, `minor`, and `access`. The local
  scanner is a temporary host-side stand-in for `json-tape.wat` field
  traversal plus `json-scalar.wat` scalar validation.
- `crates/edgerun-oci` no longer imports or depends on `edgerun-json`.
- `crates/protocol/edgerun-protocols/src/oci/config.rs` no longer imports or
  depends on deleted `edgerun-json` APIs. Manifest/index/config parsing is now
  fixed field projection over typed OCI records, and `ExposedPorts` / `Volumes`
  use explicit presence markers instead of arbitrary JSON values.
- `crates/protocol/edgerun-protocols/src/oci/runtime_spec_json.rs` is deleted;
  it was pure `impl_json_struct!` compatibility glue for the removed JSON
  object model.
- `crates/protocol/edgerun-protocols/src/oci/runtime_spec.rs` is clear of
  `edgerun-json`. Runtime spec parse fails closed until a WAT-backed field
  projector lands; spec emission uses a fixed typed writer for generated OCI
  specs.
- `crates/protocol/edgerun-protocols` no longer enables `dep:edgerun-json`
  through the `oci` feature. The package still has ACME and Tuya JSON lanes that
  keep the manifest dependency alive outside protocol OCI.

Documentation:

- `standards/deletion-edgerun-json-caller-queue.md`
- `standards/codex-jsonrpc-json-caller-routing.md`
- `standards/codelyzer-json-toml-caller-routing.md`

Protocol OCI scan status:

- Exact requested broad scan on `src/oci` plus the protocol manifest:
  `96 -> 23` lines. Remaining lines are ACME/Tuya manifest references and
  typed `BTreeMap` / `OciIdMapping` false positives from the plain `Map`
  pattern.
- Focused deleted-API scan on `src/oci` is clear.

Next protocol OCI packets:

- WAT-backed runtime spec parser for `ociVersion`, `process`, `root`, linux
  namespace/path/resource fields, and `mounts`.
- ACME and Tuya JSON demolition so `edgerun-protocols` can remove the shared
  `edgerun-json` manifest dependency.

### HPACK caller demolition

Primary target:

- `crates/utility/edgerun-hpack` is deleted and removed from live Cargo
  references.
- Route remaining HTTP/2 HPACK runtime state to WAT
  header-block/string/huffman/table records without restoring the
  `edgerun_hpack` crate.

Verified WAT coverage:

- `http-prefix-int.wat`
- `hpack-string.wat`
- `hpack-huffman.wat`
- `hpack-header-block.wat`
- `hpack-table-core.wat`

Documentation:

- `standards/deletion-edgerun-hpack-caller-queue.md`
- `standards/wat-hpack-http2-runtime-adapter-queue.md`

Current gate:

- Protocol server handlers now use `HpackContext` from
  `crates/protocol/edgerun-protocols/src/http/http2/hpack.rs`.
- Decode still returns explicit `NotYetRouted` until a host WebAssembly
  invocation surface lands in the runtime.
- `HpackContext` now has WAT record/status boundary types for
  `hpack-header-block.wat` and can apply host-scanned records for static
  indexed fields and raw literal strings through `decode_wat_records`.
- `encode_header_block` can emit the current server 200 response path directly:
  static `:status: 200` plus never-indexed raw literals, without restoring the
  deleted encoder crate.
- HTTP client/node `src/http/http2/*` mirrors now use `HpackContext`.
- HTTP client/node `src/http/server.rs` wider HTTP/2 server adapter copies now
  use `HpackContext` and map adapter failures to `COMPRESSION_ERROR`.
- Protocol server tests now use `HpackContext` and literal HPACK header-block
  bytes. No live protocol HTTP/2 test imports the retired `Decoder` / `Encoder`
  fixtures.
- `HpackContext::set_max_table_size` is now called by the HTTP/2 client mirrors
  and the two wider HTTP/2 server adapter copies when
  `SETTINGS_HEADER_TABLE_SIZE` is received and an `HpackContext` is owned.
- `standards/runners/hpack-response-encoder-proof.js` proves the direct server
  200 response encoder bytes against `http-prefix-int.wat` and
  `hpack-string.wat` primitives.

Next packets:

- Add a host WebAssembly invocation surface so `decode_header_block` can call
  `hpack-header-block.wat`, `hpack-string.wat`, `hpack-huffman.wat`, and
  `hpack-table-core.wat`.
- Route protocol-only h2c upgrade setting helpers into a caller-owned
  `HpackContext` if that path is revived; the current helper does not own HPACK
  state.
- Replace direct response prefix/string emission with host WAT calls once the
  invocation surface exists, or keep it as the proved bootstrap path.

### Percent and form

Status:

- `crates/utility/edgerun-percent-encoding` deleted.
- `crates/utility/edgerun-form-urlencoded` deleted.
- `crates/utility/edgerun-percent-encoding-upstream` deleted after extracting
  the only concrete live upstream-style policy, OpenTelemetry/W3C baggage
  percent encoding, to `percent-url-form.wat::percent_encode_baggage`.

Verified WAT coverage:

- `percent-url-form.wat`
- `percent-url-form-smoke.js`
- `codec-composition-http1-query.js`
- `percent_encode_baggage` baggage policy smoke coverage in
  `percent-url-form-smoke.js`

Cleanup status:

- Closed: `standards/runners/rust-parity-misc-codecs.js` no longer compiles a
  Rust oracle for the deleted local percent/form crates. It records fixed WAT
  expectations and a deleted-oracle note instead.
- Closed: upstream-shaped `percent-encoding` source is deleted. `AsciiSet`,
  iterator/Cow, and permissive malformed decode behavior are not portable
  standards ownership.

Documentation:

- `standards/deletion-percent-form-caller-queue.md`
- `standards/deletion-edgerun-percent-encoding-upstream.md`

### Terminal control scan

Status:

- `crates/utility/edgerun-terminal-parser` is deleted.
- Root workspace member/dependency metadata and the lock package entry are
  removed.
- `crates/edgerun-term/edgerun-term-core` still contains broken
  `edgerun_terminal_parser::*` imports as caller-demolition fallout.

Verified WAT coverage:

- `terminal-control-scan.wat` (`300112`)
- `terminal-control-scan-smoke.js`

Covered behavior:

- printable spans, C0/DEL execute records, ESC, CSI params/final byte, OSC
  BEL/ST payload spans, DCS hook/data/end, incomplete sequences, and output-cap
  failure.

Documentation:

- `standards/deletion-edgerun-terminal-parser.md`

### SIXEL decode

Status:

- `crates/utility/edgerun-sixel` is deleted.
- Root workspace member/dependency metadata, terminal dependency metadata, and
  the lock package entry are removed.
- `crates/edgerun-term/edgerun-term-core` still contains broken
  `edgerun_sixel::*` imports as caller-demolition fallout.

Verified WAT coverage:

- `sixel-decode.wat` (`300071`)
- `sixel-decode-smoke.js`

Covered behavior:

- DCS payload scanning, sixel char bit mapping, repeat handling, color register
  selection, RGB-percent color definitions, `$` and `-` raster movement,
  corrected dimensions, empty/invalid repeat/invalid color rejection, and
  output-cap failure.

Documentation:

- `standards/deletion-edgerun-sixel.md`

## Freed-slot queue

Assign the next available agent to the highest item that does not overlap an
active write set:

1. Codelyzer cargo-metadata JSON projection and report emitter cleanup.
2. Codex app-protocol `common.rs` test-fixture cleanup, or the first concrete
   WAT-backed server request projector for `account/chatgptAuthTokens/refresh`.
3. HPACK HTTP/2 WAT-backed adapter implementation.
4. Codex protocol common server notification family projection from raw
   `params` spans.
5. `edgerun-encoding` caller demolition for byteorder/base64/hex/cstring/TLV
   paths now that the crate shell is gone.
6. Compression caller adapter implementation for OCI gzip read, HTTP
   compression, APK ZIP method 8, and tor bench.
7. Terminal renderer caller demolition: replace `edgerun_terminal_parser`
   callback imports in `edgerun-term-core` with host/WAT record routing.
8. Terminal SIXEL caller demolition: replace `edgerun_sixel` image decode calls
   in `edgerun-term-core` with host/WAT RGBA record routing.

## Verification snippets

Use these as cheap evidence after each batch:

```bash
comm -23 \
  <(find standards/build/wasm/codec-primitives -maxdepth 1 -name '*.wat' -printf '%f\n' | sed 's/\.wat$//' | sort) \
  <(find standards/components/manifests/codec-primitives -maxdepth 1 -name '*.toml' -printf '%f\n' | sed 's/\.toml$//' | sort)

rg -n "^standard_id\s*=\s*[0-9]+" standards/components/manifests/codec-primitives \
  | sed -E 's/.*=\s*([0-9]+).*/\1/' \
  | sort \
  | uniq -d

rg -n "miniz_oxide::|edgerun_encoding::compression" crates standards
rg -n "edgerun_adler2|edgerun-adler2|\badler2\b|Adler32" crates Cargo.toml standards --glob '*.rs' --glob 'Cargo.toml' --glob '*.md'
rg -n "edgerun-encoding|edgerun_encoding" crates Cargo.toml standards --glob '*.rs' --glob 'Cargo.toml' --glob '*.md'
rg -n "edgerun_json::|use edgerun_json" crates --glob '*.rs'
rg -n "edgerun_hpack|edgerun-hpack" crates Cargo.toml standards
rg -n "edgerun_percent_encoding|edgerun_form_urlencoded|edgerun-percent-encoding|edgerun-form-urlencoded" crates Cargo.toml standards
rg -n "edgerun_sixel|edgerun-sixel" crates Cargo.toml standards
```
