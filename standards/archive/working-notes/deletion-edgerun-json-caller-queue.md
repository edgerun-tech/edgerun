# edgerun-json caller demolition queue

Purpose: assign the next deletion agents for retiring Rust `edgerun-json`
caller usage now that the useful codec behavior lives in WAT:

- `json-scalar.wat`
- `json-tape.wat`
- `json-emit.wat`
- `json-value-core.wat`
- `toml-scan.wat`
- `yaml-scan.wat`

Status:

- `crates/utility/edgerun-json` parser/model source is already deleted.
- `crates/utility/edgerun-json` package metadata, README, licenses, git hooks,
  and test metadata have now been deleted too.
- `crates/utility/edgerun-json-derive` was pure proc-macro compatibility for
  `ToJson` / `FromJson` and has now been deleted.
- `crates/edgerun-oci/src/registry/push_manifest.rs` no longer uses
  `impl_json_struct!` or `to_json_string`; it emits the fixed OCI push manifest
  shape directly with a local JSON string escaper.
- Do not recreate the Rust crate, traits, derive macros, object model, `Map`,
  `JsonValue`, `TomlValue`, or dynamic serde-style compatibility surface.
- Cargo is expected to stay broken until caller queues remove their imports and
  manifests. That is intentional for this deletion branch.

This is a caller queue only. Do not rebuild the retired `edgerun-json` API.
Do not add a new dynamic JSON crate. Move useful behavior to WAT-backed field
scans, fixed schema emitters, explicit local records, or rkyv wire records.

Inventory command used for this queue:

```bash
rg -l "edgerun_json::|use edgerun_json" crates --glob '*.rs' | sort
```

Current direct Rust API hit count from that command: `197` files.

## Queue 1: generated Codex protocol and tool JSON

Shape:

- `116` direct Rust call-site files under `crates/edgerun-codex/*`.
- Heavy use of `ToJson`, `FromJson`, `Value`, `Map`, `json!`, schema helpers,
  `from_json_*`, and `to_json_*`.
- Many types already have protocol schemas or generated JSON schema fixtures.
  The remaining Rust JSON model is convenience glue, not the durable authority.

First files:

- Done: `crates/edgerun-codex/app-protocol/src/jsonrpc_lite.rs` no longer
  imports `edgerun-json` or derives generic JSON traits. JSON-RPC now has fixed
  record emission/projection for `jsonrpc`, `id`, `method`, `params`, `trace`,
  `result`, `error`, `code`, `message`, and `data`, with dynamic payloads kept
  as validated raw JSON spans.
- Advanced: `crates/edgerun-codex/app-protocol/src/protocol/common.rs`
  no longer converts `JSONRPCRequest` / `JSONRPCNotification` through
  `edgerun_json::to_value(...).from_json_value(...)`. It now dispatches by
  method and returns an explicit `JsonrpcProtocolProjectionError` with
  direction, method, expected params type, expected method field set, and raw
  params span length. Generated `common.rs` client/server response and server
  notification helpers no longer call generic `edgerun_json::to_value` /
  `from_json_value`; they fail closed until fixed emit/project paths land.
  The target scan is now `85 -> 75` refs, with remaining refs isolated to the
  embedded test module.
- `crates/edgerun-codex/app-protocol/src/protocol/v2/thread.rs`
- `crates/edgerun-codex/tools/src/json_schema.rs`
- `crates/edgerun-codex/tools/src/tool_spec.rs`
- `crates/edgerun-codex/tools/src/responses_api.rs`
- `crates/edgerun-codex/api/src/common.rs`
- `crates/edgerun-codex/api/src/sse/responses.rs`
- `crates/edgerun-codex/api/src/endpoint/realtime_websocket/protocol.rs`
- `crates/edgerun-codex/api/src/endpoint/session.rs`

Replacement strategy:

- Treat generated app protocol JSON as a schema-to-WAT bridge. Keep the
  schema files and generated Rust protocol records; replace derive-generated
  `ToJson` / `FromJson` paths with explicit schema field readers and emitters.
- For inbound JSON-RPC and SSE payloads, use `json-tape.wat` to validate the
  document and locate object fields, then project directly into the protocol
  record. Use `json-scalar.wat` for scalar classification and numeric checks.
- For outbound protocol messages, use `json-emit.wat` for string/scalar/object
  emission and local fixed-field builders. Do not materialize
  `edgerun_json::Value` just to serialize.
