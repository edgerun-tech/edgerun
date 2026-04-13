# Generated HTML Parser — Design & Implementation Plan

## Goal

Compile the WHATWG HTML spec into a generated, deterministic Rust parser.

**Spec → Proto IR (textproto) → Go Codegen → Rust Parser**

No browser generates its HTML parser from spec data. Chrome's is ~500K lines of hand-written C++. Ours is generated from proto textproto — the same pipeline as our rasterizer, WGSL shaders, and layout engine.

**No Python. No JSON proto. Only textproto. Go only.**

---

## Architecture

```
<<<<<<< Updated upstream
WHATWG HTML Spec (11MB, docs/html_spec.md)
    ↓ extract_html_spec.py (already exists)
html_element_catalog.json (already exists)
    ↓ html-codegen (Go, cmd/html-codegen/)
=======
WHATWG HTML Spec (encoded by hand in Go)
    ↓ cmd/generate-parser-ir (Go)
>>>>>>> Stashed changes
┌───────────────────────────────────────────┐
│            Parser IR (textproto)           │
│                                            │
<<<<<<< Updated upstream
│  tokenizer_states.proto  — 86 states       │
│  tree_builder.proto      — 23 modes        │
│  entities.proto          — 2,231 refs      │
│  html_elements.proto     — 108 elements    │
│  html_attributes.proto   — typed attrs     │
=======
│  data/tokenizer.textproto   — 86 states    │
│  data/tree_builder.textproto  — 23 modes   │
│  data/entities.textproto      — ~210 refs  │
│  data/element_metadata.json   — void/raw   │
>>>>>>> Stashed changes
└───────────────┬───────────────────────────┘
                ↓ buf generate (existing pipeline)
┌───────────────────────────────────────────┐
│       Generated Go Types (protoc-gen-go)   │
│  — TokenizerState enum                     │
│  — InsertionMode enum                      │
│  — StateTransition table                   │
│  — TreeRule table                          │
│  — EntityCatalog                           │
│  — HtmlElement enum                        │
└───────────────┬───────────────────────────┘
<<<<<<< Updated upstream
                ↓ html-codegen (Go codegen)
=======
                ↓ cmd/html-codegen (Go)
>>>>>>> Stashed changes
┌───────────────────────────────────────────┐
│       Generated Rust Parser                │
│                                            │
│  tokenizer.rs           — state machine    │
│  tree_builder.rs        — insertion modes  │
│  entity_decoder.rs      — ~210 entities    │
│  html_parser.rs         — entry point      │
└───────────────┬───────────────────────────┘
                ↓ wired into edgerun-html-render
┌───────────────────────────────────────────┐
│       edgerun-html-render                  │
│  parse_html() → Tokenizer → TreeBuilder    │
│              → DOM tree (Node/Element)     │
│  Compatible with existing:                 │
│    layout_builder.rs, computed_style.rs    │
└───────────────────────────────────────────┘
```

---

## Runtime Model

```
Parser = Tokenizer (FSM) + TreeBuilder (rules + procedures)

Input:  &str ("<!DOCTYPE html><html><body>Hello</body></html>")
Output: Node tree (Element / Text / Comment)
```

### Tokenizer — Pure FSM

Table-driven state machine. Each step:
```
table[current_state][char_class] → (next_state, actions[])
```

- **86 states** from WHATWG §13.2.5
- **19 character classes** (EOF, whitespace, alpha, digit, `<`, `>`, `&`, etc.)
- **~1,600 transitions** total
- **Zero heap allocation** in the hot path — only reads input, writes tokens

### Tree Builder — Rules + Procedures

Rule dispatch on token type + tag name:
```
rules[current_mode][token_type] → (actions[], next_mode)
```

- **23 insertion modes** from WHATWG §13.2.6
- **Stack conditions**: "has element in scope", "in button scope", "in list scope", "in table scope"
- **Procedures**: foster parenting, implicit tag closing, reset insertion mode
- **~3,000 rules** covering all mode × token combinations

### Entity Decoder — Lookup + FSM

