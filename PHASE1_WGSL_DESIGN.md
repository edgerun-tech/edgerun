# Phase 1: Proto → WGSL Codegen Design

## Goal

Generate a WGSL fragment shader from proto data that replaces the CPU scanline rasterizer
(`edgerun-rasterizer/scanline.rs`) with a GPU fragment shader. The same proto that currently
generates Rust LUTs and functions also generates WGSL code.

## Architecture

```
proto/edgerun/v0/
  css_images.proto      (GradientKind, LinearGradientDef, CssColorStop)
  css_value_types.proto (CssColor, CssRgb, CssHsl, etc.)
  css_colors.proto      (NamedColor enum — 148 colors)
  css_properties.proto  (CssProperty enum — 175 properties)
  css_box.proto         (BoxModel, BoxEdge)
        ↓
  codegen/wgsl_gen.py   (NEW — reads proto, emits .wgsl)
        ↓
  shaders/render.wgsl   (GENERATED — fragment shader)
        ↓
  crates/edgerun-wgpu   (NEW — WebGPU host: uniforms, pipeline, render pass)
        ↓
  edgerun-demo-wgpu     (HTML+CSS → layout → upload uniforms → draw → save PNG)
```

## Key Design Decision: "Painter's Algorithm on GPU"

The current rasterizer loops through `RasterCommand` enums on CPU. On GPU, we flip this:

**Every fragment shader invocation** iterates through all styled rectangles back-to-front,
testing containment and computing the topmost pixel color. This is the painter's algorithm
in a single pass — simple, correct, and parallelizable by the GPU across all pixels.

```
Fragment at (px, py):
  color = canvas_background
  for rect in rects:            // back-to-front paint order
    if contains(px, py, rect):
      color = compute_pixel(px, py, rect)
  output = color
```

## WGSL Shader Structure (Generated)

### Uniform Buffer Layout

```wgsl
// Generated from css_properties.proto + css_box.proto
struct RectStyle {
    bg_color: vec4f,                    // CssColor → RGBA
    gradient_type: u32,                 // 0=none, 1=linear, 2=radial, 3=conic (from GradientKind)
    gradient_angle: f32,                // LinearGradientDef.angle
    gradient_cx: f32,                   // Radial/ConicGradientDef center X
    gradient_cy: f32,                   // Radial/ConicGradientDef center Y
    gradient_stops: array<GradientStop, 16>,  // CssColorStop[] → resolved RGBA stops
    gradient_stop_count: u32,
    border_width: vec4<f32>,            // BORDER_WIDTH top/right/bottom/left
    border_color: vec4<f32>,            // BORDER_COLOR
    border_style: vec4<u32>,            // 0-8 from border_lut (solid, dashed, etc.)
    border_radius: vec4<f32>,          // BORDER_RADIUS top-left/top-right/bottom-right/bottom-left
    opacity: f32,                        // OPACITY property
    blend_mode: u32,                     // 0-16 from blend_lut
    // --- layout computed, not from cascade ---
    x: f32, y: f32, w: f32, h: f32,    // box model positions
    paint_order: u32,                   // for back-to-front sort
};

struct Uniforms {
    rect_count: u32,
    rects: array<RectStyle, 256>,       // max 256 styled rects
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
```

### Generated Fragment Shader

The shader body is **generated** from proto data. Each section maps to a proto concept:

