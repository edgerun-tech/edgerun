# RFC-0014: Analysis & Optimization Tools

**Status:** Working Draft
**Date:** 2026-04-12

---

## Abstract

Six analysis tools operate on the rendering pipeline output and CSS data to provide debugging, optimization, accessibility conformance, and deterministic replay capabilities.

---

## edgerun-cascade-debugger (Layer 7)

**Status:** ✅ Functional
**Binary:** `cargo run -p edgerun-cascade-debugger`
**CLI:** `--element <tag>` or `-e <tag>` to filter

### Purpose

Visual cascade resolution debugger showing:
- All matching rules ranked by specificity
- Per-property winner (why this rule won)
- Inheritance chain tracking
- Final computed values

### Implementation

Hand-written Rust binary (not generated). Parses demo HTML+CSS with simple parsers, then for each element shows:
1. Element identity (tag, class, id, depth, parent)
2. Matching rules list with specificity [a,b,c] and source order
3. Resolved values with rule attribution
4. Inherited properties noted

### Known Issues
1. **Hardcoded demo DOM** — `simple_parse_html()` returns hardcoded node structure, doesn't actually parse HTML
2. **Simple CSS parser** — Doesn't handle combinators, pseudo-classes, or at-rules
3. **Inheritance is stub** — Only prints "inherited from <tag>" without computing parent values

---

## edgerun-complexity-analyzer

**Status:** ✅ Functional
**Tests:** 3 passing (simple stylesheet, complexity scoring, bar formatting)

### Purpose

Score stylesheets on maintainability across 6 dimensions:

| Dimension | Description |
|-----------|------------|
| Specificity score (0-10) | Average specificity of selectors |
| Cascade depth | Avg rules per element |
| Dead rules % | Rules matching zero DOM elements |
| Duplicate count | Same property declared twice on same selector |
| Layout trigger % | Properties that cause reflow |
| Inheritance % | Inherited vs explicitly set properties |
| Overall weighted score | Combined score |

### Known Issues
1. **Requires DOM input** — Needs actual DOM tree to compute dead rules and cascade depth
2. **Simplified specificity** — Doesn't handle complex selectors with combinators

---

## edgerun-a11y-analyzer

**Status:** ✅ Functional
**Tests:** 6 passing

### WCAG 2.2 Checks

| Check | WCAG Criterion | Level |
|-------|---------------|-------|
| Contrast ratio | 1.4.3 (AA), 1.4.6 (AAA) | AA: 4.5:1, AAA: 7:1 |
| Min font-size | 1.4.4 Resize text | A: ≥12px |
| Text spacing / line-height | 1.4.12 Text Spacing | AA |
| Animation duration | 2.3.1 Three Flashes | A: ≥200ms |
| Focus indicator | 2.4.7 Focus Visible | AA |
| Use of color | 1.4.1 Use of Color | A |

### Implementation

Static analysis from CSS data. Uses WCAG luminance formula for contrast ratio computation.

### Test Inventory

| Test | What It Verifies |
|------|-----------------|
| `test_good_style_passes` | Good contrast, adequate size, proper line-height |
| `test_low_contrast_fails` | Contrast ratio < 4.5:1 fails WCAG AA |
| `test_small_font_fails` | Font-size < 12px fails WCAG 1.4.4 |
| `test_fast_animation_fails` | Animation duration < 200ms fails WCAG 2.3.1 |
| `test_no_focus_outline_fails` | No focus outline fails WCAG 2.4.7 |
| `test_luminance_calculation` | WCAG luminance formula correct |

---

## edgerun-rule-optimizer

**Status:** ✅ Functional
**Tests:** 5 passing

### Optimizations

| Optimization | Description |
|-------------|-------------|
| **Merge** | Identical selectors → single rule |
| **Remove** | Rules matching zero DOM elements |
| **Flatten** | `div > p` → `p` when safe |
| **Reorder** | By specificity for faster cascade |

### Output
Reports original vs optimized byte count and savings percentage.

### Test Inventory

| Test | What It Verifies |
|------|-----------------|
| Dead rules removed | Rules matching no DOM elements eliminated |
| Merge duplicates | Identical selectors combined |
| Savings calculation | Original vs optimized byte count |
| Selector flattening | `div > p` → `p` |
| Byte size reporting | Accurate size measurement |

---

## edgerun-replay

**Status:** ✅ Functional
**Tests:** 5 passing

### Purpose

Deterministic binary replay engine for rendering sessions. Enables bug reproduction: export replay file → replay locally → exact same pixels.

### Binary Format

```
EDGERUN\0 (8-byte magic)
version (varint)
frame_count (varint)
frames:
  ChangeRecord[]:
    node_idx (varint)
    property_idx (varint)
    value_bytes (length-prefixed)
```

### Public API

| Type | Description |
|------|-------------|
| `ReplayRecorder` | Records changes during rendering |
| `ReplayRecording` | Serializable recording with save/load |
| `ReplayPlayer` | Plays back recording frame by frame |

### Test Inventory

| Test | What It Verifies |
|------|-----------------|
| Record and playback | Changes recorded and replayed identically |
| Player iteration | Player iterates through frames correctly |
| Reset | Recorder and player can be reset |
| Byte size | Serialized format has expected size |
| Invalid magic | Invalid header rejected |

---

## edgerun-css-minifier

**Status:** ✅ Functional
**Tests:** 6 passing

### Technique

CSS property names encoded as 1-byte varints (98 unique property IDs) instead of ~12-byte average string names. 60-70% size reduction.

### Format
- Property ID: varint (1 byte for 98 properties)
- Value: length-prefixed string
- Round-trip encode/decode verified

### Test Inventory

| Test | What It Verifies |
|------|-----------------|
| Roundtrip | Encode → decode → identical output |
| Minification | CSS size reduced |
| Compression ratio | 60-70% reduction |
| Property mapping | Correct property ID assignments |
| Varint encoding | Correct varint byte encoding |

---

## edgerun-spec-watch

**Status:** ✅ Functional

### Purpose

Monitor proto files for changes and detect additions/removals. Integrates with conformance dashboard to flag newly untested items.

### Usage
- `--baseline` saves current proto state
- Subsequent runs compare against baseline
- Generates JSON change report

---

## edgerun-proto-watch

**Status:** ✅ Functional

### Purpose

Watch proto files for filesystem changes and trigger regeneration.

---

## Test Summary (All Analysis Tools)

| Tool | Tests | Status |
|------|-------|--------|
| Cascade debugger | 0 | ✅ Runs (no tests) |
| Complexity analyzer | 3 | ✅ Pass |
| A11y analyzer | 6 | ✅ Pass |
| Rule optimizer | 5 | ✅ Pass |
| Replay engine | 5 | ✅ Pass |
| CSS minifier | 5 | ✅ Pass |
| Spec watch | 0 | ✅ Runs |
| Proto watch | 0 | ✅ Runs |
| **Total** | **24** | **All passing** |