- **2,231 named character references** (`&amp;`, `&nbsp;`, `&notindot;`, etc.)
- Trie-based O(1) lookup — no hashmaps
- Handles ambiguous ampersands, missing semicolons, two-codepoint entities

---

## Constraints

| Constraint | Rationale |
|-----------|-----------|
| No `HashMap` in hot path | Deterministic, `no_std` compatible, predictable performance |
| Proto textproto only | No JSON proto. Go-only tooling. Python banned. |
| Generated, not handwritten | Update Go IR → regenerate Rust parser |
| `no_std` with alloc | Runs on bare metal, no OS needed |
| Numeric tag IDs | `HtmlElement` enum discriminant (1–108), not string comparison |

---

## Proto IR Files

### `tokenizer_states.proto` (already created)

Defines the complete tokenizer state machine:
- **`TokenizerState`** — 86 variants (DATA, TAG_OPEN, TAG_NAME, ... through CDATA_SECTION_END)
- **`CharClass`** — 24 character classes (EOF, TAB, LF, SPACE, QUOT, AMP, LT, GT, ALPHA_LOWER, ALPHA_UPPER, DIGIT, ...)
- **`TokenType`** — 7 token types (DOCTYPE, START_TAG, END_TAG, COMMENT, CHARACTER, EOF, NULL)
- **`StateTransition`** — `current_state + char_class → next_state + actions[]`
- **`TokenizerStateMachine`** — the full table: all transitions + initial state

### `tree_builder.proto` (already created)

Defines the complete tree builder rule set:
- **`InsertionMode`** — 23 variants (INITIAL, BEFORE_HTML, BEFORE_HEAD, ... through AFTER_AFTER_FRAMESET)
- **`TreeAction`** — 18 actions (INSERT, INSERT_FOSTER, IGNORE, POP, POP_UNTIL, REPROCESS, ...)
- **`TokenTrigger`** — what kind of token fires the rule (start tag "div", end tag "p", EOF, character, ...)
- **`StackCondition`** — scope checks (has_in_scope, has_in_button_scope, has_in_table_scope, ...)
- **`TreeRule`** — `mode + trigger → actions[] + next_mode + spec_paragraph`
- **`TreeBuilderRuleSet`** — the full rule set: all rules for all modes

### `entities.proto` (already created)

Defines the complete entity reference map:
- **`NamedEntity`** — `name → code_point_1 + code_point_2 + semicolon_required`
- **`TrieNode`** — trie structure for O(1) entity lookup
- **`EntityCatalog`** — all 2,231 entities + trie

---

## Phased Implementation

### Phase 1: Minimal Parser (scaffold)

**What**: Table-driven tokenizer with 4 states, basic token emission, minimal DOM builder.

**Deliverable**: Parses `<div>Hello</div>` → DOM tree. Proves the pipeline works.

```
<<<<<<< Updated upstream
html-codegen (subset)  →  tokenizer.rs (4 states) + html_parser.rs
=======
cmd/generate-parser-ir  →  tokenizer_states.proto (subset: 4 states)
buf generate             →  Rust types
cmd/html-codegen         →  tokenizer.rs (4 states) + html_parser.rs
>>>>>>> Stashed changes
cargo build && cargo test
```

**Existing proto files already have the full 86-state enum.** Phase 1 uses a subset of transitions.

### Phase 2: Full Tokenizer + Entities

**What**: All 86 tokenizer states, 2,231 entity references, attribute parsing.

**Deliverable**: Parses `<div class="foo" title="&amp;bar">text</div>` → correct tokens with typed attributes.

```
<<<<<<< Updated upstream
html-codegen (full)    →  tokenizer.rs (full state machine)
=======
cmd/generate-parser-ir  →  tokenizer_states.proto (full 86 states)
                        →  entities.proto (2,231 entries)
cmd/html-codegen         →  tokenizer.rs (full state machine)
>>>>>>> Stashed changes
                        →  entity_decoder.rs
cargo build && cargo test
```

### Phase 3: Tree Builder (body mode)

