# RFC-0007: CSS Cascade & Computed Style

**Status:** Working Draft
**Date:** 2026-04-12

---

## Abstract

Two CSS cascade implementations exist in the workspace:
1. **`edgerun-css-cascade`** — Full cascade engine with specificity, origins, layers, combinators, pseudo-classes (no_std, generated)
2. **`edgerun-render::css`** — Simple cascade used by the render pipeline (limited selector support)

---

## edgerun-css-cascade (Full Cascade Engine)

**Status:** ✅ Generated, `no_std`
**Tests:** 7 passing

### Purpose

Complete CSS cascade resolution from proto data, implementing the full CSS-cascade-5 algorithm:
- Origin ordering (UA → User → Author → Animation → Transition)
- Importance handling (normal vs `!important`)
- Cascade layers
- Specificity computation (a=IDs, b=classes/attributes/pseudo-classes, c=types/pseudo-elements)
- Source order tie-breaking

### Architecture

```
CSS string → CssParser → CascadeRule[] → CascadeStylesheet
CascadeStylesheet::resolve(DomElement) → BTreeMap<property, value>
```

### Public API

| Type | Description |
|------|-------------|
| `CascadeStylesheet` | Container for cascade rules with `resolve()` method |
| `CascadeRule` | Selector + declarations + origin + importance + layer |
| `CascadeDeclaration` | Single property declaration with full cascade key |
| `CascadeKey` | (origin, importance, layer_order, specificity, order) — fully ordered |
| `DomElement` | DOM element view for matching (tag, id, classes, attributes, parent, siblings) |
| `Specificity(u32, u32, u32)` | (a, b, c) tuple with add methods |
| `Origin` enum | Unspecified, Ua, User, Author, Animation, Transition |
| `Importance` enum | Unspecified, Normal, Important |
| `CascadeLayer` | Layer ordering enum |
| `Selector` | Sequence of `SelectorSequence` with combinators |
| `SelectorSequence` | `CompoundSelector` + optional `Combinator` |
| `CompoundSelector` | Vec of `SelectorComponent` |
| `SelectorComponent` | Type, Universal, Class, Id, Attr, PseudoClass, PseudoElement |
| `Combinator` | Descendant, Child, NextSibling, SubsequentSibling, Column |
| `PseudoClass` | Root, Empty, FirstChild, LastChild, NthChild(a,b), Hover, Focus, etc. |
| `AttrOp` | Exists, Equals, ContainsWord, PrefixHyphen, Prefix, Suffix, Contains |
| `parse_stylesheet(css, origin) -> CascadeStylesheet` | Parse CSS string into cascade stylesheet |
| `compute_specificity(selector) -> Specificity` | Compute (a,b,c) for selector |

### Cascade Ordering

The `CascadeKey` `Ord` implementation follows CSS spec:
1. Origin + Importance (combined rank: origin × 10 + importance)
2. Layer order
3. Specificity (a > b > c comparison)
4. Source order (later wins)

### Selector Matching

- Type selectors (case-insensitive tag name comparison)
- Class selectors (case-insensitive)
- ID selectors (exact match)
- Attribute selectors (all 7 operations: exists, equals, ~=, |=, ^=, $=, *=)
- Pseudo-class selectors (stub: always returns true)
- Pseudo-element selectors (ignored in matching)
- All 5 combinators (Descendant, Child, NextSibling, SubsequentSibling, Column)

### Test Inventory

| Test | What It Verifies |
|------|-----------------|
| `test_specificity` | add_id + add_class ×2 + add_type = (1,2,1) |
| `test_cascade_order` | Same specificity, later source order wins |
| `test_important_wins` | `!important` (0,0,1) beats normal (1,0,0) |
| `test_parse_stylesheet` | `"p { color: red; }"` parses to 1 rule, resolves correctly |
| `test_parse_multiple_rules` | Two rules parsed separately |
| `test_parse_whitespace_around_braces` | Whitespace handling |
| `test_cascade_order_wins` | Later rule wins for same selector |

---

## edgerun-render::css (Simple Cascade)

**Status:** ⚠️ Partial (used by render pipeline)
**Tests:** 17 passing (computed_style)

### Purpose

Simple CSS parsing for the render pipeline. Not a full cascade — handles basic selector matching and property resolution.

### Public API

| Function | Description |
|----------|-------------|
| `parse_css(css: &str) -> Stylesheet` | Parse CSS string |
| `Stylesheet::compute(tag, class, id) -> Declarations` | Cascade lookup for element |
| `compute_style(decls, inherited) -> ComputedStyle` | Resolve typed style with inheritance |
| `default_style() -> ComputedStyle` | Default: font-size=16, weight=400, opacity=1.0 |

### Supported Selectors
- `element` (e.g., `p`, `h1`, `div`)
- `.class` (e.g., `.title`, `.highlighted`)
- `#id`
- `element.class` (e.g., `h1.title`)
- `*` (universal)
- Comma-separated selectors

### NOT Supported
- Descendant combinators (`div p`)
- Child combinators (`div > p`)
- Sibling combinators (`p + p`, `p ~ p`)
- Attribute selectors (`[data-foo]`)
- Pseudo-classes (`:hover`, `:first-child`)
- Pseudo-elements (`::before`, `::after`)
- `@media`, `@import`, `@keyframes`

### Properties Resolved (~12 of 255)
| Property | Resolution |
|----------|-----------|
| `display` | → `FormattingContext` enum |
| `position` | → `PositionType` enum |
| `opacity` | Clamped 0-1 |
| `z-index` | `Option<i64>` (auto = None) |
| `color` | Named colors (52), hex via value-parser, fallback black for HSL |
| `background-color` | Same as color |
| `border-color` | Same as color |
| `border-width` | thin/medium/thick or numeric |
| `font-size` | px, em, rem, keywords (xx-small through xxx-large) |
| `font-weight` | 100-900 or normal/bold |
| `font-family` | First in comma-separated list, quotes stripped |
| `line-height` | Unitless multiplier, length, normal |

### Style Inheritance
- `em`/`rem` resolve against inherited parent font-size
- Unitless `line-height` multiplies computed font-size
- Parent style passed to `compute_style()` for relative unit resolution

---

## edgerun-property-graph (CSS Knowledge Graph)

**Status:** ✅ Functional
**Tests:** Part of edgerun-render budget tests

### Purpose

Query CSS property metadata as a graph:
- 98 properties with `affects_layout`, `affects_paint`, `inherits`, `animatable` flags
- Shorthand expansion (`font` → 6 longhands)
- Reverse dependency tracking

### Public API

| Function | Description |
|----------|-------------|
| `PropertyGraph::query().inherits(true).affects_layout(true).names()` | Filter properties |
| `graph.expand_shorthand("font")` | Expand shorthand to longhands |
| `graph.reverse_deps("font-size")` | Find all properties that depend on font-size |

---

## Known Issues

1. **Cascade engine not wired to render pipeline** — `edgerun-css-cascade` exists but `edgerun-render` uses its own simpler `css_parser.rs`
2. **Pseudo-class matching is a stub** — `pseudo_class_matches()` always returns `true`
3. **No @layer support in render cascade** — `edgerun-render::css_parser` doesn't handle `@layer`
4. **`is_valid_property` dead code** — Defined but never called
5. **Color conversion gap** — HSL/OKLCh fall to black despite `color_convert.rs` having conversion functions
