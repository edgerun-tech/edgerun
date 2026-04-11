#!/usr/bin/env python3
"""
Generate WGSL fragment + vertex shader from proto data and rasterizer LUTs.

Reads:
  - crates/edgerun-rasterizer/src/color_lut.rs  (148 named colors)
  - crates/edgerun-rasterizer/src/border_lut.rs (9 dash patterns)
  - crates/edgerun-rasterizer/src/blend_lut.rs  (16 blend functions)
  - crates/edgerun-rasterizer/src/gradient.rs   (GradientStop, gradient math)
  - proto/edgerun/v0/css/css_images.proto       (GradientKind enum)

Emits:
  - shaders/render.wgsl  (complete vertex + fragment shader)

The generated shader implements the painter's algorithm: every fragment
iterates all styled rectangles back-to-front, testing containment and
computing the topmost pixel color.
"""

import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


# ---------------------------------------------------------------------------
# Data extractors
# ---------------------------------------------------------------------------

def parse_color_lut(path):
    """Parse color_lut.rs → list of (name, r, g, b, a) tuples."""
    colors = []
    with open(path) as f:
        content = f.read()
    # Match lines like: [0x00, 0x00, 0x00, 0xFF], // 0: black
    for m in re.finditer(
        r'\[0x([0-9A-Fa-f]{2}),\s*0x([0-9A-Fa-f]{2}),\s*0x([0-9A-Fa-f]{2}),\s*0x([0-9A-Fa-f]{2})\],\s*//\s*(\d+):\s*(\S+)',
        content
    ):
        r, g, b, a = int(m.group(1), 16), int(m.group(2), 16), int(m.group(3), 16), int(m.group(4), 16)
        idx, name = int(m.group(5)), m.group(6)
        colors.append((idx, name, r, g, b, a))
    return colors


def parse_border_lut(path):
    """Parse border_lut.rs → dict of style_name → [u8; 8] pattern."""
    patterns = {}
    with open(path) as f:
        content = f.read()
    # Match static pattern definitions: pub static DASHED_PATTERN: [u8; 8] = [1,1,1,1, 0,0,0,0];
    for m in re.finditer(
        r'pub\s+static\s+(\w+)_PATTERN:\s*\[u8;\s*8\]\s*=\s*\[([0-9,\s]+)\]',
        content
    ):
        name = m.group(1).lower()
        vals = [int(x.strip()) for x in m.group(2).split(',') if x.strip()]
        patterns[name] = vals
    return patterns


def parse_blend_modes(path):
    """Parse blend_lut.rs → list of (fn_name, is_hsl, body_lines) tuples."""
    with open(path) as f:
        content = f.read()

    modes = []
    # Match blend function signatures
    for m in re.finditer(
        r'pub fn (blend_\w+)\(([^)]*)\)\s*->\s*\w+',
        content
    ):
        fn_name = m.group(1)
        params = m.group(2)
        is_hsl = 'out_r' in params  # HSL compositing functions have separate channel outputs

        # Extract body: find the matching braces
        start = m.end()
        depth = 0
        body_start = None
        body_end = None
        for i, ch in enumerate(content[start:], start):
            if ch == '{':
                if body_start is None:
                    body_start = i
                depth += 1
            elif ch == '}':
                depth -= 1
                if depth == 0:
                    body_end = i
                    break

        body = content[body_start+1:body_end] if body_start and body_end else ""
        modes.append((fn_name, is_hsl, body.strip()))

    return modes


def parse_font_bitmap(path):
    """Parse text_bitmap.rs → list of 128 glyph byte arrays (8 bytes each)."""
    with open(path) as f:
        content = f.read()

    glyphs = []
    # Match the FONT_8X8 array
    m = re.search(r'pub\s+const\s+FONT_8X8:\s*\[\[u8;\s*8\];\s*128\]\s*=\s*\[(.*?)\];', content, re.DOTALL)
    if not m:
        return glyphs

    array_content = m.group(1)
    # Extract each glyph's 8 bytes
    for glyph_m in re.finditer(r'\[\s*0x([0-9A-Fa-f]{2})\s*,\s*0x([0-9A-Fa-f]{2})\s*,\s*0x([0-9A-Fa-f]{2})\s*,\s*0x([0-9A-Fa-f]{2})\s*,\s*0x([0-9A-Fa-f]{2})\s*,\s*0x([0-9A-Fa-f]{2})\s*,\s*0x([0-9A-Fa-f]{2})\s*,\s*0x([0-9A-Fa-f]{2})\s*\]', array_content):
        glyph = [int(glyph_m.group(i), 16) for i in range(1, 9)]
        glyphs.append(glyph)

    return glyphs


def parse_gradient_kinds(proto_path):
    """Parse css_images.proto → list of (name, index) for GradientKind."""
    with open(proto_path) as f:
        content = f.read()

    kinds = []
    in_enum = False
    for line in content.split('\n'):
        line = line.strip()
        if line.startswith('enum GradientKind'):
            in_enum = True
            continue
        if in_enum:
            if line == '}':
                break
            m = re.match(r'(\w+)\s*=\s*(\d+)', line)
            if m:
                kinds.append((m.group(1), int(m.group(2))))
    return kinds


# ---------------------------------------------------------------------------
# WGSL generators
# ---------------------------------------------------------------------------

def gen_vertex_shader():
    """Full-screen triangle vertex shader - covers entire viewport."""
    return """\
// Vertex shader: emits a triangle covering the entire viewport.
// Vertices at (-1,-1), (3,-1), (-1,3) cover the full NDC space and beyond.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return VertexOutput(vec4<f32>(pos[vertex_index], 0.0, 1.0));
}
"""