- For tool input schemas that intentionally keep dynamic schema fragments, use
  `json-value-core.wat` only as a bounded value-record helper for kind checks,
  object lookup, and number conversion policy. Do not reintroduce arbitrary
  indexing or serde-style conversion traits.

Delete or tombstone:

- `edgerun_json::{ToJson, FromJson}` derives in app-protocol records once each
  schema has a WAT-backed decode/emit path.
- `edgerun_json::json!` usage in tool tests after fixtures become raw JSON
  bytes or small fixed emitters.
- `crates/edgerun-codex/*/Cargo.toml` `edgerun-json` dependencies after the
  last local call site moves.

Work packet:

1. Done: `app-protocol/src/jsonrpc_lite.rs` implements JSON-RPC request,
   notification, response, and error decode/emit directly from field names
   `jsonrpc`, `id`, `method`, `params`, `trace`, `result`, `error`, `code`,
   `message`, and `data`. Follow-up:
   `standards/codex-jsonrpc-json-caller-routing.md` records the raw-span
   handoff into typed protocol records.
2. Advanced: `app-protocol/src/protocol/common.rs` removed the generic deleted
   JSON conversion from `TryFrom<JSONRPCRequest>` and
   `TryFrom<JSONRPCNotification>`, removed generic response conversion helpers
   from the live generated macro layer, and documents the field sets for
   command/file approvals, tool user input, MCP elicitation, dynamic tool call,
   ChatGPT auth refresh, and the two legacy approval methods. Next packet:
   either convert the embedded `common.rs` JSON tests to raw JSON-RPC strings,
   or implement the first real WAT-backed projector for the smallest request
   family, `account/chatgptAuthTokens/refresh`.
3. Convert `tools/src/json_schema.rs` to a bounded schema walker over
   `json-tape.wat` spans. Preserve supported fields: `type`, `description`,
   `enum`, `items`, `properties`, `required`, `additionalProperties`, `anyOf`,
   and `const`.
4. Convert `api/src/common.rs` outbound request builders to fixed-field JSON
   emitters. Leave unknown provider payloads as borrowed raw JSON spans until
   their owner queue handles them.

## Queue 2: app JSON payloads

Shape:

- `20` direct files across app/runtime surfaces outside Codex.
- Mostly narrow API payloads: secret metadata, exchange provider responses, app
  marketplace records, node provisioning, mesh payloads, ACME/Tuya
  compatibility JSON, and PocketBase's larger dynamic app surface.

Completed:

- `crates/apps/edgerun-oauth` is clear of `edgerun-json`. Token response,
  credential-store, OIDC discovery, JWKS, JWT claim, and JWK verifier paths now
  use fixed owner-local field projection and JSON emission instead of
  `JsonValue`, `Map`, `to_string`, or `parse_json_tape`. The owner-local
  scanner is a temporary Rust host shim for the existing JSON WAT modules, not a
  reusable JSON object model.

First files:

- `crates/apps/edgerun-secret-service/src/backend.rs`
- `crates/edgerun-exchange/src/provider_http.rs`
- `crates/edgerun-exchange/src/changenow.rs`
- `crates/edgerun-exchange/src/ffio.rs`
- `crates/edgerun-exchange/src/sideshift.rs`
- `crates/edgerun-sdk/src/marketplace.rs`
- `crates/node/edgerun-node/src/provisioning_listener.rs`
- `crates/node/edgerun-node/src/services/http_runtime.rs`
- `crates/apps/edgerun-pocketbase/src/main.rs`

Replacement strategy:

- Split narrow fixed payloads from dynamic app stores.
- Secret metadata, exchange providers, marketplace specs, node provisioning,
  mesh payloads, ACME, and Tuya should use direct `json-tape.wat` field lookups
  plus `json-emit.wat` object builders.
- Use `json-scalar.wat` for strict string/bool/u64/finite-number checks at
  provider boundaries.
- Use `json-value-core.wat` only where a field must preserve arbitrary JSON,
  such as provider-specific `dps`, unknown tool arguments, or schema fragments.
- PocketBase is a separate large pass. Do not start there. First extract narrow
  apps that can delete an entire manifest dependency after one file group.

Delete or tombstone:

- Local `JsonValue` request-body builders in secret-service and exchange
  providers after fixed emitters land.
- `edgerun_json::parse_json_tape(...).root(...).to_json_value()` conversion
  paths in provider HTTP helpers.
- `edgerun-json` dependency entries in app manifests once fixed payload groups
  no longer import it.
- PocketBase should be tombstoned as a dynamic compatibility island until a
  dedicated app-storage JSON plan exists.

