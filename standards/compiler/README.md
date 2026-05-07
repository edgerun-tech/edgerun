# Standards WASM Compiler

`scripts/standards` is the current compiler entrypoint. It launches the Rust
`edgerun-standards` crate; the standards semantics live in a dependency-free
`no_std` library and the CLI uses `std` only for filesystem and process I/O.

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

## Current Commands

```bash
./scripts/standards validate
./scripts/standards compile udp-tftp-must-program
./scripts/standards components udp-tftp-must-program
./scripts/standards definitions udp-tftp-must-program
./scripts/standards clauses udp-tftp-must-program
./scripts/standards units udp-tftp-must-program
./scripts/standards graph udp-tftp-must-program
./scripts/standards check udp-tftp-must-program
./scripts/standards run udp-tftp-must-program --wasm --hex <hex-bytes>
```

`check` is the high-level command. It validates the catalog, compiles the
program, validates the WASM modules, runs the corpus through the Rust core, and
compares requirement/severity signatures against explicit corpus expectations.
Corpus expectations live in `standards/corpus/<program>/cases.toml`; checks do
not infer pass/fail from filenames.

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
now `edgerun-standards/src/lib.rs`: a `no_std` Rust core that parses external
standard wire bytes, evaluates reviewed clauses, and can be compiled for bare
targets. The `std` CLI is only an adapter for files, WAT-to-WASM tool execution,
and JSON output.

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
