# RFC-0009: CPU Rasterizer

**Status:** Working Draft
**Date:** 2026-04-12

---

## Abstract

`edgerun-rasterizer` is a **scanline renderer generated from CSS proto data**. It implements solid fills, 3 gradient types, 9 border patterns, 16 blend modes, bitmap text, and tile-based multicore rendering — all in `no_std` with AVX2 SIMD support.

**All files are auto-generated** from CSS proto data via `scripts/generate_rasterizer.py`.

---

## Architecture

```
CSS proto data (css_colors, css_images, css_box, css_text, etc.)
    ↓ scripts/generate_rasterizer.py
11 generated Rust modules
    ↓
RasterCommand enum → scanline::rasterize() → Framebuffer (XRGB8888)
```

---

## Modules

### scanline.rs — Main Rasterizer

**Public API:**
- `enum RasterCommand` — FillRect, StrokeRect, Text, LinearGradient, RadialGradient, ConicGradient, PushClip, PopClip, PushOpacity, PopOpacity
- `fn rasterize(fb: &mut Framebuffer, commands: &[RasterCommand])` — Main dispatch
- `fn cmd_fill(x, y, w, h, r, g, b) -> RasterCommand` — Convenience constructor
- `fn cmd_text(x, y, text, r, g, b) -> RasterCommand` — Convenience constructor
- `fn rasterize_tile(fb, commands)` — Tile-local rendering

**Dispatch Logic:**
- FillRect: AVX2 or scalar based on compile-time `#[cfg(target_feature = "avx2")]`
- StrokeRect → `rect::stroke_rect()`
- Text → `text_bitmap::draw_text()`
- LinearGradient/RadialGradient/ConicGradient → `gradient::*`
- Clip/Opacity → stub (no-op in tiled mode)

**SIMD Dispatch:**
```rust
#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
unsafe { crate::simd_blend::fill_rect_avx2(...) }

#[cfg(not(all(target_arch = "x86_64", target_feature = "avx2")))]
unsafe { scalar loop }
```

### framebuffer.rs — Pixel Buffer

**Format:** XRGB8888 (little-endian: [B, G, R, X]), matching DRM dumb buffer layout

**Public API:**
- `Framebuffer::new(pixels, width, height) -> Self`
- `fill_solid(x, y, w, h, r, g, b)` — Opaque fill
- `fill_alpha(x, y, w, h, r, g, b, alpha)` — Alpha-blended fill
- `set_pixel_alpha(x, y, r, g, b, alpha)` — Single pixel with alpha
- `clear()` — Clear to `#1a1a2e` default background

### gradient.rs — CSS Gradients

**Generated from:** css_images.proto (GradientKind, LinearGradientDef, etc.)

**Public API:**
- `struct GradientStop { r, g, b, a, position: f64 }`
- `linear_gradient(pixels, stride, fb_w, fb_h, x, y, w, h, angle, stops)`
- `radial_gradient(pixels, stride, fb_w, fb_h, x, y, w, h, cx, cy, stops)`
- `conic_gradient(pixels, stride, fb_w, fb_h, x, y, w, h, from_angle, cx, cy, stops)`

**Implementation:** Per-pixel `f64` trig + sqrt. No SIMD acceleration. Uses `libm` for `sin`, `cos`, `atan2`, `sqrt`.

### tile.rs — Tile Multicore Rendering

**Public API:**
- `struct Tile { x, y, w, h, fb_byte_offset, fb_stride }`
- `compute_tiles(width, height, stride, num_tiles) -> Vec<Tile>` — Horizontal stripe decomposition
- `filter_commands_for_tile(commands, tile) -> Vec<RasterCommand>` — Clip commands to tile bounds
- `suggested_tile_count(height) -> usize` — Default 4-16 tiles

**Parallel Usage Pattern:**
```rust
// With std:
let tiles = compute_tiles(w, h, stride, num_threads);
scope(|s| {
    for tile in &tiles {
        s.spawn(|| render_tile(&fb.pixels, tile, &commands));
    }
});

// no_std: render tiles sequentially
```

**Performance:** ~6× speedup on 6-core CPU (from DESIGN.md)

### color_lut.rs — Named Colors (148 colors)

**Generated from:** css_colors.proto

