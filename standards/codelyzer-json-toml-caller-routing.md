# codelyzer TOML/JSON/YAML caller routing

Status: active 2026-06-05.

`crates/edgerun-codelyzer` still imports deleted `edgerun-json` for three jobs:

1. TOML manifest field projection.
2. Cargo metadata and compiler-message JSON field projection.
3. JSON report/MCP output emission.

Do not recreate `TomlValue`, `JsonValue`, `Map`, `ToJson`, `FromJson`,
`impl_json_struct!`, or `json!`. The replacement is fixed field projection
through WAT-owned scanners and fixed emitters:

- `toml-scan.wat`
- `json-tape.wat`
- `json-scalar.wat`
- `json-emit.wat`
- `yaml-scan.wat` only if a live YAML consumer is found.

Initial scan:

```bash
rg -n "edgerun_json|edgerun-json|TomlValue|JsonValue|impl_json_struct|json!" crates/edgerun-codelyzer
```

Current result count after the first Cargo TOML demolition packet: `27` for
the targeted scan:

```bash
rg -n "edgerun_json|edgerun-json|TomlValue|from_toml_str" crates/edgerun-codelyzer
```

The remaining hits are JSON input/output surfaces and the crate manifest
dependency, not Cargo TOML manifest readers.

Second packet status:

- Exact broad scan requested by the deletion loop moved from `358` to `329`
  lines, then this report/footprint packet moved it to `257` lines:

```bash
rg -n "edgerun_json|edgerun-json|JsonValue|\bValue\b|Map|ToJson|FromJson|json!" crates/edgerun-codelyzer
```

- Useful deleted-API scan is now limited to:
  `Cargo.toml` and `src/mcp_rust_ast.rs`.
- The broad scan still includes ordinary `HashMap` and JavaScript `Map` hits;
  those are not deleted JSON model blockers.

## Cargo Manifest Projection

### `src/codealyzer/cargo_toml.rs`

Deleted API: `edgerun_json::from_toml_str`, `edgerun_json::TomlValue`.

Required fields:

- `[package].name`
- `[package].version`
- `[package].description`
- `[package].license`
- `[package].edition`
- `[package].rust-version`
- `[features]` table keys
- `[lib]` presence
- `[[bin]].name`
- `[[test]].name`
- `[[bench]].name`

Replacement:

- Done: added owner-local `CargoTomlProjection` with those fields only in
  `src/codealyzer/cargo_toml_projection.rs`.
- Temporary bridge: the helper is a tiny fixed Rust projection parser until this
  crate has a direct host invocation surface for `toml-scan.wat`. It maps to
  `toml-scan.wat` table, array-of-table, string, array-of-string, boolean, and
  key spans.
- Keep filesystem auto-discovery of `src/lib.rs`, `src/main.rs`, and
  `src/bin/*.rs` unchanged.
- Done: deleted `is_lib_target(package: &TomlValue, ...)` and
  `collect_declared_targets(toml: &TomlValue, ...)`.

### `src/codealyzer/dependency_analyzer.rs`

Deleted API: `edgerun_json::from_toml_str`, `edgerun_json::TomlValue`.

Required fields for dependency tables:

- `[dependencies]`
- `[dev-dependencies]`
- `[build-dependencies]`
- dotted dependency keys such as `foo.workspace`
- string dependency versions
- inline table keys: `package`, `version`, `optional`, `features`, `path`,
  `git`, `workspace`
- inline table array of strings for `features`

Required fields for workspace root:

- `[workspace].members`
- `[workspace.dependencies]` keys

Replacement:

- Done: reused the manifest projection helper with dependency-row spans.
- Done: replaced `TomlValue` with owner-local `DependencyValue`: `String`,
  `Bool`, `StringArray`, `InlineTable(Vec<(String, DependencyValue)>)`, and
  `Bare` for raw scalar cells.
- Keep package alias, source kind, optional flag, feature list, local path, git
  source, workspace inheritance, and dotted dependency property merge.

### `src/dead_code.rs`

Deleted API: `edgerun_json::from_toml_str`, `edgerun_json::TomlValue`.

Required fields:

- root `[workspace].members`
- root `[workspace].default-members`
- package `[package].name`
- package `[features]` keys
- package dependency table keys for `dependencies`, `dev-dependencies`, and
  `build-dependencies`

Replacement:

- Done: reused the same manifest projection helper.
- Done: deleted `table_get` and `collect_manifest_dependencies(TomlValue)`.
- Keep `expand_member_pattern` and VFS path normalization unchanged.

### `src/codealyzer/standards.rs`

Deleted API: `edgerun_json::from_toml_str`.

Required fields:

- top-level standard table names
- each table's `status` string

Replacement:

