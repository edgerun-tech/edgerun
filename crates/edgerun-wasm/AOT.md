# EdgeRun WASM AOT

`edgerun-aot` is the first baseline ahead-of-time compiler path for EdgeRun WASM.

The design goal is not peak speed yet. The design goal is an auditable compiler boundary that EdgeRun can own.

## Invariant

The original `.wasm` bundle remains the source of truth.

The `.eraot` file is an optimization artifact. A runtime must be able to reject it and fall back to validated WASM execution or interpreter verification.

## Current artifact

`edgerun-aot` writes a deterministic `.eraot` file containing:

- magic: `ERAOT001`
- artifact version
- target triple string
- compiler version string
- SHA-256 of the input WASM
- compiled functions
- rendered EdgeRun IR for audit/debugging
- baseline x86_64 SysV machine-code bytes

## Current subset

The first compiler only accepts the small deterministic integer subset below:

- `i32.const`
- `i64.const`
- `local.get`
- `i32.add`
- `i32.sub`
- `i32.mul`
- `i64.add`
- `i64.sub`
- `i64.mul`
- `return`
- `end`

Unsupported on purpose for now:

- floats
- SIMD
- threads
- GC/reference execution
- WASI
- memory load/store
- calls/hostcalls
- indirect calls
- branches/loops
- multi-value returns

## Usage

```bash
cargo run -p edgerun-wasm --bin edgerun-aot -- app.wasm --emit-ir --verbose
```

Output defaults to:

```text
app.eraot
```

Explicit output:

```bash
cargo run -p edgerun-wasm --bin edgerun-aot -- app.wasm -o app.eraot
```

## Next compiler steps

1. Add a loader that mmaps/verifies `.eraot` artifacts and checks the embedded WASM hash.
2. Add deterministic hostcall ABI lowering.
3. Add `local.set` and `local.tee`.
4. Add memory load/store with explicit bounds traps.
5. Add block/loop/br/br_if lowering.
6. Add aarch64 backend.
7. Replace SHA-256 with the repo-wide EdgeRun crypto boundary once this path is wired into runtime artifacts.

## Trust model

AOT output must never become the only proof of execution.

Execution receipts should record:

- WASM bundle hash
- compiler version
- target architecture
- artifact hash
- input hash
- output hash
- runtime/hostcall ABI version

That keeps compiler bugs recoverable instead of making them consensus bugs.
