# EdgeRun Standards — Architecture Overview

## What This Is

A **WebAssembly Text (.wat) standards library** for the EdgeRun decentralized edge computing platform. ~360 files, ~378K lines of WAT across 15 modules. All hand-written WAT (except `tor/` which is machine-generated from assembly). No external tooling — pure WAT, zero dependencies.

---

## Module Map

| Module | Files | Lines | Role |
|--------|------:|------:|------|
| **runtime/** | 2 | 767 | **Shared core** — memory, char LUTs, `pack` helpers, syscall constants, math utils |
| **compiler/** | 20 | 31,911 | WASM interpreter + JIT compiler (x86-64, AArch64, ARM32 backends) |
| **pipeline/** | 9 | 6,788 | **Pipeline framework** — stage dispatch, framing, mux, WASM exec |
| **protocol/** | 50 | 26,072 | Network protocol parsers/serializers (HTTP/1-3, TLS, DNS, WebSocket, QUIC, DHCP, HPACK, QPACK, DER/ASN.1) |
| **codec/** | 38 | 21,841 | Encoding/decoding (base64/64url/32hex, JSON, TOML, YAML, PEM, zlib/gzip, UTF-8, deflate) |
| **crypto/** | 19 | 7,658 | Cryptographic primitives (SHA-256/512, AES-* , HMAC, HKDF, X25519, ECDSA, Ed25519, RSA) |
| **app/** | 44 | 16,788 | Application-level semantics (OAuth, SSH, X.509, ACME, DKIM, wallets, OCI, CDP) |
| **tor/** | 24 | 190,536 | **Tor protocol** — machine-generated from assembly; self-contained crypto, HS, directory, relay |
| **ui/** | 92 | 56,678 | **UI framework** — component gallery, SVG icon pipeline, font system, rendering, layout, 50+ components |
| **system/** | 23 | 6,712 | System utilities — bump allocator, async state, event loop, FFI bridge, logging, compositor |
| **data/** | 15 | 6,205 | Data utilities — byte search, glob, HTML strip, UUID, string distance, terminal control |
| **device/** | 14 | 4,449 | Device abstractions — BLE, Bluetooth, TPM, virtio, unikernel, ESP32-S3 |
| **net/** | 7 | 1,610 | Networking — sockets, sessions, Bluetooth frame codec, mail protocol, block transfer |
| **tools/** | 1 | 38 | Hex encoder utility |
| **tests/** | 0 | 0 | Empty — no test infrastructure yet |

---

## Architecture & Conventions

### No WASM Imports
Every `.wat` file is a **self-contained standalone module** with its own `(memory ...)`. Despite existing in the same tree, files are NOT linked via WASM `(import)`. Inter-module relationships are *conceptual* — modules share memory layout conventions and function-calling conventions but run as independent WASM instances (or are composed via the pipeline framework).

The one exception: `runtime/edgerun-core.wat` is the *intended* shared core (memory + helpers) that other modules should import from — but currently files duplicate its constants inline rather than importing.

### Module Registration System
Every module exports a `proto_standard_id` function returning a unique integer ID (e.g., SHA-256 = `300075`, Tor full spec = `300208`). This is the module registry/versioning mechanism.

### i64 Packed Return Convention
Functions return `i64` where low 32 bits = status/error code, high 32 bits = value/byte count. Implemented via `pack` in `runtime/edgerun-core.wat`.

### Pipeline Architecture
`pipeline/pipe-core.wat` defines a funcref dispatch table (`stage_table`) where stages follow a uniform signature: `(input, output, config, clen, scratch, scap, state) -> result`. Stages can be composed arbitrarily.

### Two Code Styles
1. **Hand-written** (most modules): semantic names, comments, consistent formatting, `$meaningful_names`
2. **Generated** (`tor/`): machine-translated from x86_64 assembly — `$m_3_0`, `$t_3_0` naming, scratch global at `1048576` (0x100000)

---

## Duplication Status

| Issue | Status | Details |
|-------|--------|---------|
| compiler-x86 naming collision | ✅ Fixed | Deleted empty `compiler-x86-64.wat` |
| `interpreter-base.wat` stale copy | ✅ Fixed | Deleted `compiler/interpreter-base.wat` |
| SHA-256 (crypto vs pipeline) | ✅ Fixed | `pipeline/sha256-stage.wat` now imports from `crypto/crypto-sha256.wat` |
| HMAC-SHA-256 (crypto vs pipeline) | ✅ Fixed | `pipeline/hmac-sha256-stage.wat` now imports from `crypto/crypto-hmac-sha256.wat` |
| Inline helper duplication | ✅ Fixed | Created `runtime/math-utils.wat` with canonical `min`/`max`/`sat_sub`/`round_up`/`clamp`/`max0` |
| Dual Interpreter (compiler/ + pipeline/) | 🔄 Deferred | Both serve different purposes: `compiler/interpreter.wat` is standalone (+WAT lexer, +syscall init), `pipeline/wasm-interpreter.wat` is a fragment for the pipeline module. Need pipeline to import from compiler when linking infrastructure exists. |
| UI Prelude duplication | 🔄 Deferred | `00_prelude.wat` is a source fragment; `ui_framework.wat` is auto-generated output. Fix requires build pipeline change (`build_wat.mjs`). |
| Base64/Base64url shared core | 🔄 Deferred | Algorithms diverged structurally beyond just alphabet. Requires deeper refactor to extract shared core. |
| Tor vs Shared Crypto | 🔄 Deferred | Tor is machine-generated from assembly with `$m_3_0` naming. Would need manual re-port. |

### 1. Dual Interpreter (compiler/ + pipeline/)
- `compiler/interpreter.wat` — 6,163 lines, standalone WASM interpreter with WAT lexer + syscall init
- `pipeline/wasm-interpreter.wat` — 5,587 lines, pipeline fragment (omits WAT lexer, no syscall init loop)

**Status**: Both serve different roles (standalone module vs pipeline fragment). 95% identical. Merge requires pipeline to `(import)` from compiler once linking infrastructure exists.

### 2. ~~compiler-x86 Naming Collision~~ ✅ Fixed
- `compiler/compiler-x86-64.wat` — DELETED (was 0 bytes)
- `compiler/compiler-x86_64.wat` — 186,878 bytes (canonical)

### 3. ~~SHA-256 / HMAC-SHA-256~~ ✅ Fixed
- `pipeline/sha256-stage.wat` (28 lines) — now imports `sha256` from `crypto/crypto-sha256.wat`
- `pipeline/hmac-sha256-stage.wat` (33 lines) — now imports `hmac_sha256` from `crypto/crypto-hmac-sha256.wat`
- Saved ~1,260 lines of duplicated SHA-256 internals

### 4. Base64/Base64url (codec/)
- `codec/encoding-base64.wat` (724 lines) — standard alphabet `+/`, `=` padding
- `codec/encoding-base64url.wat` (508 lines) — URL-safe `-_`, no padding
- Encode/decode logic diverged structurally; alphabet mapping only difference

**Status**: Deferred. Needs `encoding-base64-core.wat` with parameterized alphabet + padding.

### 5. UI Prelude (ui/)
- `ui/00_prelude.wat` (971 lines) — source fragment with writer primitives + math + layout helpers
- `ui/ui_framework.wat` (28,339 lines) — auto-generated output that embeds prelude content
- Both export identical `(memory 64)`, data strings, and first 86 globals

**Status**: Deferred. `ui_framework.wat` says "Generated by build_wat.mjs". Fix is in the build tool, not the output file.

### 6. ~~Inline Helper Duplication~~ ✅ Fixed
- `runtime/math-utils.wat` — canonical `min`/`max`/`min_u`/`max_u`/`sat_sub`/`round_up`/`clamp`/`max0`/`min_f32`/`max_f32`/`clamp_f32`
- Consumers should `(import "math" "min" (func $min ...))` instead of redefining
- Stale duplicated definitions remain inline (standalone modules); migrate to imports when possible

### 7. Tor vs Shared Crypto
`tor/` directory contains its own AES, SHA-256, HMAC, X25519 implementations — independent copies of what's in `crypto/`. Tor is machine-generated from x86_64 assembly (naming: `$m_3_0`).

**Status**: Deferred. Adding imports would require manually re-porting the generated Tor code to use shared crypto, a significant undertaking.

### 8. Codec Inline Function Refactoring (Session 1 — 2026-06-07) ✅
Replaced 8 files' worth of duplicate inline utility functions in `codec/` with imports from shared runtime modules.

| File | Changes | Lines Saved |
|------|---------|:-----------:|
| `yaml-scan.wat` | `$m202byte` → `(import "edgerun" "load8_u")` | 2 |
| `json-emit.wat` | `$m123byte` → `(import "edgerun" "load8_u")` | 2 |
| `json-scalar.wat` | `$m124byte` → `(import "edgerun" "load8_u")` | 3 |
| `json-value-core.wat` | `$m126byte_at` → `(import "edgerun" "load8_u")` | 5 |
| `utf8-scan.wat` | `$m190byte` → `load8_u`, `$m190is_cont` → `is_cont` | 13 |
| `cesu8-mutf8.wat` | `$m41byte` → `load8_u`, `$m41is_cont` → `is_cont` | 13 |
| `toml-scan.wat` | `$m185byte` → `(import "edgerun" "load8_u")` | 2 |
| `model-decode.wat` | `$min2` → `(import "math" "min")`, `$max2` → `(import "math" "max")` | 2 |
| **Total** | 8 files, 11 inline defs replaced | **42** |

**Candidates intentionally NOT replaced** (different behavior from shared runtime):
- `$m202is_ws` (yaml-scan), `$m185is_ws` (toml-scan): Only check space/tab — narrower than edgerun LUT-based `is_ws`
- `$m125is_ws` (json-tape): Checks space/tab/CR/LF
- `$m39is_upper` (case-convert), `$m146is_upper` (pem): Range-check `c >= 65 && c <= 90` vs LUT-based
- `$m67copy` (deflate-stored): Parameter order `(src, dst, len)` reversed from `memcpy`'s `(dst, src, len)`
- `encoding-core.wat`: Fragment file (no `(module` wrapper); can't use imports

### 9. App Inline Function Refactoring (Session 2 — 2026-06-07) ✅
Replaced 17 files' worth of duplicate inline utility functions in `app/` with imports from shared runtime modules.

| File | Changes | Lines Saved |
|------|---------|:-----------:|
| `acme-core.wat` | `$m30eq` → `string_eq` | 11 |
| `apk-api-scan.wat` | `$m31ch` → `load8_u`, `$m31lower` → `to_lower` | 13 |
| `authority-storage-core.wat` | `$m28lower` → `to_lower` | 4 |
| `codelyzer-source-scan.wat` | `$m44ch` → `load8_u` | 5 |
| `codex-app-protocol-core.wat` | `$m45eq` → `string_eq` | 13 |
| `codex-patch-core.wat` | `$m46eq` → `string_eq` | 11 |
| `codex-shell-command-core.wat` | `$m47eq` → `string_eq` | 11 |
| `dkim-body.wat` | `$m76byte` → `load8_u` | 3 |
| `exchange-provider-status.wat` | `$m93ascii_lower` → `to_lower`, `$m93fnv_lower` → `fnv1a_lower` | 19 |
| `oauth-core.wat` | `$strlen_at` → `strlen` | 15 |
| `oci-config-core.wat` | `$m140ascii_lower` → `to_lower`, `$m140fnv_lower` → `fnv1a_lower` | 22 |
| `oci-reference.wat` | `$m142b` → `load8_u`, `$m142eq_lit` → `string_eq` | 14 |
| `oci-runtime-state.wat` | `$m143b` → `load8_u`, `$m143eq_lit` → `string_eq` | 14 |
| `sdk-standards-seed.wat` | `$eq_mem` → `string_eq` | 14 |
| `ssh-authorized-key.wat` | `$m169is_upper` → `is_upper`, `$m169is_lower` → `is_lower` | 12 |
| `ui-core-semantics.wat` | `$m188eq` → `string_eq` | 13 |
| **Total** | **16 files, 22 inline defs replaced** | **204** |

**Kept inline** (different signatures/behavior from shared):
- `browser-core.wat` — `$min64`/`$max64` use unsigned comparison (`i64.lt_u`/`i64.gt_u`) vs math-utils signed (`i64.lt_s`/`i64.gt_s`)
- `oauth-core.wat` — `$m137memcpy` param order `(src, dst, len)` vs shared memcpy `(dst, src, len)`
- `oauth-flow.wat` — `$m138memcpy` param order `(src, dst, len)` vs shared `(dst, src, len)`; `$m138memcpy_to_off` has 4 params vs shared `memcpy_off` has 5
- `sdk-standards-seed.wat` — `$m162copy` param order `(src, len, dst)` vs shared `(dst, src, len)`; `$m162is_lower` direct range check vs LUT-based
- `codex-*` files — `$m4?starts` (starts_with, not string_eq) stay inline
- `cdp-core.wat`, `url-scan.wat`, `browser-core.wat` — no matching patterns

**Candidates for next session** (missing imports for already-referenced functions in app/):
- Several app files call `$pack` without definition/import
- Several app files call `$is_digit` without definition/import
- Several app files call `$is_hex` without definition/import

**Sessions remaining** (in priority order):
1. `net/` module — scan for inline duplicates
2. `device/` module — scan for inline duplicates
3. `data/` module — scan for inline duplicates
4. `system/` module — scan for inline duplicates
5. `protocol/` module (50 files) — scan for inline duplicates
6. `compiler/` module — scan for inline duplicates
7. `ui/` module (92 files) — scan for inline duplicates (low priority; prelude is deferred)

---

## Common Refactoring Patterns

**Before:**
```wat
(func $sha256_transform (param $state i32) (param $block i32)
  ;; 300 lines of SHA-256 core
)
```

**After:**
```wat
;; crypto/sha256-core.wat
(func (export "sha256_transform") (param $state i32) (param $block i32) ...)

;; pipeline/sha256-stage.wat
(import "sha256" "transform" (func $sha256_transform (param i32 i32)))
```

**Before:**
```wat
(func $min (param $a i32) (param $b i32) (result i32)
  (if (i32.lt_s (local.get $a) (local.get $b))
    (then (return (local.get $a)))
    (else (return (local.get $b)))
  )
)
;; ...same function 15 lines later in another file
```

**After:**
```wat
;; runtime/math-utils.wat
(func (export "min") (param $a i32) (param $b i32) (result i32) ...)

;; consumer.wat
(import "math" "min" (func $min (param i32 i32) (result i32)))
```

---

## Code Style Guide

- No comments in new code (existing files may have them)
- Use semantic `$snake_case` names (not generated `$m_3_0` style)
- Export everything from shared modules; import explicitly in consumers
- i64 packed returns for status+value
- Expose `proto_standard_id` in every top-level module
- `(memory (export "memory") N)` — all modules export memory under the same name

---

## Build & Test

Currently no build system or test infrastructure. All files are raw `.wat` — no `wat2wasm`, no runner, no test framework. This is a WIP standards library (all commits are "wip: checkpoint").
