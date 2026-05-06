//! CPU rendering helpers: text clipping, grid drawing, overlays.
pub use crate::render::primitives::{draw_text_line, draw_text_line_clipped};
use crate::terminal::Rgba;
use crate::text::GlyphCache;

pub fn text_width(glyphs: &mut GlyphCache, text: &str) -> i32 {
    let mut w = 0;
    for ch in text.chars() {
        w += glyphs.advance_width(ch);
    }
    w
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