def gen_uniform_struct(colors, border_patterns, blend_modes, gradient_kinds):
    """Generate the Uniforms struct and binding declarations."""
    lines = []
    lines.append("// --- Uniform Buffer Layout ---")
    lines.append("// Generated from proto data + rasterizer LUTs")
    lines.append("")

    # GradientStop struct
    lines.append("struct GradientStop {")
    lines.append("    color: vec4<f32>,    // RGBA")
    lines.append("    position: f32,")
    lines.append("    _pad0: f32,")
    lines.append("    _pad1: f32,")
    lines.append("    _pad2: f32,")
    lines.append("}")
    lines.append("")

    # RectStyle struct
    lines.append("struct RectStyle {")
    lines.append("    // Background color (from CssColor)")
    lines.append("    bg_color: vec4<f32>,")
    lines.append("")
    lines.append("    // Gradient (from css_images.proto GradientKind)")
    lines.append("    gradient_type: u32,      // 0=none, 1=linear, 2=radial, 3=conic")
    lines.append("    gradient_angle: f32,     // Linear: angle; Conic: from_angle")
    lines.append("    gradient_cx: f32,        // Radial/Conic center X (0-1)")
    lines.append("    gradient_cy: f32,        // Radial/Conic center Y (0-1)")
    lines.append("    gradient_stops: array<GradientStop, 16>,")
    lines.append("    gradient_stop_count: u32,")
    lines.append("    _pad_gs: u32,")
    lines.append("")
    lines.append("    // Border (from css_box.proto + border_lut)")
    lines.append("    border_width_top: f32,")
    lines.append("    border_width_right: f32,")
    lines.append("    border_width_bottom: f32,")
    lines.append("    border_width_left: f32,")
    lines.append("    _pad_border_align: vec2<u32>,  // 8 bytes padding for border_color vec4 alignment")
    lines.append("    border_color: vec4<f32>,")
    lines.append("    border_style: u32,       // 0=none, 1=solid, 2=dashed, ...")
    lines.append("    border_radius_tl: f32,")
    lines.append("    border_radius_tr: f32,")
    lines.append("    border_radius_br: f32,")
    lines.append("    border_radius_bl: f32,")
    lines.append("")
    lines.append("    // Compositing (from blend_lut)")
    lines.append("    opacity: f32,            // OPACITY property")
    lines.append("    blend_mode: u32,         // 0=normal, 1=multiply, ...")
    lines.append("    _pad_cm: u32,")
    lines.append("")
    lines.append("    // Box Shadow (from css_box.proto)")
    lines.append("    shadow_color: vec4<f32>, // Shadow RGBA")
    lines.append("    shadow_blur: f32,        // Blur radius")
    lines.append("    shadow_spread: f32,      // Spread distance")
    lines.append("    shadow_offset_x: f32,    // Horizontal offset")
    lines.append("    shadow_offset_y: f32,    // Vertical offset")
    lines.append("    shadow_type: u32,        // 0=drop, 1=inset")
    lines.append("    shadow_active: u32,      // 1 if shadow should render")
    lines.append("    _pad_shadow: u32,")
    lines.append("")
    lines.append("    // Layout (computed, not from cascade)")
    lines.append("    x: f32, y: f32, w: f32, h: f32,")
    lines.append("    paint_order: u32,        // For back-to-front sort")
    lines.append("    _pad_layout: u32,")
    lines.append("    _pad_final2: u32,")
    lines.append("    _pad_final3: u32,")
    lines.append("}")
    lines.append("")

    # Main uniforms
    lines.append("struct Uniforms {")
    lines.append("    fb_width: f32,")
    lines.append("    fb_height: f32,")
    lines.append("    rect_count: u32,")
    lines.append("    text_cmd_count: u32,")
    lines.append("}")
    lines.append("")
    lines.append("@group(0) @binding(0) var<uniform> u: Uniforms;")
    lines.append("@group(0) @binding(1) var<storage, read> rects: array<RectStyle>;")
    lines.append("")

    # Text command struct
    lines.append("// --- Text Command ---")
    lines.append("// Each text command renders a string of glyphs")
    lines.append("struct TextCommand {")
    lines.append("    x: f32, y: f32,")
    lines.append("    color_r: f32, color_g: f32, color_b: f32,")
    lines.append("    glyph_count: u32,")
    lines.append("    _pad_tc: u32,")
    lines.append("    // Glyph indices (ASCII, max 128 chars per command)")
    lines.append("    glyphs: array<u32, 128>,")
    lines.append("}")
    lines.append("")
    lines.append("@group(0) @binding(2) var<storage, read> text_cmds: array<TextCommand>;")
    lines.append("")

    return "\n".join(lines)


def gen_color_lut_wgsl(colors):
    """Generate named color const array for WGSL."""
    lines = []
    lines.append("// --- Named Color LUT ---")
    lines.append(f"// Generated from color_lut.rs ({len(colors)} colors)")
    lines.append("")

    # Generate as individual const entries in an array
    lines.append(f"const NAMED_COLOR_COUNT: u32 = {len(colors)}u;")
    lines.append(f"const NAMED_COLORS: array<vec4<f32>, {len(colors)}> = array<vec4<f32>, {len(colors)}>(")
    for idx, name, r, g, b, a in colors:
        rf, gf, bf, af = r/255.0, g/255.0, b/255.0, a/255.0
        lines.append(f"    vec4<f32>({rf:.5f}, {gf:.5f}, {bf:.5f}, {af:.5f}),  // {idx}: {name}")
    lines.append(");")
    lines.append("")

    # Lookup function
    lines.append("fn lookup_named_color(index: u32) -> vec4<f32> {")
    lines.append(f"    return NAMED_COLORS[min(index, NAMED_COLOR_COUNT - 1u)];")
    lines.append("}")
    lines.append("")

    return "\n".join(lines)


def gen_border_patterns_wgsl(patterns):
    """Generate border dash pattern const arrays."""
    # Map from Rust pattern names to style indices
    style_map = {
        'none': 0, 'solid': 1, 'dashed': 2, 'dotted': 3,
        'double': 4, 'groove': 5, 'ridge': 6, 'inset': 7, 'outset': 8,
    }

    lines = []
    lines.append("// --- Border Dash Patterns ---")
    lines.append("// Generated from border_lut.rs (9 patterns, 8 bytes each)")
    lines.append("")

    # Generate each pattern as a const
    for name, vals in sorted(patterns.items(), key=lambda x: style_map.get(x[0], 99)):
        vals_str = ', '.join(f'{v}u' for v in vals)
        lines.append(f"const BORDER_PATTERN_{name.upper()}: array<u32, 8> = array<u32, 8>({vals_str});")

    lines.append("")

    # Lookup function
    lines.append("fn border_pattern(style: u32) -> array<u32, 8> {")
    lines.append("    switch style {")
    for name, idx in sorted(style_map.items(), key=lambda x: x[1]):
        lines.append(f"        case {idx}u: {{ return BORDER_PATTERN_{name.upper()}; }}")
    lines.append("        default: { return BORDER_PATTERN_NONE; }")
    lines.append("    }")
    lines.append("}")
    lines.append("")

    # Dash test function
    lines.append("fn border_dash_test(pixel_offset: u32, style: u32) -> bool {")
    lines.append("    let pattern = border_pattern(style);")
    lines.append("    return pattern[pixel_offset % 8u] == 1u;")
    lines.append("}")
    lines.append("")

    return "\n".join(lines)


