# edgerun-json retirement audit

This audit maps the remaining `crates/utility/edgerun-json` dependency surface
after the JSON/TOML parser source deletion pass.

Scope: references under `crates/` to the crate, its manifests, and exported
APIs. This document is planning evidence only; no Rust source in
`edgerun-json` was edited for this audit.

## Current state

WAT already owns the portable scanner/parser primitives for the deleted source:

- `standards/build/wasm/codec-primitives/json-scalar.wat`
- `standards/build/wasm/codec-primitives/json-tape.wat`
- `standards/build/wasm/codec-primitives/toml-scan.wat`

The remaining Rust surface is not just parsing. It is mostly:

- `JsonValue` / `Value` / `Map` model construction and indexing.
- `ToJson` / `FromJson` traits plus `impl_json_struct!` and derive macros.
- JSON string escaping and compact/pretty serialization.
- `JsonNumber`, `JsonValueError`, `JsonError` compatibility types.
- YAML support behind the `yaml` feature.
- TOML value callers that still depend on `TomlValue` and `from_toml_str`.

## Verification commands

```bash
rg -l "edgerun_json::|use edgerun_json|edgerun-json" crates --glob '*.rs' --glob 'Cargo.toml' | sort
```

Result: remaining manifest/source references exist across app, Codex, protocol,
node, OCI, SDK, mesh, and utility crates.

```bash
find crates -path '*/Cargo.toml' -print | sort | while read f; do if rg -q 'edgerun-json' "$f"; then printf '%s\n' "$f"; fi; done | wc -l
```

Result: `35` Cargo manifests still mention `edgerun-json`.

```bash
rg -l "edgerun_json::|use edgerun_json" crates --glob '*.rs' | wc -l
```

Result: `205` Rust files still directly reference exported APIs.

```bash
rg -o "edgerun_json::[A-Za-z_][A-Za-z0-9_!]*" crates --glob '*.rs' | sed 's/.*edgerun_json:://' | sort | uniq -c | sort -nr | head -20
```

Top direct API symbols:

```text
684 FromJson
682 ToJson
144 json!
139 to_value
126 Value
92 JsonValue
81 from_value
74 Map
73 impl_json_struct!
55 JsonValueError
39 from_json_value
38 to_string
34 from_json_str
30 parse_json_tape
28 from_str
26 json
21 Error
13 to_json_string
11 JsonError
10 yaml
```

## Cargo manifest references

These manifests keep the crate in the graph:

```text
Cargo.toml
crates/apps/edgerun-exchange-api/Cargo.toml
crates/apps/edgerun-oauth/Cargo.toml
crates/apps/edgerun-pocketbase/Cargo.toml
crates/apps/edgerun-secret-service/Cargo.toml
crates/edgerun-codelyzer/Cargo.toml
crates/edgerun-codex/api/Cargo.toml
crates/edgerun-codex/app-protocol/Cargo.toml
crates/edgerun-codex/client/Cargo.toml
crates/edgerun-codex/code/Cargo.toml
crates/edgerun-codex/core/Cargo.toml
crates/edgerun-codex/debug-context/Cargo.toml
crates/edgerun-codex/host/Cargo.toml
crates/edgerun-codex/model/catalog/Cargo.toml
crates/edgerun-codex/model/info/Cargo.toml
crates/edgerun-codex/model/provider/Cargo.toml
crates/edgerun-codex/protocol/Cargo.toml
crates/edgerun-codex/shell/Cargo.toml
crates/edgerun-codex/tools/Cargo.toml
crates/edgerun-codex/tui/Cargo.toml
crates/edgerun-exchange/Cargo.toml
crates/edgerun-oci/Cargo.toml
crates/edgerun-sdk/Cargo.toml
crates/mesh/edgerun-mesh/Cargo.toml
crates/node/edgerun-node/Cargo.toml
crates/protocol/edgerun-core/Cargo.toml
crates/protocol/edgerun-protocols/Cargo.toml
crates/utility/edgerun-json/Cargo.toml
crates/utility/edgerun-json-derive/Cargo.toml
crates/utility/edgerun-opentelemetry-sdk-upstream/Cargo.toml
crates/utility/edgerun-reqwest/Cargo.toml
crates/utility/edgerun-rmcp/Cargo.toml
crates/utility/edgerun-tracing-subscriber/Cargo.toml
crates/utility/edgerun-url/Cargo.toml
crates/utility/edgerun-wasm-bindgen/Cargo.toml
crates/utility/edgerun-wiremock/Cargo.toml
```

