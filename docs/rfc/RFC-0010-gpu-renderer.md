# RFC-0010: GPU Renderer (wgpu)

**Status:** Working Draft
**Date:** 2026-04-12

---

## Abstract

`edgerun-wgpu` implements a **WebGPU fragment shader renderer** that replaces the CPU scanline rasterizer. It uses painter's algorithm (back-to-front sorted rectangles) in a single fragment shader pass. Additionally, it implements a **GPU CSS cascade + block layout compute pipeline** that runs CSS cascade resolution and height computation on the GPU via compute shaders.

---

## Architecture

### Render Pipeline (5 modules)

```
GpuRectStyle[] + GpuTextCommand[] → GpuRenderer
    ↓ upload to GPU buffers
Fragment shader (shaders/render.wgsl) → texture
Texture → readback buffer → Vec<u8> (RGBA8)
```

### Layout Compute Pipeline

```
GpuDomNode[] + GpuCssRule[] + text data → LayoutComputePipeline
    ↓ Pass 0: Cascade resolution (GPU)
    ↓ Pass 1: Style inheritance (GPU)
    ↓ Pass 2: Height computation ×2 (GPU)
    ↓ CPU: Sequential Y positioning (block_layout_y)
Vec<GpuLayoutResult> → GpuRectStyle[] → Render pipeline
```

---

## Components

### render.rs — High-Level Renderer

**Public API:**
- `struct GpuRenderer` — Complete GPU renderer
- `GpuRenderer::new(device, queue, w, h, font_data, font_size, text) -> Self`
- `GpuRenderer::render_and_readback(rects, text_cmds) -> Vec<u8>`
- `struct GpuGlyphInfo` — 32 bytes, matches WGSL GlyphInfo
- `fn render_to_pixels(w, h, rects, text_cmds, font_data, font_size, text) -> Vec<u8>` — Standalone convenience

**Resource Layout:**
| Buffer | Purpose | Size |
|--------|---------|------|
| uniform_buffer | Frame info | 16 bytes |
| rect_storage | Styled rects | 704 × 1024 max |
| text_storage | Text commands | 544 × 1024 max |
| glyph_info_buffer | Glyph atlas entries | 32 × 4096 |
| texture | Render target (RGBA8) | w × h |
| readback_buffer | CPU readback | w × h × 4 (aligned) |

**Bind Group Layout:**
| Binding | Resource |
|---------|----------|
| 0 | Uniforms (uniform buffer) |
| 1 | Rects storage (read-only storage buffer) |
| 2 | Text commands storage (read-only storage buffer) |
| 3 | Font atlas texture (sampled) |
| 4 | Font sampler (linear filtering) |
| 5 | Glyph info buffer (read-only storage buffer) |

### uniforms.rs — GPU Data Types

**Key Types:**

| Type | Size | Description |
|------|------|-------------|
| `GpuRectStyle` | 704 bytes | Full styled rect: bg_color, gradients (16 stops), border (width/color/style/radius), shadows (color/blur/spread/offset/type), opacity, blend_mode, x/y/w/h, paint_order |
| `GpuGradientStop` | 32 bytes | RGBA + position + 3 pad |
| `GpuUniforms` | 16 bytes | fb_width, fb_height, rect_count, text_cmd_count |
| `GpuTextCommand` | 544 bytes | x, y, color, 128 glyph indices, count |
| `GpuDomNode` | 32 bytes | tag_hash, class_hash, id_hash, parent/child/sibling indices, text offset/len |
| `GpuCssRule` | 96 bytes | tag/class/id hashes, specificity (a,b,c), 10 declarations, source_order, is_important |
| `GpuStyleResult` | 72 bytes | Computed style from GPU cascade |
| `GpuLayoutResult` | 88 bytes | Layout output: x/y/w/h, content box, styles, margin/padding |

**Constructors on `GpuRectStyle`:**
- `solid(x, y, w, h, r, g, b, a, paint_order)`
- `with_border(x, y, w, h, r, g, b, a, border_w, border_style, br, bg, bb, radius, paint_order)`
- `linear_gradient(x, y, w, h, angle, stops, paint_order)`
- `radial_gradient(x, y, w, h, cx, cy, stops, paint_order)`
- `conic_gradient(x, y, w, h, from_angle, cx, cy, stops, paint_order)`
- `with_shadow(x, y, w, h, r, g, b, a, shadow_rgba, blur, spread, offset, type, paint_order)`

### pipeline.rs — WGPU Pipeline Creation

