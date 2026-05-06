# Standards WASM Compiler

`scripts/standards` is the current compiler entrypoint.

The compiler takes reviewed RFC IR and emits one WASM module per executable
statement:

```text
definition IR -> definition.wasm
clause IR     -> clause.wasm
program IR    -> program.json
```

The compiled program identity is the hash of:

- the selected program IR,
- the selected policy profile,
- the compiler identity,
- and every emitted statement module hash.

## Current Commands

```bash
./scripts/standards validate
./scripts/standards compile udp-tftp-must-program
./scripts/standards components udp-tftp-must-program
./scripts/standards check udp-tftp-must-program
```

`check` is the high-level command. It validates the catalog, compiles the
program, validates the WASM modules, runs the corpus through both the
interpreter and WASM engine, and compares requirement/severity signatures.

## Current Scope

The first compiled program is intentionally small:

```text
udp-rfc768-length-0001
tftp-rfc1350-opcode-0001
tftp-rfc1350-ack-length-0001
```

This proves the compiler and assembly loop. The next step is to replace
hand-coded lowering cases with a typed expression compiler and make definition
modules perform real field extraction.