## Can delete caller

These callers are tests, local compatibility glue, reports, or tooling surfaces
whose value is only proving the old Rust crate. They are the first deletion
targets once this crate is intentionally retired.

- `crates/utility/edgerun-json/tests/depth_small.rs`
- `crates/utility/edgerun-json/tests/derive.rs`
- `crates/utility/edgerun-json/tests/model_macro.rs`
- `crates/utility/edgerun-json/tests/unicode.rs`
- `crates/utility/edgerun-bytes-upstream/src/json_compat.rs`
- `crates/utility/edgerun-bytes-upstream/tests/test_json_compat.rs`
- `crates/utility/edgerun-slab/src/json_compat.rs`
- `crates/utility/edgerun-smallvec/src/lib.rs` feature-gated
  `edgerun_json_compat` support
- `crates/utility/edgerun-tinyvec/src/arrayvec.rs` feature-gated
  `edgerun_json_compat` support
- `crates/utility/edgerun-tinyvec/src/tinyvec.rs` feature-gated
  `edgerun_json_compat` support
- `crates/edgerun-codelyzer/src/codealyzer/tests.rs` old JSON parser
  normalization probe

Deletion proof needed:

- Remove these compatibility features/manifests together.
- Keep WAT smoke/parity runners as replacement evidence.
- Do not keep tests whose only purpose is proving `edgerun-json` API
  compatibility.

## Replace with WAT pipeline

These callers use the parser/scanner path that WAT now owns. They should move to
the canonical WAT pipeline and consume spans/tape tokens directly, or a small
runtime-owned adapter that materializes only the exact fields needed.

JSON tape and parse callers:

- `crates/apps/edgerun-exchange-api/src/handlers.rs`
- `crates/apps/edgerun-oauth/src/discovery.rs`
- `crates/apps/edgerun-oauth/src/jwt.rs`
- `crates/apps/edgerun-oauth/src/token_store.rs`
- `crates/apps/edgerun-oauth/src/types.rs`
- `crates/apps/edgerun-pocketbase/src/main.rs`
- `crates/apps/edgerun-secret-service/src/backend.rs`
- `crates/edgerun-codelyzer/src/codealyzer/dependency_footprint.rs`
- `crates/edgerun-codelyzer/src/codealyzer/workspace_membership.rs`
- `crates/edgerun-codex/api/src/api_bridge.rs`
- `crates/edgerun-codex/api/src/endpoint/compact.rs`
- `crates/edgerun-codex/api/src/endpoint/memories.rs`
- `crates/edgerun-codex/api/src/endpoint/models.rs`
- `crates/edgerun-codex/api/src/endpoint/realtime_websocket/methods.rs`
- `crates/edgerun-codex/api/src/endpoint/realtime_websocket/protocol_common.rs`
- `crates/edgerun-codex/api/src/endpoint/realtime_websocket/protocol_v2.rs`
- `crates/edgerun-codex/api/src/endpoint/responses_websocket.rs`
- `crates/edgerun-codex/api/src/endpoint/session.rs`
- `crates/edgerun-codex/api/src/files.rs`
- `crates/edgerun-codex/api/src/rate_limits.rs`
- `crates/edgerun-codex/api/src/sse/responses.rs`
- `crates/edgerun-codex/api/tests/realtime_websocket_e2e.rs`
- `crates/edgerun-codex/app-protocol/src/export.rs`
- `crates/edgerun-codex/core/examples/live_smoke.rs`
- `crates/edgerun-codex/core/src/stub_lib.rs`
- `crates/edgerun-codex/debug-context/src/lib.rs`
- `crates/edgerun-codex/host/src/main.rs`
- `crates/edgerun-codex/model/catalog/src/cache.rs`
- `crates/edgerun-codex/model/catalog/src/lib.rs`
- `crates/edgerun-codex/model/catalog/src/manager_tests.rs`
- `crates/edgerun-codex/model/info/src/model_provider_info_tests.rs`
- `crates/edgerun-codex/protocol/src/account.rs`
- `crates/edgerun-codex/protocol/src/auth.rs`
- `crates/edgerun-codex/protocol/src/config_types.rs`
- `crates/edgerun-codex/protocol/src/error.rs`
- `crates/edgerun-codex/protocol/src/models.rs`
- `crates/edgerun-codex/protocol/src/protocol.rs`
- `crates/edgerun-codex/tui/src/main.rs`
- `crates/edgerun-codex/tui/src/ui.rs`
- `crates/edgerun-exchange/src/provider_http.rs`
- `crates/edgerun-oci/bin/ert.rs`
- `crates/edgerun-oci/cli/update.rs`
- `crates/edgerun-oci/src/state.rs`
- `crates/node/edgerun-node/src/provisioning_listener.rs`
- `crates/node/edgerun-node/src/services/http_runtime.rs`
- `crates/protocol/edgerun-protocols/src/acme/types.rs`
- `crates/protocol/edgerun-protocols/src/oci/config.rs`
- `crates/protocol/edgerun-protocols/src/oci/runtime_spec.rs`
- `crates/protocol/edgerun-protocols/src/tuya.rs`
- `crates/utility/edgerun-opentelemetry-sdk-upstream/src/trace/sampler/jaeger_remote/sampler.rs`
- `crates/utility/edgerun-reqwest/src/native.rs`
- `crates/utility/edgerun-tracing-subscriber/src/fmt/format/json.rs`

