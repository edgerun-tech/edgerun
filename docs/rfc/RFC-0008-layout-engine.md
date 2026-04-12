# RFC-0008: Layout Engine

**Status:** Working Draft
**Date:** 2026-04-12

---

## Abstract

The layout engine implements a **two-pass block layout algorithm** with infrastructure for flex, grid, and incremental relayout. Currently only block layout is fully functional.

---

## Components

### position_layout.rs (Two-Pass Block Layout)

**Status:** ✅ Functional
**Tests:** 5 passing

#### Algorithm

```
Pass 1: measure_node() — Post-order traversal computing heights
  Text nodes: height = font.size × 1.25 × ceil(chars / chars_per_line)
  Containers: height = sum of children's heights + padding

Pass 2: position_node() — Pre-order traversal assigning Y positions
  X is fixed at margin
  Y accumulates from previous sibling heights
```

#### Public API

| Function | Description |
|----------|-------------|
| `position_tree(root, viewport_width, margin) -> Vec<PositionedNode>` | Entry point |
| `PositionedNode` | `x, y, width, height, kind` |
| `PositionedKind` | `Container { children, style }`, `TextRun { text, font, style }`, `Image { src, style }` |

#### Known Issues
1. **Text width estimation is crude** — `char_w = font.size * 0.5`, no actual font metrics
2. **Only block flow** — FlexContainer and GridContainer use same block algorithm as BlockContainer
3. **No position offsets** — relative/absolute/fixed/sticky offsets never applied
4. **Returns single root** — Top-level call returns `Vec` with 1 element

### layout_builder.rs (DOM → RenderObject)

**Status:** ✅ Functional

#### Public API

| Function | Description |
|----------|-------------|
| `build_layout(node, stylesheet, _viewport_width) -> RenderObject` | DOM + CSS → RenderObject tree |

#### Process
1. Recursively walk DOM nodes
2. Compute styles via `compute_style()` with parent inheritance
3. Detect block vs inline content
4. Determine formatting context and layout algorithm
5. Group mixed inline/block children into synthetic BlockContainer wrappers

#### Known Issues
1. **`_viewport_width` ignored** — Parameter prefixed with underscore
2. **Magic numbers** — `1` for block, `2` for inline instead of proper enum values
3. **No flex/grid layout algorithms** — Despite having `FlexContainer`/`GridContainer` variants

### incremental.rs (Dirty Subtree Tracking)

**Status:** ✅ Functional (standalone, not integrated into pipeline)
**Tests:** 3 passing

#### Architecture

```
mark_style_change(node_idx, nodes, affects_height):
  cascade_dirty ← descendants of node (style re-resolution needed)
  height_dirty ← ancestors of node (height recomputation if affects_layout)
  position_dirty ← following siblings (repositioning needed)
```

#### Public API

| Type | Description |
|------|-------------|
| `DomNodeRef` | `parent_idx, first_child_idx, next_sibling_idx` — index-based tree |
| `DirtySet` | Three dirty sets: cascade_dirty, height_dirty, position_dirty |
| `speedup_ratio(dirty, total) -> f64` | Estimated N× improvement over full relayout |

#### Test Results
- Leaf change: cascade={leaf}, height={ancestors}, position={leaf}
- Root change: cascade={all nodes}
- Sibling isolation: change on one sibling doesn't affect prior sibling

### budget.rs (Predictive Cost Estimator)

**Status:** ✅ Functional (standalone, not integrated)
**Tests:** 4 passing

#### Calibrated Costs
| Phase | Cost per Node |
|-------|--------------|
| Cascade | 0.01ms |
| Height | 0.005ms |
| Position | 0.008ms |

#### Public API

| Function | Description |
|----------|-------------|
| `LayoutBudget::estimate(graph, nodes, declarations) -> Self` | Predict cost |
| `is_cheap() -> bool` | Returns true if <10% of tree affected |
| `print_summary() -> String` | Human-readable breakdown |

### Other Layout Modules (All GENERATED)

| Module | Purpose |
|--------|---------|
| `render_object.rs` | Core types: `RenderObject`, `ComputedStyle`, `FontSelection`, `FormattingContext` |
| `layout_context.rs` | Dispatch: `determine_formatting_context()`, `determine_layout_algorithm()` |
| `box_model.rs` | `EdgeValues`, `establishes_bfc()`, `clips_overflow()` |
| `text_layout.rs` | `apply_text_transform()`, `should_wrap()`, `resolve_font_weight()` |
| `color_convert.rs` | `hsl_to_rgba()`, `oklch_to_rgba()` via `libm` |
| `paint_command.rs` | `PaintCommand` enum with FillRect, StrokeRect, DrawText, etc. |

---

## Dead Code in Layout (verified against source)

These modules are **fully defined but never called** in the layout pipeline:

| Module | Functions | Never Called Because |
|--------|-----------|---------------------|
| `box_model.rs` | `EdgeValues`, `establishes_bfc`, `clips_overflow` | Margin/border/padding not resolved from CSS; BFC detection not used in layout dispatch |
| `layout_context.rs` | `IntrinsicSizes`, `resolve_intrinsic_size`, `resolve_box_dimensions` | No intrinsic sizing keywords (auto, min-content, fit-content) used in layout |
| `text_layout.rs` | `apply_text_transform`, `should_wrap`, `resolve_font_weight` | Text processing not wired into layout pipeline |
| `color_convert.rs` | `hsl_to_rgba`, `oklch_to_rgba` | Color parsing falls to black for non-RGB; converters not called |
| `paint_command.rs` | `build_display_list` | Emits zero-sized rects; `render.rs` uses `RasterCommand` directly instead |

---

## Known Issues

1. **No flex algorithm** — `FlexContainer` uses same code as `BlockContainer`
2. **No grid algorithm** — `GridContainer` uses same code as `BlockContainer`
3. **No table layout** — `TableGrid` algorithm variant exists but no implementation
4. **No actual box model** — Margin/border/padding never resolved or applied
5. **No text wrapping** — `should_wrap` defined but never called
6. **No text transforms** — `apply_text_transform` defined but never called
7. **No transforms/opacity applied** — `has_transform`, `will_change`, `z_index`, `opacity` parsed but not used during painting
8. **Incremental layout not wired** — `DirtySet` exists but not integrated into render pipeline
9. **Budget estimator not called** — `LayoutBudget::estimate` exists but not called from render
