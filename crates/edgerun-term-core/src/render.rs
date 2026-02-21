// Render entry points will eventually hold drawing primitives. For now, re-export the text glyph
// cache so external users keep working while the renderer is modularized.
pub use crate::text::GlyphCache;
pub mod primitives;
pub use primitives::*;
pub mod cpu;
pub mod layout;
pub use primitives::{draw_text_line, draw_text_line_clipped};
pub mod grid;
pub use grid::{draw_cursor_overlay, draw_grid};
pub mod border;
pub mod ui;
#[cfg(not(target_arch = "wasm32"))]
pub use border::build_border_gpu;
pub use border::draw_border_cpu;
#[cfg(not(target_arch = "wasm32"))]
pub use ui::{
    OVERLAY_ACCENT, OVERLAY_BADGE, OVERLAY_BAR, OVERLAY_DIM, OVERLAY_PANEL, OVERLAY_PANEL_INNER,
    OVERLAY_TEXT, OVERLAY_TEXT_MUTED, TabVisual, build_help_bar_gpu, build_tab_bar_gpu,
    draw_help_bar_cpu, draw_tab_bar_cpu, rgba_bytes,
};
#[cfg(target_arch = "wasm32")]
pub use ui::{
    OVERLAY_ACCENT, OVERLAY_BADGE, OVERLAY_BAR, OVERLAY_DIM, OVERLAY_PANEL, OVERLAY_PANEL_INNER,
    OVERLAY_TEXT, OVERLAY_TEXT_MUTED, TabVisual, draw_help_bar_cpu, draw_tab_bar_cpu, rgba_bytes,
};

/// Default font bytes and size used for fallback rendering.
pub const FONT_DATA: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");
pub const FONT_SIZE: f32 = 16.0;

#[cfg(test)]
mod tests {
    use super::GlyphCache;

    const FONT_DATA: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");

    #[test]
    fn text_width_sums_advance_widths() {
        let mut cache = GlyphCache::new(std::sync::Arc::new(FONT_DATA.to_vec()), 16.0);

        let expected =
            cache.advance_width('a') + cache.advance_width('b') + cache.advance_width('c');
        let measured: i32 = "abc".chars().map(|c| cache.advance_width(c)).sum();

        assert_eq!(measured, expected);
    }
}
