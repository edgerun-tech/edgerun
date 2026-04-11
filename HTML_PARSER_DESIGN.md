# Generated HTML Parser — Architecture & Implementation Plan

## The Problem

Current HTML parser (`edgerun-html-render/src/html_parser.rs`, ~140 lines) is handwritten ad-hoc code:
- No entity decoding (`&amp;`, `&#x27;`, etc.)
- No DOCTYPE parsing
- No foreign content (SVG/MathML)
- No error recovery
- No implicit tag closing (`<p><div>` should auto-close `<p>`)
- No foster parenting / table-specific parsing
- Breaks the project's core pattern: **Spec → Proto → Generated Code**

Chrome's HTML parser is ~500,000 lines of hand-written, unverified C++. No browser generates its parser from spec data.

---

## The Architecture

```
WHATWG HTML Spec (11MB markdown)
    ↓ extract_html_spec.py (already exists)
html_element_catalog.json (108 elements, content models, attributes)
    ↓ generate_html_proto.py (already exists)
html_elements.proto + html_attributes.proto (already exist)
    ↓ generate_parser_ir.py (NEW — generates 3 new proto files)
┌─────────────────────────────────────────────────────────────┐
│                   Parser IR (Proto Files)                     │
│                                                              │
│  tokenizer_states.proto    — 80+ tokenizer states +          │
│                               transition table               │
│  tree_builder.proto        — 21 insertion modes +            │
│                               action table                   │
│  entities.proto            — 2,231 named character           │
│                               references → code points       │
└──────────────────┬──────────────────────────────────────────┘
                   ↓ buf generate (same as all other protos)
┌─────────────────────────────────────────────────────────────┐
│                Generated Rust Types (prost)                  │
│                                                              │
│  edgerun-html-parser-types — no_std crate with               │
│    TokenizerState enum (80+ variants)                        │
│    InsertionMode enum (21 variants)                          │
│    TransitionTable (state × char_class → next_state, token)  │
│    ActionTable (mode × token_type → action, next_mode)       │
│    EntityMap (name → char, 2,231 entries)                    │
└──────────────────┬──────────────────────────────────────────┘
                   ↓ generate_html_parser.py (NEW — code gen)
┌─────────────────────────────────────────────────────────────┐
│              edgerun-html-render/src/                        │
│                                                              │
│  html_parser.rs         — GENERATED: parse_html() entry      │
│  tokenizer.rs           — GENERATED: 80+ state machine       │
│  tree_builder.rs        — GENERATED: 21 insertion modes      │
│  entity_decoder.rs      — GENERATED: 2,231 entity lookups    │
│  attribute_validator.rs — GENERATED: typed attrs per element  │
│                                                              │
│  All marked "DO NOT EDIT. Regenerate with:                   │
│  scripts/generate_html_parser.py"                            │
└──────────────────┬──────────────────────────────────────────┘
                   ↓ wired into existing pipeline
┌─────────────────────────────────────────────────────────────┐
│              edgerun-html-render                             │
│                                                              │
│  parse_html() → Tokenizer → Tree Builder → DOM tree          │
│  Uses existing Node/Element types from html_parser.rs        │
│  Compatible with existing layout_builder.rs                  │
│  Compatible with existing computed_style.rs                  │
└─────────────────────────────────────────────────────────────┘
```

---

## Why This Will Work

### 1. You Already Have 90% of the Infrastructure

| Component | Status | What Exists |
|-----------|--------|-------------|
| Spec extraction | ✅ Done | `extract_html_spec.py` → `html_element_catalog.json` |
| Proto generation | ✅ Done | `generate_html_proto.py` → 3 proto files |
| Rust type generation | ✅ Done | `generate_html_crate.py` → `edgerun-html/` |
| `buf` build pipeline | ✅ Done | 40 proto files → 37+ crates |
| Code generation pattern | ✅ Done | `generate_wgsl.py`, `generate_conformance_tests.py` |
| Conformance dashboard | ✅ Done | 2,888 spec items tracked |
| DOM tree structure | ✅ Done | `html_parser.rs` `Node`/`Element` types |
| CSS cascade integration | ✅ Done | `computed_style.rs` consumes parsed DOM |

The missing piece is only the **parser IR** (tokenizer states, tree builder rules, entities) and the **code generator** that turns them into Rust.