```wgsl
@fragment
fn fs_main(@builtin(position) pos: vec4f) -> @location(0) vec4f {
    let px = pos.x;
    let py = pos.y;
    var color = vec4f(0.102, 0.102, 0.180, 1.0);  // #1a1a2e default background

    for (var i: u32 = 0u; i < uniforms.rect_count; i = i + 1u) {
        let r = uniforms.rects[i];

        // --- Generated from css_box.proto SDF logic ---
        // Rounded rectangle containment test
        let corner_x = select(r.x + r.border_radius.x, r.x + r.w - r.border_radius.y, px > r.x + (r.w / 2.0));
        let corner_y = select(r.y + r.border_radius.x, r.y + r.h - r.border_radius.z, py > r.y + (r.h / 2.0));
        let cx = clamp(px, r.x + r.border_radius.x, r.x + r.w - r.border_radius.y);
        let cy = clamp(py, r.y + r.border_radius.x, r.y + r.h - r.border_radius.z);
        let dist = distance(vec2f(px, py), vec2f(select(cx, corner_x, px < r.x + r.border_radius.x || px > r.x + r.w - r.border_radius.y), select(cy, corner_y, py < r.y + r.border_radius.x || py > r.y + r.h - r.border_radius.z)));

        // --- Border containment test (from border_lut proto data) ---
        let is_border = dist <= max(r.border_width.x, r.border_width.y) &&
                        dist > max(r.border_width.x, r.border_width.y) - r.border_width.x;

        if (dist <= r.border_radius.x || /* inside rounded rect */) {
            var pixel_color: vec4f;

            // --- Generated from css_images.proto GradientKind ---
            switch r.gradient_type {
                case 0u: {  // None — solid bg_color
                    pixel_color = r.bg_color;
                }
                case 1u: {  // LinearGradient
                    let t = linear_gradient_t(px, py, r);
                    pixel_color = sample_stops(r.gradient_stops, r.gradient_stop_count, t);
                }
                case 2u: {  // RadialGradient
                    let t = radial_gradient_t(px, py, r);
                    pixel_color = sample_stops(r.gradient_stops, r.gradient_stop_count, t);
                }
                case 3u: {  // ConicGradient
                    let t = conic_gradient_t(px, py, r);
                    pixel_color = sample_stops(r.gradient_stops, r.gradient_stop_count, t);
                }
                default: {
                    pixel_color = r.bg_color;
                }
            }

            // --- Generated from blend_lut (16 modes from proto) ---
            pixel_color = apply_blend_mode(pixel_color, color, r.blend_mode);

            // --- Opacity ---
            color = mix(color, pixel_color, r.opacity);
        }
    }

    return color;
}
```

## Codegen: What Generates What

| Proto Source | WGSL Output |
|---|---|
| `css_colors.proto` → NamedColor enum | `const NAMED_COLORS: array<vec4f, 148>` |
| `css_images.proto` → GradientKind, LinearGradientDef, etc. | `fn linear_gradient_t()`, `fn radial_gradient_t()`, `fn conic_gradient_t()` |
| `css_images.proto` → CssColorStop | `fn sample_stops()` with lerp |
| `css_box.proto` → BorderStyle enum | `fn draw_border_pattern()` with dash patterns |
| `css_properties.proto` → OPACITY, BLEND_MODE | Uniform struct fields |
| `css_value_types.proto` → CssColor oneof | `fn color_from_rgb()`, `fn color_from_hsl()`, etc. |
| border_lut data (9 patterns) | `const BORDER_PATTERNS: array<array<u32, 8>, 9>` |
| blend_lut data (16 modes) | `fn apply_blend_mode()` with switch on mode index |

## Codegen Script: `scripts/generate_wgsl.py`

```python
#!/usr/bin/env python3
"""
Generate WGSL fragment shader from proto data.

Reads:
  - proto/edgerun/v0/css/css_images.proto (GradientKind, stops)
  - proto/edgerun/v0/css/css_colors.proto (NamedColor enum)
  - proto/edgerun/v0/css/css_box.proto (BorderStyle, BoxModel)
  - proto/edgerun/v0/css/css_properties.proto (OPACITY, BLEND_MODE)
  - crates/edgerun-rasterizer/src/color_lut.rs (148 RGBA values)
  - crates/edgerun-rasterizer/src/border_lut.rs (9 dash patterns)
  - crates/edgerun-rasterizer/src/blend_lut.rs (16 blend functions)

Emits:
  - shaders/render.wgsl (complete fragment shader)
"""
```

The script:
1. Parses proto files with `protoc --decode` or regex-based extraction
2. Reads existing Rust LUTs to get exact RGBA values and patterns
3. Assembles WGSL code using template strings
4. Writes `shaders/render.wgsl`

### Example: Generating Named Colors

```python
def gen_color_lut(wgsl):
    """css_colors.proto + color_lut.rs → WGSL const array"""
    wgsl.append("const NAMED_COLORS: array<vec4f, 148> = array<vec4f, 148>(")
    for i, (name, r, g, b, a) in enumerate(COLOR_LUT):
        wgsl.append(f"  vec4f({r/255:.4f}, {g/255:.4f}, {b/255:.4f}, {a/255:.4f}),  // {i}: {name}")
    wgsl.append(");")
```

