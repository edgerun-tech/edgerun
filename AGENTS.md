# Agent Rules for EdgeRun

This file defines strict rules for AI agents working in this repository.
Follow these rules exactly. They override any general coding conventions.

## 1. Project Ethos

EdgeRun is a **self-hosted, single-module WAT runtime** with zero external
dependencies. The project's core commitments:

- **Reduction.** Consolidate tooling, remove external deps, merge scripts.
  Every PR must have fewer files or fewer dependencies than before.
- **Integration.** New capabilities must hook into the existing build system
  (`tools/build.mjs`) and pipeline stage system (`package.json` → `edgerun.registry`).
  No standalone scripts, no new npm commands.
- **Self-hosting.** Everything is written in WAT. JS/TS tooling is a temporary
  bridge for build-time orchestration only. Runtime code never leaves WAT.
- **Cleanliness.** No stale artifacts, no temp files, no commented-out code,
  no dead fragments. `bun run clean` before committing if any `out/` files
  were generated.

## 2. Internal Tooling First

Before adding any dependency (npm package, external binary, new script),
check this table:

| Capability | Internal tool | Use instead of |
|---|---|---|---|
| WAT → WASM compile | `er-codec.mjs:compileWat(src)` | `wat2wasm`, `wasm-tools` (banned) |
| LEB128 encode/decode | `er-codec.mjs` (LEB128 helpers) | third-party LEB128 libs |
| Endian read/write | `er-codec.mjs` (endian helpers) | third-party binary libs |
| Build orchestration | `tools/build.mjs` (all commands) | Make, just, shell scripts |
| Source viewer | `tools/er.mjs` (`er list`, `er cat`) | custom dump scripts |
| File I/O | `build-lib.mjs` (`readText`, `writeText`, `fileExists`, `ensureDir`) | `fs` calls |
| Memory range validation | `build.mjs` `validateMemoryRanges` | manual checking |

**Rule:** If an internal tool provides the capability you need, use it. Do
not add a new dependency. Do not call external binaries. If no internal tool
exists, add it to the toolchain (not as a standalone script).

## 3. Cleanup Discipline

Every change must leave the repo cleaner than it was found:

- **No stale files.** Delete orphaned fragments, dead code, unused imports.
- **No temp files.** Never commit `/tmp/` files, `*.elf`, or `.wasm` files
  outside `out/`. If you create temp files during a build, remove them.
- **`out/` is ephemeral.** Everything under `out/` is gitignored and can be
  regenerated with `bun run build`. Never commit generated files.
- **`bun run clean` before commit** if you generated any `out/` files during
  your work.
- **No commented-out code.** Delete it. Git history has the original.
- **No dead fragments.** Delete any `.wat` files that are orphaned (no longer reachable from the filesystem discovery).

## 4. No External Dependencies

- **Bun** is the only allowed external dependency. Everything else must be
  self-hosted.
- Never add a new npm package, pip package, cargo crate, or apt/homebrew
  dependency without explicit approval.
- `wat2wasm`, `wasm-tools`, `wasm2elf` are **banned** — never invoke them.
  All WAT compilation uses `er-codec.mjs:compileWat()` which calls
  the built-in `load_wat`/`emit_wasm` exports in `edgerun.wasm`.
- The ARCHITECTURE.md "Prerequisites" lists only **Bun**. If you see a
  reference to another external tool in the docs, remove it.

## 5. Optimization & Correctness

All WAT code must follow these rules:

- **Bounds-check every read.** Never index linear memory without checking
  the length first. All pipeline stages receive `scratch` and `scap` — use them.
- **Handle all return codes.** Pipeline stages return non-zero on error.
  Callers must check and propagate. Never ignore a status return.
- **Use the pattern system.** New pipeline stages must use one of the
  existing patterns in `edgerun.registry`: `scan`, `decode`, `transform`,
  `store`, `map`, or `raw`. Only use `raw`/`custom` when no pattern fits.
