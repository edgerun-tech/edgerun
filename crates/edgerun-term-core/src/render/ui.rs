use std::time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use pixels::wgpu;

#[cfg(not(target_arch = "wasm32"))]
use crate::gpu::{GlyphAtlas, GlyphVertex, RectVertex};
use crate::render::cpu::text_width;
use crate::render::{GlyphCache, draw_text_line_clipped, fill_rect};
use crate::terminal::{FG, Rgba};

pub const OVERLAY_DIM: Rgba = Rgba {
    r: 10,
    g: 12,
    b: 16,
    a: 180,
};
pub const OVERLAY_PANEL: Rgba = Rgba {
    r: 18,
    g: 22,
    b: 30,
    a: 235,
};
pub const OVERLAY_PANEL_INNER: Rgba = Rgba {
    r: 24,
    g: 28,
    b: 38,
    a: 235,
};
pub const OVERLAY_BAR: Rgba = Rgba {
    r: 18,
    g: 22,
    b: 30,
    a: 210,
};
pub const OVERLAY_BADGE: Rgba = Rgba {
    r: 18,
    g: 22,
    b: 30,
    a: 210,
};
pub const OVERLAY_TEXT: Rgba = Rgba {
    r: 230,
    g: 235,
    b: 245,
    a: 255,
};
pub const OVERLAY_TEXT_MUTED: Rgba = Rgba {
    r: 200,
    g: 205,
    b: 215,
    a: 220,
};
pub const OVERLAY_ACCENT: Rgba = Rgba {
    r: 140,
    g: 200,
    b: 255,
    a: 255,
};

pub const fn rgba_bytes(c: Rgba) -> [u8; 4] {
    [c.r, c.g, c.b, c.a]
}

/// Lightweight tab descriptor for rendering.
#[derive(Clone, Copy, Debug)]
pub struct TabVisual<'a> {
    pub title: &'a str,
}

