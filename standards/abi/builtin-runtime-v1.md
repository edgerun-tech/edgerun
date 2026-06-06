# Built-in Runtime Module ABI v1

This ABI extends the Standard Module ABI v1 (`abi/standard-module-v1.md`) with
conventions for modules designed as built-in runtime components — importable
by downstream WASM applications, optimized for performance, and optionally
accelerated with SIMD.

## Status

**Draft** — v1 not yet finalized. Live document.

## Required Exports (inherited)

Every built-in runtime module exports:

```
proto_abi_version() -> i32
proto_standard_id() -> i32
```

`proto_abi_version` is `2` for this model.
`proto_standard_id` is the stable numeric identifier from the standards catalog
(codec-primitives use 300xxx range).

## Return Conventions

Three return encoding schemes are used depending on the operation type:

### i64 Packed Return

Functions that produce both a status and a value return `i64` with the value
in the high 32 bits and the status code in the low 32 bits:

```wat
(func $pack (param $status i32) (param $value i32) (result i64)
  (i64.or
    (i64.extend_i32_u (local.get $status))
    (i64.shl (i64.extend_i32_u (local.get $value)) (i64.const 32))))
```

Caller extracts:
```wat
(local $result i64)
(local.get $result) (i32.wrap_i64)              ;; -> status
(local.get $result) (i64.shr_u (i64.const 32)) (i32.wrap_i64)  ;; -> value
```

### i32 Status-Only Return

Functions that only need to signal success/failure return `i32` status directly.

### i32 Scalar Return

Functions that compute a simple hash or metric (CRC32, Adler32, status flags)
return `i32` directly — the value *is* the result, not a status.

### Shared Status Codes

| Code | Name         | Meaning                          |
|------|--------------|----------------------------------|
| 0    | OK           | Success                          |
| 1    | INPUT_SHORT  | Input buffer too short           |
| 2    | OUTPUT_SHORT | Output buffer too short          |
| 3    | INVALID      | Invalid input data               |
| 4    | OVERFLOW     | Numeric overflow                 |
| 5    | TRUNCATED    | Input unexpectedly truncated     |
| 6    | TOO_LONG     | Exceeds maximum allowed length   |
| 7+   | MODULE       | Module-defined (documented)      |

## Memory Model

### Owned Memory

Each module declares its own linear memory and exports it:

```wat
(memory (export "memory") 1)
```

Modules must **not** import another module's memory. Value transfer between
modules is performed by copying bytes into the target module's memory, calling
its API, and copying results out. No pointer sharing across module boundaries.

### Scratch Space

Modules may reserve a fixed scratch region in their memory for internal use.
The scratch offset must be at least 4 KiB from the base:

```wat
;; In memory init block or first call:
(i32.store (i32.const 0) (i32.const 4096))  ;; scratch base at page 1
```

Callers must not rely on data at or above the scratch offset persisting across
calls.

### Memory Growth

Memory may be grown with `memory.grow` **once** during initialization to a
known fixed size. Grow-on-demand per invocation is discouraged for performance
reasons. Pass the required size as a parameter when composing.

## SIMD Extension Pattern

Modules may provide SIMD-accelerated variants alongside their scalar exports.
Follow these rules for discoverability and compatibility.

### Naming Convention

SIMD variants use the export name suffix `_simd`:

| Scalar export          | SIMD variant               |
|------------------------|----------------------------|
| `http_find_crlf`       | `http_find_crlf_simd`      |
| `crc32`                | `crc32_simd`               |
| `utf8_validate`        | `utf8_validate_simd`       |

### Capability Discovery

If a module exports any SIMD function, it must also export:

```
simd_capabilities() -> i32
```

Returns a bitmask of available SIMD capabilities:

| Bit | Feature      |
|-----|--------------|
| 0   | v128 SIMD    |
| 1   | relaxed SIMD |
| 2   | half-precision |

A caller checks capabilities at composition time and selects the appropriate
export:

