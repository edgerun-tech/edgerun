# RFC-0015: Code Generation Pipeline

**Status:** Working Draft
**Date:** 2026-04-12

---

## Abstract

The code generation pipeline transforms W3C/WHATWG specifications into working Rust code through multiple stages of extraction, proto definition, and code generation. No browser does this — Chrome's HTML parser is ~500,000 lines of hand-written C++; ours is ~7,000 lines of generated Rust.

---

## Stage 1: Spec Extraction

### `edgerun spec extract <domain>` (Go CLI at `cmd/edgerun/`)

Extracts specification data from W3C/WHATWG docs into **Catalog protobuf messages**.

| Domain | Source | Method | Output |
|--------|--------|--------|--------|
| `css <specs_dir>` | W3C CSS markdown | Regex extraction of properties (`Name:`, `dfn-type="property"`) | Catalog with properties, value types, at-rules |
| `dom <file.md>` | WHATWG DOM Living Standard | Regex extraction of WebIDL interface blocks | Catalog with DOM interfaces |
| `html <file.md>` | WHATWG HTML spec | Element catalog extraction | Catalog with 105 HTML elements |
| `ecmascript <file.md>` | ECMA-262 spec | Built-in objects + abstract ops | Catalog with 127 intrinsics, 121 abstract ops, 27 objects |
| `selectors <file.md>` | Selectors Level 4 | Pseudo-classes, combinators | Catalog with selector types |
| `encoding` | Hardcoded table | 38 encodings + BOM table | Catalog with encoding definitions |
| `fetch` | Hardcoded | Fetch spec enums | Catalog with fetch types |
| `url` | Regex extraction | URL spec states | Catalog with URL types |
| `uievents` | Hardcoded | UI Events key codes | Catalog with event types |
| `webidl` | Extraction | Web IDL type system | Catalog with IDL types |
| `flexbox` | Hardcoded | CSS Flexbox properties | Catalog with flexbox types |
| `grid` | Hardcoded | CSS Grid properties | Catalog with grid types |
| `media` | Hardcoded | CSS media query types | Catalog with media types |
| `syntax` | Hardcoded | CSS syntax token types | Catalog with syntax tokens |
| `color` | Hardcoded | CSS color definitions | Catalog with color definitions |

### Standalone Extractors (12 commands)

| Command | Purpose |
|---------|---------|
| `extract-css-specs` | CSS properties from W3C markdown |
| `extract-dom-spec` | DOM interfaces from WHATWG |
| `extract-html-spec` | HTML elements from WHATWG |
| `extract-ecmascript-spec` | ECMAScript built-ins from ECMA-262 |
| `extract-selectors-spec` | Selectors L4 types |
| `extract-encoding-spec` | 38 encodings + BOM |
| `extract-fetch-spec` | Fetch types |
| `extract-url-spec` | URL types |
| `extract-uievents-spec` | UI event codes |
| `extract-webidl-spec` | Web IDL types |
| `extract-flexbox-spec` | Flexbox properties |
| `extract-grid-spec` | Grid properties |

---

## Stage 2: Proto Generation

### `generate-*-proto` Commands

| Command | Input | Output |
|---------|-------|--------|
| `generate-ecmascript-proto` | JSON catalog of JS objects/ops | 3 proto files: objects, abstract_ops, globals |
| `generate-css-proto` | JSON catalog of CSS properties/types | 3 proto files: properties, value_types, at_rules |
| `generate-selector-dom` | JSON catalogs for Selectors + DOM | 2 Rust crates: edgerun-selectors, edgerun-dom |
| `generate-browser` | 3 JSON catalogs (HTML, CSS, JS) | edgerun-browser crate with 8 modules |
| `generate-renderer` | Proto data | edgerun-layout crate with 6 modules |
| `generate-batch4` | Proto data | 4 crates: images, fonts, media-queries, css-syntax |

### Manual Proto Files

44 proto files maintained in `proto/edgerun/v0/`:
- **HTML** (5): tree_builder, tokenizer_states, html_elements, html_attributes, entities
- **CSS** (18): animations, at-rules, box, cascade, contain, display, fonts, images, media queries, page, properties, sizing, syntax, text, transforms, transitions, ui, value_types, values, writing_modes
- **ECMAScript** (3): objects, abstract_ops, globals
- **Core** (12): common, capability, capability_runtime, access, identity, indexeddb, network, object, service_workers, spec_catalog, stream, trust, xml_types
- **Web/UI** (2): web/trusted_types, ui/touch_events
- **DOM** (1): dom/dom_tree

---

## Stage 3: buf Generates Rust/Go Bindings

### Configuration (`buf.gen.yaml`)

```yaml
version: v2
plugins:
  - plugin: protoc-gen-prost
    out: crates/edgerun-proto/src/gen
    opt:
      - enable_type_names
      - strategy: all
```

Generates into 4 crate `src/gen` directories:
- `crates/edgerun-proto/src/gen` — General edgerun.v0 types
- `crates/edgerun-html/src/gen` — HTML types
- `crates/edgerun-css/src/gen` — CSS types
- `crates/edgerun-ecmascript/src/gen` — ECMAScript types