**Shader:** `shaders/render.wgsl` loaded via `include_str!`

**Pipeline:**
- Full-screen triangle (3 vertices, no vertex buffers)
- Fragment shader does all the work (painter's algorithm)
- No blend state — blending done in shader
- Format: Rgba8Unorm

### layout_compute.rs — GPU CSS Cascade + Layout

**Public API:**
- `struct LayoutComputePipeline` — GPU compute pipeline
- `LayoutComputePipeline::new(device, max_nodes, max_rules) -> Self`
- `LayoutComputePipeline::run_full_layout(device, queue, nodes, rules, text, avail_width, base_y) -> Vec<GpuLayoutResult>`
- `struct LayoutConfig` — node_count, rule_count, text_len, compute_phase, avail_width, base_x, base_y

**Three GPU Passes + CPU Y Positioning:**

| Pass | compute_phase | What it does |
|------|--------------|-------------|
| 0 | Cascade | Per-node CSS cascade: match rules, resolve by specificity/importance |
| 1 | Inheritance | Propagate font-size and color from parent to children |
| 2 | Heights ×2 | First pass: leaf heights; Second pass: sum children heights |
| CPU | — | `block_layout_y()` — sequential Y positioning from heights |

**Why CPU for Y?** — WGSL can't recurse; block layout Y accumulation is inherently sequential.

### font_atlas.rs — TTF Glyph Rasterization

**Public API:**
- `struct FontAtlas` — 512×512 RGBA8 texture
- `FontAtlas::new(device, font_data, font_size, text) -> Self`
- `FontAtlas::upload(queue)` — Upload pixels to GPU texture
- `glyphs: BTreeMap<char, GlyphEntry>` — Atlas coordinates per character

**Implementation:** Uses `fontdue` crate to rasterize TTF glyphs into bitmap, then packs into 512×512 texture atlas.

---

## WGSL Shaders

### shaders/render.wgsl (Fragment Shader)

**Technique:** Painter's algorithm — every fragment invocation iterates through all rectangles back-to-front:

```wgsl
@fragment fn fs_main(pos: vec4f) -> @location(0) vec4f {
    var color = background;
    for rect in rects (back-to-front) {
        if contains(pixel, rect) {
            color = compute_pixel(pixel, rect);
        }
    }
    return color;
}
```

**Features:**
- Solid fills (bg_color)
- Linear/radial/conic gradients
- Border radius (SDF containment test)
- Border patterns (dash patterns from border_lut)
- Box shadows (drop and inset)
- Opacity blending
- Blend modes (16 modes)
- Text rendering (font atlas texture sampling)

### shaders/layout.wgsl (Compute Shader)

**Technique:** Per-node workgroup — each GPU thread handles one DOM node:

```wgsl
@compute @workgroup_size(64) fn main() {
    let node_idx = global_invocation_id.x;
    match config.compute_phase {
        0 => cascade_resolve(node_idx);
        1 => inherit_styles(node_idx);
        2 => compute_height(node_idx);
    }
}
```

**Phase 0 (Cascade):**
- For each node, iterate all CSS rules
- Match by tag_hash, class_hash, id_hash
- Resolve by specificity (a,b,c) + source order
- Handle `!important` via bitmask

**Phase 1 (Inheritance):**
- Propagate font-size and color from parent
- Skip if node has explicit values

**Phase 2 (Heights):**
- Run twice: first computes leaf heights, second sums children
- Uses parent_idx/first_child_idx/next_sibling_idx for tree traversal

---

## Known Issues

1. **No blend modes in fragment shader** — Despite supporting 16 blend modes in the CPU rasterizer, the WGSL shader does `mix()` only (normal blending)
2. **Box shadows unimplemented** — Shadow fields exist in `GpuRectStyle` but WGSL doesn't compute them
3. **Font atlas is 512×512 fixed** — No dynamic sizing, no multi-atlas for large fonts
4. **`pollster::block_on` everywhere** — Synchronous GPU rendering, no async pipeline
5. **Hardcoded buffer sizes** — 1024 rects max, 1024 text commands max, 4096 glyphs max
6. **No damage tracking** — Full framebuffer redraw every frame
7. **No text shaping** — Char code points directly index glyph_info buffer; no ligatures, kerning, or substitution
8. **CPU Y positioning bottleneck** — GPU can't do sequential accumulation; CPU must do block layout Y pass
9. **`render.wgsl` loaded at compile time** — `include_str!` means shader changes require rebuild