- **No magic numbers.** Use named globals from the config system
  (`package.json` → `edgerun.config` / `edgerun.globals`). Memory addresses
  should use `{{NAME}}` template references resolved at build time.
- **Memory zones must not overlap.** Run `bun run gen-config` after changing
  memory ranges — it validates overlap and will fail the build if violated.
- **Keep it tight.** WAT bytecode size matters. Reuse existing helpers
  (`memcpy`, `pack`, `char_class`, etc.) instead of reimplementing.

## 6. Contributing Patterns

### Adding a pipeline stage
1. Add an entry to `package.json` → `edgerun.registry` with a unique slot #,
   name, pattern, and function name.
2. Write the backing WAT function in the appropriate directory (e.g.
   `data/`, `codec/`, `crypto/`, `protocol/`, etc.).
3. Run `bun run build` to regenerate pipeline stages and rebuild.

### Adding a constant or global
1. Add to `package.json` → `edgerun.config` (grouped constants) or
   `edgerun.globals` (exported globals).
2. Run `bun run gen-config` to regenerate `out/gen/config.wat`.

### Removing something
1. Remove from registry. Delete the `.wat` fragment file.
2. Run `bun run clean && bun run build` to verify nothing breaks.
3. Delete the `.wat` fragment file.
4. If removing from the registry, do not reuse the slot number (it is
   reserved to prevent pipeline config drift).

## 7. Template System (`{{NAME}}` Address Resolution)

All fixed memory addresses in WAT source files use `{{NAME}}` template
placeholders that are resolved to numeric hex values at build time.

### How it works

1. **Define the address** in `package.json` → `edgerun.memory_ranges`:
   ```json
   "CHAR_CLASS_LUT": { "start": "0x1000", "size": 256 }
   ```
2. **Use the template** in any `.wat` source file:
   ```wat
   (data (i32.const {{CHAR_CLASS_LUT}}) "\\00\\01\\02...")
   ```
   or in code:
   ```wat
   i32.const {{SHA256_K}}   ;; resolved to 0x7530
   ```
3. **Build resolves it** — `tools/build.mjs:resolveTemplates()` replaces every
   `{{NAME}}` with the hex address from `loadAddressTable()` after fragment
   concatenation, before WAT output.

### Rules

- Template names must match `memory_ranges` keys exactly (case-sensitive,
  underscores convention).
- Never hardcode addresses from `memory_ranges` in WAT source. Use the
  template name instead.
- After adding/changing a range, run `bun run build` — `validateMemoryRanges()`
  checks all 28+ ranges for overlaps and exits with an error on conflict.
- Templates can appear anywhere in WAT source — in `(data)` offsets, `i32.const`
  operands, or other positions. The substitution is a simple string replace.

## 8. `package.json` Constants Reference

Fixed addresses, LUT data blocks, and global configuration live in
`package.json` under the `edgerun` key:

### `edgerun.memory_ranges`

Central registry of all fixed memory addresses. Each entry has `start`
(hex or decimal) and `size` (bytes). Build-time overlap validation uses
this exclusively.

Current ranges (2026-06):
| Name | Start | Size | Purpose |
|---|---|---|---|
| `CHAR_CLASS_LUT` | 0x1000 | 256 | Character class lookup table |
| `CASE_TABLE` | 0x2000 | 256 | ASCII case-mapping table |
| `WALLET_ORDER_STATUSES` | 0x4000 | 256 | Wallet order status strings |
| `SHA256_K` | 0x7530 | 256 | SHA-256 K constants |
| `SHA256_W` | 0x10000 | 320 | SHA-256/SHA-1 W buffer |
| `MSG_BUF` | 62000 | 100 | Message buffer |
| `DIGEST_BUF` | 62100 | 256 | HMAC digest buffer |
| `GALLERY_ICON_CHECK` | 65000 | 64 | Gallery icon check data |
| `GALLERY_HIT_STRING` | 65100 | 20 | Gallery hit string |
| `GALLERY_*` | 66000–72800 | various (13 entries) | Gallery UI data tables |
| `DBUS_*` | 0x7190–0x71F0 | 24/37/33 | DBus secret service paths |