### 2. The Spec Is Deterministic

WHATWG defines exact algorithms:
- Tokenizer: §13.2.5 — "Tokenization" — a state machine with 80+ states, each with exact character-by-character behavior
- Tree builder: §13.2.6 — "Tree construction" — 21 insertion modes, each with exact token handling rules
- Entities: §13.1.4.22 — "Named character references" — 2,231 entries with exact code points

There is no ambiguity. The spec is a program waiting to be compiled.

### 3. Your Proto System Already Captures the Data

`html_elements.proto` already defines:
- 108 elements with `ContentModel` classification
- `ContentCategory` enums (flow, phrasing, metadata, etc.)
- Per-element attributes with typed fields
- Tag omission text descriptions

The tokenizer states and tree builder rules are just more structured data — same pattern, new proto files.

### 4. Proto > JSON for This Use Case

| Concern | JSON | Proto |
|---------|------|-------|
| Schema validation | Manual | `buf lint` + `buf breaking` |
| Rust accessors | Write `serde` derive | `prost` generates free |
| Type safety | Runtime panics | Compile-time checks |
| Evolution | Breaking changes silent | `buf breaking` catches them |
| Pipeline consistency | One-off format | Same `buf generate` as everything |
| `no_std` support | `serde` needs alloc | `prost` works `no_std` |

You already run `buf generate` for 40 proto files. Adding 3 more is zero friction.

---

## Exact Steps to Make It Reality

### Step 0: Create Proto Files for Parser IR (3 new files)

**`proto/edgerun/v0/html/tokenizer_states.proto`**

Captures the WHATWG §13.2.5 tokenizer as data:

```protobuf
syntax = "proto3";
package edgerun.v0.html.tokenizer;

// Character classes for tokenizer state transitions.
// Each input byte maps to exactly one class.
enum CharClass {
  CHAR_CLASS_UNSPECIFIED = 0;
  CHAR_CLASS_EOF = 1;
  CHAR_CLASS_TAB = 2;          // U+0009
  CHAR_CLASS_LF = 3;           // U+000A
  CHAR_CLASS_FF = 4;           // U+000C
  CHAR_CLASS_CR = 5;           // U+000D
  CHAR_CLASS_SPACE = 6;        // U+0020
  CHAR_CLASS_QUOT = 7;         // U+0022 "
  CHAR_CLASS_AMP = 8;          // U+0026 &
  CHAR_CLASS_APOS = 9;         // U+0027 '
  CHAR_CLASS_LT = 10;          // U+003C <
  CHAR_CLASS_EQ = 11;          // U+003D =
  CHAR_CLASS_GT = 12;          // U+003E >
  CHAR_CLASS_SLASH = 13;       // U+002F /
  CHAR_CLASS_NULL = 14;        // U+0000
  CHAR_CLASS_ALPHA_UPPER = 15; // A-Z
  CHAR_CLASS_ALPHA_LOWER = 16; // a-z
  CHAR_CLASS_DIGIT = 17;       // 0-9
  CHAR_CLASS_ALPHA_OTHER = 18; // non-ASCII letters
  CHAR_CLASS_OTHER = 19;       // everything else
}

// Token types emitted by the tokenizer.
enum TokenType {
  TOKEN_TYPE_UNSPECIFIED = 0;
  TOKEN_TYPE_DOCTYPE = 1;
  TOKEN_TYPE_START_TAG = 2;
  TOKEN_TYPE_END_TAG = 3;
  TOKEN_TYPE_COMMENT = 4;
  TOKEN_TYPE_CHARACTER = 5;     // single character
  TOKEN_TYPE_EOF = 6;
  TOKEN_TYPE_NULL = 7;          // emitted for null bytes
}

// All tokenizer states from WHATWG §13.2.5.
enum TokenizerState {
  TOKENIZER_STATE_UNSPECIFIED = 0;
  DATA_STATE = 1;
  RCDATA_STATE = 2;
  RAWTEXT_STATE = 3;
  SCRIPT_DATA_STATE = 4;
  PLAINTEXT_STATE = 5;
  TAG_OPEN_STATE = 6;
  END_TAG_OPEN_STATE = 7;
  TAG_NAME_STATE = 8;
  RCDATA_LESS_THAN_SIGN_STATE = 9;
  // ... all 80+ states
}

// A single state transition: given state + char class → next state + optional token
message StateTransition {
  TokenizerState current_state = 1;
  CharClass char_class = 2;
  TokenizerState next_state = 3;
  TokenType emit_token = 4;        // NONE if no token emitted
  bool reconsume = 5;              // reconsume current char in next state
  bool emit_char = 6;              // emit the character as a character token
  bool emit_null = 7;              // emit a null token instead of character
}

// Complete tokenizer state machine.
message TokenizerStateMachine {
  string spec_section = 1;         // "13.2.5"
  string spec_url = 2;             // WHATWG URL
  repeated StateTransition transitions = 3;
  TokenizerState initial_state = 4;
}
```

