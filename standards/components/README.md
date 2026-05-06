# WASM Conformance Components

Components are small WASM binaries that define, parse, generate, validate, or
judge protocol behavior. They should be deterministic and independent of the
host operating system. Runtime-specific code belongs in adapters.

## Component Graph

A protocol proof is a graph of components:

```text
definition -> parser -> validator -> oracle -> report
```

Definitions can be reused by generators and validators. For example, a UDP
datagram definition component can drive both a parser and a malformed-length
test generator.

## ABI

The initial ABI is defined in `interfaces/conformance.wit`. It is intentionally
small:

- `check-bytes`: inspect one byte buffer.
- `check-trace`: inspect an implementation trace.
- `generate`: produce test cases for a requirement.

The ABI uses strings and byte lists so components remain language-independent.
Structured payloads can be encoded with a deterministic format later, but that
format must be part of the component manifest and hash.

## Manifests

Every component has a manifest based on `manifests/component.toml`. The manifest
records:

- component kind,
- WASM artifact path,
- WASM SHA-256,
- inputs and outputs,
- standards and requirements covered,
- dependencies on lower-layer components.

The manifest and WASM hash are evidence. Changing either means prior reports do
not apply to the new checker.