- Done through the temporary fixed projection bridge: iterate top-level tables
  and the `status` key only. This maps directly to `toml-scan.wat` table and
  string spans when host invocation is available.
- Preserve the current fallback: if no standards are projected, return the
  single `unknown` coverage row.

Completed first packet:

- `src/codealyzer/cargo_toml.rs` no longer calls deleted TOML APIs.
- `src/codealyzer/dependency_analyzer.rs` no longer imports `TomlValue` or
  calls `from_toml_str`.
- `src/dead_code.rs` no longer calls deleted TOML APIs for workspace/package
  indexing.
- `src/codealyzer/standards.rs` no longer calls deleted TOML APIs.

Second packet completed:

1. Done: `src/codealyzer/workspace_membership.rs` no longer uses
   `parse_json_tape`; it projects only `workspace_members`, package `id`, and
   package `manifest_path` from cargo metadata JSON.
2. Done: `src/codealyzer/tests.rs` no longer uses `parse_json` or `Value`; it
   checks only `reason == "compiler-message"` and
   `message.level == "warning"`.
3. Done: `src/codealyzer/issue_report.rs` now emits the fixed local issue
   object directly as JSON string bytes.
4. Done: `src/codealyzer/errors.rs` now has a fixed `to_json_string` method
   and no `ToJson` implementation.

Third packet completed:

1. Done: `src/codealyzer/dependency_footprint.rs` removed generated cargo
   metadata JSON macros and the `from_json_str::<CargoMetadata>` path. It now
   uses explicit package, dependency, target, workspace member, and resolve-edge
   projection records.
2. Done: `src/codealyzer/crate_model.rs` removed report-model `ToJson`
   compatibility impls and `edgerun-json` imports. Report records now have
   fixed JSON string emitters.
3. Done: `src/codealyzer/report.rs` writes `report.json` through the fixed
   `CrateReport::to_json_string()` emitter.

Next packet:

1. `src/mcp_rust_ast.rs`: replace `Value`/`json!` request and response handling
   with fixed request projection plus response JSON emitters.
2. Remove the `edgerun-codelyzer` manifest dependency after `mcp_rust_ast.rs`
   is clear.

## JSON Input Projection

### `src/codealyzer/workspace_membership.rs`

Deleted API: `edgerun_json::parse_json_tape`.

Required cargo metadata fields:

- root `workspace_members` array of package id strings
- root `packages` array
- package `id`
- package `manifest_path`

Replacement:

- Done through an owner-local fixed projection bridge for the exact cargo
  metadata fields above. This is a temporary host shim for `json-tape.wat` /
  `json-scalar.wat`, not a general JSON object model.
- `cargo metadata` execution stays unchanged.

### `src/codealyzer/tests.rs`

Deleted API: `edgerun_json::parse_json`, `edgerun_json::Value`.

Required compiler-message fields:

- `reason == "compiler-message"`
- `message.level == "warning"`

Replacement:

- Done through an owner-local fixed field check for those two string fields.
- Non-JSON lines still return false unless they contain the legacy text
  `: warning:` marker.

### `src/codealyzer/dependency_footprint.rs`

Deleted API: `impl_json_struct!`, `edgerun_json::from_json_str::<CargoMetadata>`.

Required cargo metadata fields:

- package `id`
- package `name`
- package `version`
- package `manifest_path`
- package `targets[*].kind`
- package `targets[*].name`
- package dependency `name`, `source`, `req`, `kind`, `optional`,
  `uses_default_features`
- resolve graph node `id`
- resolve dependency edge `pkg`
- workspace members if consumed by report logic

Replacement:

- Done: generated structs and `impl_json_struct!` were removed.
- Done: cargo metadata loading now projects explicit records shaped for the
  footprint report. The temporary owner-local scanner maps to `json-tape.wat`
  object/array traversal and `json-scalar.wat` string validation; it does not
  construct an intermediate dynamic JSON value.

## JSON Output Emission

### `src/mcp_rust_ast.rs`

Deleted API: `edgerun_json::Value`, `edgerun_json::json!`.

Inbound required fields:

- all operations: `op`, `path`
- `find_fn`: `name`
- `replace_fn_body`: `name`, `body`
- `add_fn`: `name`, optional `args`, optional `ret`, `body`
- `remove_fn`: `name`
- `add_use`: `use_path`
- `add_derive`: `name`, `derive`
- `rename_type`: `old`, `new`
- `new_file`: `path`, optional `content`
- `incoming_refs`: `path`

Outbound shapes:

- `{"items": [...] }`
- `{"name": "...", "found": bool, "source": string|null}`
- `{"changed": bool, "op": "...", ...}`
- `{"incoming_refs": [...] }`

Replacement:

- Change `call` to accept raw JSON bytes or a small request record projected
  from `json-tape.wat`.