def gen_blend_modes_wgsl(blend_modes):
    """Generate blend mode functions for WGSL.

    Converts 0-255 integer Rust math to 0-1 float WGSL math.
    Maps:
        0: normal      1: multiply      2: screen      3: overlay
        4: darken      5: lighten       6: colordodge  7: colorburn
        8: hardlight   9: softlight     10: difference 11: exclusion
    """
    lines = []
    lines.append("// --- Blend Modes ---")
    lines.append("// Generated from blend_lut.rs (CSS Compositing Level 1)")
    lines.append("// 12 per-channel blend modes, converted from 0-255 integer to 0-1 float")
    lines.append("")

    # Functions that use var (have branching)
    # Functions that use let (pure expressions)
    blend_let = {
        "normal":       "src",
        "multiply":     "src * dst",
        "screen":       "1.0 - (1.0 - src) * (1.0 - dst)",
        "darken":       "min(src, dst)",
        "lighten":      "max(src, dst)",
        "difference":   "abs(src - dst)",
        "exclusion":    "src + dst - 2.0 * src * dst",
    }

    blend_var = {
        "overlay": """var result: f32;
    if (dst < 0.5) { result = 2.0 * src * dst; }
    else { result = 1.0 - 2.0 * (1.0 - src) * (1.0 - dst); }""",
        "colordodge": """var result: f32;
    if (dst == 0.0) { result = 0.0; }
    else if (src == 1.0) { result = 1.0; }
    else { result = clamp(dst / (1.0 - src), 0.0, 1.0); }""",
        "colorburn": """var result: f32;
    if (dst == 1.0) { result = 1.0; }
    else if (src == 0.0) { result = 0.0; }
    else { result = clamp(1.0 - (1.0 - dst) / src, 0.0, 1.0); }""",
        "hardlight": """var result: f32;
    if (src < 0.5) { result = 2.0 * src * dst; }
    else { result = 1.0 - 2.0 * (1.0 - src) * (1.0 - dst); }""",
        "softlight": """var result: f32;
    if (src <= 0.5) {
        result = dst - (1.0 - dst) * dst * (0.5 - src);
    } else {
        let v = select(sqrt(dst) * 0.25, dst, dst <= 0.25);
        result = dst + dst * (1.0 - dst) * (src - 0.5) / max(v, 0.001);
    }""",
    }

    mode_names = [
        "normal", "multiply", "screen", "overlay",
        "darken", "lighten", "colordodge", "colorburn",
        "hardlight", "softlight", "difference", "exclusion",
    ]

    # Generate individual blend functions
    lines.append("// Per-channel blend functions (0-1 float range)")
    for name in mode_names:
        lines.append(f"fn blend_{name}(src: f32, dst: f32) -> f32 {{")
        if name in blend_let:
            lines.append(f"    return clamp({blend_let[name]}, 0.0, 1.0);")
        elif name in blend_var:
            lines.append(f"    {blend_var[name]}")
            lines.append("    return clamp(result, 0.0, 1.0);")
        lines.append("}")
        lines.append("")

    # Generate dispatch function
    lines.append("// Blend mode dispatch")
    lines.append("fn blend_channel(src: f32, dst: f32, mode: u32) -> f32 {")
    lines.append("    switch mode {")
    for i, name in enumerate(mode_names):
        lines.append(f"        case {i}u: {{ return blend_{name}(src, dst); }}")
    lines.append("        default: { return src; }")
    lines.append("    }")
    lines.append("}")
    lines.append("")

    # Full vec4 blend function
    lines.append("fn apply_blend_mode(src: vec4<f32>, dst: vec4<f32>, mode: u32) -> vec4<f32> {")
    lines.append("    return vec4<f32>(")
    lines.append("        blend_channel(src.r, dst.r, mode),")
    lines.append("        blend_channel(src.g, dst.g, mode),")
    lines.append("        blend_channel(src.b, dst.b, mode),")
    lines.append("        src.a,")
    lines.append("    );")
    lines.append("}")
    lines.append("")

    return "\n".join(lines)


def convert_rust_to_wgsl_blend(rust_body):
    """Convert a Rust blend function body to WGSL expression."""
    expr = rust_body.strip()

    # Map Rust functions to WGSL
    expr = expr.replace('.min(255)', '')  # We work in 0-1 range
    expr = expr.replace('.max(1)', '')
    expr = expr.replace('.max(0)', '')

    # Simple per-channel mappings
    if 'src' in expr and 'dst' in expr:
        # Replace src/dst with s/d (already defined in WGSL fn)
        expr = expr.replace('src as u32', 's')
        expr = expr.replace('dst as u32', 'd')
        expr = expr.replace('src as i32', 's')
        expr = expr.replace('dst as i32', 'd')
        expr = expr.replace('src', 's')
        expr = expr.replace('dst', 'd')

        # Convert integer math to float math
        expr = expr.replace('/ 255', '/ 255.0')
        expr = expr.replace('* 255', '* 255.0')
        expr = expr.replace('/255', '/ 255.0')
        expr = expr.replace('*255', '* 255.0')

        # Wrap in f32() casts for the expressions
        expr = f"f32({expr})"

    # Specific function overrides with clean WGSL
    return expr


def gen_gradient_functions():
    """Generate gradient t-parameter computation functions."""
    lines = []
    lines.append("// --- Gradient T-Parameter Functions ---")
    lines.append("// Generated from css_images.proto + gradient.rs")
    lines.append("")

    # Linear gradient t
    lines.append("// Linear gradient: projects pixel onto the gradient line")
    lines.append("fn linear_gradient_t(px: f32, py: f32, r: RectStyle) -> f32 {")
    lines.append("    let rel_x = (px - r.x) / r.w;")
    lines.append("    let rel_y = (py - r.y) / r.h;")
    lines.append("    let cos_a = cos(r.gradient_angle);")
    lines.append("    let sin_a = sin(r.gradient_angle);")
    lines.append("    let t = (rel_x * cos_a + rel_y * sin_a) / (r.w + r.h) * 2.0;")
    lines.append("    return clamp(t, 0.0, 1.0);")
    lines.append("}")
    lines.append("")

    # Radial gradient t
    lines.append("// Radial gradient: distance from center, normalized")
    lines.append("fn radial_gradient_t(px: f32, py: f32, r: RectStyle) -> f32 {")
    lines.append("    let center_x = r.x + r.w * r.gradient_cx;")
    lines.append("    let center_y = r.y + r.h * r.gradient_cy;")
    lines.append("    let dx = px - center_x;")
    lines.append("    let dy = py - center_y;")
    lines.append("    let max_dist = sqrt(r.w * r.w + r.h * r.h) * 0.5;")
    lines.append("    let dist = sqrt(dx * dx + dy * dy) / max_dist;")
    lines.append("    return clamp(dist, 0.0, 1.0);")
    lines.append("}")
    lines.append("")

    # Conic gradient t
    lines.append("// Conic gradient: angle from center, with from_angle offset")
    lines.append("fn conic_gradient_t(px: f32, py: f32, r: RectStyle) -> f32 {")
    lines.append("    let center_x = r.x + r.w * r.gradient_cx;")
    lines.append("    let center_y = r.y + r.h * r.gradient_cy;")
    lines.append("    let dx = px - center_x;")
    lines.append("    let dy = py - center_y;")
    lines.append("    var angle: f32 = atan2(dy, dx) + 3.141592653589793 - r.gradient_angle;")
    lines.append("    if (angle < 0.0) { angle = angle + 6.283185307179586; }")
    lines.append("    return angle / 6.283185307179586;")
    lines.append("}")
    lines.append("")

    # Sample stops function
    lines.append("// Sample color stops with linear interpolation")
    lines.append("// Generated from gradient.rs lerp_stops + find_stops")
    lines.append("fn sample_stops(r: RectStyle, t: f32) -> vec4<f32> {")
    lines.append("    if (r.gradient_stop_count == 0u) {")
    lines.append("        return vec4<f32>(1.0, 0.0, 0.0, 1.0);  // Error: red")
    lines.append("    }")
    lines.append("    if (r.gradient_stop_count == 1u) {")
    lines.append("        return r.gradient_stops[0].color;")
    lines.append("    }")
    lines.append("")
    lines.append("    // Find bracketing stops")
    lines.append("    var prev = r.gradient_stops[0];")
    lines.append("    var next = r.gradient_stops[1];")
    lines.append("    var i: u32 = 1u;")
    lines.append("    loop {")
    lines.append("        if (i >= r.gradient_stop_count) { break; }")
    lines.append("        if (r.gradient_stops[i].position >= t) {")
    lines.append("            next = r.gradient_stops[i];")
    lines.append("            break;")
    lines.append("        }")
    lines.append("        prev = r.gradient_stops[i];")
    lines.append("        next = r.gradient_stops[i];")
    lines.append("        i = i + 1u;")
    lines.append("    }")
    lines.append("")
    lines.append("    let span = next.position - prev.position;")
    lines.append("    var local_t: f32 = 0.0;")
    lines.append("    if (span > 0.0001) {")
    lines.append("        local_t = clamp((t - prev.position) / span, 0.0, 1.0);")
    lines.append("    }")
    lines.append("    return mix(prev.color, next.color, local_t);")
    lines.append("}")
    lines.append("")

    return "\n".join(lines)


