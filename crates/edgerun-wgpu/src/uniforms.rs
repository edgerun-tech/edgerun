//! Uniform buffer types that exactly match the WGSL struct layout.
//!
//! WGSL std140 alignment rules:
//! - scalar: natural alignment
//! - vec4: 16-byte alignment
//! - struct: padded to 16-byte boundary
//! - arrays: stride must be a multiple of 16

use bytemuck::{Pod, Zeroable};

const ZEROED_STOP: GpuGradientStop = GpuGradientStop {
    color: [0.0; 4],
    position: 0.0,
    _pad0: 0.0,
    _pad1: 0.0,
    _pad2: 0.0,
};

/// Matches WGSL `GradientStop` struct (32 bytes, 16-byte aligned).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuGradientStop {
    pub color: [f32; 4],    // RGBA
    pub position: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

impl GpuGradientStop {
    pub fn new(r: f32, g: f32, b: f32, a: f32, position: f32) -> Self {
        Self {
            color: [r, g, b, a],
            position,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }

    pub fn from_u8(r: u8, g: u8, b: u8, a: u8, position: f32) -> Self {
        Self::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
            position,
        )
    }
}

/// Matches WGSL `RectStyle` struct (640 bytes).
///
/// Layout:
///   0-15:    bg_color (vec4<f32>)
///   16-19:   gradient_type (u32)
///   20-23:   gradient_angle (f32)
///   24-27:   gradient_cx (f32)
///   28-31:   gradient_cy (f32)
///   32-543:  gradient_stops (16 × 32 bytes)
///   544-547: gradient_stop_count (u32)
///   548-551: _pad_gs (u32)
///   552-555: border_width_top (f32)
///   556-559: border_width_right (f32)
///   560-563: border_width_bottom (f32)
///   564-567: border_width_left (f32)
///   568-583: border_color (vec4<f32>)
///   584-587: border_style (u32)
///   588-591: border_radius_tl (f32)
///   592-595: border_radius_tr (f32)
///   596-599: border_radius_br (f32)
///   600-603: border_radius_bl (f32)
///   604-607: opacity (f32)
///   608-611: blend_mode (u32)
///   612-615: _pad_cm (u32)
///   616-619: x (f32)
///   620-623: y (f32)
///   624-627: w (f32)
///   628-631: h (f32)
///   632-635: paint_order (u32)
///   636-639: _pad_layout (u32)
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuRectStyle {
    // Background color (from CssColor)
    pub bg_color: [f32; 4],

    // Gradient (from css_images.proto GradientKind)
    pub gradient_type: u32,     // 0=none, 1=linear, 2=radial, 3=conic
    pub gradient_angle: f32,    // Linear: angle; Conic: from_angle
    pub gradient_cx: f32,       // Radial/Conic center X (0-1)
    pub gradient_cy: f32,       // Radial/Conic center Y (0-1)
    pub gradient_stops: [GpuGradientStop; 16],
    pub gradient_stop_count: u32,
    pub _pad_gs: u32,

    // Border (from css_box.proto + border_lut)
    pub border_width_top: f32,
    pub border_width_right: f32,
    pub border_width_bottom: f32,
    pub border_width_left: f32,
    pub _pad_border_align: [u32; 2], // 8 bytes padding: WGSL aligns vec4<f32> to 16 bytes
    pub border_color: [f32; 4],
    pub border_style: u32,      // 0=none, 1=solid, 2=dashed, ...
    pub border_radius_tl: f32,
    pub border_radius_tr: f32,
    pub border_radius_br: f32,
    pub border_radius_bl: f32,

    // Compositing (from blend_lut)
    pub opacity: f32,           // OPACITY property
    pub blend_mode: u32,        // 0=normal, 1=multiply, ...
    pub _pad_cm: u32,

    // Box Shadow (from css_box.proto)
    pub shadow_color: [f32; 4], // Shadow RGBA
    pub shadow_blur: f32,       // Blur radius
    pub shadow_spread: f32,     // Spread distance
    pub shadow_offset_x: f32,   // Horizontal offset
    pub shadow_offset_y: f32,   // Vertical offset
    pub shadow_type: u32,       // 0=drop, 1=inset
    pub shadow_active: u32,     // 1 if shadow should render
    pub _pad_shadow: u32,

    // Layout (computed, not from cascade)
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub paint_order: u32,       // For back-to-front sort
    pub _pad_layout: u32,
    pub _pad_final2: u32,       // Pad struct to 704 bytes (WGSL array stride)
    pub _pad_final3: u32,
}

