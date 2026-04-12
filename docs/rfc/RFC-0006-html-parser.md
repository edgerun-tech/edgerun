# RFC-0006: HTML Parser — WHATWG Generated Parser

**Status:** Working Draft
**Date:** 2026-04-12

---

## Abstract

`edgerun-render` includes a **generated WHATWG-compliant HTML parser** consisting of:
- An 87-state tokenizer (table-driven FSM)
- A 24-mode tree builder (insertion rules)
- A 2,231-entity decoder (binary search lookup)

All three are auto-generated from proto IR data via the Go `html-codegen` tool. No browser generates its HTML parser from spec data.

---

## Components

### Tokenizer (`html/tokenizer.rs` — GENERATED)

**Source:** `data/tokenizer.textproto` → `cmd/html-codegen` → `tokenizer.rs`

**WhatWG Spec:** §13.2.5 — The HTML Tokenizer

| Metric | Value |
|--------|-------|
| States | 87 (proto enum) |
| Character classes | 24 |
| Transitions | ~1,600 |

**Public API:**
- `Tokenizer::new(input: &str) -> Self`
- `Tokenizer::tokenize(&mut self) -> Vec<Token>`
- `Tokenizer::step(&mut self) -> Option<Token>` — Advance one character
- `Tokenizer::parse_errors(&self) -> usize`
- `Tokenizer::set_state(&mut self, State)`
- `Tokenizer::set_raw_text_tag(&mut self, tag: &str, State)`

**Token Types:**
- `StartTag { name: String, attrs: Vec<(String, String)>, self_closing: bool }`
- `EndTag { name: String }`
- `Character(String)`, `Comment(String)`, `Doctype`, `Eof`

**Key Features:**
- Table-driven state machine — `match self.state { match c { ... } }`
- Raw text mode detection (script, style elements)
- Character reference handling via entity_decoder
- Numeric character reference parsing (decimal + hex)
- Parse error tracking

### Tree Builder (`html/tree_builder.rs` — GENERATED)

**Source:** `data/tree_builder.textproto` → `cmd/html-codegen` → `tree_builder.rs`

**WhatWG Spec:** §13.2.6 — The Tree Construction Stage

| Metric | Value |
|--------|-------|
| Insertion modes | 24 (proto enum) |
| Rules | ~3,000 mode × token combinations |

**Public API:**
- `TreeBuilder::new() -> Self`
- `TreeBuilder::handle_token(&mut self, token: &Token)`
- `TreeBuilder::finish(self) -> Vec<Node>`
- `TreeBuilder::insertion_mode(&self) -> InsertionMode`
- `TreeBuilder::parse_errors(&self) -> usize`
- Tokenizer mode switches: `switch_to_rawtext()`, `switch_to_rcdata()`, `switch_to_script_data()`

**Scope Checking:**
- `has_in_scope(tag)` — Generic scope check
- `has_in_button_scope(tag)` — Button-specific blockers
- `has_in_list_item_scope(tag)` — List-specific blockers

**Foster Parenting:** `insert_foster()` for misplaced table content

### Entity Decoder (`html/entity_decoder/` — GENERATED)

**Source:** `data/entities.textproto` → `cmd/html-codegen` → `entity_decoder/mod.rs` + `table.inc`

| Metric | Value |
|--------|-------|
| Named entities | 2,231 |
| Lookup method | Binary search over sorted table |
| Two-codepoint entities | Supported (e.g., `&NotEqual;`) |
| Legacy entities (no semicolon) | Supported |

**Public API:**
- `lookup_entity(name: &str) -> Option<(u32, u32)>` — Binary search
- `decode_entities_in_text(input: &str) -> String` — Decode all entities in text

### HTML Parser Entry Point (`html/html_parser.rs` — 130 lines, GENERATED)

**Public API:**
- `parse_html(html: &str) -> Node` — Main entry point
- `count_nodes(node: &Node) -> usize`
- `Node` enum: `Element(Element)`, `Text(String)`, `Comment(String)`
- `Element` struct: `tag: String`, `attrs: BTreeMap<String, String>`, `children: Vec<Node>`
- `BLOCK_ELEMENTS` (45 tags), `INLINE_ELEMENTS` (58 tags)

---

## Data Flow

```
HTML string
    ↓
Tokenizer::new() → Tokenizer
    ↓ step() loop
Vec<Token>
    ↓
TreeBuilder::new() → TreeBuilder
    ↓ handle_token() per token
TreeBuilder::finish() → Vec<Node>
    ↓ entity decoding
Single root Node (wrapped in synthetic <div> if 0 or >1 roots)
```

---

## Test Coverage (from edgerun-render tests)

| Test | Module | What It Verifies |
|------|--------|-----------------|
| `test_decode_amp` | entity_decoder | `&amp;` → `&` |
| `test_decode_lt` | entity_decoder | `&lt;` → `<` |
| `test_decode_gt` | entity_decoder | `&gt;` → `>` |
| `test_decode_quot` | entity_decoder | `&quot;` → `"` |
| `test_decode_nbsp` | entity_decoder | `&nbsp;` → `\u{00A0}` |
| `test_decode_multiple` | entity_decoder | Multiple entities in one string |
| `test_unknown_passthrough` | entity_decoder | Unknown entity passed through |
| `test_no_entity` | entity_decoder | Plain text unchanged |
| `test_render_empty` | render.rs | Empty input produces correct buffer |
| `test_render_text_produces_non_black` | render.rs | Styled text renders visible pixels |
| `test_render_multiple_elements` | render.rs | Multiple styled elements render |

**Total: 11 tests** for HTML parsing/rendering integration.

---

## Known Issues

### Generated Code Issues
1. **Many token states are incomplete** — `InSelect`, `InSelectInTable`, `InTemplate` modes have minimal rules (mostly parse errors)
2. **`handle_fallback` is dead code** — Marked `#[allow(dead_code, unreachable_code)]`, all 24 modes implemented
3. **DOCTYPE handling** — Tree builder has `// TODO: create proper DOCTYPE node`
4. **Duplicate constants** — `VOID_ELEMENTS` defined in both `tokenizer.rs` and `tree_builder.rs`
5. **`TokenType` enum unused externally** — Internal to `emit_token()`, never matched outside

### Pipeline Issues
6. **CSS selector limitations** — Only simple `element`, `.class`, `#id`, `element.class`, `*` selectors. No combinators (descendant ` `, child `>`, sibling `+`/`~`), no attribute selectors, no pseudo-classes/elements
7. **No at-rule support** — `@media`, `@import`, `@keyframes`, `@font-face` not parsed
8. **No specificity scoring** — Cascade is purely order-based (later rule wins)
9. **Only ~12 of 255 properties resolved** — Most CSS properties parsed but ignored in `compute_style`

### Architecture Issues
10. **Paint command disconnected** — `paint_command.rs` defines `PaintCommand` with zero-sized rects; `render.rs` bypasses it and creates `RasterCommand` directly
11. **HSL/OKLCh colors fall to black** — `color_convert.rs` has conversion functions but they're not wired into `computed_style.rs`
12. **No real font metrics** — Text width estimated as `font.size * 0.5`, no glyph-specific widths
13. **Dead code modules** — `EdgeValues`, `IntrinsicSizes`, `resolve_box_dimensions`, `apply_text_transform`, `should_wrap`, `resolve_font_weight` all defined but never called