**`proto/edgerun/v0/html/tree_builder.proto`**

Captures the WHATWG §13.2.6 tree builder as data:

```protobuf
syntax = "proto3";
package edgerun.v0.html.tree_builder;

import "edgerun/v0/html/tokenizer_states.proto";

// All insertion modes from WHATWG §13.2.6.
enum InsertionMode {
  INSERTION_MODE_UNSPECIFIED = 0;
  INITIAL_MODE = 1;
  BEFORE_HTML_MODE = 2;
  BEFORE_HEAD_MODE = 3;
  IN_HEAD_MODE = 4;
  IN_HEAD_NOSCRIPT_MODE = 5;
  AFTER_HEAD_MODE = 6;
  IN_BODY_MODE = 7;
  TEXT_MODE = 8;
  IN_TABLE_MODE = 9;
  IN_TABLE_TEXT_MODE = 10;
  IN_CAPTION_MODE = 11;
  IN_COLUMN_GROUP_MODE = 12;
  IN_TABLE_BODY_MODE = 13;
  IN_ROW_MODE = 14;
  IN_CELL_MODE = 15;
  IN_SELECT_MODE = 16;
  IN_SELECT_IN_TABLE_MODE = 17;
  IN_TEMPLATE_MODE = 18;
  AFTER_BODY_MODE = 19;
  IN_FRAMESET_MODE = 20;
  AFTER_FRAMESET_MODE = 21;
  AFTER_AFTER_BODY_MODE = 22;
  AFTER_AFTER_FRAMESET_MODE = 23;
}

// Actions the tree builder can take.
enum TreeAction {
  TREE_ACTION_UNSPECIFIED = 0;
  TREE_ACTION_INSERT = 1;          // create element, push to open elements
  TREE_ACTION_INSERT_FOSTER = 2;   // foster parent insertion (table mode)
  TREE_ACTION_IGNORE = 3;          // ignore the token
  TREE_ACTION_PARSE_ERROR = 4;     // report parse error + ignore
  TREE_ACTION_REPROCESS = 5;       // reprocess token in new mode
  TREE_ACTION_POP = 6;             // pop current element
  TREE_ACTION_POP_UNTIL = 7;       // pop until matching element
  TREE_ACTION_RESET_INSERTION = 8; // reset insertion mode
  TREE_ACTION_ACKNOWLEDGE_SELF_CLOSING = 9;
  TREE_ACTION_SWITCH_TO_RAWTEXT = 10;   // switch tokenizer state
  TREE_ACTION_SWITCH_TO_RCDATA = 11;
  TREE_ACTION_SWITCH_TO_SCRIPT_DATA = 12;
  TREE_ACTION_APPEND_TEXT = 13;    // append text to current node
  TREE_ACTION_APPEND_COMMENT = 14; // append comment to current node
  TREE_ACTION_APPEND_DOCTYPE = 15; // append doctype
}

// A rule: given insertion mode + token type → actions + next mode.
message TreeRule {
  InsertionMode mode = 1;

  // Trigger: which token type this rule matches.
  oneof trigger {
    TokenType token_type = 2;
    string start_tag_name = 3;      // e.g. "div", "table", "p"
    string end_tag_name = 4;        // e.g. "body", "html", "p"
    bool any_start_tag = 5;         // matches any start tag not caught above
    bool any_end_tag = 6;           // matches any end tag not caught above
    bool character_token = 7;       // any character token
    bool comment_token = 8;         // any comment token
    bool doctype_token = 9;         // any doctype token
    bool eof_token = 10;            // EOF token
  }

  repeated TreeAction actions = 11;
  InsertionMode next_mode = 12;
  string spec_paragraph = 13;       // e.g. "13.2.6.4.16.7 In body"
}

// Optional element stack condition for the rule to apply.
message ConditionalRule {
  TreeRule rule = 1;
  StackCondition condition = 2;
}

// Condition on the stack of open elements.
message StackCondition {
  // "has element in scope" checks
  repeated string has_in_scope = 1;
  // "has element in button scope" checks
  repeated string has_in_button_scope = 2;
  // "has element in list item scope" checks
  repeated string has_in_list_scope = 3;
  // "has element in table scope" checks
  repeated string has_in_table_scope = 4;
  // "has element in select scope" checks
  repeated string has_in_select_scope = 5;
}

// Complete tree builder rule set.
message TreeBuilderRules {
  string spec_section = 1;
  repeated TreeRule rules = 2;
  repeated ConditionalRule conditional_rules = 3;
}
```