impl GpuRectStyle {
    pub const SIZE: usize = 704; // Must match WGSL sizeof(RectStyle) including padding

    /// Solid color rectangle.
    pub fn solid(x: f32, y: f32, w: f32, h: f32, r: u8, g: u8, b: u8, a: u8, paint_order: u32) -> Self {
        Self {
            bg_color: [
                r as f32 / 255.0,
                g as f32 / 255.0,
                b as f32 / 255.0,
                a as f32 / 255.0,
            ],
            gradient_type: 0,
            gradient_angle: 0.0,
            gradient_cx: 0.0,
            gradient_cy: 0.0,
            gradient_stops: [ZEROED_STOP; 16],
            gradient_stop_count: 0,
            _pad_gs: 0,
            border_width_top: 0.0,
            border_width_right: 0.0,
            border_width_bottom: 0.0,
            border_width_left: 0.0,
            _pad_border_align: [0; 2],
            border_color: [0.0; 4],
            border_style: 0,
            border_radius_tl: 0.0,
            border_radius_tr: 0.0,
            border_radius_br: 0.0,
            border_radius_bl: 0.0,
            opacity: 1.0,
            blend_mode: 0,
            _pad_cm: 0,
            shadow_color: [0.0; 4],
            shadow_blur: 0.0,
            shadow_spread: 0.0,
            shadow_offset_x: 0.0,
            shadow_offset_y: 0.0,
            shadow_type: 0,
            shadow_active: 0,
            _pad_shadow: 0,
            x, y, w, h,
            paint_order,
            _pad_layout: 0,
            _pad_final2: 0,
            _pad_final3: 0,
        }
    }

    /// Solid color rectangle with border.
    pub fn with_border(
        x: f32, y: f32, w: f32, h: f32,
        r: u8, g: u8, b: u8, a: u8,
        border_width: f32, border_style: u32,
        br: u8, bg: u8, bb: u8,
        radius: f32,
        paint_order: u32,
    ) -> Self {
        let mut s = Self::solid(x, y, w, h, r, g, b, a, paint_order);
        s.border_width_top = border_width;
        s.border_width_right = border_width;
        s.border_width_bottom = border_width;
        s.border_width_left = border_width;
        s.border_color = [br as f32 / 255.0, bg as f32 / 255.0, bb as f32 / 255.0, 1.0];
        s.border_style = border_style;
        s.border_radius_tl = radius;
        s.border_radius_tr = radius;
        s.border_radius_br = radius;
        s.border_radius_bl = radius;
        s
    }

    /// Linear gradient rectangle.
    pub fn linear_gradient(
        x: f32, y: f32, w: f32, h: f32,
        angle: f32,
        stops: &[(u8, u8, u8, u8, f32)],
        paint_order: u32,
    ) -> Self {
        let mut gpu_stops = [ZEROED_STOP; 16];
        for (i, &(r, g, b, a, pos)) in stops.iter().take(16).enumerate() {
            gpu_stops[i] = GpuGradientStop::from_u8(r, g, b, a, pos);
        }
        Self {
            bg_color: [0.0; 4],
            gradient_type: 1,  // LinearGradient
            gradient_angle: angle,
            gradient_cx: 0.0,
            gradient_cy: 0.0,
            gradient_stops: gpu_stops,
            gradient_stop_count: stops.len() as u32,
            _pad_gs: 0,
            border_width_top: 0.0,
            border_width_right: 0.0,
            border_width_bottom: 0.0,
            border_width_left: 0.0,
            _pad_border_align: [0; 2],
            border_color: [0.0; 4],
            border_style: 0,
            border_radius_tl: 0.0,
            border_radius_tr: 0.0,
            border_radius_br: 0.0,
            border_radius_bl: 0.0,
            opacity: 1.0,
            blend_mode: 0,
            _pad_cm: 0,
            shadow_color: [0.0; 4],
            shadow_blur: 0.0,
            shadow_spread: 0.0,
            shadow_offset_x: 0.0,
            shadow_offset_y: 0.0,
            shadow_type: 0,
            shadow_active: 0,
            _pad_shadow: 0,
            x, y, w, h,
            paint_order,
            _pad_layout: 0,
            _pad_final2: 0,
            _pad_final3: 0,
        }
    }