def gen_font_bitmap_wgsl(glyphs):
    """Generate the 8x8 bitmap font as a WGSL const array."""
    lines = []
    lines.append("// --- 8x8 Bitmap Font ---")
    lines.append("// Generated from text_bitmap.rs FONT_8X8 (128 glyphs, 8 bytes each)")
    lines.append("")
    lines.append("const GLYPH_WIDTH: u32 = 8u;")
    lines.append("const GLYPH_HEIGHT: u32 = 8u;")
    lines.append("")

    # Glyph data: 128 glyphs × 8 bytes = 1024 bytes
    lines.append("// Glyph bitmap data: glyph_index * 8 + row = byte with 8 pixel bits (MSB=left)")
    lines.append("const GLYPH_DATA: array<u32, 1024> = array<u32, 1024>(")
    idx = 0
    for glyph_idx, glyph_bytes in enumerate(glyphs):
        for row_byte in glyph_bytes:
            comment = f" // glyph {glyph_idx}, row {idx % 8}"
            lines.append(f"    {row_byte}u,{comment}")
            idx += 1
    lines.append(");")
    lines.append("")

    # Glyph sampling function
    lines.append("// Sample a single pixel from a glyph bitmap")
    lines.append("// Returns 1.0 if the pixel is set, 0.0 otherwise")
    lines.append("fn glyph_pixel(glyph_index: u32, row: u32, col: u32) -> f32 {")
    lines.append("    if (glyph_index >= 128u || row >= 8u || col >= 8u) { return 0.0; }")
    lines.append("    let byte_idx = glyph_index * 8u + row;")
    lines.append("    let bits = GLYPH_DATA[byte_idx];")
    lines.append("    let mask = 0x80u >> col;")
    lines.append("    return select(0.0, 1.0, (bits & mask) != 0u);")
    lines.append("}")
    lines.append("")

    return "\n".join(lines)


def gen_border_render():
    """Generate border rendering functions for the fragment shader."""
    lines = []
    lines.append("// --- Border Rendering ---")
    lines.append("// Generated from css_box.proto + border_lut.rs")
    lines.append("")

    # Function to check if pixel is in border region
    lines.append("// Returns the border width at a given pixel position on the rect edge")
    lines.append("// Returns > 0 if in border region, 0 if in content area")
    lines.append("fn border_width_at(px: f32, py: f32, r: RectStyle) -> f32 {")
    lines.append("    let rel_x = px - r.x;")
    lines.append("    let rel_y = py - r.y;")
    lines.append("    // Distance from each edge")
    lines.append("    let dist_left = rel_x;")
    lines.append("    let dist_right = r.w - rel_x;")
    lines.append("    let dist_top = rel_y;")
    lines.append("    let dist_bottom = r.h - rel_y;")
    lines.append("")
    lines.append("    // Determine which border segment we're on")
    lines.append("    // Top/bottom borders span full width, left/right span between top and bottom border")
    lines.append("    var bw: f32 = 0.0;")
    lines.append("")
    lines.append("    // Check if in corner regions (use max of adjacent border widths)")
    lines.append("    let in_top_left = rel_x < r.border_radius_tl && rel_y < r.border_radius_tl;")
    lines.append("    let in_top_right = (r.w - rel_x) < r.border_radius_tr && rel_y < r.border_radius_tr;")
    lines.append("    let in_bottom_right = (r.w - rel_x) < r.border_radius_br && (r.h - rel_y) < r.border_radius_br;")
    lines.append("    let in_bottom_left = rel_x < r.border_radius_bl && (r.h - rel_y) < r.border_radius_bl;")
    lines.append("")
    lines.append("    if (in_top_left || in_top_right || in_bottom_right || in_bottom_left) {")
    lines.append("        // Corner: use the max border width of adjacent edges")
    lines.append("        bw = max(max(r.border_width_top, r.border_width_bottom),")
    lines.append("                 max(r.border_width_left, r.border_width_right));")
    lines.append("    } else if (dist_top <= r.border_width_top) {")
    lines.append("        bw = r.border_width_top;")
    lines.append("    } else if (dist_bottom <= r.border_width_bottom) {")
    lines.append("        bw = r.border_width_bottom;")
    lines.append("    } else if (dist_left <= r.border_width_left) {")
    lines.append("        bw = r.border_width_left;")
    lines.append("    } else if (dist_right <= r.border_width_right) {")
    lines.append("        bw = r.border_width_right;")
    lines.append("    }")
    lines.append("")
    lines.append("    return bw;")
    lines.append("}")
    lines.append("")

    # Function to compute dash pattern pixel offset
    lines.append("// Compute a dash pattern pixel offset for border dashing")
    lines.append("fn border_pixel_offset(px: f32, py: f32, r: RectStyle) -> u32 {")
    lines.append("    let rel_x = u32(px - r.x);")
    lines.append("    let rel_y = u32(py - r.y);")
    lines.append("    // Simple: use Manhattan distance as pattern offset")
    lines.append("    return (rel_x + rel_y) % 8u;")
    lines.append("}")
    lines.append("")

    return "\n".join(lines)