TOML callers:

- `crates/edgerun-codelyzer/src/codealyzer/cargo_toml.rs`
- `crates/edgerun-codelyzer/src/codealyzer/dependency_analyzer.rs`
- `crates/edgerun-codelyzer/src/codealyzer/standards.rs`
- `crates/edgerun-codelyzer/src/dead_code.rs`

These should use `toml-scan.wat` and a strict manifest-field extractor. The
old Rust TOML path was intentionally loose; do not preserve malformed scalar
fallbacks as the canonical path.

## Replace with another crate/model

These callers mainly need a JSON value/model/typed conversion layer, not a
parser. They should be moved to a smaller EdgeRun-owned model crate or to local
wire/domain records before `edgerun-json` can be removed.

Codex protocol/model crates:

- `crates/edgerun-codex/api/src/common.rs`
- `crates/edgerun-codex/api/src/endpoint/realtime_call.rs`
- `crates/edgerun-codex/api/src/endpoint/realtime_websocket/methods_common.rs`
- `crates/edgerun-codex/api/src/endpoint/realtime_websocket/methods_v2.rs`
- `crates/edgerun-codex/api/src/endpoint/realtime_websocket/protocol.rs`
- `crates/edgerun-codex/api/src/endpoint/realtime_websocket/protocol_v1.rs`
- `crates/edgerun-codex/api/src/endpoint/realtime_websocket/protocol_v2.rs`
- `crates/edgerun-codex/api/src/endpoint/responses.rs`
- `crates/edgerun-codex/api/src/endpoint/responses_websocket.rs`
- `crates/edgerun-codex/api/src/requests/responses.rs`
- `crates/edgerun-codex/app-protocol/src/jsonrpc_lite.rs`
- `crates/edgerun-codex/app-protocol/src/protocol/common.rs`
- `crates/edgerun-codex/app-protocol/src/protocol/thread_history.rs`
- `crates/edgerun-codex/app-protocol/src/protocol/v1.rs`
- `crates/edgerun-codex/app-protocol/src/protocol/v2/*.rs`
- `crates/edgerun-codex/model/catalog/src/cache.rs`
- `crates/edgerun-codex/model/info/src/lib.rs`
- `crates/edgerun-codex/model/provider/src/provider.rs`
- `crates/edgerun-codex/protocol/src/*.rs`
- `crates/edgerun-codex/tools/src/*.rs`

Protocol and app models:

- `crates/apps/edgerun-exchange-api/src/routes.rs`
- `crates/apps/edgerun-exchange-api/src/types.rs`
- `crates/apps/edgerun-oauth/src/discovery.rs`
- `crates/apps/edgerun-oauth/src/jwt.rs`
- `crates/apps/edgerun-oauth/src/token_store.rs`
- `crates/apps/edgerun-oauth/src/types.rs`
- `crates/edgerun-exchange/src/changenow.rs`
- `crates/edgerun-exchange/src/ffio.rs`
- `crates/edgerun-exchange/src/provider_http.rs`
- `crates/edgerun-exchange/src/sideshift.rs`
- `crates/edgerun-oci/cli/*.rs`
- `crates/edgerun-oci/src/hooks.rs`
- `crates/edgerun-oci/src/registry/*.rs`
- `crates/edgerun-oci/src/state.rs`
- `crates/edgerun-sdk/src/marketplace.rs`
- `crates/mesh/edgerun-mesh/src/mesh_payload.rs`
- `crates/protocol/edgerun-core/src/conformance.rs`
- `crates/protocol/edgerun-protocols/src/acme/types.rs`
- `crates/protocol/edgerun-protocols/src/oci/*.rs`
- `crates/protocol/edgerun-protocols/src/tuya.rs`
- `crates/utility/edgerun-rmcp/src/lib.rs`
- `crates/utility/edgerun-url/src/lib.rs`
- `crates/utility/edgerun-wiremock/src/lib.rs`

Recommended replacement shape:

- `edgerun-wire` records for durable cross-runtime payloads.
- A tiny `no_std + alloc` JSON value facade only if dynamic JSON is truly
  required at runtime.
- Local domain builders for outbound JSON where the schema is fixed.
- WAT parse/scan modules for inbound validation and field spans.

## Leave for later

These are broad compatibility surfaces where immediate deletion would turn a
codec cleanup into a larger product/API rewrite:

- `crates/apps/edgerun-pocketbase/src/main.rs`: large dynamic app surface with
  JSON construction, parse, and persistence paths.
- `crates/edgerun-codex/*`: Codex-derived protocol/client/API/tool surfaces
  are saturated with `ToJson`, `FromJson`, `Value`, `Map`, `json!`, and schema
  generation. Retiring this cleanly needs a dedicated Codex protocol model pass.
- `crates/edgerun-oci/*`: OCI JSON object models use `impl_json_struct!` and
  typed conversion heavily. Prefer moving OCI to protocol-owned records and WAT
  parse validation as a separate crate pass.
- `crates/utility/edgerun-opentelemetry-sdk-upstream/*` and
  `crates/utility/edgerun-tracing-subscriber/*`: upstream-style compatibility
  crates should not be churned until the local model replacement exists.
- `crates/utility/edgerun-json/src/yaml*.rs`: YAML has no WAT replacement yet.
  Delete only if no real caller needs YAML; otherwise write a YAML retirement
  note first.

## Full direct Rust API hit list

Generated with:

```bash
rg -l "edgerun_json::|use edgerun_json" crates --glob '*.rs' | sort
```