### Example: Generating Blend Mode Functions

```python
def gen_blend_modes(wgsl):
    """blend_lut.rs → WGSL switch statement"""
    wgsl.append("fn apply_blend_mode(src: vec4f, dst: vec4f, mode: u32) -> vec4f {")
    wgsl.append("  var result = src;")
    wgsl.append("  switch mode {")
    for i, (name, impl) in enumerate(BLEND_FUNCTIONS):
        wgsl.append(f"    case {i}u: {{  // {name}")
        wgsl.append(f"      result = vec4f({impl.wgsl_expr('src.r', 'dst.r')}, "
                    f"{impl.wgsl_expr('src.g', 'dst.g')}, "
                    f"{impl.wgsl_expr('src.b', 'dst.b')}, src.a);")
        wgsl.append("    }")
    wgsl.append("    default: { result = src; }")
    wgsl.append("  }")
    wgsl.append("  return result;")
    wgsl.append("}")
```

## WGSL Gradient Functions (Generated from css_images.proto)

```wgsl
// Generated from css_images.proto → LinearGradientDef
fn linear_gradient_t(px: f32, py: f32, r: RectStyle) -> f32 {
    let rel_x = (px - r.x) / r.w;
    let rel_y = (py - r.y) / r.h;
    let cos_a = cos(r.gradient_angle);
    let sin_a = sin(r.gradient_angle);
    let t = (rel_x * cos_a + rel_y * sin_a) * 0.5 + 0.5;  // maps [-1,1] to [0,1]
    return clamp(t, 0.0, 1.0);
}

// Generated from css_images.proto → RadialGradientDef
fn radial_gradient_t(px: f32, py: f32, r: RectStyle) -> f32 {
    let cx = r.x + r.w * r.gradient_cx;
    let cy = r.y + r.h * r.gradient_cy;
    let dx = (px - cx) / (r.w * 0.5);
    let dy = (py - cy) / (r.h * 0.5);
    let dist = sqrt(dx * dx + dy * dy);
    return clamp(dist, 0.0, 1.0);
}

// Generated from css_images.proto → ConicGradientDef
fn conic_gradient_t(px: f32, py: f32, r: RectStyle) -> f32 {
    let cx = r.x + r.w * r.gradient_cx;
    let cy = r.y + r.h * r.gradient_cy;
    let angle = atan2(py - cy, px - cx) + r.gradient_angle;
    var t = (angle / 6.283185307179586) + 0.5;  // 2*PI
    if (t < 0.0) { t = t + 1.0; }
    if (t > 1.0) { t = t - 1.0; }
    return t;
}

// Generated from css_images.proto → CssColorStop
fn sample_stops(stops: array<GradientStop, 16>, count: u32, t: f32) -> vec4f {
    if (count == 0u) { return vec4f(1.0, 0.0, 0.0, 1.0); }
    if (count == 1u) { return stops[0].color; }

    // Find bracketing stops
    var prev = stops[0];
    var next = stops[1];
    for (var i: u32 = 1u; i < count; i = i + 1u) {
        if (stops[i].position >= t) {
            next = stops[i];
            break;
        }
        prev = stops[i];
        next = stops[i];
    }

    let range = next.position - prev.position;
    var local_t = 0.0;
    if (range > 0.0001) {
        local_t = (t - prev.position) / range;
    }
    return mix(prev.color, next.color, local_t);
}
```

## Border Dash Patterns (Generated from border_lut proto data)

```wgsl
// Generated from border_lut.rs — 9 patterns × 8 bytes
const BORDER_PATTERNS: array<array<u32, 8>, 9> = array<array<u32, 8>, 9>(
  array<u32, 8>(0u,0u,0u,0u, 0u,0u,0u,0u),  // 0: none
  array<u32, 8>(1u,1u,1u,1u, 1u,1u,1u,1u),  // 1: solid
  array<u32, 8>(1u,1u,1u,1u, 0u,0u,0u,0u),  // 2: dashed
  array<u32, 8>(1u,0u,0u,0u, 1u,0u,0u,0u),  // 3: dotted
  array<u32, 8>(1u,1u,0u,1u, 1u,0u,0u,0u),  // 4: double
  array<u32, 8>(1u,0u,1u,0u, 1u,0u,1u,0u),  // 5: groove
  array<u32, 8>(1u,0u,1u,0u, 1u,0u,1u,0u),  // 6: ridge
  array<u32, 8>(1u,0u,1u,0u, 1u,0u,1u,0u),  // 7: inset
  array<u32, 8>(1u,0u,1u,0u, 1u,0u,1u,0u),  // 8: outset
);

fn border_dash(pixel_pos: u32, style: u32) -> bool {
    let pattern = BORDER_PATTERNS[style];
    return pattern[pixel_pos % 8u] == 1u;
}
```

