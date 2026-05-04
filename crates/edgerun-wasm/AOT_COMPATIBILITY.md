# EdgeRun WASM AOT compatibility backlog

This file tracks the baseline x86_64 AOT compiler compatibility work so missing WASM operators can be implemented deliberately instead of as one-off surprises.

## Implemented baseline groups

- constants: `i32.const`, `i64.const`
- locals: `local.get`, `local.set`, `local.tee`
- globals: local module globals with const initializers, `global.get`, `global.set`
- integer arithmetic: `i32.add/sub/mul`, `i64.add/sub/mul`
- comparisons: basic signed/unsigned `i32`/`i64` comparisons and `eqz`
- structured markers: empty `block`, empty `loop`, `end`
- branches: depth-0 `br`, depth-0 `br_if`
- select: untyped `select`
- bulk memory: `memory.copy`, `memory.fill`
- linear memory: `i32.load`, `i64.load`, `i32.store`, `i64.store`
- byte memory: `i32.load8_u`, `i32.load8_s`, `i32.store8`
- direct internal calls: lowered and inlined by module backend

## Immediate next operators

These are the next likely blockers for Rust/C/AssemblyScript generated WASM:

- `i32.load16_u`
- `i32.load16_s`
- `i32.store16`
- `i64.load8_u`
- `i64.load8_s`
- `i64.load16_u`
- `i64.load16_s`
- `i64.load32_u`
- `i64.load32_s`
- `i64.store8`
- `i64.store16`
- `i64.store32`

## Integer ops still needed

- `i32.div_s`, `i32.div_u`, `i32.rem_s`, `i32.rem_u`
- `i64.div_s`, `i64.div_u`, `i64.rem_s`, `i64.rem_u`
- `i32.and`, `i32.or`, `i32.xor`
- `i64.and`, `i64.or`, `i64.xor`
- `i32.shl`, `i32.shr_s`, `i32.shr_u`, `i32.rotl`, `i32.rotr`
- `i64.shl`, `i64.shr_s`, `i64.shr_u`, `i64.rotl`, `i64.rotr`
- `i32.clz`, `i32.ctz`, `i32.popcnt`
- `i64.clz`, `i64.ctz`, `i64.popcnt`

## Control flow still needed

- `if`
- `else`
- branch depth greater than zero
- block result types
- loop result types
- explicit returns from non-inlined/top-level paths with stack cleanup

## Calls still needed

Current direct internal calls are handled by inlining in the module backend. This is useful for simple internal calls but is not the final design.

Still needed:

- real module-level call patching using `call rel32`
- imported function / hostcall ABI
- `call_indirect`
- recursion support or explicit recursion rejection at validation time

## Runtime memory ABI still needed

The backend currently assumes linear memory base in `r10` for memory operations. Artifact execution needs a real memory runtime ABI before memory-using functions can be safely run.

Needed:

- memory base pointer setup
- memory size metadata
- bounds checks / traps
- memory growth semantics, if supported

## Preferred implementation order

1. Complete narrow memory ops.
2. Add integer bitwise/shift ops.
3. Add division/remainder with traps.
4. Add `if`/`else` and branch depth support.
5. Replace call inlining with module-level `call rel32` patching.
6. Add imported hostcall ABI.
7. Add memory runtime ABI and bounds checks.
