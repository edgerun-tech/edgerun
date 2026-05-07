# Standards Catalog

This directory is a local protocol discovery and conformance workspace. It is
for external standards such as HTTP, DNS, TLS, QUIC, DHCP, NFC, HPACK, and
QPACK. Edgerun's internal wire protocol remains rkyv-only; do not use this
catalog to introduce alternate internal encodings or compatibility bridges.

## Goals

- Discover protocols by layer, transport, registry, and runtime requirements.
- Track canonical standards, drafts, and IANA registries for each protocol.
- Extract normative requirements into testable units.
- Map requirements to Edgerun crates, tests, fuzz targets, and known gaps.
- Compare protocol candidates against no_std, bare-metal, and unikernel
  constraints before implementation work starts.
- Compose hash-addressed WASM components into proof pipelines that validate
  bytes, frames, state transitions, and implementation traces.

## Layout

```text
standards/
  README.md
  catalog.toml
  PROVABLE-IMPLEMENTATIONS.md
  protocols/
    ethernet.toml
    ipv4.toml
    udp.toml
    tcp.toml
    dns.toml
    http.toml
    tls.toml
    quic.toml
  ir/
    proof-model.md
    trace.schema.toml
  policy/
    README.md
    must-only.toml
  compiler/
    README.md
  clauses/
    udp.toml
    tftp.toml
  definitions/
    udp.toml
    tftp.toml
  programs/
    udp-tftp.toml
  implementations/
    edgerun-http.toml
    edgerun-protocols-tftp.toml
    edgerun-protocols-dhcp.toml
  components/
    README.md
    interfaces/
      conformance.wit
    manifests/
      component.toml
  registries/
    iana/
      README.md
  templates/
    protocol.toml
    requirement.toml
    test-case.toml
```

## Workflow

1. Add or update a protocol in `protocols/<name>.toml`.
2. Record canonical RFCs, active drafts, and standards bodies.
3. Link the relevant IANA registries and assigned-number tables.
4. Capture environment assumptions: clock, entropy, allocator, packet I/O,
   stream I/O, persistence, threading, and crypto requirements.
5. Extract normative requirements into `[[requirement]]` entries as they become
   implementation or test work.
6. Map each requirement to crate paths, tests, fuzz targets, interop suites, or
   an explicit unsupported status.
7. When a requirement is executable, attach a WASM component manifest. The
   manifest hash becomes part of the evidence chain for conformance reports.

## Proof Model

The catalog treats conformance as evidence, not a claim. A program is conformant
to a profile when a reproducible pipeline can show:

- the selected standards and profiles,
- the exact requirement set,
- the hash of each checker/generator/oracle component,
- the input corpus and generated cases,
- the implementation trace,
- and the findings produced by those components.

See `ir/proof-model.md` and `components/README.md`.

For the migration from current crates to reproducible conformance evidence, see
`PROVABLE-IMPLEMENTATIONS.md`.

For the clause-level RFC policy model, see `policy/README.md`. The seed
`must-only` profile composes definitions plus mandatory `MUST`/`MUST NOT`
clauses into hash-addressed programs.

## Runnable Seed

The current executable seed is `udp-tftp-must-program`. The entrypoint is
`scripts/standards`, a thin launcher for the Rust `edgerun-standards` crate.
The crate has a dependency-free `no_std` core and a `std` CLI that compiles
definition and clause IR into WASM modules, assembles them into `program.json`,
and runs input bytes through the same Rust core semantics.

```bash
./scripts/standards validate
./scripts/standards hash udp-tftp-must-program
./scripts/standards compile udp-tftp-must-program
./scripts/standards components udp-tftp-must-program
./scripts/standards definitions udp-tftp-must-program
./scripts/standards clauses udp-tftp-must-program
./scripts/standards units udp-tftp-must-program
./scripts/standards graph udp-tftp-must-program
./scripts/standards check udp-tftp-must-program
./scripts/standards run udp-tftp-must-program \
  --wasm \
  --hex "$(cat standards/corpus/udp-tftp/valid-ack.hex)"
```

`--wasm` is accepted for workflow compatibility, but there is no JavaScript or
Python runtime in the standards path now. The Rust core owns the UDP/TFTP
definition extraction and clause evaluation; the CLI also emits the
hash-addressed WASM statement modules for the component graph.

`compile` also writes content-addressed contract units to
`standards/build/units/<sha256>.json` and the composed graph to
`standards/build/units/<sha256>.graph.json`. A program can refer to these hashes
instead of carrying the full standards catalog or generated artifacts inline.

The runner emits JSON containing:

- `program_sha256`: hash of the composed contract graph.
- `component_graph_sha256`: hash of the assembled statement modules.
- `findings`: requirement-addressed pass/reject results.

The current corpus covers:

- valid UDP payload carrying a TFTP ACK,
- invalid UDP length,
- invalid TFTP ACK length with otherwise valid UDP framing.
- invalid TFTP DATA length with otherwise valid UDP framing.
- invalid TFTP opcode with otherwise valid UDP framing.

Corpus expectations are explicit in `standards/corpus/udp-tftp/cases.toml`.

Try the rejection path:

```bash
./scripts/standards run udp-tftp-must-program \
  --wasm \
  --hex "$(cat standards/corpus/udp-tftp/invalid-length.hex)"
```

`check` is the preferred smoke test: it compiles the program, validates the WASM
modules, runs the corpus with the Rust core, and verifies that the explicit
requirement/severity signatures match.

## Requirement Status

Use these status values consistently:

- `unknown`: requirement has not been evaluated.
- `planned`: implementation or tests are planned.
- `implemented`: behavior is implemented and covered by tests or interop.
- `partial`: behavior exists but has known gaps.
- `unsupported`: intentionally not supported by this implementation.
- `not_applicable`: requirement does not apply to the selected profile.

## Useful Sources

- RFC Editor: `https://www.rfc-editor.org/`
- IETF Datatracker: `https://datatracker.ietf.org/`
- IANA protocol registries: `https://www.iana.org/protocols`
- Wireshark dissectors: useful for practical protocol discovery.

Prefer RFC XML when extracting requirements because it preserves section
structure better than plain text:

```text
https://www.rfc-editor.org/rfc/rfc9000.xml
```