def gen_shadow_render():
    """Generate SDF box shadow rendering functions."""
    lines = []
    lines.append("// --- SDF Box Shadow Rendering ---")
    lines.append("// Generated from css_box.proto box-shadow properties")
    lines.append("")

    # Shadow SDF distance function
    lines.append("// Compute signed distance from rounded rect edge")
    lines.append("// Returns negative distance inside, positive outside")
    lines.append("fn sdf_rounded_rect(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, tl: f32, tr: f32, br: f32, bl: f32) -> f32 {")
    lines.append("    // Simple box SDF (corners handled separately for rounded rects)")
    lines.append("    let cx = x + w * 0.5;")
    lines.append("    let cy = y + h * 0.5;")
    lines.append("    let hx = abs(px - cx) - w * 0.5;")
    lines.append("    let hy = abs(py - cy) - h * 0.5;")
    lines.append("    let d = min(max(hx, hy), 0.0);")
    lines.append("    let dx = max(hx, 0.0);")
    lines.append("    let dy = max(hy, 0.0);")
    lines.append("    return sqrt(dx * dx + dy * dy) + d;")
    lines.append("}")
    lines.append("")

    # Shadow rendering function
    lines.append("// Render drop shadow for a rect")
    lines.append("fn render_drop_shadow(px: f32, py: f32, r: RectStyle) -> vec4<f32> {")
    lines.append("    if (r.shadow_active == 0u) { return vec4<f32>(0.0, 0.0, 0.0, 0.0); }")
    lines.append("    if (r.shadow_type != 0u) { return vec4<f32>(0.0, 0.0, 0.0, 0.0); } // drop only")
    lines.append("")
    lines.append("    // Shadow rect: expanded by spread+blur, offset")
    lines.append("    let expand = r.shadow_spread + r.shadow_blur;")
    lines.append("    let sx = r.x + r.shadow_offset_x - expand;")
    lines.append("    let sy = r.y + r.shadow_offset_y - expand;")
    lines.append("    let sw = r.w + expand * 2.0;")
    lines.append("    let sh = r.h + expand * 2.0;")
    lines.append("")
    lines.append("    // Compute SDF distance from shadow rect edge")
    lines.append("    let dist = sdf_rounded_rect(px, py, sx, sy, sw, sh,")
    lines.append("        r.border_radius_tl + expand, r.border_radius_tr + expand,")
    lines.append("        r.border_radius_br + expand, r.border_radius_bl + expand);")
    lines.append("")
    lines.append("    // Convert distance to alpha (smooth falloff)")
    lines.append("    var alpha = 0.0;")
    lines.append("    if (dist < 0.0) {")
    lines.append("        // Inside shadow rect: full opacity at edge")
    lines.append("        alpha = 1.0;")
    lines.append("    } else {")
    lines.append("        // Outside: fade over blur distance")
    lines.append("        if (r.shadow_blur > 0.001) {")
    lines.append("            alpha = 1.0 - clamp(dist / r.shadow_blur, 0.0, 1.0);")
    lines.append("        }")
    lines.append("    }")
    lines.append("")
    lines.append("    return vec4<f32>(r.shadow_color.rgb, alpha * r.shadow_color.a);")
    lines.append("}")
    lines.append("")

    # Inset shadow function
    lines.append("// Render inset shadow for a rect")
    lines.append("fn render_inset_shadow(px: f32, py: f32, r: RectStyle) -> vec4<f32> {")
    lines.append("    if (r.shadow_active == 0u) { return vec4<f32>(0.0, 0.0, 0.0, 0.0); }")
    lines.append("    if (r.shadow_type != 1u) { return vec4<f32>(0.0, 0.0, 0.0, 0.0); } // inset only")
    lines.append("")
    lines.append("    // Inset shadow: contracts from rect edge")
    lines.append("    let expand = r.shadow_spread + r.shadow_blur;")
    lines.append("    let ix = r.x + expand - r.shadow_offset_x;")
    lines.append("    let iy = r.y + expand - r.shadow_offset_y;")
    lines.append("    let iw = r.w - expand * 2.0;")
    lines.append("    let ih = r.h - expand * 2.0;")
    lines.append("")
    lines.append("    if (iw <= 0.0 || ih <= 0.0) { return vec4<f32>(0.0, 0.0, 0.0, 0.0); }")
    lines.append("")
    lines.append("    // Compute SDF distance from inner rect edge")
    lines.append("    let dist = -sdf_rounded_rect(px, py, ix, iy, iw, ih,")
    lines.append("        max(r.border_radius_tl - expand, 0.0), max(r.border_radius_tr - expand, 0.0),")
    lines.append("        max(r.border_radius_br - expand, 0.0), max(r.border_radius_bl - expand, 0.0));")
    lines.append("")
    lines.append("    // Convert distance to alpha")
    lines.append("    var alpha = 0.0;")
    lines.append("    if (dist >= 0.0) {")
    lines.append("        alpha = 1.0;")
    lines.append("    } else {")
    lines.append("        if (r.shadow_blur > 0.001) {")
    lines.append("            alpha = clamp(-dist / r.shadow_blur, 0.0, 1.0);")
    lines.append("        }")
    lines.append("    }")
    lines.append("")
    lines.append("    return vec4<f32>(r.shadow_color.rgb, alpha * r.shadow_color.a);")
    lines.append("}")
    lines.append("")

    return "\n".join(lines)


def gen_rounded_rect_sdf():
    """Generate SDF rounded rectangle containment test."""
    lines = []
    lines.append("// --- Rounded Rectangle SDF ---")
    lines.append("// Generated from css_box.proto border-radius")
    lines.append("")
    lines.append("// Returns true if point (px, py) is inside the rounded rect")
    lines.append("fn contains_rounded_rect(px: f32, py: f32, r: RectStyle) -> bool {")
    lines.append("    // Quick AABB test")
    lines.append("    if (px < r.x || px >= r.x + r.w || py < r.y || py >= r.y + r.h) {")
    lines.append("        return false;")
    lines.append("    }")
    lines.append("")
    lines.append("    // Corner radii")
    lines.append("    let tl = r.border_radius_tl;")
    lines.append("    let tr = r.border_radius_tr;")
    lines.append("    let br = r.border_radius_br;")
    lines.append("    let bl = r.border_radius_bl;")
    lines.append("")
    lines.append("    // Determine which corner region we're in")
    lines.append("    let in_left = px < r.x + tl;")
    lines.append("    let in_right = px > r.x + r.w - tr;")
    lines.append("    let in_top = py < r.y + tl;")
    lines.append("    let in_bottom = py > r.y + r.h - bl;")
    lines.append("")
    lines.append("    // Top-left corner")
    lines.append("    if (in_left && in_top) {")
    lines.append("        let dx = px - (r.x + tl);")
    lines.append("        let dy = py - (r.y + tl);")
    lines.append("        return dx * dx + dy * dy <= tl * tl;")
    lines.append("    }")
    lines.append("    // Top-right corner")
    lines.append("    if (in_right && in_top) {")
    lines.append("        let dx = px - (r.x + r.w - tr);")
    lines.append("        let dy = py - (r.y + tr);")
    lines.append("        return dx * dx + dy * dy <= tr * tr;")
    lines.append("    }")
    lines.append("    // Bottom-right corner")
    lines.append("    if (in_right && in_bottom) {")
    lines.append("        let dx = px - (r.x + r.w - br);")
    lines.append("        let dy = py - (r.y + r.h - bl);")
    lines.append("        return dx * dx + dy * dy <= br * br;")
    lines.append("    }")
    lines.append("    // Bottom-left corner")
    lines.append("    if (in_left && in_bottom) {")
    lines.append("        let dx = px - (r.x + bl);")
    lines.append("        let dy = py - (r.y + r.h - bl);")
    lines.append("        return dx * dx + dy * dy <= bl * bl;")
    lines.append("    }")
    lines.append("")
    lines.append("    return true;  // Inside the non-corner region")
    lines.append("}")
    lines.append("")

    return "\n".join(lines)