### `edgerun.data`

Inline hex data blocks placed at a given offset at startup (via
`(data ...)` in `config.wat`). Offsets use `{{NAME}}` templates resolved
by `gen-config`.

Three blocks: `CHAR_CLASS_LUT` (256B), `CASE_TABLE` (256B), `SHA256_K` (256B).

### `edgerun.config`

Grouped named constants emitted as WAT globals in `config.wat`.
Organized by purpose (e.g. `oci`, `buffer`, `framing`). All values
are integer literals. Use `global.get $NAME` in WAT code.

### `edgerun.globals`

Exported globals (visible to WASM host). Each entry has `type` (`i32`
or `i64`) and `value` (integer). These are the public API surface for
embedding runtimes.

### When to use which

| You need... | Use... | Example |
|---|---|---|
| Fixed memory address for data | `memory_ranges` + `{{NAME}}` template | LUTs, string tables, working buffers |
| Inline LUT data at a fixed offset | `edgerun.data` | Character classification LUT |
| A named runtime constant | `edgerun.config` | Buffer sizes, protocol limits |
| An exported global | `edgerun.globals` | `$BUF_SIZE_64K`, `$SCRATCH_BUF` |
| A runtime variable (mutable) | `edgerun.globals` | Working buffer base addresses |

## 9. WASM Exports — Use Instead of JS Reimplementations

EdgeRun's compiled WASM exports **3,262 functions** covering crypto, codecs,
protocols, character classification, math, memory ops, and more. Before
writing a JS implementation of any utility, check if the WASM module already
provides it.

### Rules

- **Use WASM everywhere.** In test scripts (`tests/*.mjs`), build tools
  (`tools/*.mjs`), and any other JS code that runs after the WASM module is
  compiled, prefer calling WASM exports over reimplementing in JS. The WASM
  module is loaded once; calling its exports is faster and more correct than
  duplicating logic in JS.
- **Bridge code** (e.g. `er-codec.mjs`) that orchestrates WASM compilation
  itself should still call into the compiled WASM for any heavy work after
  the module is ready.

### Common WASM functions to use instead of JS

| JS pattern | WASM replacement | Notes |
|---|---|---|
| `buf.map(b => b.toString(16).padStart(2,'0')).join('')` | `e.hex_encode_lower(ptr, len, out_ptr)` then read from out_ptr | Hex encode bytes in WASM memory |
| `crypto.createHash('sha256').update(s).digest('hex')` | `e.sha256(ptr, len, out_ptr)` | Output 32 bytes at out_ptr |
| `crypto.createHash('sha1')` | `e.sha1(ptr, len, out_ptr)` | Output 20 bytes at out_ptr |
| `crypto.createHmac('sha256', key)` | `e.hmac_sha256(kptr, klen, mptr, mlen, optr, 32)` | Returns 0 on success |
| `s.toLowerCase()` / `s.toUpperCase()` | `e.to_lower(c)` / `e.to_upper(c)` | Single character only |
| `char.match(/[a-zA-Z]/)` | `e.char_class(c) & 6` | Bit flags (see char_class LUT) |
| `char.match(/[0-9]/)` | `e.char_class(c) & 1` | |
| `char.match(/\\s/)` | `e.char_class(c) & 32` | |
| `Buffer.from(mem).toString('hex')` | Use `hex(ptr, len)` pattern from tests | Calls `hex_encode_lower` internally |

### How to check available exports

Run this to list all WASM exports:

```js
const e = instance.exports;
console.log(Object.keys(e).sort().join('\n'));
```

Or search for relevant functions:

```js
const crypto = Object.keys(e).filter(k => k.includes('sha') || k.includes('aes'));
```