    /// Radial gradient rectangle.
    pub fn radial_gradient(
        x: f32, y: f32, w: f32, h: f32,
        cx: f32, cy: f32,
        stops: &[(u8, u8, u8, u8, f32)],
        paint_order: u32,
    ) -> Self {
        let mut gpu_stops = [ZEROED_STOP; 16];
        for (i, &(r, g, b, a, pos)) in stops.iter().take(16).enumerate() {
            gpu_stops[i] = GpuGradientStop::from_u8(r, g, b, a, pos);
        }
        Self {
            bg_color: [0.0; 4],
            gradient_type: 2,  // RadialGradient
            gradient_angle: 0.0,
            gradient_cx: cx,
            gradient_cy: cy,
            gradient_stops: gpu_stops,
            gradient_stop_count: stops.len() as u32,
            _pad_gs: 0,
            border_width_top: 0.0,
            border_width_right: 0.0,
            border_width_bottom: 0.0,
            border_width_left: 0.0,
            _pad_border_align: [0; 2],
            border_color: [0.0; 4],
            border_style: 0,
            border_radius_tl: 0.0,
            border_radius_tr: 0.0,
            border_radius_br: 0.0,
            border_radius_bl: 0.0,
            opacity: 1.0,
            blend_mode: 0,
            _pad_cm: 0,
            shadow_color: [0.0; 4],
            shadow_blur: 0.0,
            shadow_spread: 0.0,
            shadow_offset_x: 0.0,
            shadow_offset_y: 0.0,
            shadow_type: 0,
            shadow_active: 0,
            _pad_shadow: 0,
            x, y, w, h,
            paint_order,
            _pad_layout: 0,
            _pad_final2: 0,
            _pad_final3: 0,
        }
    }

    /// Conic gradient rectangle.
    pub fn conic_gradient(
        x: f32, y: f32, w: f32, h: f32,
        from_angle: f32,
        cx: f32, cy: f32,
        stops: &[(u8, u8, u8, u8, f32)],
        paint_order: u32,
    ) -> Self {
        let mut gpu_stops = [ZEROED_STOP; 16];
        for (i, &(r, g, b, a, pos)) in stops.iter().take(16).enumerate() {
            gpu_stops[i] = GpuGradientStop::from_u8(r, g, b, a, pos);
        }
        Self {
            bg_color: [0.0; 4],
            gradient_type: 3,  // ConicGradient
            gradient_angle: from_angle,
            gradient_cx: cx,
            gradient_cy: cy,
            gradient_stops: gpu_stops,
            gradient_stop_count: stops.len() as u32,
            _pad_gs: 0,
            border_width_top: 0.0,
            border_width_right: 0.0,
            border_width_bottom: 0.0,
            border_width_left: 0.0,
            _pad_border_align: [0; 2],
            border_color: [0.0; 4],
            border_style: 0,
            border_radius_tl: 0.0,
            border_radius_tr: 0.0,
            border_radius_br: 0.0,
            border_radius_bl: 0.0,
            opacity: 1.0,
            blend_mode: 0,
            _pad_cm: 0,
            shadow_color: [0.0; 4],
            shadow_blur: 0.0,
            shadow_spread: 0.0,
            shadow_offset_x: 0.0,
            shadow_offset_y: 0.0,
            shadow_type: 0,
            shadow_active: 0,
            _pad_shadow: 0,
            x, y, w, h,
            paint_order,
            _pad_layout: 0,
            _pad_final2: 0,
            _pad_final3: 0,
        }
    }

    /// Solid color rectangle with shadow.
    pub fn with_shadow(
        x: f32, y: f32, w: f32, h: f32,
        r: u8, g: u8, b: u8, a: u8,
        shadow_r: u8, shadow_g: u8, shadow_b: u8, shadow_a: u8,
        shadow_blur: f32, shadow_spread: f32,
        shadow_offset_x: f32, shadow_offset_y: f32,
        shadow_type: u32,  // 0=drop, 1=inset
        paint_order: u32,
    ) -> Self {
        let mut s = Self::solid(x, y, w, h, r, g, b, a, paint_order);
        s.shadow_color = [
            shadow_r as f32 / 255.0,
            shadow_g as f32 / 255.0,
            shadow_b as f32 / 255.0,
            shadow_a as f32 / 255.0,
        ];
        s.shadow_blur = shadow_blur;
        s.shadow_spread = shadow_spread;
        s.shadow_offset_x = shadow_offset_x;
        s.shadow_offset_y = shadow_offset_y;
        s.shadow_type = shadow_type;
        s.shadow_active = 1;
        s
    }
}

