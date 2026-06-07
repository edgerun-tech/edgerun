# EdgeRun Standards — Architecture Overview

## What This Is

A **WebAssembly Text (.wat) standards library** for the EdgeRun decentralized edge computing platform. ~336 files, ~188K lines of WAT across 15 modules. All hand-written WAT. No external tooling — pure WAT, zero dependencies.

---

## Module Map

| Module | Files | Lines | Role |
|--------|------:|------:|------|
| **runtime/** | 2 | 767 | **Shared core** — memory, char LUTs, `pack` helpers, syscall constants, math utils |
| **compiler/** | 17 | 25,779 | WASM interpreter + JIT compiler (x86-64, AArch64, ARM32 backends) |
| **pipeline/** | 9 | 6,788 | **Pipeline framework** — stage dispatch, framing, mux, WASM exec |
| **protocol/** | 13 | 25,190 | Network protocol parsers/serializers (HTTP/1-3, TLS, DNS, WebSocket, QUIC, DHCP, HPACK, QPACK, DER/ASN.1) — merged from 55 fragments |
| **codec/** | 9 | 22,352 | Encoding/decoding (base64/64url/32hex, JSON, TOML, YAML, PEM, zlib/gzip, UTF-8, deflate) — merged from 38 fragments |
| **crypto/** | 19 | 7,658 | Cryptographic primitives (SHA-256/512, AES-* , HMAC, HKDF, X25519, ECDSA, Ed25519, RSA) |
| **app/** | 12 | 17,037 | Application-level semantics (OAuth, SSH, X.509, ACME, DKIM, wallets, OCI, CDP) — merged from 44 fragments |
| **tor/** | 1 | 190,050 | **Tor protocol** — fully consolidated single module (hand-written + 16 machine-generated subsections). **DELETED** — depends on host imports not in repo, impractical standalone. |
| **ui/** | 92 | 56,678 | **UI framework** — component gallery, SVG icon pipeline, font system, rendering, layout, 50+ components |
| **system/** | 23 | 6,712 | System utilities — bump allocator, async state, event loop, FFI bridge, logging, compositor |
| **data/** | 15 | 5,354 | Data utilities — byte search, glob, HTML strip, UUID, string distance, terminal control |
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
3. **Consolidated** (`tor/tor.wat`): 8 hand-written fragments merged into one module with rebased memory layout. Uses `$m11`, `$m16`, `$m23` etc. prefixes from original files.

---

## Duplication Status

| Issue | Status | Details |
|-------|--------|---------|
| compiler-x86 naming collision | ✅ Fixed | Deleted empty `compiler-x86-64.wat` |
| `interpreter-base.wat` stale copy | ✅ Fixed | Deleted `compiler/interpreter-base.wat` |
| SHA-256 (crypto vs pipeline) | ✅ Fixed | `pipeline/sha256-stage.wat` now imports from `crypto/crypto-sha256.wat` |
| HMAC-SHA-256 (crypto vs pipeline) | ✅ Fixed | `pipeline/hmac-sha256-stage.wat` now imports from `crypto/crypto-hmac-sha256.wat` |
| Inline helper duplication | ✅ Fixed | Created `runtime/math-utils.wat` with canonical `min`/`max`/`sat_sub`/`round_up`/`clamp`/`max0` |
| Dual Interpreter (compiler/ + pipeline/) | ✅ Fixed | Created `compiler/interpreter-core.wat` (shared fragment), deleted 3 stale sub-files, trimmed wrappers to <600 lines each. 99% dedup. |
| UI Prelude duplication | 🔄 Deferred | `00_prelude.wat` is a source fragment; `ui_framework.wat` is auto-generated output. Fix requires build pipeline change (`build_wat.mjs`). |
| Base64/Base64url shared core | 🔄 Deferred | Algorithms diverged structurally beyond just alphabet. Requires deeper refactor to extract shared core. |
| Tor 8-Fragment Consolidation | ✅ Fixed | 8 hand-written fragments merged into `tor/tor.wat` (2,336 lines, parse+validate OK) |

### 1. ~~Dual Interpreter (compiler/ + pipeline/)~~ ✅ Fixed
- Created `compiler/interpreter-core.wat` (5,609 lines) — shared fragment containing all decode/execute/call/load logic
- `compiler/interpreter.wat` trimmed 6,163→578 lines (WAT lexer + syscall init wrapper)
- `pipeline/wasm-interpreter.wat` trimmed 5,587→5 lines (import + core reference)
- Deleted 3 unreferenced sub-files: `interpreter-decode.wat` (1,025), `interpreter-exec.wat` (3,856), `interpreter-opcodes.wat` (544)
- **Total savings: ~11,000 lines** (99% dedup — only ~42 lines differ across 11,750 total)

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

### 7. Tor 8-Fragment Consolidation (Session final — 2026-06-07) ✅
Consolidated 8 hand-written Tor module fragments into a single `tor/tor.wat` with verified parse + validate.

| File | Lines | Role |
|------|------:|------|
| `tor/crypto.wat` | 227 | SHA-256, HMAC, AES wrappers → merged into `tor.wat` |
| `tor/protocol.wat` | 695 | Cell codec, link protocol, relay addressing → merged |
| `tor/tor-library.wat` | 517 | memcpy/memset wrappers, b64 encode, nibble helpers → merged |
| `tor/tor-relay-full.wat` | 462 | Circuit management, relay crypto, extend/create2 → merged |
| `tor/tor-hidden-service-full.wat` | 208 | HS descriptor, intro point, rendezvous → merged |
| `tor/tor-directory-authority.wat` | 276 | Descriptor store, consensus builder, route lookup → merged |
| `tor/tor-hs-intro-relay.wat` | 213 | ESTABLISH_INTRO parsing, INTRODUCE2 validation → merged |
| `tor/tor-full-spec.wat` | 69 | Spec helpers (proto_standard_id, simd_capabilities) → merged |
| **Total deleted** | **2,667** | **8 files eliminated** |
| **`tor/tor.wat`** | **2,336** | **Consolidated module — parse + validate OK** |

**Key changes during consolidation:**
- Rebased all memory addresses to non-overlapping layout (17 pages, ~1.1MB)
- Eliminated duplicate functions (`$desc_ptr`, `$m7mem_eq`, `$is_nick_char`, `$valid_nickname`, `$find_identity`, `$alloc_desc`)
- Removed duplicate globals (`$m16STANDARD_ID`, `$m16ABI_VERSION`, `$ERR_AUTH`)
- Single `proto_standard_id` (ID 300220) and `simd_capabilities` export
- Imports from `crypto/*.wat` (sha256, hmac_sha256, aes128_ctr_xor) and `runtime/edgerun-core.wat` (memcpy, memset)
- Host relay functions imported from `"host"` module
- Fixed 12+ validation errors (dangling call returns, blank `drop`, missing return keywords)

**16 machine-generated files (~187K lines) kept as independent modules** — not touched.

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

### 10. Protocol Inline Function Refactoring (Session 10 — 2026-06-07) ✅
Replaced all 8 inline `to_lower` definitions in `protocol/` with imports from `runtime/edgerun-core.wat`.

| File | Inline Def | → Import | Lines Saved |
|------|-----------|----------|:-----------:|
| `dns-compressed-name.wat` | `$m77lower_ascii` | `to_lower` | 6 |
| `dns-name.wat` | `$m80lower_ascii` | `to_lower` | 6 |
| `tls-core-state.wat` | `$m180ascii_lower` | `to_lower` | 6 |
| `tls-name.wat` | `$m183lower_ascii` | `to_lower` | 6 |
| `http1-body.wat` | `$m111lower` | `to_lower` | 14 |
| `http1-header-block.wat` | `$m113lower` | `to_lower` | 14 |
| `http1-scan.wat` | `$m115lower` | `to_lower` | 14 |
| `http-node-state.wat` | `$m109lower` | `to_lower` | 6 |
| **Total** | 8 inline defs replaced | | **72** |

**Kept inline** (different behavior/signature from shared):
- `packet-core.wat` — `$memset` param order `(base, len, val)` vs shared `(ptr, val, len)`
- `http1-body.wat` — `$m111is_space` only checks space/tab (narrower than shared `is_ws`)
- `http1-header-block.wat` — `$m113is_space` only checks space/tab
- `http1-lines.wat` — `$m114is_space` only checks space/tab
- `http1-scan.wat` — `$m115is_space` only checks space/tab
- `tls-name.wat` — `$m183is_space` checks space + range [9,13] (HTTP whitespace)
- `http-date.wat` — `$match` is offset-based prefix match, not `string_eq`
- `tls-name.wat` — `$normalized_match` is case-sensitive full equality (unique)
- `http1-scan.wat` — `$token_match` is case-insensitive equality (unique)
- `tls-core-state.wat` — `$m180fnv_lower` is the only FNV-1a in protocol/ (kept as-is)

### 11. HTTP/DNS Family Core Refactoring (Session 11 — 2026-06-07) ✅
Created protocol family shared core files, refactored 9 files across HTTP and DNS families.

| File | Changes | Lines Saved |
|------|---------|:-----------:|
| `protocol/http-core.wat` (new) | Shared `$is_tchar`, `$is_space`, `$is_header_value_byte`, `$ascii_eq_ci` | — |
| `http1-body.wat` | `$m111is_tchar` → `http-core`, `$m111is_header_value_byte` → `http-core` | 144 |
| `http1-header-block.wat` | `$m113is_tchar` → `http-core`, `$m113is_header_value_byte` → `http-core` | 144 |
| `http1-lines.wat` | `$m114is_space` → `http-core`, `$m114is_tchar` → `http-core` | 105 |
| `http1-scan.wat` | `$m115is_space` → `http-core` (kept `$m115is_tchar` inline, non-standard) | 5 |
| `http-node-state.wat` | `$m109is_tchar` + 3 helpers → `http-core` | 21 |
| `protocol/dns-core.wat` (new) | Shared `$is_label_byte` | — |
| `dns-compressed-name.wat` | `$m77is_label_byte` → `dns-core` | 8 |
| `dns-core-records.wat` | `$m78is_label_byte` → `dns-core` | 10 |
| `dns-name.wat` | `$m80is_label_byte` → `dns-core` | 8 |
| `dns-rdata-core.wat` | `$m81is_label_byte` → `dns-core` | 8 |
| **Total** | **9 protocol files + 2 new cores** | **453** |

**Notable**: `http1-scan.wat` retains its extended `$m115is_tchar` inline (includes `{` and `}` for non-standard HTTP token matching).

### 12. Cross-Family Binary Core + HPACK/QPACK Huffman (Session 11 — 2026-06-07) ✅
Created `protocol/binary-core.wat` (shared `read_u16_be`, `read_u24_be`, `read_u32_be`, `write_u16_be`, `write_u32_be`). Created `protocol/hpack-qpack-core.wat` (shared Huffman tables + decode functions + prefix helpers).

| File | Changes | Lines Saved |
|------|---------|:-----------:|
| `protocol/binary-core.wat` (new) | Shared big-endian read/write helpers | — |
| `protocol/hpack-qpack-core.wat` (new) | Shared Huffman tables + lookup + decode + prefix helpers | — |
| TLS (4 files) | `read_u16_be`/`read_u24_be` → `binary-core` | ~32 |
| DNS (4 files) | `read_u16_be`/`read_u32_be`/`write_u16_be` → `binary-core` | ~32 |
| DHCP (1 file) | `read_u16_be`/`read_u32_be` → `binary-core` | ~8 |
| HPACK/QPACK (7 files) | Huffman tables + lookup + decode + prefix helpers → `hpack-qpack-core` | ~210 |
| **Total** | **18 files refactored** | **~280** |

### 13. HPACK/QPACK Record Helpers (Session 11 — 2026-06-07) ✅
Extracted 4 identical function pairs (`clear_record`, `write_prefix_record`, `copy_name_meta`, `copy_value_meta`) from `hpack-header-block.wat` and `qpack-encoder-stream.wat` into `hpack-qpack-core.wat`.

| File | Change | Lines Saved |
|------|--------|:-----------:|
| `hpack-header-block.wat` | 4 inline defs → imports from `hpack-qpack-core` | 40 |
| `qpack-encoder-stream.wat` | 4 inline defs → imports from `hpack-qpack-core` | 40 |
| `hpack-qpack-core.wat` | +4 exported functions | +50 |
| **Total** | | **30 net** |

### 14. QUIC Varint Decode (Session 11 — 2026-06-07) ✅
Extracted `$quic_varint_decode_at` (identical in `quic-core-state.wat` and `http3-frame.wat`) into new `protocol/quic-core.wat`.

| File | Change | Lines Saved |
|------|--------|:-----------:|
| `quic-core-state.wat` | Inline def → import + re-export wrapper | 30 |
| `http3-frame.wat` | Inline def → import (also gained `$pack` import) | 35 |
| `protocol/quic-core.wat` (new) | New shared core module | +47 |
| **Total** | | **18 net** |

### 15. WebSocket Opcode Validation (Session 11 — 2026-06-07) ✅
Extracted `$is_valid_opcode` from triplicate inline pattern in `ws-frame.wat` (3 identical opcode checks in `ws_decode_prefix`, `ws_write_frame_header`, `ws_write_server_frame_header`).

| File | Change | Lines Saved |
|------|--------|:-----------:|
| `ws-frame.wat` | 3 inline checks → 1 function + 3 calls | 48 |
| **Total** | | **48** |

**Session 11 total**: ~842 lines saved across 30+ protocol files (HTTP, DNS, TLS, DHCP, HPACK/QPACK, QUIC, WebSocket)

### 16. Tor 16-Fragment Machine-Gen Consolidation (Session 17 — 2026-06-07) ✅
Merged all 16 machine-generated Tor WAT fragments (~187K lines) into `tor/tor.wat`, rebasing their overlapping scratch address `1048576` to per-file unique 64KB-aligned bases.

| File | Lines | Base |
|------|------:|:----|
| `tor-cell-codec.wat` | 1,488 | 0x100000 |
| `tor-cgo-crypto.wat` | 44,172 | 0x110000 |
| `tor-channel-handshake.wat` | 5,282 | 0x120000 |
| `tor-device.wat` | 6,310 | 0x130000 |
| `tor-directory-rsa.wat` | 11,454 | 0x140000 |
| `tor-directory-vote.wat` | 11,388 | 0x150000 |
| `tor-hs-descriptor-crypto.wat` | 3,178 | 0x160000 |
| `tor-hs-identity.wat` | 694 | 0x170000 |
| `tor-hs-intro-auth.wat` | 8,065 | 0x180000 |
| `tor-hs-intro-extensions.wat` | 217 | 0x190000 |
| `tor-hs-ntor.wat` | 21,174 | 0x1a0000 |
| `tor-hs-pow-v1.wat` | 24,340 | 0x1b0000 |
| `tor-identity-seal.wat` | 16,816 | 0x1d0000 |
| `tor-ntor-crypto.wat` | 5,965 | 0x1e0000 |
| `tor-ntor-v3-crypto.wat` | 8,657 | 0x1f0000 |
| `tor-relay-crypto.wat` | 17,870 | 0x200000 |
| **Total** | **~187K** | **33 memory pages (2,162,688 bytes)** |

**Key changes:**
- Rebased all 500+ data address references from shared `1048576` to per-file unique bases up to `0x200000` (2,097,152)
- Filtered out SHA-256 constants, Rust discriminants, and other non-address integers
- Stripped duplicate `$.rodata` data segment identifiers (16× → 0)
- Stripped 15 of 16 duplicate `(table 1 1 funcref)` declarations
- Increased memory from 17→33 pages (1,114,112→2,162,688 bytes)
- `wasm-tools parse` + `wasm-tools validate` pass
- Deleted 16 individual files
- **Deleted entire tor.wat** — depends on 6 host imports not in repo, impractical standalone (Session end — 2026-06-07)

**Sessions completed** (current):
1. ~~`compiler/` module — scan for inline duplicates~~ ✅ Done — no runtime-utility duplication found; critical bug documented (481+ mangled aarch64 call names)
2. ~~`data/` module — scan for 4 inline duplicates~~ ✅ Fixed — 4 data/ files refactored to use runtime scope: `$load8_u`, `$memcpy`, `$string_eq`, `$starts_with`
3. ~~`$emit_aarch64_baarch64_*` naming bug~~ ✅ Fixed — 488 mangled call names corrected (481 in `compiler-aarch64.wat`, 7 in `simd-aarch64.wat`). Renamed `$emit_aarch64_baarch64_<rest>` → `$emit_aarch64_<rest>` to match definitions in `emit-aarch64.wat`.
4. ~~`compiler/interpreter-wat.wat` consolidation~~ ✅ Fixed — 9 functions (`$wat_emit_byte`, `$wat_emit_leb_u32`, `$wat_emit_leb_i32`, `$wat_kw_match_rest`, `$wat_parse_body`, `$wat_parse_type_decl`, `$wat_parse_func_decl`, `$wat_parse_module`, `$load_wat`) appended to `interpreter.wat`; `interpreter-wat.wat` deleted.
5. ~~**Friction cleanup** (Session — 2026-06-07)~~ ✅ Fixed:
   - Removed 36 stale/orphan `.wasm` files from git tracking (14 orphaned with no source, 20 stale vs `.wat` sources)
   - Removed `edgerun.wat` (110K lines) and `edgerun-full.wat` (23K lines) build outputs from git
   - Added `*.wasm`, `edgerun*.wat`, `edgerun*.wasm` to `.gitignore`
   - Removed empty `tor/` directory
   - Removed stub files: `pipeline/wasm-interpreter.wat` (5-line comment shell) and `index.ts` (bun template)
   - Fixed memory page count inconsistency: `memory-map.wat` comment → `2048` (matches canonical `memory.wat`)

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

### Tooling

| Tool | Command | Description |
|------|---------|-------------|
| **Build** | `bun run build` / `make` | Concatenates fragments → `edgerun.wat`, compiles to `edgerun.wasm` via `wat2wasm`, optionally strips via `wasm-tools strip` |
| **Watch** | `bun run build:watch` / `make watch` | Rebuild on file changes (polls `.wat` files in all source dirs) |
| **Validate** | `bun run validate` / `make validate` | Runs `wasm-tools validate` on `edgerun.wat` + `edgerun.wasm` |
| **Test** | `bun run test` / `make test` | Instantiates `edgerun.wasm` in Node.js WASM runtime, runs 7 integration test groups |
| **Lint** | `bun run lint` / `make lint-wat` | Validates every individual `.wat` file with `wasm-tools validate` |
| **Stats** | `bun run stats` / `make stats` | Counts source files, lines, build output size |
| **Clean** | `bun run clean` / `make clean` | Removes all build artifacts |
| **CI** | `bun run ci` / `make ci` | Full pipeline: build → validate → test |

### Build Pipeline

```
source .wat fragments  ──►  tools/build_wat.mjs  ──►  edgerun.wat  ──►  wat2wasm  ──►  edgerun.wasm
                                  (concatenate +                       (compile)         (binary)
                                   deduplicate +                       wasm-tools strip
                                   strip wrappers)                     ──► edgerun-stripped.wasm
```

### Test Runner

`tools/test.mjs` instantiates `edgerun.wasm` (no imports) and tests:
1. Pipe I/O (create, write, read, available)
2. Frame I/O (frame_write, frame_read with stream IDs)
3. Pipeline lifecycle (create, set_stage, run, verify output)
4. Stage table (funcref table length and population)
5. UI framework exports (5 `er_ui_*` functions)
6. Compiler/interpreter exports (6 decode/execute functions)
7. Stage constants (16 `STAGE_*` constants with correct values)

### Supported Flags (build_wat.mjs)

| Flag | Effect |
|------|--------|
| `--out <path>` | Custom WAT output path (default: `edgerun.wat`) |
| `--watch` | Watch mode — rebuilds on `.wat` file changes |
| `--no-wasm` | Skip `wat2wasm` compilation |
| `--no-strip` | Skip `wasm-tools strip` |
