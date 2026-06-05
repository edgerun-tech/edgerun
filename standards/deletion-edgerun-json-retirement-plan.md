# edgerun-json full retirement plan

Current deletion status:

- Deleted parser/scanner source: `parse.rs`, `tape.rs`, `toml.rs`,
  `toml_api.rs`, `toml_parse.rs`.
- WAT proof modules: `json-scalar.wat`, `json-tape.wat`, `toml-scan.wat`.
- Deleted remaining source and crate-local tests after this batch:
  `api.rs`, `borrowed_value.rs`, `error.rs`, `index.rs`, `io.rs`,
  `json_macro.rs`, `lib.rs`, `map.rs`, `model.rs`, `number.rs`,
  `partial_eq.rs`, `util.rs`, `value.rs`, `yaml.rs`, `yaml_api.rs`,
  `yaml_parse.rs`, and `tests/*.rs`.
- New WAT/doc proof modules: `json-emit.wat`, `json-value-core.wat`,
  `yaml-scan.wat`, and `edgerun-json-value-core.md`.

Remaining useful behavior to extract before deleting the crate:

## JSON Emission

Rust files:

- `api.rs`
- `util.rs`
- `json_macro.rs`

Useful behavior:

- JSON string escaping with surrounding quotes.
- JSON object key escaping and `:` separator emission.
- Compact scalar emission for `null`, booleans, strings, signed/unsigned
  integers, and finite floats.
- Compact array/object separators.
- Pretty indentation is optional; keep only if a real runtime needs it.

Extraction target:

- `json-emit.wat`
- `json-emit-smoke.js`

## Value Records

Rust files:

- `value.rs`
- `number.rs`
- `map.rs`
- `model.rs`
- `borrowed_value.rs`
- `index.rs`
- `partial_eq.rs`

Useful behavior:

- JSON kind tags: null, bool, number, string, array, object.
- Number kind and conversion policy: `I64`, `U64`, `F64`, finite-only floats.
- Object lookup is linear first-match over insertion-order pairs.
- `Map::insert` replaces an existing key and otherwise appends.
- Required field errors are policy, not wire format.
- `ToJson` / `FromJson` derive-style conversion is Rust convenience, not a
  portable codec primitive.

Extraction target:

- `json-value-core.wat` if small record helpers are worth preserving.
- Otherwise a wire-record standard that lets app-specific schemas bypass this
  crate entirely.

## YAML

Rust files:

- `yaml.rs`
- `yaml_api.rs`
- `yaml_parse.rs`

Useful behavior:

- Empty input -> null.
- Simple scalar classification: null, bool aliases, integer, unsigned integer,
  finite float, quoted string, bare string.
- Simple mapping/list indentation scan.
- Tagged YAML is a convenience object model, not a core codec need.

Extraction target:

- `yaml-scan.wat` for structural line/scalar scanning, or a deletion note if
  YAML is not worth keeping in the runtime pipeline.

## Error And IO Glue

Rust files:

- `error.rs`
- `io.rs`

Useful behavior:

- Error strings are Rust API compatibility, not codec behavior.
- `io.rs` is host adapter glue and can be deleted with the crate once callers
  move away.

Retirement condition:

- Done: WAT JSON emission behavior has smoke coverage.
- Done: value/model behavior is extracted to WAT records and documented.
- Done: YAML has a WAT scanner.
- Done: remaining caller audit groups all `edgerun-json` dependents by
  replacement path.

This crate is retired as a Rust source implementation. The next work is caller
cleanup by dependent crate, not rebuilding compatibility inside
`edgerun-json`.