/// CPU path tab bar drawing.
#[allow(clippy::too_many_arguments)]
pub fn draw_tab_bar_cpu(
    tabs: &[TabVisual<'_>],
    active: usize,
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    width: u32,
    height: u32,
    tab_bar_height: u32,
    border_thickness: u32,
    _start_time: Instant,
) {
    let bar_top = border_thickness as i32;
    let bar_bottom = bar_top + tab_bar_height as i32;
    fill_rect(
        frame,
        width,
        height,
        0,
        bar_top,
        width as i32,
        bar_bottom,
        [24, 24, 24, 200],
    );

    let mut x = border_thickness as i32 + 8;
    let text_y = tab_text_y(bar_top, tab_bar_height, glyphs);
    for (idx, tab) in tabs.iter().enumerate() {
        if x >= width as i32 {
            break;
        }
        let label = format!(" {} ", tab.title);
        let label_width = crate::render::cpu::text_width(glyphs, &label).max(0);
        let rect_w = (label_width + 14).max(48);
        let rect_x1 = (x + rect_w as i32).min(width as i32);
        let bg = if idx == active {
            [72, 92, 110, 200]
        } else {
            [32, 32, 32, 180]
        };
        fill_rect(
            frame,
            width,
            height,
            x,
            bar_top + 2,
            rect_x1,
            bar_bottom - 2,
            bg,
        );
        let text_color = if idx == active {
            [230, 235, 240, 255]
        } else {
            FG
        };
        let text_x = x + ((rect_w as i32 - label_width) / 2).max(0);
        draw_text_line_clipped(
            glyphs,
            frame,
            width,
            height,
            text_x,
            text_y,
            &label,
            text_color,
            rect_x1 - 7,
        );
        x += rect_w + 8;
    }
}

/// GPU path tab bar building.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub fn build_tab_bar_gpu(
    rects: &mut Vec<RectVertex>,
    glyphs_out: &mut Vec<GlyphVertex>,
    atlas: &mut GlyphAtlas,
    queue: &wgpu::Queue,
    tabs: &[TabVisual<'_>],
    active: usize,
    glyphs: &mut GlyphCache,
    frame_width: u32,
    tab_bar_height: u32,
    border_thickness: u32,
    _start_time: Instant,
) {
    let bar_top = border_thickness as f32;
    let bar_bottom = bar_top + tab_bar_height as f32;
    // background
    crate::gpu::GpuRenderer::push_rect(
        rects,
        0.0,
        bar_top,
        frame_width as f32,
        bar_bottom,
        Rgba {
            r: 24,
            g: 24,
            b: 24,
            a: 200,
        },
    );

    let mut x = border_thickness as f32 + 8.0;
    let text_y = tab_text_y_f32(bar_top, tab_bar_height, glyphs);
    for (idx, tab) in tabs.iter().enumerate() {
        if x >= frame_width as f32 {
            break;
        }
        let label = format!(" {} ", tab.title);
        let label_width = crate::render::cpu::text_width(glyphs, &label) as f32;
        let rect_w = (label_width + 14.0).max(48.0);
        let rect_x1 = (x + rect_w).min(frame_width as f32);
        let bg = if idx == active {
            Rgba {
                r: 72,
                g: 92,
                b: 110,
                a: 200,
            }
        } else {
            Rgba {
                r: 32,
                g: 32,
                b: 32,
                a: 180,
            }
        };
        crate::gpu::GpuRenderer::push_rect(rects, x, bar_top + 2.0, rect_x1, bar_bottom - 2.0, bg);
        let text_color = if idx == active {
            Rgba {
                r: 230,
                g: 235,
                b: 240,
                a: 255,
            }
        } else {
            Rgba {
                r: FG[0],
                g: FG[1],
                b: FG[2],
                a: FG[3],
            }
        };
        let text_x = x + ((rect_w - label_width) / 2.0).max(0.0);
        crate::gpu::GpuRenderer::push_text_line(
            glyphs, atlas, glyphs_out, &label, text_x, text_y, text_color, queue,
        );
        x += rect_w + 8.0;
    }
}

fn tab_text_y(bar_top: i32, tab_bar_height: u32, glyphs: &GlyphCache) -> i32 {
    let cell_h = glyphs.cell_height() as i32;
    let baseline = glyphs.baseline();
    let top = bar_top + ((tab_bar_height as i32 - cell_h) / 2).max(0);
    top + (cell_h - baseline)
}

#[allow(dead_code)]
fn tab_text_y_f32(bar_top: f32, tab_bar_height: u32, glyphs: &GlyphCache) -> f32 {
    let cell_h = glyphs.cell_height() as f32;
    let baseline = glyphs.baseline() as f32;
    let top = bar_top + ((tab_bar_height as f32 - cell_h) / 2.0).max(0.0);
    top + (cell_h - baseline)
}

fn help_bar_metrics(
    height: u32,
    cell_h: u32,
    bottom_margin: u32,
    needs_status: bool,
) -> (u32, u32) {
    let bottom_margin = bottom_margin.min(height);
    let bar_bottom = height.saturating_sub(bottom_margin);
    let max_bar_h = bar_bottom.max(1);
    let bar_h = if needs_status {
        (cell_h.saturating_mul(2) + 8).min(max_bar_h)
    } else {
        (cell_h + 4).min(max_bar_h)
    };
    let bar_top = bar_bottom.saturating_sub(bar_h);
    (bar_top, bar_bottom)
}

pub fn draw_help_bar_cpu(
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    width: u32,
    height: u32,
    cell_h: u32,
    bottom_margin: u32,
    help_text: &str,
    link_label: Option<&str>,
    status_label: Option<String>,
) {
    if width == 0 || height == 0 {
        return;
    }
    let needs_status = status_label.is_some();
    let (bar_top, bar_bottom) = help_bar_metrics(height, cell_h, bottom_margin, needs_status);
    let bar_h = bar_bottom.saturating_sub(bar_top);

    fill_rect(
        frame,
        width,
        height,
        0,
        bar_top as i32,
        width as i32,
        bar_bottom as i32,
        rgba_bytes(OVERLAY_BAR),
    );

    let x = 12;
    let y = bar_top as i32 + ((bar_h as i32 - glyphs.cell_height() as i32) / 2).max(0);
    draw_text_line_clipped(
        glyphs,
        frame,
        width,
        height,
        x,
        y,
        help_text,
        rgba_bytes(OVERLAY_TEXT),
        (width as i32 - 10).max(x + 8),
    );

    if let Some(link) = link_label {
        let link_text = crate::render::primitives::truncate_label(link, 96);
        let link_w = text_width(glyphs, &link_text) as i32;
        let lx = (width as i32 - link_w - 12).max(x + 8);
        draw_text_line_clipped(
            glyphs,
            frame,
            width,
            height,
            lx,
            y,
            &link_text,
            rgba_bytes(OVERLAY_ACCENT),
            (width as i32 - 8).max(lx + 4),
        );
    }

    if let Some(status) = status_label {
        let status_text = crate::render::primitives::truncate_label(&status, 64);
        let status_w = text_width(glyphs, &status_text) as i32;
        let sx = (width as i32 - status_w - 12).max(x + 8);
        draw_text_line_clipped(
            glyphs,
            frame,
            width,
            height,
            sx,
            y + glyphs.cell_height() as i32 + 2,
            &status_text,
            rgba_bytes(OVERLAY_TEXT_MUTED),
            (width as i32 - 8).max(sx + 4),
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn build_help_bar_gpu(
    rects: &mut Vec<RectVertex>,
    glyphs_out: &mut Vec<GlyphVertex>,
    atlas: &mut GlyphAtlas,
    queue: &wgpu::Queue,
    glyphs: &mut GlyphCache,
    width: u32,
    height: u32,
    cell_h: u32,
    bottom_margin: u32,
    help_text: &str,
    link_label: Option<&str>,
    status_label: Option<String>,
) {
    if width == 0 || height == 0 {
        return;
    }
    let needs_status = status_label.is_some();
    let (bar_top, bar_bottom) = help_bar_metrics(height, cell_h, bottom_margin, needs_status);
    let bar_h = bar_bottom.saturating_sub(bar_top) as f32;
    let bar_top = bar_top as f32;
    crate::gpu::GpuRenderer::push_rect(
        rects,
        0.0,
        bar_top,
        width as f32,
        bar_bottom as f32,
        OVERLAY_BAR,
    );

    let x = 12.0;
    let y = bar_top + ((bar_h - glyphs.cell_height() as f32) / 2.0).max(0.0);
    crate::gpu::GpuRenderer::push_text_line(
        glyphs,
        atlas,
        glyphs_out,
        help_text,
        x,
        y,
        OVERLAY_TEXT,
        queue,
    );

    if let Some(link) = link_label {
        let link_text = crate::render::primitives::truncate_label(link, 96);
        let link_w = text_width(glyphs, &link_text) as f32;
        let lx = (width as f32 - link_w - 12.0).max(x + 8.0);
        crate::gpu::GpuRenderer::push_text_line(
            glyphs,
            atlas,
            glyphs_out,
            &link_text,
            lx,
            y,
            OVERLAY_ACCENT,
            queue,
        );
    }

    if let Some(status) = status_label {
        let status_text = crate::render::primitives::truncate_label(&status, 64);
        let status_w = text_width(glyphs, &status_text) as f32;
        let sx = (width as f32 - status_w - 12.0).max(x + 8.0);
        crate::gpu::GpuRenderer::push_text_line(
            glyphs,
            atlas,
            glyphs_out,
            &status_text,
            sx,
            y + glyphs.cell_height() as f32 + 2.0,
            OVERLAY_TEXT_MUTED,
            queue,
        );
    }
}
