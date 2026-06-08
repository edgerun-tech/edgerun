# EdgeRun — Single-Module WAT Runtime

Single-module WAT runtime for massively parallelizable, platform-independent
data transformation pipelines. Zero dependencies. Self-hosted tooling only.

## Principles

- **Self-hosting.** Everything is written in WAT — including the CLI toolchain
  (`er`) and the codec library. External dependencies are temporary bridges
  to be eliminated, not features to rely on.
- **Reduction.** Every new capability must integrate into the existing toolchain.
  No separate scripts, no new package.json commands, no external npm packages
  for what internal tooling already handles.
- **Cleanup.** Build artifacts go under `out/` (gitignored). Temp files must be
  removed. Dead fragments must be deleted. Every change should leave the tree
  cleaner than it was found.
- **Internal-first.** Before adding any dependency, check what internal tooling
  already provides the capability (see Tooling Reference below).

## Design

All code lives in a single `(module ...)` block in `out/edgerun.wat`. The module
owns linear memory, exports a pipeline runner, and exposes every stage through
a dispatch table. Pipelines are arrays of stage IDs + config pointers;
`pipeline_run` drives them sequentially through intermediate pipes.

### Architecture

```
out/edgerun.wat (single module)
├── Memory + LUTs           (char classification, lowercase maps)
├── Shared globals          (status codes, heap bounds, pipe offsets, syscalls)
├── Core helpers            (memcpy, pack, bounds_check, char class, SIMD)
├── Pipeline infrastructure (pipe, frame, mux, pipeline cores)
├── ~145 pipeline stages    (encoding, crypto, protocol, app, UI, etc.)
├── UI framework            (92 composable UI fragments)
└── Test/validation         (ABI tests, shape constants)
```

### Pipeline Model

```
pipeline_run(desc, input_pipe, output_pipe, scratch, scap) → status
```

Drives N stages sequentially. Each stage reads from an input pipe, writes to
an output pipe. Intermediate pipes are created/destroyed by the runtime.

### Stage Interface

```
(type $stage_fn (func (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
;; params: input_pipe, output_pipe, config_ptr, config_len, scratch_ptr, scratch_cap, state_ptr
```

### Memory Zones

| Zone | Address | Purpose |
|------|---------|---------|
| LUTs | 0x01000–0x02FFF | Char classification, lowercase maps |
| Pipe heap | 0x40000–0x80000 | User pipes, socket structs, configs |
| Stage heap | 0x80000–0x8FFFF | Persistent state blocks, decode buffers |

## Build System

The build is orchestrated via `package.json` scripts at the repo root.
All generated files go under `out/` and are gitignored. Notable outputs:
- `out/edgerun.wat` — assembled single-module WAT (~4.5 MB)
- `out/edgerun.wasm` — compiled WASM binary (~3 MB)
- `out/gen/pipeline-stages.wat` — generated pipeline stages

### Scripts

| Command | Description |
|---------|-------------|
| `bun run all` | Full build: generate configs + stages → assemble `out/edgerun.wasm` |
| `bun run gen` | Generate config (`out/gen/config.wat`) |
| `bun run build` | Build everything: config, stages, assemble `out/edgerun.wat` + `.wasm` |
| `bun run clean` | Remove all build artifacts (`rm -rf out/`) |

### Build Scripts

All build logic is in `tools/build.mjs`:

- **`tools/build.mjs`** — Single entry point: `gen-config`, `gen-stages`, `build`.
  UI fragment files under `ui/src/` are discovered and included by the same
  fragment discovery as all other `.wat` files — no separate build step.

- **`tools/build-lib.mjs`** — Shared library: fragment concatenation, WAT
  compilation (via `er-codec.mjs:compileWat` — WASM-native first), WASM
  stripping/optimization.

- **`tools/er.mjs`** — `er` CLI tool for view/edit/build/commit workflows.

- **`ui/parse-fragment.mjs`** — WAT fragment parser used by `ui/analyze_deps.mjs`.

### Tooling Reference

All internal tools live under `tools/`. Use these instead of external dependencies.

| Tool | Path | Purpose |
|------|------|---------|
| `build.mjs` | `tools/build.mjs` | Consolidated build: `gen-config`, `gen-stages`, `build` |
| `er` CLI | `tools/er.mjs` | List/cat source files embedded in `out/edgerun.wasm` |
| `er-codec.mjs` | `tools/er-codec.mjs` | LEB128 encode/decode, endian read/write, WAT→WASM compilation (`compileWat(src)`), backed by `out/edgerun.wasm` itself |
| `build-lib.mjs` | `tools/build-lib.mjs` | Shared utilities: `resolveRoot`, `rootPath`, `fileExists`, `ensureDir`, `readText`, `writeText`, `wrapModule`, `embedCustomSection` |

**Rule:** Never call `wat2wasm`, `wasm-tools`, or any external WAT compiler
directly. Always use `er-codec.mjs:compileWat()` or `bun run build`.

### Key Files

- **`package.json` → `edgerun.registry`** — Defines all pipeline stages (slot #, name,
  pattern, constraints). Used by `gen-stages` to generate stage WAT code.

### Prerequisites

- **Bun** — the only external dependency. Used for all build scripts.
  Everything else (WAT → WASM compilation, codec, etc.) is handled by internal
  tooling in `tools/`.
