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
make check
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

## Notes

- Frozen protocol choices (BLAKE3, Ed25519, committee=3 quorum=2, SOL) are captured in docs and type constants.
- This is scaffolding only; production logic is intentionally marked TODO.
