# EdgeRun WASM AOT

`edgerun-aot` is the first baseline ahead-of-time compiler path for EdgeRun WASM.

The design goal is not peak speed yet. The design goal is an auditable compiler boundary that EdgeRun can own.

## Invariant

The original `.wasm` bundle remains the source of truth.

The `.eraot` file is an optimization artifact. A runtime must be able to reject it and fall back to validated WASM execution or interpreter verification.

## Current artifact

`edgerun-aot` writes a deterministic `.eraot` file containing:

- magic: `ERAOT001`
- artifact version: `2`
- target triple string
- compiler version string
- SHA-256 of the input WASM
- compiled functions
- compact function signature string
- rendered EdgeRun IR for audit/debugging
- baseline x86_64 SysV machine-code bytes

The artifact can now be decoded, inspected, verified against the original WASM input hash, and executed for simple integer functions.

## Current subset

The first compiler only accepts the small deterministic integer subset below:

- `i32.const`
- `i64.const`
- `local.get`
- `local.set`
- `local.tee`
- `i32.add`
- `i32.sub`
- `i32.mul`
- `i64.add`
- `i64.sub`
- `i64.mul`
- `return`
- `end`

The x86_64 backend currently emits a simple stack-frame function:

- parameters are copied from SysV argument registers into frame slots
- non-parameter locals are zero-initialized
- WASM operand stack values are represented with native push/pop operations
- `i32` values are zero-extended and arithmetic is emitted with 32-bit x86 instructions to preserve wrapping/truncation semantics
- the function epilogue restores `rsp`/`rbp` before returning

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

Compile:

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

Inspect an artifact:

```bash
cargo run -p edgerun-wasm --bin edgerun-aot -- --inspect-artifact app.eraot
```

Verify an artifact against the source WASM:

```bash
cargo run -p edgerun-wasm --bin edgerun-aot -- app.wasm --verify-artifact app.eraot --verbose
```

Run a verified artifact function by export name:

```bash
cargo run -p edgerun-wasm --bin edgerun-aot -- app.wasm --run-artifact app.eraot --function add --arg 40 --arg 2 --verbose
```

Run a verified artifact function by function index:

```bash
cargo run -p edgerun-wasm --bin edgerun-aot -- app.wasm --run-artifact app.eraot --function 0 --arg 40 --arg 2
```

Arguments are passed as unsigned 64-bit integer slots. Decimal, negative signed decimal, and `0x` hex forms are accepted. The loader masks `i32` return values to 32 bits.

## Current execution loader

The loader is intentionally narrow:

- decodes `.eraot`
- verifies the artifact hash against the original `.wasm`
- checks target and compiler strings
- selects a function by export name or function index
- parses the compact integer signature
- maps the function code into executable memory
- calls it through a fixed six-argument C ABI shim

This is a local developer harness, not the final production execution sandbox.

## Next compiler steps

1. Replace RWX mapping with write-then-exec mapping or memfd-backed executable pages.
2. Add deterministic hostcall ABI lowering.
3. Add memory load/store with explicit bounds traps.
4. Add block/loop/br/br_if lowering.
5. Add direct/indirect calls with strict signature checks.
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
