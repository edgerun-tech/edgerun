# EdgeRun

Scaffold for the Phase 1 EdgeRun system described in `Whitepaper.md`.

## Workspace Layout

- `program/`: Anchor-based Solana settlement program scaffold.
- `crates/edgerun-runtime`: deterministic WASM runtime CLI scaffold.
- `crates/edgerun-worker`: worker daemon scaffold.
- `crates/edgerun-scheduler`: scheduler API/service scaffold.
- `crates/edgerun-storage`: content-addressed bundle storage scaffold.
- `crates/edgerun-types`: shared domain types.
- `crates/edgerun-crypto`: shared hashing/signature helpers.
- `docs/`: architecture and roadmap docs.

## Quick Start

```bash
./scripts/edgerun doctor
./scripts/edgerun setup
./scripts/edgerun build workspace
./scripts/edgerun test runtime
```

One-command baseline check:

```bash
./scripts/edgerun all
```

Run service scaffolds:

```bash
cargo run -p edgerun-scheduler
cargo run -p storage-engine
```

Run runtime scaffold:

```bash
cargo run -p edgerun-runtime -- run --bundle ./bundle.cbor --output ./out.bin
```

## Project CLI

`./scripts/edgerun` is the orchestrator for setup/build/test workflows.

Examples:

```bash
./scripts/edgerun doctor
./scripts/edgerun setup --install-missing
./scripts/edgerun build all
./scripts/edgerun test integration
./scripts/edgerun test e2e
./scripts/edgerun run replay-corpus
./scripts/edgerun run fuzz-weekly
```

Optional shortcut install:

```bash
./scripts/install.sh
./edgerun all
```

Policy rotation runbook:

- `docs/policy-rotation.md`

## CI

GitHub Actions workflow:

- `.github/workflows/ci.yml`

Run the same workflow locally (without pushing):

```bash
./scripts/ci-local.sh --dry-run
./scripts/ci-local.sh --job rust-checks
./scripts/ci-local.sh --job integration
```

If `act` is installed, `ci-local.sh` executes the workflow via `act`.
If `act` is missing, it falls back to equivalent local commands.

## Notes

- Frozen protocol choices (BLAKE3, Ed25519, committee=3 quorum=2, SOL) are captured in docs and type constants.
- This is scaffolding only; production logic is intentionally marked TODO.
