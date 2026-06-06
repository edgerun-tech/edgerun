# RFC Policy Layer

The policy layer turns standards into executable, hash-addressed programs.

## Development Constraints

All implementation work in this repository follows these rules:

- **No new code.** Only edit/refactor existing code. New functionality comes
  from recombining existing fragments into pipelines.
- **Self-hosted tooling only.** No external dependencies or third-party code
  generators. Only `wat2wasm`, the local test runner, and the pipeline runtime
  are permitted.
- **Pipeline decomposition.** All processing logic must be split into composable
  pipeline stages registered in the 64-slot dispatch table.
- **Remove duplicates.** Functionally equivalent code must be deduplicated into
  a single shared module.
- **Goal: performance + correctness through reduction.** Fewer lines, fewer
  modules, fewer moving parts. Meaning is preserved; verbosity is not.

The intended path is:

```text
RFC prose
  -> reviewed clause IR
  -> clause WASM predicates
  -> policy profile
  -> composed RFC program
```

The reviewed clause IR is the important abstraction. RFC prose is not a formal
program by itself; it includes definitions, requirements, recommendations,
options, examples, registries, updates, errata, and deployment policy. The IR is
where an interpretation becomes explicit and hashable.

## Clause Classes

- `definition`: field layout, frame layout, constants, encodings, registries.
- `must`: mandatory predicate from MUST or REQUIRED text.
- `must-not`: forbidden predicate from MUST NOT or SHALL NOT text.
- `should`: recommended predicate.
- `may`: optional capability or policy choice.
- `profile`: selection of clauses for one executable standard profile.

The first strict profile is:

```text
definitions + MUST + MUST NOT
```

This gives a deterministic base without pretending that SHOULD/MAY policy is
universal.

## Program Identity

An RFC program is identified by a hash of:

- selected clause IDs,
- clause IR content,
- registry snapshots,
- compiler version,
- WASM component hashes,
- composition graph,
- and profile parameters.

Changing any of those creates a different program.

## Why Clause-Level WASM

Compiling each clause or small clause group into WASM keeps the system
composable:

```text
ethernet-frame
  -> ipv4-header
  -> udp-datagram
  -> tftp-message
```

Each edge is an explicitly typed handoff between standards. A higher-level
program can depend on lower-layer clause hashes instead of copying their logic.

## Current Executable Slice

The first runnable slice is:

```text
udp-datagram definition
  + udp-rfc768-length-0001 clause
  -> tftp-message definition
  + tftp-rfc1350-opcode-0001 clause
```

Run it with:

```bash
cargo test -p edgerun-sdk --features standards-seed standards_seed
```

The checked-in `no_std` Rust core is the current compiler oracle: compiled
artifacts must match the reviewed Rust semantics for the same IR and input.
A future compiler should emit one WASM module per definition or clause under
`standards/build/wasm/<program>/`, write component manifests under
`standards/components/manifests/`, and assemble the program in `program.json`.
