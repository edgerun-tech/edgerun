# edgerun-json deletion pass

Rust parser/scanner source deleted from `crates/utility/edgerun-json` because
portable JSON/TOML parsing now lives in WAT:

- `src/parse.rs` -> `json-scalar.wat`, `json-tape.wat`
- `src/tape.rs` -> `json-tape.wat`
- `src/toml.rs` -> `toml-scan.wat`
- `src/toml_api.rs` -> `toml-scan.wat`
- `src/toml_parse.rs` -> `toml-scan.wat`

The follow-up retirement pass also deleted the remaining Rust source and
crate-local tests after extracting the useful non-parser behavior:

- `src/api.rs`, `src/util.rs`, `src/json_macro.rs` -> `json-emit.wat`
- `src/value.rs`, `src/number.rs`, `src/map.rs`, `src/model.rs`,
  `src/borrowed_value.rs`, `src/index.rs`, `src/partial_eq.rs` ->
  `json-value-core.wat` and `edgerun-json-value-core.md`
- `src/yaml.rs`, `src/yaml_api.rs`, `src/yaml_parse.rs` -> `yaml-scan.wat`
- `src/error.rs`, `src/io.rs` -> deleted as Rust API/host glue, not portable
  codec behavior
- `tests/*.rs` -> deleted with the retired Rust API surface

The crate manifest and README are left only as tombstones while dependent
crates are retired or moved to the WAT/runtime pipeline.

Proof commands:

```bash
node standards/runners/json-scalar-smoke.js
node standards/runners/json-tape-smoke.js
node standards/runners/toml-scan-smoke.js
node standards/runners/json-emit-smoke.js
node standards/runners/json-value-core-smoke.js
node standards/runners/yaml-scan-smoke.js
node standards/runners/rust-parity-misc-codecs.js
```

Do not use `cargo check` as the acceptance criterion for this deletion pass.
The deleted crate intentionally breaks old Rust import paths until callers move
to WAT/runtime consumption or are removed in their own crate pass.