**Public API:**
- `const NAMED_COLOR_COUNT: usize = 148`
- `static COLOR_LUT: [[u8; 4]; 148]` — [R, G, B, A] per color index
- `named_color(index: usize) -> (u8, u8, u8, u8)`

**Issues:** Duplicate entries in the table (white=1 and white=101, black=0 and black=102, etc.) — proto enum has 148 entries but many are duplicates.

### border_lut.rs — Border Patterns (9 styles)

**Generated from:** css_box.proto

**Public API:**
- `const DASHED_PATTERN: [u8; 8]` — 4 on, 4 off
- `const DOTTED_PATTERN: [u8; 8]` — 1 on, 3 off
- `const DOUBLE_PATTERN: [u8; 8]` — 2 on, 1 off, 2 on, 3 off
- `const SOLID/GROOVE/RIDGE/INSET/OUTSET/NONE_PATTERN` — Various
- `border_pattern(style: u8) -> &'static [u8; 8]`
- `draw_horizontal_line(pixels, stride, x, y, width, r, g, b, pattern)`

### blend_lut.rs — Blend Modes (16 modes)

**Generated from:** CSS Compositing Level 1

**Public API:**
- `blend_normal`, `blend_multiply`, `blend_screen`, `blend_overlay`
- `blend_darken`, `blend_lighten`, `blend_colordodge`, `blend_colorburn`
- `blend_hardlight`, `blend_softlight`, `blend_difference`, `blend_exclusion`
- `blend_hue`, `blend_saturation`, `blend_color`, `blend_luminosity`
- `alpha_blend(src, dst, alpha)` — Integer alpha blending

All per-channel functions use integer math only — no floats.

### simd_blend.rs — AVX2 Alpha Blending

**Generated from:** CSS compositing spec

**Public API:**
- `fill_solid_8_avx2(dst, color)` — 8 opaque pixels
- `fill_rect_avx2(fb, stride, x, y, w, h, color)` — Rectangle fill, 8 pixels/iter
- `blend_8_avx2(src, dst)` — Alpha-blend 8 pixels
- `blend_rect_avx2(src, dst, count)` — Blend rectangle, 8 pixels/iter + scalar remainder

**Implementation:** `_mm256` AVX2 intrinsics. Unpack to 16-bit, multiply by alpha, pack back to 8-bit.

### text_bitmap.rs — 8×8 Bitmap Font

**Generated from:** css_text.proto

**Font:** 128 glyphs, 8×8 pixels each. Only ASCII digits (0-9) and lowercase letters (a-z) have real bitmap data. All other characters (uppercase, punctuation, symbols) render as a box pattern (`0xFF, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0xFF`).

**Public API:**
- `const FONT_8X8: [[u8; 8]; 128]`
- `draw_char(pixels, stride, fb_w, fb_h, ch, x, y, r, g, b)`
- `draw_text(pixels, stride, fb_w, fb_h, text, x, y, r, g, b)` — 8px advance per character

### rect.rs — Rectangle Borders

**Generated from:** css_box.proto

**Public API:**
- `fill_rect(fb, x, y, w, h, r, g, b, alpha)`
- `stroke_rect(fb, x, y, w, h, r, g, b, style, thickness)` — 4 borders with pattern

---

## Performance (from DESIGN.md)

| Test | Throughput | FPS @ 4K |
|------|-----------|----------|
| Solid fills (AVX2) | 2.8 Gpix/sec | 340 |
| Solid fills (scalar) | 3.4 Gpix/sec | 407 |
| Gradients | ~0.1 Gpix/sec | 12 |

LLVM auto-vectorizes the scalar loop so well that AVX2 adds no benefit for solid fills.

---

## Known Issues

1. **AVX2 slower than scalar** — LLVM's auto-vectorization is better than hand-written AVX2
2. **Gradient math is per-pixel f64** — `sin`, `cos`, `atan2`, `sqrt` per pixel — no SIMD possible
3. **Clip/Opacity no-ops** — `PushClip`, `PopClip`, `PushOpacity`, `PopOpacity` matched but not executed
4. **Bitmap font is unusable** — 8×8 monospace, most glyphs are boxes, no antialiasing
5. **Duplicate color entries** — 148-entry LUT has ~20 duplicates
6. **No border-radius** — Rounded rectangles not implemented
7. **No box shadows** — Not implemented
8. **All code generated** — No hand-written logic; if the proto data is wrong, the generated code is wrong