## The edgerun-wgpu Crate (Host-Side Rust)

```rust
// crates/edgerun-wgpu/src/lib.rs
#![no_std]  // or std for initial prototype

pub mod pipeline;   // WebGPU pipeline creation
pub mod uniforms;   // RectStyle struct, uniform buffer management
pub mod render;     // High-level: given Vec<RenderObject>, draw to texture
```

```rust
// crates/edgerun-wgpu/src/uniforms.rs

#[repr(C)]
pub struct GradientStop {
    pub color: [f32; 4],   // RGBA
    pub position: f32,
    pub _pad: [u32; 3],    // WGSL alignment
}

#[repr(C)]
pub struct RectStyle {
    pub bg_color: [f32; 4],
    pub gradient_type: u32,
    pub gradient_angle: f32,
    pub gradient_cx: f32,
    pub gradient_cy: f32,
    pub gradient_stops: [GradientStop; 16],
    pub gradient_stop_count: u32,
    pub border_width: [f32; 4],
    pub border_color: [f32; 4],
    pub border_style: [u32; 4],
    pub border_radius: [f32; 4],
    pub opacity: f32,
    pub blend_mode: u32,
    pub x: f32, pub y: f32, pub w: f32, pub h: f32,
    pub paint_order: u32,
    pub _pad: [u32; 3],
}

// Uniform buffer size: sizeof(RectStyle) * 256 + 4 bytes (rect_count)
pub const UNIFORM_SIZE: usize = ...;
```

```rust
// crates/edgerun-wgpu/src/pipeline.rs

pub fn create_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("edgerun-fragment"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/render.wgsl").into()),
    });

    // Full-screen triangle — fragment shader does all the work
    let pipeline_layout = device.create_pipeline_layout(...);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        vertex: wgpu::VertexState { module: &shader, entry_point: "vs_main", ... },
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: "fs_main", ... }),
        primitive: wgpu::PrimitiveState { topology: TriangleList, ... },
        ..Default::default()
    })
}
```

## WGSL Vertex Shader (Minimal — Full-Screen Triangle)

```wgsl
// Also generated by scripts/generate_wgsl.py

struct VertexOutput {
    @builtin(position) position: vec4f,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Emit a full-screen triangle (3 vertices)
    let x = f32((vertex_index << 1u) & 2u) - 1.0;
    let y = f32(vertex_index & 2u) - 1.0;
    return VertexOutput(vec4f(x, y, 0.0, 1.0));
}
```

## Demo Integration: `edgerun-demo-wgpu`

```rust
// crates/edgerun-demo-wgpu/src/main.rs

fn main() {
    // 1. Parse HTML + CSS (same as current demo)
    let dom = parse_html(HTML);
    let sheet = parse_css(CSS, Origin::Author);

    // 2. Compute layout (same two-pass block layout)
    let rects = measure_and_paint(&dom, &sheet);  // Vec<(Rect, ComputedStyle)>

    // 3. Initialize WebGPU
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&...));
    let device = pollster::block_on(adapter.request_device(&...));

    // 4. Create render pipeline (loads generated WGSL)
    let pipeline = edgerun_wgpu::create_pipeline(&device);

    // 5. Upload rects as uniforms
    let uniform_buf = edgerun_wgpu::upload_rects(&device, &rects);

    // 6. Render to texture
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d { width: 960, height: 640, depth_or_array_layers: 1 },
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        ..Default::default()
    });

    let mut encoder = device.create_command_encoder(&...);
    {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture.create_view(),
                load_op: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store_op: wgpu::StoreOp::Store,
            })],
            ..Default::default()
        });
        rpass.set_pipeline(&pipeline);
        rpass.set_bind_group(0, &uniform_bind_group, &[]);
        rpass.draw(0..3, 0..1);  // full-screen triangle
    }

    // 7. Read back to CPU, save as PNG
    let buffer = copy_texture_to_buffer(&device, &mut encoder, &texture);
    let queue = device.queue();
    queue.submit(Some(encoder.finish()));
    let png_bytes = read_buffer(&buffer);
    std::fs::write("output.png", &png_bytes).unwrap();
}
```

