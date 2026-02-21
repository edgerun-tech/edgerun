//! CPU rendering helpers: text clipping, grid drawing, overlays.
pub use crate::render::primitives::{draw_text_line, draw_text_line_clipped};
use crate::terminal::Rgba;
use crate::text::{GlyphCache, MSDF_SPREAD};

pub fn text_width(glyphs: &mut GlyphCache, text: &str) -> i32 {
    let mut w = 0;
    for ch in text.chars() {
        w += glyphs.advance_width(ch);
    }
    w
}

pub fn msdf_alpha(r: u8, g: u8, b: u8, min_width: f32) -> u8 {
    let dist = median(r, g, b) / 255.0;
    let w = min_width.min(0.5 / MSDF_SPREAD).max(0.001);
    let alpha = smoothstep(0.5 - w, 0.5 + w, dist);
    (alpha.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn median(a: u8, b: u8, c: u8) -> f32 {
    let a = a as f32;
    let b = b as f32;
    let c = c as f32;
    f32::max(f32::min(a, b), f32::min(f32::max(a, b), c))
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Placeholder for future CPU grid helpers (moved progressively out of main).
#[allow(dead_code)]
pub fn _tint_color(color: Rgba, alpha: u8) -> Rgba {
    let a = alpha as u16;
    Rgba {
        r: ((color.r as u16 * a) / 255) as u8,
        g: ((color.g as u16 * a) / 255) as u8,
        b: ((color.b as u16 * a) / 255) as u8,
        a: alpha,
    }
}