**`proto/edgerun/v0/html/entities.proto`**

Captures WHATWG §13.1.4.22 named character references:

```protobuf
syntax = "proto3";
package edgerun.v0.html.entities;

// A single named character reference.
message NamedEntity {
  string name = 1;          // e.g. "amp", "nbsp", "lt", "gt"
  uint32 code_point_1 = 2;  // primary code point (always present)
  uint32 code_point_2 = 3;  // secondary code point (0 if single-char entity)
  bool semicolon_required = 4;  // false for legacy entities like &notint;
}

// Complete entity map.
message EntityCatalog {
  string spec_section = 1;
  repeated NamedEntity entities = 2;
}
```

### Step 1: Write `scripts/generate_parser_ir.py`

**Input:** WHATWG HTML spec data (existing `html_element_catalog.json` + spec markdown)
**Output:** 3 new proto files above

This script encodes the WHATWG tokenizer and tree builder algorithms as proto data. It reads the spec and produces deterministic transition tables.

Key sections to encode:
- **Tokenizer (§13.2.5):** 80+ states × 19 character classes = ~1,520 transitions. Each transition is a simple lookup: `table[state][char_class] → (next_state, token_action)`.
- **Tree builder (§13.2.6):** 21 insertion modes × token types × tag names = ~3,000 rules. Most are "ignore" or "insert" — the complexity is in the edge cases (table mode, foster parenting, list item scope).
- **Entities (§13.1.4.22):** 2,231 named references. Most are simple `name → code_point`. A few have two code points (e.g., `&notindot;` → U+22F5 U+0338).

The script is a one-time effort. Once written, spec updates just regenerate the protos.

### Step 2: `buf generate` — Generate Rust Types

Run the existing `buf generate` pipeline. The 3 new proto files produce Rust structs in `edgerun-html-parser-types` (or extend existing `edgerun-html`):

```rust
// Generated from tokenizer_states.proto
pub enum TokenizerState {
    Unspecified = 0,
    Data = 1,
    RCDATA = 2,
    // ... 80+ states
}

pub struct StateTransitionTable {
    // Lookup: table[state as usize][char_class as usize]
    transitions: [[StateTransitionEntry; 19]; 85],
}
```

### Step 3: Write `scripts/generate_html_parser.py`

**Input:** The 3 proto files (or the generated Rust types)
**Output:** 5 Rust source files in `edgerun-html-render/src/`:

| Generated File | Lines | What It Contains |
|----------------|-------|------------------|
| `tokenizer.rs` | ~1,500 | `Tokenizer` struct with `fn step(&mut self, char) → Option<Token>`. State machine driven by `StateTransitionTable`. |
| `tree_builder.rs` | ~2,000 | `TreeBuilder` struct with `fn handle_token(&mut self, Token) → ()`. Insertion mode dispatch driven by `TreeBuilderRules`. |
| `entity_decoder.rs` | ~800 | `fn decode_entity(&mut self) → Option<String>`. Trie or perfect hash lookup for 2,231 entities. |
| `attribute_validator.rs` | ~1,000 | Per-element attribute validation from `html_attributes.proto`. |
| `html_parser.rs` | ~300 | Entry point: `pub fn parse_html(input: &str) -> Node`. Wires tokenizer → tree builder. Replaces current 140-line file. |

