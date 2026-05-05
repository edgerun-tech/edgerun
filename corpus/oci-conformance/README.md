# OCI Conformance Corpus

## Contents

| File/Directory | Source | Purpose |
|---|---|---|
| `runtime-tools-validation/` | `opencontainers/runtime-tools` | 67 Go source files — the actual conformance test implementations |

The OCI spec text is not vendored here. Use the upstream runtime-spec links
emitted by the validation tools when a conformance failure needs spec context.

## Running the Tests Against Our Runtime

The conformance tests are Go binaries that expect an OCI CLI runtime (`runc`-compatible).
To run them against our runtime, we'd need to:

1. Build a CLI wrapper around our library that implements the OCI runtime CLI
   (`create`, `start`, `kill`, `delete`, `state` commands)
2. Set `RUNTIME=edgerun-oci` and run:
   ```bash
   cd runtime-tools/validation
   go run ./default
   go run ./hostname
   go run ./process
   # etc.
   ```

See `runtime-tools/docs/runtime-compliance-testing.md` for full instructions.

## Spec Versions

- **runtime-tools**: v0.9.0 @ `8a4db57`
- **runtime-spec**: v1.3.0-dev @ `6f7b71c`
- **Our ociVersion**: `1.0.2`
