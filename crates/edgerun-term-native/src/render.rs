// Render entry points will eventually hold drawing primitives. For now, re-export the text glyph
// cache so external users keep working while the renderer is modularized.
pub use term_core::render::GlyphCache;

pub mod primitives {
    pub use term_core::render::primitives::*;
}

pub mod cpu {
    pub use term_core::render::cpu::*;
}

pub mod layout {
    pub use term_core::render::layout::*;
}

pub mod grid {
    pub use term_core::render::grid::*;
}

pub mod overlay;
pub mod ui;
pub mod border;

pub use primitives::{draw_text_line, draw_text_line_clipped, draw_background, fill_rect, rainbow};
pub use ui::{
    TabVisual, build_help_bar_gpu, build_tab_bar_gpu, draw_help_bar_cpu, draw_tab_bar_cpu,
};
pub use grid::draw_grid;
pub use border::{build_border_gpu, draw_border_cpu};

pub fn push_overlay_rect(
    rects: &mut Vec<crate::gpu::RectVertex>,
    rect: primitives::OverlayRect,
) {
    crate::gpu::GpuRenderer::push_rect(rects, rect.x0, rect.y0, rect.x1, rect.y1, rect.color);
}

/// Default font bytes and size used for fallback rendering.
pub const FONT_DATA: &[u8] = term_core::render::FONT_DATA;
// Match the user's kitty config (JetBrainsMono Nerd Font 15px with 110% line height
// and 101% column width tweaks).
pub const FONT_SIZE: f32 = term_core::render::FONT_SIZE;
pub const FONT_LINE_HEIGHT_ADJUST: f32 = term_core::render::FONT_LINE_HEIGHT_ADJUST;
pub const FONT_COLUMN_ADJUST: f32 = term_core::render::FONT_COLUMN_ADJUST;