Work packet:

1. Convert `apps/edgerun-secret-service/src/backend.rs` metadata JSON. This is
   a small object with `label`, `attributes`, and `created_us`; it should become
   a direct fixed emitter/parser and delete all local `Map`/`JsonValue` use.
2. Convert `edgerun-exchange/src/provider_http.rs` after each provider response
   names its required fields. Provider adapters should receive field accessors,
   not a dynamic `JsonValue` object.

## Queue 3: OCI and registry JSON

Shape:

- `18` direct files in `crates/edgerun-oci/*` and
  `crates/protocol/edgerun-protocols/src/oci/*`.
- Heavy `impl_json_struct!` usage for OCI image configs, manifests, runtime
  specs, registry push manifests, state, and provenance.
- This is a protocol boundary. It should become explicit OCI records plus WAT
  JSON validation/projection, not a compatibility model.

First files:

- `crates/protocol/edgerun-protocols/src/oci/config.rs`
- `crates/protocol/edgerun-protocols/src/oci/runtime_spec.rs`
- `crates/protocol/edgerun-protocols/src/oci/runtime_spec_json.rs`
- `crates/edgerun-oci/src/registry/push_manifest.rs`
- `crates/edgerun-oci/src/registry/provenance.rs`
- `crates/edgerun-oci/src/registry/pull.rs`
- `crates/edgerun-oci/src/state.rs`
- `crates/edgerun-oci/cli/inspect.rs`

Replacement strategy:

- Define the OCI parser boundary as JSON bytes in, typed OCI records out.
- Use `json-tape.wat` for object/array traversal and field location.
- Use `json-scalar.wat` for media type, digest, platform, size, and boolean
  validation.
- Use `json-emit.wat` for push manifest, provenance, state, and CLI inspect
  output.
- Use `json-value-core.wat` for OCI extension maps like `ExposedPorts` and
  `Volumes` only if the exact unknown JSON object must be retained. Prefer
  raw-span preservation or typed empty-object markers where the OCI field only
  signals presence.

Delete or tombstone:

- All `impl_json_struct!` blocks in `protocols/src/oci/config.rs` and
  `runtime_spec_json.rs`.
- `ManifestProbe` dynamic-value detection once manifest kind is detected from
  WAT field spans.
- `push_manifest_json`'s temporary `PushManifest*` structs are deleted; the
  writer is now direct fixed JSON emission.
- `ImageProvenance` and `ContainerState` derive/macro glue is deleted in
  `crates/edgerun-oci`; provenance/state now use scoped fixed JSON writers, and
  state parse projects the exact OCI runtime state fields.
- `crates/edgerun-oci/src/registry/client.rs` no longer parses registry token
  responses through `edgerun-json`; it projects only `token` and
  `access_token`.
- `crates/edgerun-oci/src/hooks.rs` no longer emits hook stdin state through
  `edgerun-json`; it writes the fixed OCI hook-state object directly with local
  string escaping.
- `crates/protocol/edgerun-protocols/src/oci/config.rs` no longer imports
  `edgerun-json`, `JsonValue`, dynamic `Map`, or `impl_json_struct!`. It keeps
  the public typed `parse_manifest`, `parse_single_manifest`, and
  `parse_image_config` entry points, but projects only the OCI fields this
  protocol module owns: manifest/index descriptors, config descriptors, layer
  descriptors, platform selectors, image `architecture`/`os`, rootfs
  `type`/`diff_ids`, process config strings/lists, labels, and history. OCI
  `ExposedPorts` and `Volumes` now use explicit `OciObjectPresence` markers
  instead of arbitrary JSON values.
- `crates/protocol/edgerun-protocols/src/oci/runtime_spec_json.rs` is deleted.
  It was pure deleted-model glue for runtime spec `impl_json_struct!` blocks.
- `crates/protocol/edgerun-protocols/src/oci/runtime_spec.rs` no longer imports
  `edgerun-json`. Runtime-spec parse now fails closed with an adapter-required
  error until a WAT-backed field projector lands; generated spec output uses a
  fixed typed writer for `ociVersion`, platform, process, root, hostname,
  linux basics, mounts, and annotations.
- The `oci` feature in `crates/protocol/edgerun-protocols/Cargo.toml` no
  longer pulls `dep:edgerun-json`. The manifest dependency remains for ACME and
  Tuya JSON lanes outside this packet.
- The `edgerun-json` dependency is removed from `edgerun-oci`. The remaining
  OCI-family JSON work is in `edgerun-protocols` runtime/spec parsing, not the
  `edgerun-oci` crate.

