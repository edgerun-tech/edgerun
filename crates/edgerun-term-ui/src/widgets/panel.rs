#![allow(clippy::too_many_arguments)]

use pixels::wgpu::Queue;
use term_core::gpu::GpuRenderer;
use term_core::gpu::{GlyphAtlas, GlyphVertex, RectVertex};
use term_core::render::GlyphCache;
use term_core::render::primitives::{fill_rect, truncate_label};
use term_core::terminal::Rgba;

/// Basic panel layout used by list-style widgets.
#[derive(Clone, Copy, Debug)]
pub struct PanelLayout {
    pub rect: (i32, i32, i32, i32),
    pub padding: i32,
    pub item_height: i32,
}

pub const MODAL_PANEL_W_FRAC: f32 = 0.85;
pub const MODAL_PANEL_H_FRAC: f32 = 0.75;
pub const MODAL_PANEL_MIN_W: f32 = 360.0;
pub const MODAL_PANEL_MIN_H: f32 = 240.0;

pub fn modal_panel_rect(
    view_w: u32,
    view_h: u32,
    w_frac: f32,
    h_frac: f32,
    min_w: f32,
    min_h: f32,
) -> (i32, i32, i32, i32) {
    let panel_w = (view_w as f32 * w_frac).max(min_w).min(view_w as f32);
    let panel_h = (view_h as f32 * h_frac).max(min_h).min(view_h as f32);
    let x0 = ((view_w as f32 - panel_w) * 0.5).max(0.0) as i32;
    let y0 = ((view_h as f32 - panel_h) * 0.5).max(0.0) as i32;
    let x1 = (x0 as f32 + panel_w).min(view_w as f32) as i32;
    let y1 = (y0 as f32 + panel_h).min(view_h as f32) as i32;
    (x0, y0, x1, y1)
}

pub fn center_panel_rect(
    view_w: u32,
    view_h: u32,
    panel_w: i32,
    panel_h: i32,
    margin: i32,
) -> (i32, i32, i32, i32) {
    let w = panel_w.max(0).min(view_w as i32 - margin * 2);
    let h = panel_h.max(0).min(view_h as i32 - margin * 2);
    let x0 = ((view_w as i32 - w) / 2).max(margin);
    let y0 = ((view_h as i32 - h) / 2).max(margin);
    let x1 = (x0 + w).min(view_w as i32 - margin);
    let y1 = (y0 + h).min(view_h as i32 - margin);
    (x0, y0, x1, y1)
}

/// Clamp an anchored panel to the viewport and return its rect.
pub fn clamp_panel_to_view(
    anchor: (i32, i32),
    size: (i32, i32),
    view_w: u32,
    view_h: u32,
    margin: i32,
) -> (i32, i32, i32, i32) {
    let (w, h) = size;
    let mut x0 = anchor.0;
    let mut y0 = anchor.1;
    let max_x = view_w as i32 - w - margin;
    let max_y = view_h as i32 - h - margin;
    x0 = x0.min(max_x.max(margin)).max(margin);
    y0 = y0.min(max_y.max(margin)).max(margin);
    let x1 = (x0 + w).min(view_w as i32 - margin);
    let y1 = (y0 + h).min(view_h as i32 - margin);
    (x0, y0, x1, y1)
}

/// Render a simple list panel in the CPU path.
pub fn list_panel_cpu(
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    width: u32,
    height: u32,
    layout: PanelLayout,
    bg: [u8; 4],
    items: &[(String, bool)], // (label, enabled)
    hovered: Option<usize>,
    selected: Option<usize>,
) {
    let (x0, y0, x1, y1) = layout.rect;
    fill_rect(frame, width, height, x0, y0, x1, y1, bg);
    let padding = layout.padding;
    let item_h = layout.item_height;
    for (idx, (label, enabled)) in items.iter().enumerate() {
        let row_y = y0 + padding + idx as i32 * item_h;
        if let Some(sel) = selected
            && sel == idx
        {
            fill_rect(
                frame,
                width,
                height,
                x0 + 2,
                row_y - 2,
                x1 - 2,
                row_y + item_h - 2,
                [90, 140, 255, 64],
            );
        }
        if let Some(hov) = hovered
            && hov == idx
        {
            fill_rect(
                frame,
                width,
                height,
                x0 + 2,
                row_y - 2,
                x1 - 2,
                row_y + item_h - 2,
                [80, 120, 200, 96],
            );
        }
        let color = if *enabled {
            [235, 235, 235, 255]
        } else {
            [150, 150, 150, 200]
        };
        let clipped = truncate_label(label, 96);
        term_core::render::draw_text_line_clipped(
            glyphs,
            frame,
            width,
            height,
            x0 + padding,
            row_y,
            &clipped,
            color,
            x1 - padding,
        );
    }
}

/// Render a simple list panel in the GPU path.
pub fn list_panel_gpu(
    rects: &mut Vec<RectVertex>,
    glyphs_out: &mut Vec<GlyphVertex>,
    atlas: &mut GlyphAtlas,
    queue: &Queue,
    glyphs: &mut GlyphCache,
    layout: PanelLayout,
    bg: Rgba,
    items: &[(String, bool)],
    hovered: Option<usize>,
    selected: Option<usize>,
) {
    let (x0, y0, x1, y1) = layout.rect;
    GpuRenderer::push_rect(rects, x0 as f32, y0 as f32, x1 as f32, y1 as f32, bg);
    let padding = layout.padding as f32;
    let item_h = layout.item_height as f32;
    for (idx, (label, enabled)) in items.iter().enumerate() {
        let row_y = y0 as f32 + padding + idx as f32 * item_h;
        if let Some(sel) = selected
            && sel == idx
        {
            GpuRenderer::push_rect(
                rects,
                x0 as f32 + 2.0,
                row_y - 2.0,
                x1 as f32 - 2.0,
                row_y + item_h - 2.0,
                Rgba {
                    r: 90,
                    g: 140,
                    b: 255,
                    a: 64,
                },
            );
        }
        if let Some(hov) = hovered
            && hov == idx
        {
            GpuRenderer::push_rect(
                rects,
                x0 as f32 + 2.0,
                row_y - 2.0,
                x1 as f32 - 2.0,
                row_y + item_h - 2.0,
                Rgba {
                    r: 80,
                    g: 120,
                    b: 200,
                    a: 96,
                },
            );
        }
        let clipped = truncate_label(label, 96);
        let color = if *enabled {
            Rgba {
                r: 235,
                g: 235,
                b: 235,
                a: 255,
            }
        } else {
            Rgba {
                r: 150,
                g: 150,
                b: 150,
                a: 200,
            }
        };
        GpuRenderer::push_text_line(
            glyphs,
            atlas,
            glyphs_out,
            &clipped,
            x0 as f32 + padding,
            row_y,
            color,
            queue,
        );
    }
}