Also generates Go `.pb.go` files in `gen/go/edgerun/v0/`.

---

## Stage 4: HTML Parser Code Generation (3-Stage Pipeline)

### Stage 4a: Parser IR Generation

**Binary:** `cmd/generate-parser-ir/main.go`
**Package:** `pkg/parserir/`

Generates 4 IR files in `data/` directory:

| File | Content | Source |
|------|---------|--------|
| `tokenizer.textproto` | WHATWG HTML 13.2.5 state machine | `transitions.go` (556 lines, 86 states) |
| `tree_builder.textproto` | WHATWG HTML 13.2.6 tree rules | `rules.go` (1,537 lines, 24 insertion modes) |
| `entities.textproto` | 2,231 named character references | `entities.go` (2,145 lines) |
| `element_metadata.json` | void/raw text/escapable elements | Hardcoded |

### Stage 4b: Rust Source Generation from IR

**Binary:** `cmd/html-codegen/main.go`
**Usage:** `go run ./cmd/html-codegen [data-dir] [output-dir]`

**Package:** `cmd/html-codegen/codegen/`

| Generator | Lines | Output |
|-----------|-------|--------|
| `gen_tokenizer.go` | 533 | `tokenizer.rs` (table-driven FSM) |
| `gen_tree_builder.go` | 700+ | `tree_builder.rs` (2,264 lines) |
| `gen_entity_decoder.go` | — | `entity_decoder/mod.rs` + `table.inc` |
| `gen_html_parser.go` | — | `html_parser.rs` (130 lines) |

### Stage 4c: Layout Code Generation

**Binary:** `cmd/generate-renderer` (Go)

| Output | Generated From |
|--------|---------------|
| `layout/render_object.rs` | CSS Display + HTML element specs |
| `layout/layout_context.rs` | CSS Display proto |
| `layout/box_model.rs` | CSS Box proto |
| `layout/text_layout.rs` | CSS Text + CSS Fonts proto |
| `layout/color_convert.rs` | CSS Colors spec data |
| `layout/paint_command.rs` | CSS proto data |

---

## Stage 5: WGSL Shader Generation

**Binary:** `cmd/generate-wgsl` (Go)
**Status:** `shaders/render.wgsl` exists (823 lines), `shaders/layout.wgsl` exists

| Proto Source | WGSL Output |
|-------------|-------------|
| `css_colors.proto` → NamedColor | `const NAMED_COLORS: array<vec4f, 148>` |
| `css_images.proto` → GradientKind | `fn linear_gradient_t()`, `fn radial_gradient_t()`, `fn conic_gradient_t()` |
| `css_images.proto` → CssColorStop | `fn sample_stops()` with lerp |
| `css_box.proto` → BorderStyle | `fn draw_border_pattern()` with dash patterns |
| `css_properties.proto` → OPACITY, BLEND_MODE | Uniform struct fields |
| `css_value_types.proto` → CssColor | `fn color_from_rgb()`, `fn color_from_hsl()`, etc. |
| border_lut data (9 patterns) | `const BORDER_PATTERNS: array<array<u32, 8>, 9>` |
| blend_lut data (16 modes) | `fn apply_blend_mode()` with switch on mode index |

---

## Generation Pipeline Summary

| Step | Tool | Input | Output | Lines |
|------|------|-------|--------|-------|
| 1 | `edgerun spec extract` | W3C/WHATWG docs | Catalog protos | — |
| 2 | `generate-*-proto` | Catalog JSONs | `.proto` files | 44 files |
| 3 | `buf generate` | `.proto` files | Rust prost bindings | ~10,000 |
| 4a | `generate-parser-ir` | Hardcoded WHATWG data | `data/*.textproto` | — |
| 4b | `html-codegen` (Go) | `data/*.textproto` | Rust parser source | ~7,000 |
| 4c | `cmd/generate-renderer` | CSS proto data | Rust layout source | ~400 |
| 5 | `cmd/generate-wgsl` | CSS proto + LUTs | WGSL shaders | 823 |

---

## Known Issues

1. **`generate_wgsl.py` does not exist** — The WGSL shader is generated by `cmd/generate-wgsl` (Go binary), not a Python script. Code comments in `crates/edgerun-rasterizer/src/` reference `scripts/generate_rasterizer.py` which also doesn't exist.
2. **No CI for codegen** — No automated regeneration on proto changes
3. **Generator bugs propagate** — If the Go generator has a bug, all generated code is wrong
4. **Proto field number changes are breaking** — Field numbers are cryptographically significant (used in canonical hashing)
5. **No version control on generated code** — Generated files committed to repo; diffing shows massive changes
6. **Stale generated files** — Some generated modules contain dead code that was generated but never integrated

---

## Innovation Statement

**Chrome's approach:** Hand-translate W3C spec → 500,000 lines of C++
**Edgerun's approach:** W3C spec → proto → codegen → 7,000 lines of generated Rust

When the W3C updates a spec, Chrome rewrites C++. We regenerate. That's the difference.