/// Matches WGSL `Uniforms` struct (small, just frame info).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuUniforms {
    pub fb_width: f32,
    pub fb_height: f32,
    pub rect_count: u32,
    pub text_cmd_count: u32,
}

impl GpuUniforms {
    pub const SIZE: usize = 16;

    pub fn new(fb_width: u32, fb_height: u32, rect_count: u32, text_cmd_count: u32) -> Self {
        Self {
            fb_width: fb_width as f32,
            fb_height: fb_height as f32,
            rect_count,
            text_cmd_count,
        }
    }
}

/// A text command for GPU rendering.
/// Matches WGSL `TextCommand` struct (128 × 4 + 28 = 540 bytes, padded to 544).
/// `glyphs` stores indices into the `glyph_info` buffer (char code points).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuTextCommand {
    pub x: f32,
    pub y: f32,
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
    pub glyph_count: u32,
    pub _pad_tc: u32,
    pub glyphs: [u32; 128],
}

impl GpuTextCommand {
    pub const SIZE: usize = 544; // 7×4 + 128×4 = 540, padded to 16 = 544

    /// Create a text command.
    /// `glyphs` are char code points that index into the `glyph_info` buffer.
    pub fn new(x: f32, y: f32, r: u8, g: u8, b: u8, text: &str) -> Self {
        let mut glyphs = [0u32; 128];
        let chars: Vec<char> = text.chars().take(128).collect();
        let count = chars.len();
        for (i, ch) in chars.iter().enumerate() {
            glyphs[i] = *ch as u32; // char code point = glyph_info index
        }
        Self {
            x, y,
            color_r: r as f32 / 255.0,
            color_g: g as f32 / 255.0,
            color_b: b as f32 / 255.0,
            glyph_count: count as u32,
            _pad_tc: 0,
            glyphs,
        }
    }
}

/// A list of rects to upload as a storage buffer.
pub struct GpuRectBuffer {
    pub buffer_data: Vec<u8>,
    pub rect_count: u32,
}

impl GpuRectBuffer {
    pub fn from_rects(rects: &[GpuRectStyle]) -> Self {
        let count = rects.len().min(65536); // Practical limit
        let mut data = Vec::with_capacity(count * GpuRectStyle::SIZE);
        for r in &rects[..count] {
            data.extend_from_slice(bytemuck::bytes_of(r));
        }
        Self {
            buffer_data: data,
            rect_count: count as u32,
        }
    }
}

/// A list of text commands to upload as a storage buffer.
pub struct GpuTextBuffer {
    pub buffer_data: Vec<u8>,
    pub text_cmd_count: u32,
}

impl GpuTextBuffer {
    pub fn from_commands(cmds: &[GpuTextCommand]) -> Self {
        let count = cmds.len().min(1024); // Practical limit
        let mut data = Vec::with_capacity(count * GpuTextCommand::SIZE);
        for c in &cmds[..count] {
            data.extend_from_slice(bytemuck::bytes_of(c));
        }
        Self {
            buffer_data: data,
            text_cmd_count: count as u32,
        }
    }
}

// ─── GPU Layout Types (Phase 4) ───

/// Tag name as a simple hash for GPU matching.
pub fn tag_hash(tag: &str) -> u32 {
    // djb2 hash
    let mut h: u32 = 5381;
    for &b in tag.as_bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u32);
    }
    h
}

/// A flattened DOM node for GPU cascade + layout.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuDomNode {
    pub tag_hash: u32,
    pub class_hash: u32,
    pub id_hash: u32,
    pub parent_idx: u32,
    pub first_child_idx: u32,
    pub next_sibling_idx: u32,
    pub text_offset: u32, // offset into text buffer, 0 = element node
    pub text_len: u32,    // length of text (0 = element node)
}

impl GpuDomNode {
    pub const SIZE: usize = 32;

    pub fn element(tag: &str, class: Option<&str>, id: Option<&str>) -> Self {
        Self {
            tag_hash: tag_hash(tag),
            class_hash: class.map(tag_hash).unwrap_or(0),
            id_hash: id.map(tag_hash).unwrap_or(0),
            parent_idx: 0,
            first_child_idx: u32::MAX,
            next_sibling_idx: u32::MAX,
            text_offset: 0,
            text_len: 0,
        }
    }

    pub fn text(offset: u32, len: u32) -> Self {
        Self {
            tag_hash: 0, // text nodes have no tag
            class_hash: 0,
            id_hash: 0,
            parent_idx: 0,
            first_child_idx: u32::MAX,
            next_sibling_idx: u32::MAX,
            text_offset: offset,
            text_len: len,
        }
    }
}