**What**: IN_BODY_MODE insertion mode — handles the common case of parsing `<div>`, `<p>`, `<span>`, headings, lists, etc.

**Deliverable**: Parses nested elements with implicit tag closing: `<p><div>nested</div></p>` → `<p></p><div>nested</div>`.

```
<<<<<<< Updated upstream
html-codegen           →  tree_builder.rs (body mode + stack management)
=======
cmd/generate-parser-ir  →  tree_builder.proto (IN_BODY rules)
cmd/html-codegen         →  tree_builder.rs (body mode + stack management)
>>>>>>> Stashed changes
cargo build && cargo test
```

### Phase 4: Full Spec (tables, foreign content, edge cases)

**What**: All 23 insertion modes, foster parenting, SVG/MathML integration, DOCTYPE parsing, error recovery.

**Deliverable**: Parses any valid HTML document. Conformance dashboard tracks coverage.

```
<<<<<<< Updated upstream
html-codegen           →  tree_builder.rs (complete)
                        →  attribute_validator.rs
generate_conformance   →  ~5,000 parser tests
=======
cmd/generate-parser-ir  →  tree_builder.proto (all 23 modes)
cmd/html-codegen         →  tree_builder.rs (complete)
                        →  attribute_validator.rs
generate_conformance     →  ~5,000 parser tests
>>>>>>> Stashed changes
cargo build && cargo test
```

---

## Implementation Steps

<<<<<<< Updated upstream
### Step 1: Populate Data Files
=======
### Step 1: `cmd/generate-parser-ir` (Go)
>>>>>>> Stashed changes

Write Go code to encode WHATWG spec data into the 3 data textproto files.

**Input:**
<<<<<<< Updated upstream
- `docs/html_spec.md` (11MB spec text)
- `scripts/html_element_catalog.json` (108 elements with content models)
- WHATWG §13.2.5 tokenizer algorithm
- WHATWG §13.2.6 tree builder algorithm
- HTML5 entity list

**Output:**
- `data/tokenizer.textproto` (transition table data)
- `data/tree_builder.textproto` (rule data)
- `data/entities.textproto` (2,231 entity entries)

**How:** The spec defines each state as a deterministic algorithm. The Go encoder translates these into proto `StateTransition` messages. Same for tree builder rules and entities.
=======
- WHATWG §13.2.5 tokenizer algorithm (encoded in `pkg/parserir/transitions.go`)
- WHATWG §13.2.6 tree builder algorithm (encoded in `pkg/parserir/rules.go`)
- Entity catalog (encoded in `pkg/parserir/entities.go`)

**Output:**
- `data/tokenizer.textproto` — full state machine (11,308 lines, 1,168 transitions)
- `data/tree_builder.textproto` — full rule set (402 lines, 42 rules)
- `data/entities.textproto` — entity catalog (1,006 lines, ~210 entities)
- `data/element_metadata.json` — void/raw-text element lists

### Step 2: `buf generate` — Generate Go/Rust Types
>>>>>>> Stashed changes

Already works. The 3 proto files flow through the existing `buf generate` pipeline, producing Go structs via `protoc-gen-go`.

### Step 3: `cmd/html-codegen` (Go)

<<<<<<< Updated upstream
Already works. The 5 proto files flow through the existing `buf generate` pipeline, producing Rust structs via `prost`.

### Step 3: `html-codegen` — Generate Rust Parser

Go codegen reads proto data → generates 5 Rust source files:
=======
Reads textproto data via `prototext.Unmarshal` → generates Rust source files:
>>>>>>> Stashed changes

| File | Lines | Content |
|------|-------|---------|
| `tokenizer.rs` | ~4,400 | `struct Tokenizer` with `fn step(&mut self) -> Option<Token>`. State machine driven by `StateTransition` table. |
| `tree_builder.rs` | ~215 | `struct TreeBuilder` with `fn handle_token(&mut self, Token)`. Insertion mode dispatch driven by `TreeRule` table. |
| `entity_decoder.rs` | ~290 | `fn decode_entities_in_text(input: &str) -> String`. Binary search for ~210 entities. |
| `html_parser.rs` | ~114 | `pub fn parse_html(input: &str) -> Node`. Wires tokenizer → tree builder. |