- Emit responses with `json-emit.wat` or a local fixed emitter that only writes
  these documented shapes.
- Preserve arbitrary Rust source strings as escaped JSON strings, not dynamic
  values.

### `src/codealyzer/issue_report.rs`

Deleted API: `JsonValue`, `Map`.

Outbound fields:

- `crate`
- optional `finding_id`
- optional `file`
- optional `line`
- optional `severity`
- optional `recommendation`
- fixed `status = "pending"`

Replacement:

- Done: `generate_local_issue` returns direct JSON string bytes for exactly
  those fields.
  The GitHub issue-link URL builder is unchanged.

### `src/codealyzer/errors.rs`

Deleted API: `JsonValue`, `Map`, `ToJson`.

Outbound shapes:

- `{"CrateNotFound": "..."}`
- `{"file": "...", "message": "..."}`
- `{"IoError": "..."}`
- `{"NotImplemented": "..."}`
- `{"VisibilityViolation": "..."}`

Replacement:

- Done: `AnalyzerError::to_json_string` emits the fixed object shape directly.

### `src/codealyzer/crate_model.rs`

Deleted API: `JsonValue`, `Map`, `ToJson`.

Status:

- Done. The report-model compatibility surface for `CrateIdentity`,
  `Dependency`, API items, findings, tests, footprint, benchmarks, standards,
  visibility, and `CrateReport` no longer implements generic JSON traits.

Replacement:

- Done: model records have fixed `to_json_string` emitters backed by one local
  string/object/array helper set.
- Done: `src/codealyzer/report.rs` writes `report.json` through
  `CrateReport::to_json_string()`.
- This is a temporary owner-local host emitter for `json-emit.wat` fixed field
  output, not a generic value tree.

Report field sets now emitted:

- `CrateIdentity`: `name`, `version`, `path`, `description`, `license`,
  `edition`, `rust_version`, `features`, `lib_target`, `bin_targets`,
  `test_targets`, `bench_targets`, `crate_type`, `visible_files`,
  `hidden_files_count`, `commit_hash`
- `Dependency`: `name`, `version_req`, `kind`, `optional`, `features`,
  `reason`, `is_workspace`, `is_visible`, `source`, `source_ref`, `weight`
- `ApiItem`: `name`, `kind`, `file`, `line`, `docs`, `feature_gate`,
  `visibility`
- `CallGraphEdge`: `caller`, `callee`, `file`, `line`, `confidence`,
  `runtime_count`
- `SecurityFinding`: `id`, `severity`, `title`, `file`, `line`,
  `code_excerpt`, `explanation`, `confidence`, `recommendation`
- `TestInfo`: `total`, `unit_tests`, `integration_tests`, `doc_tests`,
  `ignored`, `passing`, `failing`, `last_run_status`, `last_run_commit`,
  `compile_status`, `compile_exit_code`, `compile_warnings`,
  `functionality_coverage`
- `FootprintInfo`: `binary_size`, `stripped_size`, `compressed_size`,
  `idle_rss`, `peak_rss`, `threads`, `open_fds`, `target_triple`, `measured`
- `BenchmarkArtifact`: `name`, `path`, `scenario`, `commit`
- `StandardCoverage`: `standard`, `status`, `code_refs`, `tests`,
  `confidence`, `notes`
- `VisibilityReport`: `visible_count`, `hidden_count`, `public_surface`,
  `blocked_refs`, `suspicious_visible`
- `CrateReport`: `identity`, `dependencies`, `public_api`, `call_graph`,
  `runtime_call_observations`, `security_findings`, `test_info`, `footprint`,
  `benchmarks`, `standards`, `visibility`, `generated_at`,
  `parser_confidence`

Replacement:

- Done: `CrateReport` emits the complete report shape in fixed order.
- Done: typed enum string helpers are used directly. No generic trait remains.

## YAML

No live YAML consumer was found in the codelyzer scan. `yaml-scan.wat` is not
needed for this crate unless a later agent finds a standards file or report
input that is actually YAML. Do not add YAML compatibility here speculatively.

## Deletion Order

1. TOML manifest projection helper, then `cargo_toml.rs`,
   `dependency_analyzer.rs`, `dead_code.rs`, and `standards.rs`.
2. Cargo metadata and compiler-message JSON input projection:
   `workspace_membership.rs`, `tests.rs`, and `dependency_footprint.rs`.
3. Fixed output writers:
   `crate_model.rs`, `errors.rs`, and `issue_report.rs` are done; only
   `mcp_rust_ast.rs` remains.
4. Remove `edgerun-json = { workspace = true, features = ["toml"] }` from
   `crates/edgerun-codelyzer/Cargo.toml`.

Do not run Cargo as a success gate for this lane. Use `rg` deltas and WAT smoke
or composition runners as evidence.