/// A flattened CSS rule for GPU matching.
/// Matches WGSL `CssRule` struct (96 bytes, 24×4-byte fields).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuCssRule {
    // Selector
    pub tag_hash: u32,     // 0 = universal
    pub class_hash: u32,   // 0 = no class selector
    pub id_hash: u32,      // 0 = no id selector
    // Specificity (a=ids, b=classes, c=elements)
    pub specificity_a: u32,
    pub specificity_b: u32,
    pub specificity_c: u32,
    // Declarations (-1.0 = not set, uses default/inherited)
    pub font_size: f32,
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
    pub bg_r: f32,
    pub bg_g: f32,
    pub bg_b: f32,
    pub height: f32,       // 0 = auto
    pub width: f32,        // 0 = auto
    pub margin_top: f32,
    pub margin_bottom: f32,
    pub padding_top: f32,
    pub padding_bottom: f32,
    // Source order (rule index, for tie-breaking at same specificity)
    pub source_order: u32,
    // Whether properties are important (bitmask)
    pub is_important: u32,
    pub _pad0: u32,
}

impl GpuCssRule {
    pub const SIZE: usize = 96;

    pub fn default() -> Self {
        Self {
            tag_hash: 0,
            class_hash: 0,
            id_hash: 0,
            specificity_a: 0,
            specificity_b: 0,
            specificity_c: 0,
            font_size: -1.0,
            color_r: -1.0, color_g: -1.0, color_b: -1.0,
            bg_r: -1.0, bg_g: -1.0, bg_b: -1.0,
            height: 0.0, width: 0.0,
            margin_top: 0.0, margin_bottom: 0.0,
            padding_top: 0.0, padding_bottom: 0.0,
            source_order: 0,
            is_important: 0,
            _pad0: 0,
        }
    }

    pub fn tag(tag: &str, source_order: u32) -> Self {
        let mut r = Self::default();
        r.tag_hash = tag_hash(tag);
        r.specificity_c = 1;
        r.source_order = source_order;
        r
    }

    pub fn class(class: &str, source_order: u32) -> Self {
        let mut r = Self::default();
        r.class_hash = tag_hash(class);
        r.specificity_b = 1;
        r.source_order = source_order;
        r
    }

    pub fn font_size(mut self, px: f32) -> Self { self.font_size = px; self }
    pub fn color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color_r = r; self.color_g = g; self.color_b = b; self
    }
    pub fn bg(mut self, r: f32, g: f32, b: f32) -> Self {
        self.bg_r = r; self.bg_g = g; self.bg_b = b; self
    }
    pub fn height(mut self, h: f32) -> Self { self.height = h; self }
    pub fn margin(mut self, top: f32, bottom: f32) -> Self {
        self.margin_top = top; self.margin_bottom = bottom; self
    }
    pub fn padding(mut self, top: f32, bottom: f32) -> Self {
        self.padding_top = top; self.padding_bottom = bottom; self
    }
    pub fn important(mut self, bits: u32) -> Self {
        self.is_important = bits;
        self
    }
}

/// Computed style from GPU cascade.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuStyleResult {
    pub font_size: f32,
    pub color_r: f32, pub color_g: f32, pub color_b: f32, pub color_a: f32,
    pub bg_r: f32, pub bg_g: f32, pub bg_b: f32, pub bg_a: f32,
    pub has_explicit_bg: u32,
    pub explicit_height: f32,
    pub margin_top: f32, pub margin_bottom: f32,
    pub padding_top: f32, pub padding_bottom: f32,
    pub computed_width: f32,
}

impl GpuStyleResult {
    pub const SIZE: usize = 72;
}

/// Output of GPU layout: one per node.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuLayoutResult {
    // Box dimensions
    pub x: f32, pub y: f32, pub w: f32, pub h: f32,
    // Content box (inside padding)
    pub content_x: f32, pub content_y: f32, pub content_w: f32, pub content_h: f32,
    // Computed styles
    pub font_size: f32,
    pub color_r: f32, pub color_g: f32, pub color_b: f32,
    pub bg_r: f32, pub bg_g: f32, pub bg_b: f32,
    pub has_bg: u32,
    pub is_text: u32,
    pub is_block: u32,
    // Box model (from cascade, needed for CPU Y positioning)
    pub margin_top: f32, pub margin_bottom: f32,
    pub padding_top: f32, pub padding_bottom: f32,
}

impl GpuLayoutResult {
    pub const SIZE: usize = 88;
}
