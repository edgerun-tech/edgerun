# OCI Conformance Corpus

## Contents

| File/Directory | Source | Purpose |
|---|---|---|
| `STATUS.md` | **Generated from test suite** | Tracks implementation status against all 57 official conformance tests |
| `runtime-tools-validation/` | `opencontainers/runtime-tools` | 67 Go source files — the actual conformance test implementations |
| `spec.md` | `opencontainers/runtime-spec` | Top-level spec index |
| `runtime.md` | `opencontainers/runtime-spec` | Runtime & lifecycle spec |
| `runtime-linux.md` | `opencontainers/runtime-spec` | Linux-specific runtime spec |
| `config.md` | `opencontainers/runtime-spec` | Configuration (config.json) spec |
| `config-linux.md` | `opencontainers/runtime-spec` | Linux-specific configuration spec |

## How STATUS.md is Generated

The `STATUS.md` file is derived from the test suite structure, not manually written:

```
1. Clone github.com/opencontainers/runtime-tools
2. Parse each validation/*.go file for:
   - Test directory name → test category
   - Set*Config() calls → config fields tested
   - specerror.NewError() → spec error codes referenced
   - struct{...} case definitions → test parameters
3. Map each test to our implementation status
```

When the test suite is updated, re-run:
```bash
cd /tmp && rm -rf runtime-tools && git clone --depth 1 https://github.com/opencontainers/runtime-tools.git
# Then regenerate STATUS.md from the test file structure
```

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
