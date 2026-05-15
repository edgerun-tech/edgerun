use super::runtime::GpuHit;
#[cfg(feature = "tabler-svg-atlas")]
use super::UiIconAtlasRect;
#[cfg(feature = "fontdue-text")]
use super::{FontAtlas, TextQuad};
use std::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color4 {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color4 {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn from_color(color: crate::Color) -> Self {
        Self::rgba(
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
            color.a as f32 / 255.0,
        )
    }

    pub const fn from_color_alpha(color: crate::Color, a: f32) -> Self {
        Self::rgba(
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
            a,
        )
    }

    pub const fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RectMode {
    Fill,
    Shadow,
    Border,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
    pub color: Color4,
    pub mode: RectMode,
    pub shadow: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuClip {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[cfg(feature = "tabler-svg-atlas")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconQuad {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub color: Color4,
}

impl GpuClip {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn intersect(self, other: Self) -> Option<Self> {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.w).min(other.x + other.w);
        let y1 = (self.y + self.h).min(other.y + other.h);
        let w = x1 - x0;
        let h = y1 - y0;
        (w > 0.0 && h > 0.0).then_some(Self::new(x0, y0, w, h))
    }
}

impl GpuRect {
    pub const fn fill(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            color,
            mode: RectMode::Fill,
            shadow: 0.0,
        }
    }

    pub const fn shadow(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        color: Color4,
        shadow: f32,
    ) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            color,
            mode: RectMode::Shadow,
            shadow,
        }
    }

    pub const fn border(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            color,
            mode: RectMode::Border,
            shadow: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GpuScene {
    pub clear: Color4,
    rects: Vec<GpuRect>,
    hits: Vec<GpuHit>,
    clip_stack: Vec<GpuClip>,
    #[cfg(feature = "tabler-svg-atlas")]
    icon_quads: Vec<IconQuad>,
    #[cfg(feature = "fontdue-text")]
    text_quads: Vec<TextQuad>,
}

impl GpuScene {
    pub fn new(clear: Color4) -> Self {
        Self {
            clear,
            rects: Vec::new(),
            hits: Vec::new(),
            clip_stack: Vec::new(),
            #[cfg(feature = "tabler-svg-atlas")]
            icon_quads: Vec::new(),
            #[cfg(feature = "fontdue-text")]
            text_quads: Vec::new(),
        }
    }

    pub fn clear_rects(&mut self) {
        self.rects.clear();
        self.hits.clear();
        self.clip_stack.clear();
        #[cfg(feature = "tabler-svg-atlas")]
        self.icon_quads.clear();
        #[cfg(feature = "fontdue-text")]
        self.text_quads.clear();
    }

    pub fn push_rect(&mut self, rect: GpuRect) {
        if let Some(rect) = self.clip_rect(rect) {
            self.rects.push(rect);
        }
    }

    pub fn push_hit(&mut self, hit: GpuHit) {
        if let Some(hit) = self.clip_hit(hit) {
            self.hits.push(hit);
        }
    }

    pub fn hit_count(&self) -> usize {
        self.hits.len()
    }

    pub fn truncate_hits(&mut self, len: usize) {
        self.hits.truncate(len);
    }

    pub fn push_clip(&mut self, clip: GpuClip) -> bool {
        let next = if let Some(current) = self.current_clip() {
            current.intersect(clip)
        } else if clip.w > 0.0 && clip.h > 0.0 {
            Some(clip)
        } else {
            None
        };
        if let Some(next) = next {
            self.clip_stack.push(next);
            true
        } else {
            false
        }
    }

    pub fn pop_clip(&mut self) {
        self.clip_stack.pop();
    }

    pub fn current_clip(&self) -> Option<GpuClip> {
        self.clip_stack.last().copied()
    }

    fn clip_rect(&self, mut rect: GpuRect) -> Option<GpuRect> {
        let Some(clip) = self.current_clip() else {
            return (rect.w > 0.0 && rect.h > 0.0).then_some(rect);
        };
        let clipped = GpuClip::new(rect.x, rect.y, rect.w, rect.h).intersect(clip)?;
        rect.x = clipped.x;
        rect.y = clipped.y;
        rect.w = clipped.w;
        rect.h = clipped.h;
        rect.radius = rect.radius.min(rect.w * 0.5).min(rect.h * 0.5);
        Some(rect)
    }

    fn clip_hit(&self, mut hit: GpuHit) -> Option<GpuHit> {
        let Some(clip) = self.current_clip() else {
            return (hit.w > 0.0 && hit.h > 0.0).then_some(hit);
        };
        let clipped = GpuClip::new(hit.x, hit.y, hit.w, hit.h).intersect(clip)?;
        hit.x = clipped.x;
        hit.y = clipped.y;
        hit.w = clipped.w;
        hit.h = clipped.h;
        Some(hit)
    }

    pub fn push_text(&mut self, mut x: f32, y: f32, text: &str, scale: f32, color: Color4) {
        let cell = scale.max(1.0);
        let step = cell * 6.0;
        let start_x = x;
        for ch in text.chars() {
            match ch {
                '\n' => {
                    x = start_x;
                }
                '\r' => {}
                ' ' => x += step,
                _ => {
                    let glyph = super::bitmap_font::glyph5x7(ch);
                    for (row, bits) in glyph.iter().copied().enumerate() {
                        for col in 0..5 {
                            if ((bits >> (4 - col)) & 1) == 0 {
                                continue;
                            }
                            self.push_rect(GpuRect::fill(
                                x + col as f32 * cell,
                                y + row as f32 * cell,
                                cell,
                                cell,
                                0.0,
                                color,
                            ));
                        }
                    }
                    x += step;
                }
            }
        }
    }

    pub fn rects(&self) -> &[GpuRect] {
        &self.rects
    }

    pub fn hits(&self) -> &[GpuHit] {
        &self.hits
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<GpuHit> {
        self.hits
            .iter()
            .rev()
            .copied()
            .find(|hit| hit.contains(x, y))
    }

    pub fn apply_color_scheme(&mut self, scheme: UiColorScheme) {
        if scheme == UiColorScheme::Dark {
            return;
        }
        let from = SchemePalette::dark();
        let to = scheme.palette();
        self.clear = remap_scheme_color(self.clear, from, to);
        for rect in &mut self.rects {
            rect.color = remap_scheme_color(rect.color, from, to);
        }
        #[cfg(feature = "tabler-svg-atlas")]
        for quad in &mut self.icon_quads {
            quad.color = remap_scheme_color(quad.color, from, to);
        }
        #[cfg(feature = "fontdue-text")]
        for quad in &mut self.text_quads {
            quad.color = remap_scheme_color(quad.color, from, to);
        }
    }

    #[cfg(feature = "tabler-svg-atlas")]
    pub fn push_icon_quad(&mut self, rect: super::UiRect, atlas: UiIconAtlasRect, color: Color4) {
        let quad = IconQuad {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
            u0: atlas.u0,
            v0: atlas.v0,
            u1: atlas.u1,
            v1: atlas.v1,
            color,
        };
        if let Some(quad) = self.clip_icon_quad(quad) {
            self.icon_quads.push(quad);
        }
    }

    #[cfg(feature = "tabler-svg-atlas")]
    fn clip_icon_quad(&self, mut quad: IconQuad) -> Option<IconQuad> {
        let Some(clip) = self.current_clip() else {
            return (quad.w > 0.0 && quad.h > 0.0).then_some(quad);
        };
        let x0 = quad.x;
        let y0 = quad.y;
        let x1 = quad.x + quad.w;
        let y1 = quad.y + quad.h;
        let clipped = GpuClip::new(quad.x, quad.y, quad.w, quad.h).intersect(clip)?;
        let u_span = quad.u1 - quad.u0;
        let v_span = quad.v1 - quad.v0;
        let left = ((clipped.x - x0) / (x1 - x0)).clamp(0.0, 1.0);
        let top = ((clipped.y - y0) / (y1 - y0)).clamp(0.0, 1.0);
        let right = ((clipped.x + clipped.w - x0) / (x1 - x0)).clamp(0.0, 1.0);
        let bottom = ((clipped.y + clipped.h - y0) / (y1 - y0)).clamp(0.0, 1.0);
        quad.x = clipped.x;
        quad.y = clipped.y;
        quad.w = clipped.w;
        quad.h = clipped.h;
        quad.u1 = quad.u0 + u_span * right;
        quad.v1 = quad.v0 + v_span * bottom;
        quad.u0 += u_span * left;
        quad.v0 += v_span * top;
        Some(quad)
    }

    #[cfg(feature = "tabler-svg-atlas")]
    pub fn icon_quads(&self) -> &[IconQuad] {
        &self.icon_quads
    }

    #[cfg(feature = "fontdue-text")]
    pub fn push_font_text(&mut self, atlas: &FontAtlas, x: f32, y: f32, text: &str, color: Color4) {
        atlas.layout_text(self, x, y, text, color);
    }

    #[cfg(feature = "fontdue-text")]
    pub(super) fn push_text_quad(&mut self, quad: TextQuad) {
        if let Some(quad) = self.clip_text_quad(quad) {
            self.text_quads.push(quad);
        }
    }

    #[cfg(feature = "fontdue-text")]
    fn clip_text_quad(&self, mut quad: TextQuad) -> Option<TextQuad> {
        let Some(clip) = self.current_clip() else {
            return (quad.w > 0.0 && quad.h > 0.0).then_some(quad);
        };
        let x0 = quad.x;
        let y0 = quad.y;
        let x1 = quad.x + quad.w;
        let y1 = quad.y + quad.h;
        let clipped = GpuClip::new(quad.x, quad.y, quad.w, quad.h).intersect(clip)?;
        let u_span = quad.u1 - quad.u0;
        let v_span = quad.v1 - quad.v0;
        let left = ((clipped.x - x0) / (x1 - x0)).clamp(0.0, 1.0);
        let top = ((clipped.y - y0) / (y1 - y0)).clamp(0.0, 1.0);
        let right = ((clipped.x + clipped.w - x0) / (x1 - x0)).clamp(0.0, 1.0);
        let bottom = ((clipped.y + clipped.h - y0) / (y1 - y0)).clamp(0.0, 1.0);
        quad.x = clipped.x;
        quad.y = clipped.y;
        quad.w = clipped.w;
        quad.h = clipped.h;
        quad.u1 = quad.u0 + u_span * right;
        quad.v1 = quad.v0 + v_span * bottom;
        quad.u0 += u_span * left;
        quad.v0 += v_span * top;
        Some(quad)
    }

    #[cfg(feature = "fontdue-text")]
    pub fn text_quads(&self) -> &[TextQuad] {
        &self.text_quads
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiColorScheme {
    #[default]
    Dark,
    Light,
    Terminal,
}

impl UiColorScheme {
    pub const fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Light,
            2 => Self::Terminal,
            _ => Self::Dark,
        }
    }

    pub const fn code(self) -> u32 {
        match self {
            Self::Dark => 0,
            Self::Light => 1,
            Self::Terminal => 2,
        }
    }

    fn palette(self) -> SchemePalette {
        match self {
            Self::Dark => SchemePalette::dark(),
            Self::Light => SchemePalette::light(),
            Self::Terminal => SchemePalette::terminal(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SchemePalette {
    bg: Color4,
    sidebar: Color4,
    topbar: Color4,
    panel: Color4,
    row: Color4,
    active: Color4,
    composer: Color4,
    text: Color4,
    muted: Color4,
    border: Color4,
    accent: Color4,
    accent_text: Color4,
    green: Color4,
    violet: Color4,
    amber: Color4,
    danger: Color4,
}

impl SchemePalette {
    const fn dark() -> Self {
        Self {
            bg: super::palette::BG,
            sidebar: super::palette::SIDEBAR,
            topbar: super::palette::TOPBAR,
            panel: super::palette::PANEL,
            row: super::palette::ROW,
            active: super::palette::ACTIVE_ROW,
            composer: super::palette::COMPOSER,
            text: super::palette::TEXT,
            muted: super::palette::MUTED,
            border: super::palette::BORDER,
            accent: super::palette::ACCENT,
            accent_text: super::palette::ACCENT_TEXT,
            green: super::palette::GREEN,
            violet: super::palette::VIOLET,
            amber: super::palette::AMBER,
            danger: super::palette::DANGER,
        }
    }

    const fn light() -> Self {
        Self {
            bg: Color4::rgba(0.965, 0.965, 0.94, 1.0),
            sidebar: Color4::rgba(0.92, 0.925, 0.91, 1.0),
            topbar: Color4::rgba(0.985, 0.985, 0.965, 1.0),
            panel: Color4::rgba(1.0, 1.0, 0.98, 1.0),
            row: Color4::rgba(0.925, 0.93, 0.91, 1.0),
            active: Color4::rgba(0.86, 0.91, 0.92, 1.0),
            composer: Color4::rgba(0.955, 0.955, 0.94, 1.0),
            text: Color4::rgba(0.07, 0.075, 0.07, 1.0),
            muted: Color4::rgba(0.32, 0.33, 0.31, 1.0),
            border: Color4::rgba(0.74, 0.75, 0.71, 1.0),
            accent: Color4::rgba(0.0, 0.48, 0.54, 1.0),
            accent_text: Color4::rgba(0.98, 1.0, 1.0, 1.0),
            green: Color4::rgba(0.0, 0.48, 0.32, 1.0),
            violet: Color4::rgba(0.43, 0.28, 0.62, 1.0),
            amber: Color4::rgba(0.68, 0.42, 0.0, 1.0),
            danger: Color4::rgba(0.72, 0.15, 0.12, 1.0),
        }
    }

    const fn terminal() -> Self {
        Self {
            bg: Color4::rgba(0.0, 0.03, 0.018, 1.0),
            sidebar: Color4::rgba(0.0, 0.07, 0.04, 1.0),
            topbar: Color4::rgba(0.0, 0.09, 0.055, 1.0),
            panel: Color4::rgba(0.0, 0.065, 0.04, 1.0),
            row: Color4::rgba(0.0, 0.11, 0.065, 1.0),
            active: Color4::rgba(0.02, 0.20, 0.11, 1.0),
            composer: Color4::rgba(0.0, 0.085, 0.052, 1.0),
            text: Color4::rgba(0.63, 1.0, 0.72, 1.0),
            muted: Color4::rgba(0.32, 0.67, 0.43, 1.0),
            border: Color4::rgba(0.08, 0.37, 0.17, 1.0),
            accent: Color4::rgba(0.0, 0.92, 0.42, 1.0),
            accent_text: Color4::rgba(0.0, 0.025, 0.015, 1.0),
            green: Color4::rgba(0.05, 0.90, 0.36, 1.0),
            violet: Color4::rgba(0.48, 0.90, 0.68, 1.0),
            amber: Color4::rgba(0.78, 0.95, 0.32, 1.0),
            danger: Color4::rgba(1.0, 0.24, 0.20, 1.0),
        }
    }
}

fn remap_scheme_color(color: Color4, from: SchemePalette, to: SchemePalette) -> Color4 {
    const EPS: f32 = 0.002;
    let candidates = [
        (from.bg, to.bg),
        (from.sidebar, to.sidebar),
        (from.topbar, to.topbar),
        (from.panel, to.panel),
        (from.row, to.row),
        (from.active, to.active),
        (from.composer, to.composer),
        (from.text, to.text),
        (from.muted, to.muted),
        (from.border, to.border),
        (from.accent, to.accent),
        (from.accent_text, to.accent_text),
        (from.green, to.green),
        (from.violet, to.violet),
        (from.amber, to.amber),
        (from.danger, to.danger),
    ];
    for (source, target) in candidates {
        if (color.r - source.r).abs() < EPS
            && (color.g - source.g).abs() < EPS
            && (color.b - source.b).abs() < EPS
        {
            return target.with_alpha(color.a);
        }
    }
    color
}