```text
crates/apps/edgerun-exchange-api/src/handlers.rs
crates/apps/edgerun-exchange-api/src/routes.rs
crates/apps/edgerun-exchange-api/src/types.rs
crates/apps/edgerun-oauth/src/discovery.rs
crates/apps/edgerun-oauth/src/jwt.rs
crates/apps/edgerun-oauth/src/token_store.rs
crates/apps/edgerun-oauth/src/types.rs
crates/apps/edgerun-pocketbase/src/main.rs
crates/apps/edgerun-secret-service/src/backend.rs
crates/edgerun-codelyzer/src/codealyzer/cargo_toml.rs
crates/edgerun-codelyzer/src/codealyzer/crate_model.rs
crates/edgerun-codelyzer/src/codealyzer/dependency_analyzer.rs
crates/edgerun-codelyzer/src/codealyzer/dependency_footprint.rs
crates/edgerun-codelyzer/src/codealyzer/errors.rs
crates/edgerun-codelyzer/src/codealyzer/issue_report.rs
crates/edgerun-codelyzer/src/codealyzer/standards.rs
crates/edgerun-codelyzer/src/codealyzer/tests.rs
crates/edgerun-codelyzer/src/codealyzer/workspace_membership.rs
crates/edgerun-codelyzer/src/dead_code.rs
crates/edgerun-codelyzer/src/mcp_rust_ast.rs
crates/edgerun-codex/api/src/api_bridge.rs
crates/edgerun-codex/api/src/api_bridge_tests.rs
crates/edgerun-codex/api/src/common.rs
crates/edgerun-codex/api/src/endpoint/compact.rs
crates/edgerun-codex/api/src/endpoint/memories.rs
crates/edgerun-codex/api/src/endpoint/models.rs
crates/edgerun-codex/api/src/endpoint/realtime_call.rs
crates/edgerun-codex/api/src/endpoint/realtime_websocket/methods.rs
crates/edgerun-codex/api/src/endpoint/realtime_websocket/methods_common.rs
crates/edgerun-codex/api/src/endpoint/realtime_websocket/methods_v2.rs
crates/edgerun-codex/api/src/endpoint/realtime_websocket/protocol.rs
crates/edgerun-codex/api/src/endpoint/realtime_websocket/protocol_common.rs
crates/edgerun-codex/api/src/endpoint/realtime_websocket/protocol_v1.rs
crates/edgerun-codex/api/src/endpoint/realtime_websocket/protocol_v2.rs
crates/edgerun-codex/api/src/endpoint/responses.rs
crates/edgerun-codex/api/src/endpoint/responses_websocket.rs
crates/edgerun-codex/api/src/endpoint/session.rs
crates/edgerun-codex/api/src/files.rs
crates/edgerun-codex/api/src/rate_limits.rs
crates/edgerun-codex/api/src/requests/responses.rs
crates/edgerun-codex/api/src/sse/responses.rs
crates/edgerun-codex/api/tests/clients.rs
crates/edgerun-codex/api/tests/realtime_websocket_e2e.rs
crates/edgerun-codex/api/tests/sse_end_to_end.rs
crates/edgerun-codex/app-protocol/src/export.rs
crates/edgerun-codex/app-protocol/src/jsonrpc_lite.rs
crates/edgerun-codex/app-protocol/src/protocol/common.rs
crates/edgerun-codex/app-protocol/src/protocol/common_tests.rs
crates/edgerun-codex/app-protocol/src/protocol/thread_history.rs
crates/edgerun-codex/app-protocol/src/protocol/v1.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/account.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/apps.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/collaboration_mode.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/command_exec.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/config.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/experimental_feature.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/feedback.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/fs.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/hook.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/item.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/mcp.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/model.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/notification.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/permissions.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/plugin.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/process.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/realtime.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/remote_control.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/review.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/shared.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/tests.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/thread.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/thread_data.rs
crates/edgerun-codex/app-protocol/src/protocol/v2/turn.rs
crates/edgerun-codex/app-protocol/src/schema_fixtures.rs
crates/edgerun-codex/client/src/default_client.rs
crates/edgerun-codex/client/src/request.rs
crates/edgerun-codex/code/src/description.rs
crates/edgerun-codex/code/src/response.rs
crates/edgerun-codex/code/src/runtime/mod.rs
crates/edgerun-codex/code/src/service.rs
crates/edgerun-codex/core/examples/live_smoke.rs
crates/edgerun-codex/core/src/stub_lib.rs
crates/edgerun-codex/debug-context/src/lib.rs
crates/edgerun-codex/host/src/main.rs
crates/edgerun-codex/model/catalog/src/cache.rs
crates/edgerun-codex/model/catalog/src/lib.rs
crates/edgerun-codex/model/catalog/src/manager_tests.rs
crates/edgerun-codex/model/info/src/lib.rs
crates/edgerun-codex/model/info/src/model_provider_info_tests.rs
crates/edgerun-codex/model/provider/src/provider.rs
crates/edgerun-codex/protocol/src/account.rs
crates/edgerun-codex/protocol/src/agent_path.rs
crates/edgerun-codex/protocol/src/approvals.rs
crates/edgerun-codex/protocol/src/auth.rs
crates/edgerun-codex/protocol/src/compat.rs
crates/edgerun-codex/protocol/src/config_types.rs
crates/edgerun-codex/protocol/src/dynamic_tools.rs
crates/edgerun-codex/protocol/src/error.rs
crates/edgerun-codex/protocol/src/items.rs
crates/edgerun-codex/protocol/src/mcp.rs
crates/edgerun-codex/protocol/src/memory_citation.rs
crates/edgerun-codex/protocol/src/models.rs
crates/edgerun-codex/protocol/src/network_policy.rs
crates/edgerun-codex/protocol/src/openai_models.rs
crates/edgerun-codex/protocol/src/parse_command.rs
crates/edgerun-codex/protocol/src/permissions.rs
crates/edgerun-codex/protocol/src/plan_tool.rs
crates/edgerun-codex/protocol/src/protocol.rs
crates/edgerun-codex/protocol/src/request_user_input.rs
crates/edgerun-codex/protocol/src/session_id.rs
crates/edgerun-codex/protocol/src/thread_id.rs
crates/edgerun-codex/protocol/src/tool_name.rs
crates/edgerun-codex/protocol/src/user_input.rs
crates/edgerun-codex/tools/src/code_mode.rs
crates/edgerun-codex/tools/src/code_mode_tests.rs
crates/edgerun-codex/tools/src/dynamic_tool.rs
crates/edgerun-codex/tools/src/dynamic_tool_tests.rs
crates/edgerun-codex/tools/src/image_detail_tests.rs
crates/edgerun-codex/tools/src/json_schema.rs
crates/edgerun-codex/tools/src/json_schema_tests.rs
crates/edgerun-codex/tools/src/mcp_tool.rs
crates/edgerun-codex/tools/src/mcp_tool_tests.rs
crates/edgerun-codex/tools/src/request_plugin_install.rs
crates/edgerun-codex/tools/src/request_plugin_install_tests.rs
crates/edgerun-codex/tools/src/responses_api.rs
crates/edgerun-codex/tools/src/responses_api_tests.rs
crates/edgerun-codex/tools/src/tool_config_tests.rs
crates/edgerun-codex/tools/src/tool_definition.rs
crates/edgerun-codex/tools/src/tool_definition_tests.rs
crates/edgerun-codex/tools/src/tool_discovery.rs
crates/edgerun-codex/tools/src/tool_discovery_tests.rs
crates/edgerun-codex/tools/src/tool_spec.rs
crates/edgerun-codex/tools/src/tool_spec_tests.rs
crates/edgerun-codex/tui/src/main.rs
crates/edgerun-codex/tui/src/ui.rs
crates/edgerun-exchange/src/changenow.rs
crates/edgerun-exchange/src/ffio.rs
crates/edgerun-exchange/src/provider_http.rs
crates/edgerun-exchange/src/sideshift.rs
crates/edgerun-oci/bin/ert.rs
crates/edgerun-oci/cli/events.rs
crates/edgerun-oci/cli/features.rs
crates/edgerun-oci/cli/images.rs
crates/edgerun-oci/cli/inspect.rs
crates/edgerun-oci/cli/ps.rs
crates/edgerun-oci/cli/state.rs
crates/edgerun-oci/cli/update.rs
crates/edgerun-oci/src/clap/cli.rs
crates/edgerun-oci/src/hooks.rs
crates/edgerun-oci/src/registry/client.rs
crates/edgerun-oci/src/registry/provenance.rs
crates/edgerun-oci/src/registry/pull.rs
crates/edgerun-oci/src/registry/push_manifest.rs
crates/edgerun-oci/src/state.rs
crates/edgerun-sdk/src/marketplace.rs
crates/mesh/edgerun-mesh/src/mesh_payload.rs
crates/node/edgerun-node/src/provisioning_listener.rs
crates/node/edgerun-node/src/services/http_runtime.rs
crates/protocol/edgerun-core/src/conformance.rs
crates/protocol/edgerun-protocols/src/acme/types.rs
crates/protocol/edgerun-protocols/src/oci/config.rs
crates/protocol/edgerun-protocols/src/oci/runtime_spec.rs
crates/protocol/edgerun-protocols/src/oci/runtime_spec_json.rs
crates/protocol/edgerun-protocols/src/tuya.rs
crates/utility/edgerun-bytes-upstream/src/json_compat.rs
crates/utility/edgerun-bytes-upstream/tests/test_json_compat.rs
crates/utility/edgerun-crypto/src/crypto_bigint/checked.rs
crates/utility/edgerun-crypto/src/crypto_bigint/limb.rs
crates/utility/edgerun-crypto/src/crypto_bigint/non_zero.rs
crates/utility/edgerun-crypto/src/crypto_bigint/uint.rs
crates/utility/edgerun-crypto/src/crypto_bigint/wrapping.rs
crates/utility/edgerun-crypto/src/curve25519_dalek/edwards.rs
crates/utility/edgerun-crypto/src/curve25519_dalek/ristretto.rs
crates/utility/edgerun-crypto/src/curve25519_dalek/scalar.rs
crates/utility/edgerun-crypto/src/ed25519_dalek/signing.rs
crates/utility/edgerun-crypto/src/ed25519_dalek/verifying.rs
crates/utility/edgerun-crypto/src/generic_array_impl/impl_json_compat.rs
crates/utility/edgerun-crypto/src/num_bigint/biguint.rs
crates/utility/edgerun-crypto/src/num_bigint/smallvec_impl.rs
crates/utility/edgerun-crypto/src/primeorder/affine.rs
crates/utility/edgerun-crypto/src/rand_core/block.rs
crates/utility/edgerun-json-derive/src/lib.rs
crates/utility/edgerun-json/src/api.rs
crates/utility/edgerun-json/src/error.rs
crates/utility/edgerun-json/src/lib.rs
crates/utility/edgerun-json/src/value.rs
crates/utility/edgerun-json/tests/depth_small.rs
crates/utility/edgerun-json/tests/derive.rs
crates/utility/edgerun-json/tests/model_macro.rs
crates/utility/edgerun-json/tests/unicode.rs
crates/utility/edgerun-opentelemetry-sdk-upstream/src/trace/sampler/jaeger_remote/remote.rs
crates/utility/edgerun-opentelemetry-sdk-upstream/src/trace/sampler/jaeger_remote/sampler.rs
crates/utility/edgerun-reqwest/src/lib.rs
crates/utility/edgerun-reqwest/src/native.rs
crates/utility/edgerun-rmcp/src/lib.rs
crates/utility/edgerun-slab/src/json_compat.rs
crates/utility/edgerun-smallvec/src/lib.rs
crates/utility/edgerun-tinyvec/src/arrayvec.rs
crates/utility/edgerun-tinyvec/src/tinyvec.rs
crates/utility/edgerun-tracing-subscriber/src/fmt/format/json.rs
crates/utility/edgerun-tracing-subscriber/src/fmt/writer.rs
crates/utility/edgerun-tracing/src/lib.rs
crates/utility/edgerun-url/src/lib.rs
crates/utility/edgerun-wiremock/src/lib.rs
```

## Retirement order

1. Delete self-tests and compatibility glue whose only purpose is preserving
   `edgerun-json` behavior.
2. Move `parse_json_tape`, `parse_json`, `from_str`, `from_slice`,
   `from_json_str`, and `from_json_slice` callers to `json-tape.wat` /
   `json-scalar.wat` with small field extractors.
3. Move TOML callers to `toml-scan.wat` and strict manifest-field extraction.
4. Introduce or select the replacement for dynamic JSON model construction:
   preferably wire records and local schema builders; only keep a tiny
   `no_std + alloc` dynamic value facade where dynamic JSON is required.
5. Retire `edgerun-json-derive` after all `ToJson` / `FromJson` derives and
   `impl_json_struct!` users move to explicit model builders or wire records.
6. Remove the root workspace dependency and stale documentation references.

## Open proof gaps

- WAT validates/scans JSON and TOML but does not yet replace every outbound
  JSON serializer or dynamic value builder.
- YAML has no WAT replacement and should be treated as a separate retirement
  decision.
- The largest dependency clusters are Codex-derived protocol/API/tool crates
  and OCI model crates; these need dedicated crate passes instead of a single
  blind dependency deletion.