```wat
(if (i32.and (call $http_scan.simd_capabilities) (i32.const 1))
  (then (call $http_scan.http_find_crlf_simd ...))
  (else (call $http_scan.http_find_crlf ...)))
```

### SIMD Contract

A SIMD variant must:

1. Accept identical parameters as the scalar variant
2. Return identical results (same status codes, same output bytes)
3. Be strictly faster than the scalar variant for all input sizes ≥ 64 bytes
4. Degrade gracefully (or fall back to scalar) for inputs < 16 bytes

### Implementation Pattern

```wat
(func (export "crc32_simd") (param $ptr i32) (param $len i32) (result i32)
  ;; Process 16 bytes at a time with v128
  (local $v0 v128) (local $v1 v128)
  (local $i i32) (local $result i32)
  ...
  (loop $simd_loop
    ;; 16-byte vector load & CRC accumulate
    (local.set $v0 (v128.load (i32.add (local.get $ptr) (local.get $i))))
    ...
    (br_if $simd_loop
      (i32.ge_u (local.tee $i (i32.add (local.get $i) (i32.const 16)))
                (local.get $len))))
  ;; scalar tail for remaining < 16 bytes
  ...)
```

## Linking Conventions

### Module Identity

Each module is identified by the tuple `(proto_standard_id, proto_abi_version)`.
The combination must be globally unique within a composition.

### Import Schema

When module A needs to call module B, A declares B as an import:

```wat
(module
  (import "b" "memory" (memory 1))
  (import "b" "proto_abi_version" (func $b_ver (result i32)))
  (import "b" "proto_standard_id" (func $b_id (result i32)))
  (import "b" "http_find_crlf" (func $b_find_crlf
    (param i32 i32 i32) (result i64)))
  ...)
```

### Version Negotiation

At composition init, the caller should verify ABI compatibility:

```wat
(func $check_abi
  (if (i32.ne (call $b_ver) (i32.const 2))
    (then unreachable))
  (if (i32.ne (call $b_id) (i32.const 300003))
    (then unreachable)))
```

## WAT Style Guide

### Recommended Form

All WAT should use **unfolded (plain) notation**:

```wat
;; Good — unfolded
(local.get $ptr)
(local.get $i)
(i32.add)
(i32.load8_u)
(local.set $b)

;; Avoid — folded/nested
(local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
```

Rationale: unfolded is easier to read in diffs, easier to generate mechanically,
and avoids deeply nested parentheses that obscure logic.

### Label Conventions

- Loop labels: `$scan`, `$again`, `$bits` (descriptive)
- Block labels: `$done`, `$found`, `$not_found` (describe the exit condition)
- No numeric labels

### Local Naming

- `$ptr`, `$len`, `$offset` for buffer parameters
- `$i`, `$j` for loop indices
- `$b` for a single byte
- `$status`, `$result` for return values

### Helper Functions

- `$pack`: only when needed (see "Inlining" below)
- `$has`: bounds check helper if used 3+ times
- `$is_*`: character class predicates (e.g., `$is_digit`, `$is_tchar`)

### Inlining

The `$pack` helper should be inlined at each call site in performance-critical
paths. The 13 word overhead per call (call + return + param setup) is measurable
in Liftoff-tier compilation. Reserve `$pack` as a named function only for
seldom-called paths.

For hot character-class predicate helpers (`$is_tchar`, `$is_digit`, etc.),
prefer a 256-byte lookup table in linear memory over a chain of `i32.or` or
nested `if` expressions. Initialize the table once per module lifetime.

## Module Manifest

Every built-in runtime module directory must contain a `MODULE.md` with:

- `proto_standard_id`
- Standard implemented (RFC, spec, algorithm)
- Exported functions with signatures and status code meanings
- SIMD capabilities (if any)
- Memory requirements (initial + max pages)
- Scratch space layout (if any)
- Dependencies (imported modules)

## Test Requirements

- 100% of exported functions must have at least one test
- Each non-zero status code must have a test
- SIMD variants must match scalar output for 100+ random inputs
- Tests run with `node` and `wat2wasm` — no external test framework
