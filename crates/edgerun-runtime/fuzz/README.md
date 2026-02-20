# edgerun-runtime fuzzing

This directory contains `cargo-fuzz` targets for the runtime's highest-risk surfaces:

- `fuzz_bundle_decode`: canonical bundle decoder robustness
- `fuzz_validate_wasm`: wasm validator robustness
- `fuzz_hostcall_boundary`: hostcall pointer/length boundary behavior

## Setup

```bash
cargo install cargo-fuzz
```

## Run examples

```bash
cd crates/edgerun-runtime/fuzz
cargo fuzz run fuzz_bundle_decode
cargo fuzz run fuzz_validate_wasm
cargo fuzz run fuzz_hostcall_boundary
```

## Notes

- Fuzz outputs are written under `artifacts/` and corpora under `corpus/`.
- Keep crashing inputs and add them to regression tests in `crates/edgerun-runtime/src/lib.rs`.