Work packet:

1. Done: `protocols/src/oci/config.rs` replaced `parse_manifest`,
   `parse_single_manifest`, and `parse_image_config` with owner-local fixed
   field projection. This is the Rust-side stand-in for `json-tape.wat` field
   traversal plus `json-scalar.wat` scalar validation; it is not a reusable JSON
   object model.
2. Done: `edgerun-oci/src/registry/provenance.rs`, `registry/pull.rs`, and
   `state.rs` no longer import `edgerun-json` or `impl_json_struct!`.
3. Done: `edgerun-oci/src/registry/client.rs` token response parsing now uses a
   fixed field projector for `token` and `access_token`, and `hooks.rs` state
   emission now uses a direct fixed JSON emitter.
4. Done: `crates/edgerun-oci/src/clap/cli.rs` no longer uses
   `edgerun_json::Value` as argument-match storage; it has a private
   `ArgValue` enum for strings, string lists, bools, and count flags.
5. Done: CLI JSON output files under `crates/edgerun-oci/cli/` now use direct
   fixed emitters for features, events, state, ps, images, and inspect output.
   `bin/ert.rs` also dropped its `parse_json_tape` PID probe, and the stale
   `oci_spec.rs` edgerun-json owner comment is removed.
6. Done: `cli/update.rs::parse_update_resources` no longer parses resource
   update JSON through `edgerun_json::from_json_slice`. It uses an owner-local
   fixed projector for the accepted update fields: `memory.limit`,
   `memory.reservation`, `memory.swap`, CPU shares/quota/period/realtime/cpuset,
   `pids.limit`, `blockIO.weight`, and device cgroup `allow`, `type`, `major`,
   `minor`, and `access`. This maps to `json-tape.wat` object/array traversal
   and `json-scalar.wat` scalar validation once OCI has a host WAT invocation
   surface.
7. Done: `crates/edgerun-oci/Cargo.toml` no longer depends on `edgerun-json`.
   Targeted scans of `crates/edgerun-oci` are clear.
8. Done: protocol OCI config/runtime macro demolition. Exact requested scan on
   `crates/protocol/edgerun-protocols/src/oci` plus
   `crates/protocol/edgerun-protocols/Cargo.toml` moved from `96` broad hits to
   `23`. The remaining broad hits are the shared protocol manifest dependency
   for ACME/Tuya and ordinary typed names containing `Map` such as `BTreeMap`
   and `OciIdMapping`.
9. Next packet: implement WAT-backed runtime spec parsing for the narrow fields
   used by `edgerun-oci` lifecycle and CLI: `ociVersion`, `process.args/env/cwd`
   and user ids, `root.path/readonly`, linux namespaces, masked/readonly paths,
   mounts, and resource update fields. After that, route ACME and Tuya off the
   shared `edgerun-json` dependency so `edgerun-protocols` can drop the
   manifest entry completely.

## Queue 4: codelyzer TOML/JSON/YAML

Shape:

- `11` direct files in `crates/edgerun-codelyzer`.
- Cargo manifest inspection has started moving off
  `edgerun_json::from_toml_str` and `TomlValue`.
- Remaining dependency is JSON values for cargo metadata, MCP AST output,
  codealyzer tests, and issue/report payloads.

Completed:

- `src/codealyzer/cargo_toml.rs`, `src/codealyzer/dependency_analyzer.rs`,
  `src/dead_code.rs`, and `src/codealyzer/standards.rs` now use
  `CargoTomlProjection` instead of deleted `edgerun-json` TOML APIs.
  `CargoTomlProjection` is an owner-local fixed parser bridge until codelyzer
  has direct host calls into `toml-scan.wat`; it is not a generic TOML model.
- `src/codealyzer/workspace_membership.rs` now projects cargo metadata fields
  directly and no longer calls `edgerun_json::parse_json_tape`.
- `src/codealyzer/tests.rs` now checks cargo compiler-message JSON with fixed
  field checks for `reason` and `message.level`; it no longer imports
  `edgerun_json::Value`.
- `src/codealyzer/issue_report.rs` and `src/codealyzer/errors.rs` now emit
  their fixed JSON object shapes directly and no longer import `JsonValue`,
  `Map`, or `ToJson`.
- `src/codealyzer/crate_model.rs` no longer imports `edgerun-json` or provides
  report-model `ToJson` compatibility impls. It now has fixed report JSON
  string emitters, and `src/codealyzer/report.rs` writes `report.json` through
  `CrateReport::to_json_string()`.