The generator reads proto data and emits Rust code. Every state transition, every rule, every entity reference becomes generated code.

### Step 4: Wire Into Existing Pipeline

Replace the current `parse_html()` call sites:
- `edgerun-demo/src/main.rs` — currently uses hardcoded HTML strings + ad-hoc parsing
- `edgerun-html-render/src/lib.rs` — exposes `parse_html()`
- `edgerun-cascade-debugger/src/main.rs` — hardcoded `simple_parse_html()`

The new `parse_html()` returns the same `Node` enum (Element/Text/Comment), so existing layout and rendering code needs zero changes.

### Step 5: Generate Conformance Tests

Extend `scripts/generate_conformance_tests.py` to emit parser tests:

```rust
// Generated from tokenizer_states.proto
#[test]
fn tokenizer_data_state_lt_emits_start_tag_token() {
    let mut t = Tokenizer::new("<div>");
    assert_eq!(t.step(), Some(Token::Character('<'))); // or consumed
    // ... verify transition matches spec
}

// Generated from tree_builder.proto
#[test]
fn tree_builder_in_body_div_inserts_element() {
    let dom = parse_html("<div>hello</div>");
    // ... verify DOM tree matches spec
}

// Generated from entities.proto
#[test]
fn entity_decode_amp() {
    let dom = parse_html("&amp;");
    assert_eq!(extract_text(&dom), "&");
}
```

Each test maps to a spec item in the conformance dashboard.

### Step 6: Build, Test, Iterate

```bash
# Generate proto files from spec data
python3 scripts/generate_parser_ir.py

# Generate Rust types from proto
buf generate

# Generate parser code
python3 scripts/generate_html_parser.py

# Build
cargo build

# Test
cargo test

# Check conformance dashboard
python3 scripts/generate_conformance_tests.py
cargo test --package edgerun-conformance
```

---

## What We Get

| Feature | Current Parser | Generated Parser |
|---------|---------------|------------------|
| Lines of code | ~140 | ~5,600 (generated) |
| Spec compliance | ~10% | ~95% |
| Entity decoding | ❌ | ✅ 2,231 entities |
| DOCTYPE parsing | ❌ | ✅ |
| Implicit tag closing | ❌ | ✅ |
| Foster parenting | ❌ | ✅ |
| Foreign content (SVG/MathML) | ❌ | ✅ (phase 2) |
| Error recovery | ❌ | ✅ parse errors + recovery |
| Spec-mapped decisions | ❌ | ✅ each rule traces to spec |
| Conformance tests | 0 | ~5,000 (generated) |
| Maintained by | Hand edits | Update proto → regenerate |
| Breaks project pattern | Yes | No — follows Spec→Proto→Code |

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Spec is ambiguous in places | Match Chrome/Firefox behavior for edge cases, document divergence |
| Tokenizer state machine is complex (~80 states) | Encode as data table, generator emits the code — no hand-writing |
| Tree builder has subtle scope rules (button scope, list scope, table scope) | Each scope is a separate proto message, generator emits the check |
| Entity list is large (2,231 entries) | Perfect hash function generated at build time, O(1) lookup |
| Generator script is a big one-time effort | Start with core 20 states + 5 insertion modes, expand incrementally |

---

## Implementation Order

1. **Week 1:** `tokenizer_states.proto` + `generate_parser_ir.py` for tokenizer only → generate `tokenizer.rs` → test token output
2. **Week 2:** `tree_builder.proto` + extend generator → generate `tree_builder.rs` → test DOM output for simple cases
3. **Week 3:** `entities.proto` + generate `entity_decoder.rs` → test all 2,231 entities
4. **Week 4:** Wire everything together → `parse_html()` → replace ad-hoc parser → conformance tests → dashboard update

Each phase delivers working code. No phase depends on a future phase being complete.

---

## The Bottom Line

This is not "let's write a better HTML parser." This is **compile the WHATWG spec into a program**. The spec is the source of truth. The parser is what happens when you run the compiler.

No browser does this. Chrome engineers hand-translate 11MB of spec prose into C++. You encode it once as proto data and generate the code. When the spec changes, you regenerate.

Innovation #1: Spec → Proto → Generated Code
Innovation #2: The generated parser is spec-compliant, self-testing, and auto-updating