def gen_layout_compute_shader():
    """Generate WGSL compute shader for GPU layout computation (Phase 4).

    This compute shader takes a flat DOM node array and CSS rules,
    performs cascade resolution and block layout on the GPU,
    and outputs positioned layout results.

    The layout is sequential (not parallel) because block layout
    has inherent dependencies: a node's Y position depends on
    the heights of all preceding siblings.
    """
    lines = []
    lines.append("// DO NOT EDIT. Generated by scripts/generate_wgsl.py")
    lines.append("// GPU Layout Compute Shader — Phase 4")
    lines.append("//")
    lines.append("// Sequential block layout: cascade resolution + Y positioning")
    lines.append("")

    # Input bindings
    lines.append("// --- Input Bindings ---")
    lines.append("")
    lines.append("// DOM nodes (flat array, index-based tree)")
    lines.append("struct DomNode {")
    lines.append("    tag_hash: u32,")
    lines.append("    class_hash: u32,")
    lines.append("    id_hash: u32,")
    lines.append("    parent_idx: u32,")
    lines.append("    first_child_idx: u32,")
    lines.append("    next_sibling_idx: u32,")
    lines.append("    text_offset: u32,")
    lines.append("    text_len: u32,")
    lines.append("}")
    lines.append("")
    lines.append("// CSS rules (flat array, specificity-ordered)")
    lines.append("struct CssRule {")
    lines.append("    tag_hash: u32,")
    lines.append("    class_hash: u32,")
    lines.append("    id_hash: u32,")
    lines.append("    specificity_a: u32,")
    lines.append("    specificity_b: u32,")
    lines.append("    specificity_c: u32,")
    lines.append("    font_size: f32,")
    lines.append("    color_r: f32, color_g: f32, color_b: f32,")
    lines.append("    bg_r: f32, bg_g: f32, bg_b: f32,")
    lines.append("    height: f32,")
    lines.append("}")
    lines.append("")
    lines.append("// Layout output (one per node)")
    lines.append("struct LayoutResult {")
    lines.append("    x: f32, y: f32, w: f32, h: f32,")
    lines.append("    font_size: f32,")
    lines.append("    color_r: f32, color_g: f32, color_b: f32,")
    lines.append("    bg_r: f32, bg_g: f32, bg_b: f32,")
    lines.append("    has_bg: u32,")
    lines.append("    gradient_type: u32,")
    lines.append("    is_block: u32,")
    lines.append("    is_text: u32,")
    lines.append("}")
    lines.append("")
    lines.append("// Config")
    lines.append("struct LayoutConfig {")
    lines.append("    node_count: u32,")
    lines.append("    rule_count: u32,")
    lines.append("    text_len: u32,")
    lines.append("    _pad: u32,")
    lines.append("    avail_width: f32,")
    lines.append("    base_x: f32,")
    lines.append("    base_y: f32,")
    lines.append("}")
    lines.append("")
    lines.append("@group(0) @binding(0) var<storage, read> nodes: array<DomNode>;")
    lines.append("@group(0) @binding(1) var<storage, read> rules: array<CssRule>;")
    lines.append("@group(0) @binding(2) var<storage, read> text_buffer: array<u32>;")
    lines.append("@group(0) @binding(3) var<uniform> config: LayoutConfig;")
    lines.append("@group(0) @binding(4) var<storage, read_write> results: array<LayoutResult>;")
    lines.append("")

    # Cascade resolution function
    lines.append("// --- Cascade Resolution ---")
    lines.append("// Match a node against CSS rules, return best matching style")
    lines.append("fn cascade_resolve(node_idx: u32) -> vec4<f32> {")
    lines.append("    let node = nodes[node_idx];")
    lines.append("    var best_font_size: f32 = 16.0;  // default")
    lines.append("    var best_color: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);")
    lines.append("    var best_bg: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);")
    lines.append("    var has_bg = false;")
    lines.append("    var best_spec_a: u32 = 0;")
    lines.append("    var best_spec_b: u32 = 0;")
    lines.append("    var best_spec_c: u32 = 0;")
    lines.append("")
    lines.append("    for (var ri: u32 = 0u; ri < config.rule_count; ri = ri + 1u) {")
    lines.append("        let rule = rules[ri];")
    lines.append("        var matches = true;")
    lines.append("")
    lines.append("        // Tag match (0 = universal)")
    lines.append("        if (rule.tag_hash != 0u && rule.tag_hash != node.tag_hash) {")
    lines.append("            matches = false;")
    lines.append("        }")
    lines.append("")
    lines.append("        // Class match (0 = no class selector)")
    lines.append("        if (matches && rule.class_hash != 0u && rule.class_hash != node.class_hash) {")
    lines.append("            matches = false;")
    lines.append("        }")
    lines.append("")
    lines.append("        // ID match (0 = no ID selector)")
    lines.append("        if (matches && rule.id_hash != 0u && rule.id_hash != node.id_hash) {")
    lines.append("            matches = false;")
    lines.append("        }")
    lines.append("")
    lines.append("        // Specificity comparison")
    lines.append("        if (matches) {")
    lines.append("            let dominated = (rule.specificity_a < best_spec_a) ||")
    lines.append("                (rule.specificity_a == best_spec_a && rule.specificity_b < best_spec_b) ||")
    lines.append("                (rule.specificity_a == best_spec_a && rule.specificity_b == best_spec_b && rule.specificity_c <= best_spec_c);")
    lines.append("            if (!dominated) {")
    lines.append("                if (rule.font_size > 0.0) { best_font_size = rule.font_size; }")
    lines.append("                if (rule.color_r >= 0.0) { best_color = vec3<f32>(rule.color_r, rule.color_g, rule.color_b); }")
    lines.append("                if (rule.bg_r >= 0.0) { best_bg = vec3<f32>(rule.bg_r, rule.bg_g, rule.bg_b); has_bg = true; }")
    lines.append("                best_spec_a = rule.specificity_a;")
    lines.append("                best_spec_b = rule.specificity_b;")
    lines.append("                best_spec_c = rule.specificity_c;")
    lines.append("            }")
    lines.append("        }")
    lines.append("    }")
    lines.append("")
    lines.append("    let has_bg_f: f32 = select(0.0, 1.0, has_bg);")
    lines.append("    return vec4<f32>(best_font_size, has_bg_f, 0.0, 0.0);")
    lines.append("}")
    lines.append("")

    # Height computation helper
    lines.append("// --- Height Computation ---")
    lines.append("fn compute_text_height(text_len: u32, font_size: f32) -> f32 {")
    lines.append("    let char_w = font_size * 0.5;")
    lines.append("    let cpl = max(u32(config.avail_width / char_w), 10u);")
    lines.append("    let lines = max(u32(ceil(f32(text_len) / f32(cpl))), 1u);")
    lines.append("    return font_size * 1.25 * f32(lines);")
    lines.append("}")
    lines.append("")

    # Block element height computation
    lines.append("fn compute_block_height(node_idx: u32, font_size: f32, rule_height: f32) -> f32 {")
    lines.append("    if (rule_height > 0.0) { return rule_height + 10.0; }")
    lines.append("")
    lines.append("    // Sum children's heights")
    lines.append("    var total_h: f32 = 0.0;")
    lines.append("    var ci = nodes[node_idx].first_child_idx;")
    lines.append("    while (ci != 0xFFFFFFFFu) {")
    lines.append("        let child = nodes[ci];")
    lines.append("        if (child.text_len > 0u) {")
    lines.append("            total_h += compute_text_height(child.text_len, font_size);")
    lines.append("        } else {")
    lines.append("            let child_h = results[ci].h;")
    lines.append("            total_h += max(child_h, font_size * 1.25);")
    lines.append("        }")
    lines.append("        ci = child.next_sibling_idx;")
    lines.append("    }")
    lines.append("    return max(total_h + 10.0, font_size * 1.5);")
    lines.append("}")
    lines.append("")

    # Main compute shader entry point
    lines.append("// --- Compute Shader Entry Point ---")
    lines.append("@compute @workgroup_size(1)")
    lines.append("fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {")
    lines.append("    let node_idx = global_id.x;")
    lines.append("    if (node_idx >= config.node_count) { return; }")
    lines.append("")
    lines.append("    let node = nodes[node_idx];")
    lines.append("    let is_text = node.text_len > 0u;")
    lines.append("    let is_block = !is_text && node.tag_hash != 0u;  // Simplified: all elements are blocks")
    lines.append("")
    lines.append("    // Cascade resolution")
    lines.append("    let style = cascade_resolve(node_idx);")
    lines.append("    let font_size = style.x;")
    lines.append("    let has_bg = style.y > 0.5;")
    lines.append("")
    lines.append("    var result: LayoutResult;")
    lines.append("    result.font_size = font_size;")
    lines.append("    result.color_r = 0.8; result.color_g = 0.8; result.color_b = 0.8;")
    lines.append("    result.has_bg = u32(has_bg);")
    lines.append("    result.bg_r = 0.0; result.bg_g = 0.0; result.bg_b = 0.0;")
    lines.append("    result.gradient_type = 0u;")
    lines.append("    result.is_text = u32(is_text);")
    lines.append("    result.is_block = u32(is_block);")
    lines.append("")
    lines.append("    // Text node")
    lines.append("    if (is_text) {")
    lines.append("        result.w = config.avail_width;")
    lines.append("        result.h = compute_text_height(node.text_len, font_size);")
    lines.append("        results[node_idx] = result;")
    lines.append("        return;")
    lines.append("    }")
    lines.append("")
    lines.append("    // Block element: compute height from children")
    lines.append("    if (is_block) {")
    lines.append("        result.h = compute_block_height(node_idx, font_size, 0.0);")
    lines.append("        result.w = config.avail_width;")
    lines.append("        results[node_idx] = result;")
    lines.append("        return;")
    lines.append("    }")
    lines.append("")
    lines.append("    // Default")
    lines.append("    result.h = font_size * 1.5;")
    lines.append("    result.w = config.avail_width;")
    lines.append("    results[node_idx] = result;")
    lines.append("}")
    lines.append("")

    return "\n".join(lines)