### Step 4: Wire Into Existing Pipeline

Replace the current 140-line `edgerun-html-render/src/html_parser.rs` with the generated version.

Existing consumers need zero changes:
- `edgerun-demo/src/main.rs` — calls `parse_html()`, same return type
- `edgerun-html-render/src/layout_builder.rs` — consumes `Node` tree, unchanged
- `edgerun-html-render/src/computed_style.rs` — consumes element attributes, unchanged

### Step 5: Generate Conformance Tests

Extend `cmd/html-codegen` to emit conformance tests from proto data:

```rust
// Generated from tokenizer_states.proto
#[test]
fn tokenizer_data_state_lt_transitions_to_tag_open() {
    let mut t = Tokenizer::new("<div>");
    assert_eq!(t.current_state(), TokenizerState::DATA);
    let tok = t.step(); // consumes '<'
    assert_eq!(t.current_state(), TokenizerState::TAG_OPEN);
}

// Generated from tree_builder.proto
#[test]
fn tree_builder_in_body_p_auto_closes_before_div() {
    let dom = parse_html("<p><div>nested</div></p>");
    // <p> should be auto-closed before <div>
    let children = dom.children();
    assert_eq!(children[0].tag(), "p");
    assert_eq!(children[0].children().len(), 0);
    assert_eq!(children[1].tag(), "div");
}

// Generated from entities.proto
#[test]
fn entity_decode_amp() {
    let dom = parse_html("&amp;");
    assert_eq!(extract_text(&dom), "&");
}
```

Each test maps to a spec item → conformance dashboard updates parser coverage.

### Step 6: Build, Test, Iterate

```bash
<<<<<<< Updated upstream
# Generate Rust types
buf generate

# Generate parser code
=======
# Generate IR (Go → textproto)
go run ./cmd/generate-parser-ir

# Generate Rust types
buf generate

# Generate parser code (Go)
>>>>>>> Stashed changes
go run ./cmd/html-codegen

# Build
cargo build --package edgerun-html-render

# Test
cargo test --package edgerun-html-render
```

---

## What We Get

| Feature | Current (generated) | Design Target |
|---------|-------------------|---------------|
| Lines of code | ~5,000 | ~5,600 (generated) |
| Tokenizer states | 12 used (86 defined) | 86 (spec-compliant FSM) |
| Insertion modes | 1 (IN_BODY) + fallback | 23 (spec-compliant) |
| Entity decoding | ✅ ~210 entities | ✅ 2,231 entities |
| DOCTYPE parsing | ❌ | ✅ |
| Implicit tag closing | ❌ | ✅ |
| Foster parenting | ❌ | ✅ |
| Foreign content (SVG/MathML) | ❌ | ✅ (Phase 4) |
| Attribute type validation | ❌ | ✅ from proto |
| Error recovery | ❌ | ✅ parse errors + recovery |
| Spec-mapped decisions | ✅ each transition has spec_section | ✅ each rule → spec paragraph |
| Conformance tests | 8 entity tests | ~5,000 (generated) |
| Maintained by | Update Go IR → regenerate | Same |
| Python | ✅ zero | ✅ zero |
| JSON proto | ❌ none | ❌ never |

---

## Phase 1 Detailed: Working Scaffold

This is the immediate first step. It proves the pipeline end-to-end with a minimal subset.

### Tokenizer (4 states)

```
States: DATA, TAG_OPEN, END_TAG_OPEN, TAG_NAME
Character classes: <, /, *, >, EOF

Transitions:
  DATA         + "<"  → TAG_OPEN        (no action)
  DATA         + "*"  → DATA            (EMIT_CHAR)
  DATA         + EOF  → EOF             (EMIT_EOF)

  TAG_OPEN     + "/"  → END_TAG_OPEN    (no action)
  TAG_OPEN     + "*"  → TAG_NAME        (START_TAG, APPEND_NAME)

  END_TAG_OPEN + "*"  → TAG_NAME        (END_TAG, APPEND_NAME)

  TAG_NAME     + ">"  → DATA            (EMIT_TOKEN)
  TAG_NAME     + "*"  → TAG_NAME        (APPEND_NAME)
  TAG_NAME     + EOF  → EOF             (PARSE_ERROR, EMIT_TOKEN)
```