- `src/codealyzer/dependency_footprint.rs` no longer uses
  `impl_json_struct!` or `edgerun_json::from_json_str`. Cargo metadata is
  projected into the exact package, dependency, target, workspace member, and
  resolve-edge fields used by the footprint report.
- The exact broad codelyzer scan moved from `358` to `329` to `257` lines. A
  focused deleted-API scan is now limited to `src/mcp_rust_ast.rs` and the
  manifest dependency.

First files:

- `crates/edgerun-codelyzer/src/codealyzer/cargo_toml.rs`
- `crates/edgerun-codelyzer/src/dead_code.rs`
- `crates/edgerun-codelyzer/src/codealyzer/workspace_membership.rs`
- `crates/edgerun-codelyzer/src/codealyzer/standards.rs`
- `crates/edgerun-codelyzer/src/codealyzer/tests.rs`
- `crates/edgerun-codelyzer/src/mcp_rust_ast.rs`
- `crates/edgerun-codelyzer/src/codealyzer/issue_report.rs`

Next files:

- `crates/edgerun-codelyzer/src/mcp_rust_ast.rs`
- `crates/edgerun-codelyzer/Cargo.toml` after the last source import is gone.

Replacement strategy:

- Cargo manifest scanning should not build a full TOML tree. Use
  `toml-scan.wat` to walk tables, arrays, strings, booleans, and dependency
  table names needed by the analyzer.
- JSON test/report output should use `json-emit.wat` or existing local
  hand-written string emitters in codelyzer, not `edgerun_json::json!`.
- JSON input tests should use `json-tape.wat` for normalization and field
  lookup.
- YAML standard files should use `yaml-scan.wat` only if codelyzer actually
  consumes YAML. Otherwise tombstone YAML support in this crate.

Delete or tombstone:

- `TomlValue`-based helpers in `cargo_toml.rs` and `dead_code.rs`.
- `from_toml_str` manifest parse path.
- `edgerun-codelyzer`'s `edgerun-json` dependency and `features = ["toml"]`
  after manifest scanning moves.

Status:

- Scoped in `standards/codelyzer-json-toml-caller-routing.md`.
- Initial codelyzer scan for
  `edgerun_json|edgerun-json|TomlValue|JsonValue|impl_json_struct|json!`
  returned `133` lines.
- No code was converted in the scoping pass because the small candidates still
  return deleted dynamic `JsonValue` / `Value` boundaries. The next useful work
  is the shared Cargo manifest projection helper, not one-off emitters.

Work packet:

1. Convert `codealyzer/cargo_toml.rs` first. It only needs `[package]`,
   target declarations, features, and basic string fields.
2. Convert `dead_code.rs` workspace manifest indexing next. It needs
   `[workspace] members`, `default-members`, feature names, and dependency
   table keys.
3. Convert `mcp_rust_ast.rs` and `issue_report.rs` output to direct
   `json-emit.wat` or local fixed emitters.

## Queue 5: tracing and OpenTelemetry compatibility

Shape:

- `9` direct files in compatibility crates:
  `edgerun-tracing-subscriber`, `edgerun-tracing`,
  `edgerun-opentelemetry-sdk-upstream`, `edgerun-reqwest`, `edgerun-rmcp`, and
  `edgerun-wiremock`.
- These crates mimic external APIs and carry `edgerun_json_compat` style
  serializer/deserializer surfaces.

First files:

- `crates/utility/edgerun-tracing-subscriber/src/fmt/format/json.rs`
- `crates/utility/edgerun-tracing-subscriber/src/fmt/writer.rs`
- `crates/utility/edgerun-opentelemetry-sdk-upstream/src/trace/sampler/jaeger_remote/remote.rs`
- `crates/utility/edgerun-opentelemetry-sdk-upstream/src/trace/sampler/jaeger_remote/sampler.rs`
- `crates/utility/edgerun-reqwest/src/native.rs`
- `crates/utility/edgerun-wiremock/src/lib.rs`
- `crates/utility/edgerun-rmcp/src/lib.rs`

Replacement strategy:

- Treat this as a compatibility tombstone pass, not a feature rewrite.
- For tracing JSON logs, use `json-emit.wat` for event/span object emission.
  Use `json-tape.wat` only where existing code reparses formatted span fields.
- For OpenTelemetry remote sampler config, project the exact remote JSON fields
  with `json-tape.wat` and `json-scalar.wat`.