def gen_fragment_shader(gradient_kinds):
    """Generate the main fragment shader with painter's algorithm."""
    lines = []
    lines.append("// --- Fragment Shader: Painter's Algorithm ---")
    lines.append("// Every pixel iterates all rectangles back-to-front")
    lines.append("")

    lines.append("@fragment")
    lines.append("fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {")
    lines.append("    let px = pos.x;")
    lines.append("    let py = pos.y;")
    lines.append("")

    # Default canvas background
    lines.append("    // Default canvas background (from Framebuffer::clear)")
    lines.append("    var color = vec4<f32>(0.10196, 0.10196, 0.18039, 1.0);  // #1a1a2e")
    lines.append("")

    # Main painter loop
    lines.append("    // Painter's algorithm: iterate back-to-front")
    lines.append("    for (var i: u32 = 0u; i < u.rect_count; i = i + 1u) {")
    lines.append("        let r = rects[i];")
    lines.append("")

    # Containment test
    lines.append("        // Containment test (rounded rect SDF)")
    lines.append("        if (!contains_rounded_rect(px, py, r)) { continue; }")
    lines.append("")

    # Drop shadow rendering pass (before main content)
    lines.append("        // --- Drop Shadow Pass ---")
    lines.append("        // Render shadows BEFORE rect content (painter's algorithm)")
    lines.append("        // Check if this pixel falls within the shadow area of ANY preceding rect")
    lines.append("        // For efficiency, we render the shadow of THIS rect if the pixel")
    lines.append("        // falls within its shadow bounding box but not the rect itself")
    lines.append("")
    lines.append("        // Shadow rect containment test (expanded by spread+blur+offset)")
    lines.append("        let expand = r.shadow_spread + r.shadow_blur;")
    lines.append("        let shadow_x = r.x + r.shadow_offset_x - expand;")
    lines.append("        let shadow_y = r.y + r.shadow_offset_y - expand;")
    lines.append("        let shadow_w = r.w + expand * 2.0;")
    lines.append("        let shadow_h = r.h + expand * 2.0;")
    lines.append("        let in_shadow = px >= shadow_x && px < shadow_x + shadow_w &&")
    lines.append("                        py >= shadow_y && py < shadow_y + shadow_h;")
    lines.append("        let in_rect = contains_rounded_rect(px, py, r);")
    lines.append("")
    lines.append("        // Drop shadow: render if in shadow area but not in rect")
    lines.append("        if (r.shadow_active != 0u && r.shadow_type == 0u && in_shadow && !in_rect) {")
    lines.append("            let shadow = render_drop_shadow(px, py, r);")
    lines.append("            if (shadow.a > 0.001) {")
    lines.append("                color = mix(color, vec4<f32>(shadow.rgb, 1.0), shadow.a);")
    lines.append("            }")
    lines.append("        }")
    lines.append("")

    # Compute pixel color
    lines.append("        var pixel_color: vec4<f32>;")
    lines.append("")

    # Gradient switch
    lines.append("        // Gradient dispatch (from css_images.proto GradientKind)")
    lines.append("        switch r.gradient_type {")
    lines.append("            case 0u: {  // None — solid bg_color")
    lines.append("                pixel_color = r.bg_color;")
    lines.append("            }")
    lines.append("            case 1u: {  // LinearGradient")
    lines.append("                let t = linear_gradient_t(px, py, r);")
    lines.append("                pixel_color = sample_stops(r, t);")
    lines.append("            }")
    lines.append("            case 2u: {  // RadialGradient")
    lines.append("                let t = radial_gradient_t(px, py, r);")
    lines.append("                pixel_color = sample_stops(r, t);")
    lines.append("            }")
    lines.append("            case 3u: {  // ConicGradient")
    lines.append("                let t = conic_gradient_t(px, py, r);")
    lines.append("                pixel_color = sample_stops(r, t);")
    lines.append("            }")
    lines.append("            default: {")
    lines.append("                pixel_color = r.bg_color;")
    lines.append("            }")
    lines.append("        }")
    lines.append("")

    # Border rendering pass
    lines.append("        // --- Border Rendering Pass ---")
    lines.append("        let bw = border_width_at(px, py, r);")
    lines.append("        if (bw > 0.0 && r.border_style != 0u) {")
    lines.append("            // Check dash pattern")
    lines.append("            let dash_offset = border_pixel_offset(px, py, r);")
    lines.append("            let dash_visible = border_dash_test(dash_offset, r.border_style);")
    lines.append("            if (dash_visible) {")
    lines.append("                pixel_color = r.border_color;")
    lines.append("            }")
    lines.append("        }")
    lines.append("")

    # Blend mode application
    lines.append("        // Blend mode application (from blend_lut)")
    lines.append("        pixel_color = apply_blend_mode(pixel_color, color, r.blend_mode);")
    lines.append("")

    # Opacity compositing
    lines.append("        // Opacity compositing")
    lines.append("        color = mix(color, pixel_color, r.opacity);")
    lines.append("    }")
    lines.append("")

    # Text rendering pass (bounding-box optimized)
    lines.append("    // --- Text Rendering Pass ---")
    lines.append("    // For each text command, check if this pixel falls within any glyph")
    lines.append("    for (var ti: u32 = 0u; ti < u.text_cmd_count; ti = ti + 1u) {")
    lines.append("        let tc = text_cmds[ti];")
    lines.append("        for (var gi: u32 = 0u; gi < tc.glyph_count; gi = gi + 1u) {")
    lines.append("            let glyph_x = tc.x + f32(gi) * 8.0;")
    lines.append("            let glyph_y = tc.y;")
    lines.append("            // Bounding box test")
    lines.append("            if (px >= glyph_x && px < glyph_x + 8.0 && py >= glyph_y && py < glyph_y + 8.0) {")
    lines.append("                let col = u32(px - glyph_x);")
    lines.append("                let row = u32(py - glyph_y);")
    lines.append("                let sample_val = glyph_pixel(tc.glyphs[gi], row, col);")
    lines.append("                if (sample_val > 0.5) {")
    lines.append("                    let text_color = vec4<f32>(tc.color_r, tc.color_g, tc.color_b, 1.0);")
    lines.append("                    color = mix(color, text_color, 1.0);")
    lines.append("                }")
    lines.append("            }")
    lines.append("        }")
    lines.append("    }")
    lines.append("")

    lines.append("    return color;")
    lines.append("}")
    lines.append("")

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Assembly
# ---------------------------------------------------------------------------