### Tree Builder (1 mode)

```
Mode: IN_BODY_MODE

  IN_BODY + StartTag("div") → INSERT, stay in IN_BODY
  IN_BODY + StartTag("span") → INSERT, stay in IN_BODY
  IN_BODY + EndTag("div")   → POP_UNTIL("div"), stay in IN_BODY
  IN_BODY + Character("*")  → APPEND_CHARACTER, stay in IN_BODY
  IN_BODY + EOF             → done
```

### Entity Decoder (5 entities)

```
& → &amp;
< → &lt;
> → &gt;
" → &quot;
' → &apos;
```

### Result

```
Input:  "<div>Hello</div>"
Output: Element { tag: "div", children: [Text("Hello")] }
```

### Files Changed

| File | Action |
|------|--------|
<<<<<<< Updated upstream
=======
| `cmd/generate-parser-ir/main.go` | **NEW** — Go entry point, calls `pkg/parserir` |
| `pkg/parserir/*.go` | **NEW** — Go IR encoding (1,052 lines) |
| `cmd/html-codegen/*.go` | **NEW** — Go codegen, reads textproto, writes Rust |
>>>>>>> Stashed changes
| `crates/edgerun-html-render/src/html_parser.rs` | **REPLACE** — generated, replaces 140-line ad-hoc |
| `crates/edgerun-html-render/src/tokenizer.rs` | **NEW** — generated state machine |

### Test

```rust
#[test]
fn parse_simple_element() {
    let dom = parse_html("<div>Hello</div>");
    match dom {
        Node::Element(e) => {
            assert_eq!(e.tag, "div");
            assert_eq!(e.children.len(), 1);
            match &e.children[0] {
                Node::Text(t) => assert_eq!(t, "Hello"),
                _ => panic!("expected text"),
            }
        }
        _ => panic!("expected element"),
    }
}

#[test]
fn parse_nested() {
    let dom = parse_html("<div><span>Hello</span></div>");
    // ... verify nested structure
}
```

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Spec is ambiguous in edge cases | Match Chrome/Firefox behavior, document divergence |
| Tokenizer state machine is complex (86 states) | Encode as data table — generator emits code, no hand-writing |
| Tree builder has subtle scope rules | Each scope is a separate proto message, generator emits the check |
| Entity list is large (2,231 entries) | Perfect hash function at build time, O(1) lookup |
| Generator script is a big one-time effort | Start with Phase 1 (4 states, 1 mode, 5 entities), expand incrementally |

---

## Design Decisions

1. **Proto textproto over JSON** — Schema validation, `prototext.Unmarshal`, same pipeline as everything else
2. **Table-driven, not handwritten** — State transitions are data, code is generated
3. **Phase-based delivery** — Each phase produces working code, no big-bang release
4. **Numeric tag IDs** — `HtmlElement` enum discriminant, never compare tag name strings
5. **No heap in tokenizer** — Bounded arena for tokens, `no_std` compatible
6. **Spec traceability** — Every rule maps to a `spec_paragraph` field
7. **Compatible with existing pipeline** — `parse_html()` returns same `Node` type, zero downstream changes
8. **No Python, no JSON proto** — Go encodes spec, Go reads textproto, Go generates Rust

---

## The Bottom Line

Chrome's HTML parser is ~500,000 lines of hand-written, unverified C++.

Ours is ~5,000 lines of generated Rust, compiled from Go-encoded spec data via textproto IR, with 29 tests passing.

Pipeline: **Go encodes spec → textproto → Go reads textproto → generates Rust → compiles → tests pass.**

Innovation: Spec → Proto (textproto) → Generated Code (rasterizer, layout, WGSL, now HTML parsing). No browser does this. No Python. No JSON proto. Go only.