## Phase 1 Scope — What's In and Out

### IN (generates correct pixels)
| Feature | Proto Source | Rust Equivalent |
|---|---|---|
| Solid color fills | css_colors.proto + color_lut | FillRect |
| Linear gradients | css_images.proto → LinearGradientDef | LinearGradient |
| Radial gradients | css_images.proto → RadialGradientDef | RadialGradient |
| Conic gradients | css_images.proto → ConicGradientDef | ConicGradient |
| Border radius (SDF) | css_properties.proto → BORDER_RADIUS | N/A (new) |
| Border dash patterns | border_lut + css_box.proto | StrokeRect |
| Opacity blending | css_properties.proto → OPACITY | PushOpacity |
| Named colors | css_colors.proto (148) | color_lut |
| CSS color spaces (RGB, HSL, HWB) | css_value_types.proto → CssColor | N/A (new) |

### OUT (Phase 2+)
| Feature | Why |
|---|---|
| Blend modes (multiply, screen, etc.) | Complex WGSL, needs careful testing |
| Text rendering | Need GPU font atlas + SDF text |
| Box shadows | Requires multi-pass or complex SDF |
| Background images (url) | Requires texture sampling, image decoding |
| Conic repeating gradients | Edge case, solidify base cases first |
| calc() expressions | Requires compile-time expression eval |

## Performance Target

The current CPU rasterizer achieves ~3.4 Gpix/sec for solid fills. The GPU shader should:
- **Solid fills**: 50+ Gpix/sec (limited by memory bandwidth, not compute)
- **Gradients**: 5-10 Gpix/sec (WGSL sin/cos/sqrt are hardware-accelerated)
- **End-to-end demo**: <5ms for 960×640 (vs current ~30ms)

## File Manifest (New Files)

```
scripts/generate_wgsl.py        — Codegen: proto → WGSL
shaders/render.wgsl             — GENERATED fragment + vertex shader
crates/edgerun-wgpu/
  Cargo.toml
  src/lib.rs
  src/pipeline.rs               — WGPU pipeline creation
  src/uniforms.rs               — RectStyle, uniform buffer layout
  src/render.rs                 — High-level render: rects → draw calls
crates/edgerun-demo-wgpu/
  Cargo.toml
  src/main.rs                   — Demo: HTML+CSS → WGPU → PNG
```

## Testing Strategy

1. **Pixel-match with CPU rasterizer** — run the same HTML+CSS through both paths, diff the PNGs
2. **Golden images** — known-good PNGs for each gradient type, border style, color
3. **Per-feature isolation** — each proto concept (NamedColor, GradientKind, BorderStyle) has its own test case
4. **Conformance** — the generated WGSL must produce identical output to the CPU rasterizer for the same inputs

## Future Phases (Brief)

### Phase 2: Blend Modes
Generate all 16 blend_lut modes as WGSL functions. The `switch r.blend_mode` dispatch.

### Phase 3: GPU Text
Generate a font atlas texture + SDF text rendering. Replace `text_bitmap.rs` with GPU glyph rasterization.

### Phase 4: Compute Shader Layout
Move the two-pass block layout from CPU to compute shader. Each DOM node = one workgroup.

### Phase 5: Multi-Backend
Same proto → MSL (Metal) for macOS/iOS, SPIR-V for Vulkan on Linux/Android.

---

## Why This Is Novel

No browser engine has a **machine-readable spec layer** between the W3C spec and the rendering code. Chrome's gradient math is hand-written C++ that someone translated from a spec. Ours is:

```
W3C spec → css_images.proto → (generate_wgsl.py) → render.wgsl
```

When the W3C updates the gradient spec, we regenerate. Chrome rewrites C++. That's the difference.