- For `reqwest::json`, `wiremock::set_body_json`, and `rmcp` aliases, stop
  exposing generic `ToJson` / `FromJson` traits. Either require raw JSON bytes
  or move callers to explicit request/response builders.

Delete or tombstone:

- `edgerun_json_compat` serializer glue in tracing once log objects emit
  directly.
- Generic `json<T: edgerun_json::ToJson>` and
  `json<T: edgerun_json::FromJson>` compatibility methods in
  `edgerun-reqwest` unless a current caller proves they still need the external
  API shape.
- `edgerun-rmcp` `JsonValue`/`JsonObject` aliases after Codex tool JSON moves
  to bounded schema records.

Work packet:

1. Leave `reqwest`, `wiremock`, and `rmcp` until Codex callers no longer depend
   on their generic JSON helpers.
2. Start with tracing JSON formatting. It is output-dominant and maps directly
   to `json-emit.wat`.
3. Convert OpenTelemetry remote sampler config separately with a narrow inbound
   field scanner.

## Queue 6: derive macro and serde-compat leftovers

Shape:

- `22` direct files are either the derive implementation itself, retired crate
  self-tests, or compatibility modules in utility crates.
- This is not product JSON behavior. It exists to preserve old Rust ecosystem
  APIs.

First files:

- `crates/utility/edgerun-json-derive/src/lib.rs`
- `crates/utility/edgerun-json/tests/derive.rs`
- `crates/utility/edgerun-json/tests/model_macro.rs`
- `crates/utility/edgerun-bytes-upstream/src/json_compat.rs`
- `crates/utility/edgerun-bytes-upstream/tests/test_json_compat.rs`
- `crates/utility/edgerun-slab/src/json_compat.rs`
- `crates/utility/edgerun-crypto/src/generic_array_impl/impl_json_compat.rs`
- `crates/utility/edgerun-smallvec/src/lib.rs`
- `crates/utility/edgerun-tinyvec/src/arrayvec.rs`
- `crates/utility/edgerun-tinyvec/src/tinyvec.rs`

Replacement strategy:

- Delete derive macro use by removing callers, not by teaching the macro to
  emit WAT adapters.
- Keep crypto/vector compatibility code only if a live non-JSON feature still
  requires its external serde-like trait surface. Otherwise tombstone the JSON
  compatibility modules.
- If a low-level utility must expose structured state, use native typed APIs or
  rkyv wire records, not `edgerun_json_compat`.

Delete or tombstone:

- `crates/utility/edgerun-json-derive` after queues 1 and 3 remove
  `#[derive(edgerun_json::ToJson, edgerun_json::FromJson)]` and
  `impl_json_struct!`.
- Retired `edgerun-json` self-tests after no active crate imports the API.
- Utility `json_compat` modules that exist only for ecosystem parity.

Work packet:

1. Track derive users with:

   ```bash
   rg -n "edgerun_json::(ToJson|FromJson|impl_json_struct!)|derive\\([^\\)]*(ToJson|FromJson)" crates --glob '*.rs'
   ```

2. Do not edit the macro first. Delete or replace every macro caller first.
3. Remove `edgerun-json-derive` and its tests only after the caller graph is
   empty.

## Suggested agent order

1. `secret-service-json-metadata`: tiny fixed object parser/emitter; good proof
   for direct app deletion and likely removes one app manifest dependency.
2. `oauth-token-json`: narrow provider boundary using `json-tape.wat`,
   `json-scalar.wat`, and `json-emit.wat`.
3. `oci-push-manifest-writer`: smallest OCI outbound writer; establishes the
   direct emit helper before the larger OCI parser pass.
4. `codelyzer-toml-scan`: small, contained, proves `toml-scan.wat` replaces
   useful TOML manifest scanning without restoring `TomlValue`.
5. `codex-jsonrpc-lite`: first generated/protocol Codex demolition target;
   removes derive dependence from a central but bounded message shape.
6. `derive-callsite-burn-down`: remove `impl_json_struct!` and
   `ToJson` / `FromJson` derives from mesh, ACME/Tuya, OCI, and Codex protocol
   records by replacing each with direct record projection or fixed emitters.

Completion condition for this queue:

- `rg -l "edgerun_json::|use edgerun_json" crates --glob '*.rs'` returns no
  product/runtime call sites outside explicitly tombstoned compatibility
  directories.
- `rg -n "edgerun-json" crates --glob Cargo.toml` returns only tombstoned or
  excluded compatibility manifests, then those manifests are removed from the
  active workspace in a separate manifest cleanup pass.