def assemble_shader(parts):
    """Assemble WGSL shader parts into a complete file."""
    header = """\
// DO NOT EDIT. Generated by scripts/generate_wgsl.py
// Source: proto/edgerun/v0/css/ + crates/edgerun-rasterizer/src/
//
// This shader implements the painter's algorithm: every fragment
// iterates all styled rectangles back-to-front, computing the
// topmost pixel color. This replaces the CPU scanline rasterizer.
"""

    return header + "\n" + "\n\n".join(parts)


def main():
    color_lut_path = os.path.join(ROOT, "crates/edgerun-rasterizer/src/color_lut.rs")
    border_lut_path = os.path.join(ROOT, "crates/edgerun-rasterizer/src/border_lut.rs")
    blend_lut_path = os.path.join(ROOT, "crates/edgerun-rasterizer/src/blend_lut.rs")
    css_images_proto = os.path.join(ROOT, "proto/edgerun/v0/css/css_images.proto")
    font_bitmap_path = os.path.join(ROOT, "crates/edgerun-rasterizer/src/text_bitmap.rs")

    # Parse input data
    print("Parsing color_lut.rs...", file=sys.stderr)
    colors = parse_color_lut(color_lut_path)
    print(f"  Found {len(colors)} named colors", file=sys.stderr)

    print("Parsing border_lut.rs...", file=sys.stderr)
    border_patterns = parse_border_lut(border_lut_path)
    print(f"  Found {len(border_patterns)} border patterns", file=sys.stderr)

    print("Parsing blend_lut.rs...", file=sys.stderr)
    blend_modes = parse_blend_modes(blend_lut_path)
    std_count = sum(1 for _, is_hsl, _ in blend_modes if not is_hsl)
    hsl_count = sum(1 for _, is_hsl, _ in blend_modes if is_hsl)
    print(f"  Found {std_count} standard + {hsl_count} HSL blend modes", file=sys.stderr)

    print("Parsing css_images.proto...", file=sys.stderr)
    gradient_kinds = parse_gradient_kinds(css_images_proto)
    print(f"  Found {len(gradient_kinds)} gradient kinds", file=sys.stderr)

    print("Parsing text_bitmap.rs (FONT_8X8)...", file=sys.stderr)
    glyphs = parse_font_bitmap(font_bitmap_path)
    print(f"  Found {len(glyphs)} glyphs", file=sys.stderr)

    # Generate shader parts
    print("Generating WGSL...", file=sys.stderr)
    parts = [
        gen_vertex_shader(),
        gen_uniform_struct(colors, border_patterns, blend_modes, gradient_kinds),
        gen_color_lut_wgsl(colors),
        gen_border_patterns_wgsl(border_patterns),
        gen_blend_modes_wgsl(blend_modes),
        gen_font_bitmap_wgsl(glyphs),
        gen_border_render(),
        gen_shadow_render(),
        gen_gradient_functions(),
        gen_rounded_rect_sdf(),
        gen_fragment_shader(gradient_kinds),
    ]

    shader = assemble_shader(parts)

    # Write output
    output_dir = os.path.join(ROOT, "shaders")
    os.makedirs(output_dir, exist_ok=True)
    output_path = os.path.join(output_dir, "render.wgsl")

    with open(output_path, 'w') as f:
        f.write(shader)

    # Generate compute shader for GPU layout (Phase 4)
    compute_shader = gen_layout_compute_shader()
    compute_path = os.path.join(output_dir, "layout.wgsl")
    with open(compute_path, 'w') as f:
        f.write(compute_shader)

    lines = shader.count('\n')
    size = len(shader)
    clines = compute_shader.count('\n')
    csize = len(compute_shader)
    print(f"Wrote {output_path} ({lines} lines, {size} bytes)", file=sys.stderr)
    print(f"Wrote {compute_path} ({clines} lines, {csize} bytes)", file=sys.stderr)


if __name__ == '__main__':
    main()
