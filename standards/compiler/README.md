# Standards WASM Compiler

There is no checked-in compiler entrypoint at the moment. The reviewed
standards semantics live in the SDK's dependency-free `no_std`
`standards_seed` module, enabled with the `standards-seed` feature. A future
`std` runner can use that module for filesystem, WAT-to-WASM, and JSON output
without reintroducing a standalone seed crate.

The compiler takes reviewed RFC IR and emits one WASM module per executable
statement:

```text
definition IR -> definition.wasm
clause IR     -> clause.wasm
contract unit -> build/units/<sha256>.json
contract graph -> build/units/<sha256>.graph.json
program IR    -> program.json
rust core     -> extract fields, invoke clauses, emit reports
```

The compiled program identity is the contract graph hash. A graph is composed
from:

- selected contract unit hashes,
- typed interfaces between units,
- binding rules selected by the profile,
- and the graph profile.

The compiled component graph separately records emitted WASM module hashes.

## Current Check

```bash
cargo test -p edgerun-sdk --features standards-seed standards_seed
```

The test target validates the reviewed Rust core semantics. A future `check`
command should validate the catalog, compile the program, validate the WASM
modules, run the corpus through the Rust core, and compare
requirement/severity signatures against explicit corpus expectations. Corpus
expectations live in `standards/corpus/<program>/cases.toml`; checks should not
infer pass/fail from filenames.

## Current Scope

The first compiled program is intentionally small:

```text
udp-rfc768-length-0001
tftp-rfc1350-opcode-0001
tftp-rfc1350-ack-length-0001
tftp-rfc1350-data-length-0001
```

This proves the compiler and assembly loop. Each reviewed definition or clause
is now a `ContractUnit` with a stable hash over standard metadata, reviewed
normative text, canonical IR, imports, exports, capabilities, and WASM
entrypoint. Definition modules export field-access functions such as
`udp.length_at(ptr)` and `tftp.opcode_at(ptr)`. The Rust core currently owns the
reviewed seed semantics directly; the CLI emits matching WASM statement modules
and a `program.json` component graph.

## Runtime Boundary

There is no JavaScript or Python runtime in the standards path. The boundary is
now `edgerun-sdk::standards_seed`: a `no_std` Rust core that parses external
standard wire bytes, evaluates reviewed clauses, and can be compiled for bare
targets. Any future `std` CLI should only adapt files, WAT-to-WASM tool
execution, and JSON output.

## Clause Expression Language

Clause predicates use `edgerun-clause-expr`. The compiler currently supports:

```text
ident
integer
(expr)
expr && expr
expr || expr
expr == expr
expr != expr
expr >= expr
expr <= expr
expr > expr
expr < expr
expr + expr
expr - expr
expr in [integer, integer, ...]
len(bound_identifier)
```

All values lower to `i32` for now. Identifiers are taken from
`[clause.input].bindings` and become WASM function parameters in binding order.

Example:

```toml
[clause.input]
bindings = { byte_len = "len(input.bytes)", opcode = "field.opcode" }

[clause.predicate]
expr = "opcode != 4 || byte_len == 4"
```

This compiles to a statement module with:

```text
check(byte_len: i32, opcode: i32) -> i32
```
