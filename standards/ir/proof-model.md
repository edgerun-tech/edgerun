# Standards Proof Model

This model is for making protocol conformance reproducible and composable. It
does not prove that a standard is correct. It proves that a selected
implementation profile was tested against a specific, hash-addressed
interpretation of the standard.

## Unit Of Proof

A proof target is a tuple:

```text
implementation + profile + requirement set + component graph + evidence
```

- `implementation`: crate, binary, WASM component, firmware image, or adapter.
- `profile`: supported subset such as `udp-parser`, `tcp-minimal`, `http1`.
- `requirement set`: normative requirements selected from standards and
  registries.
- `component graph`: hash-addressed WASM components that parse, validate,
  generate, or judge behavior.
- `evidence`: inputs, outputs, traces, findings, and component hashes.

## Component Kinds

- `definition`: declares protocol fields, frame layouts, state names, and
  profile boundaries.
- `parser`: turns bytes into structured frames or rejects them.
- `validator`: checks structured data, traces, or state transitions against
  requirements.
- `generator`: creates valid and invalid cases from a requirement or field
  definition.
- `oracle`: compares implementation behavior against required behavior.
- `adapter`: connects an implementation under test to the common trace ABI.
- `registry`: resolves assigned numbers, code points, extension IDs, or option
  identifiers from a registry snapshot.

## Composition

Lower layers feed higher layers. A TCP checker should not parse Ethernet if it
can consume an already-validated IPv4 or IPv6 payload. This keeps each component
small and lets the proof graph say which assumptions were already discharged.

Example:

```text
ethernet-frame
  -> ipv4-packet
  -> udp-datagram
  -> dns-message
  -> dns-name-compression
```

Another example:

```text
ethernet-frame
  -> ipv4-packet
  -> tcp-segment
  -> tcp-state-machine
  -> http1-message
```

## Hashing Rule

Every component manifest has a `sha256` for the WASM artifact. Reports should
also hash:

- the manifest,
- the selected requirement records,
- registry snapshots,
- generated input cases,
- implementation artifact or source revision,
- and the final trace.

Hashing these inputs makes conformance reports reproducible and prevents a test
result from floating away from the exact definitions that produced it.

## Profile Rule

Most standards are too large to implement all at once. A profile must state what
is supported and what is intentionally out of scope. Unsupported requirements
are acceptable only when the selected profile excludes them loudly.

Good profile names:

- `ethernet-ii-frame-parser`
- `ipv4-minimal-host`
- `udp-datagram-codec`
- `tcp-passive-open-minimal`
- `http1-origin-server`

Bad profile names:

- `compliant`
- `standard`
- `full`

## Edgerun Boundary

External protocols keep their standards-defined byte encodings. Edgerun
internal wire, storage, cache, signing, hashing, and local bridge payloads remain
rkyv-only. WASM conformance components can inspect external protocol bytes, but
they are not an alternate Edgerun internal wire protocol.

