// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::too_many_arguments)]

use std::time::Instant;

use crate::debug::{DebugOverlay, DebugRendererUsed};
use crate::widgets::center_panel_rect;
use term_core::gpu::{GlyphAtlas, GlyphVertex, GpuRenderer, RectVertex};
use term_core::render::primitives::{OverlayRect, push_overlay_rect};
use term_core::render::{
    OVERLAY_DIM, OVERLAY_PANEL, OVERLAY_PANEL_INNER, OVERLAY_TEXT, OVERLAY_TEXT_MUTED, fill_rect,
    rgba_bytes,
};
use term_core::terminal::{Rgba, ansi_color};
use term_core::text::GlyphCache;

pub fn debug_overlay_lines(
    overlay: &DebugOverlay,
    last_used: Option<DebugRendererUsed>,
    cell_w: u32,
    cell_h: u32,
) -> Vec<(String, Rgba)> {
    let last = last_used
        .map(|r| r.to_string())
        .unwrap_or_else(|| "n/a".to_string());
    let bench = overlay.benchmark_snapshot(Instant::now());
    let (bench_line, bench_color) = if bench.active {
        (
            format!(
                "Benchmark: running {:.1}/{:.1}s frames={} avg={:.2}ms max={:.2}ms",
                bench.elapsed.as_secs_f32(),
                bench.duration.as_secs_f32().max(0.1),
                bench.frames,
                bench.avg_ms,
                bench.max_ms
            ),
            ansi_color(2, true),
        )
    } else if bench.last_frames > 0 {
        (
            format!(
                "Benchmark: last {:.1}s frames={} avg={:.2}ms max={:.2}ms (B to run)",
                bench.last_duration.as_secs_f32(),
                bench.last_frames,
                bench.last_avg_ms,
                bench.last_max_ms
            ),
            ansi_color(6, false),
        )
    } else {
        (
            "Benchmark: idle (B to start)".to_string(),
            OVERLAY_TEXT_MUTED,
        )
    };
    vec![
        ("Debug overlay (F3 to close)".to_string(), OVERLAY_TEXT),
        (
            format!("Render mode: {} (last used: {last})", overlay.render_mode()),
            OVERLAY_TEXT,
        ),
        (bench_line, bench_color),
        (
            format!("Input mode: {}", overlay.input_mode()),
            ansi_color(6, false),
        ),
        (
            "Keys: R cycle renderer • I cycle input • B benchmark • Esc exits".to_string(),
            ansi_color(4, true),
        ),
        (
            "Preview shows mixed-width glyphs to expose shaping issues.".to_string(),
            OVERLAY_TEXT_MUTED,
        ),
        (
            format!("Cell size: {}x{}", cell_w.max(1), cell_h.max(1)),
            ansi_color(7, false),
        ),
        ("".to_string(), OVERLAY_TEXT),
        (
            "ASCII: The quick brown fox jumps over 13 lazy dogs.".to_string(),
            ansi_color(2, true),
        ),
        (
            "Unicode: こんにちは – Привет – 😃 – 🏳️‍🌈 – café – fi ligature?".to_string(),
            ansi_color(5, true),
        ),
        (
            "Box drawing: ┌─┬─┐ │ │ │ │ └─┴─┘ █▓▒░".to_string(),
            ansi_color(4, false),
        ),
        (
            "Wide chars: 你好世界 • हिन्दी • 한국어 • العربية".to_string(),
            ansi_color(3, true),
        ),
    ]
}

fn color_bytes(c: Rgba) -> [u8; 4] {
    [c.r, c.g, c.b, c.a]
}

fn palette_colors() -> Vec<Rgba> {
    let mut colors = Vec::with_capacity(16);
    for i in 0..8 {
        colors.push(ansi_color(i, false));
    }
    for i in 0..8 {
        colors.push(ansi_color(i, true));
    }
    colors
}

fn draw_palette_row(frame: &mut [u8], width: u32, height: u32, x: i32, y: i32, h: i32) {
    let colors = palette_colors();
    let sw = (h * 2).max(10);
    let mut cursor_x = x;
    for c in colors {
        fill_rect(
            frame,
            width,
            height,
            cursor_x,
            y,
            cursor_x + sw,
            y + h,
            [c.r, c.g, c.b, 255],
        );
        cursor_x += sw + 4;
    }
}

fn push_palette_row(rects: &mut Vec<RectVertex>, x: f32, y: f32, h: f32, cell_w: f32) {
    let colors = palette_colors();
    let sw = (h * 2.0).max(cell_w * 2.0).max(10.0);
    let mut cursor_x = x;
    for c in colors {
        push_overlay_rect(
            rects,
            OverlayRect {
                x0: cursor_x,
                y0: y,
                x1: cursor_x + sw,
                y1: y + h,
                color: Rgba {
                    r: c.r,
                    g: c.g,
                    b: c.b,
                    a: 255,
                },
            },
        );
        cursor_x += sw + 4.0;
    }
}

pub fn draw_debug_panel(
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    width: u32,
    height: u32,
    overlay: &DebugOverlay,
    last_used: Option<DebugRendererUsed>,
    cell_w: u32,
    cell_h: u32,
) {
    let lines = debug_overlay_lines(overlay, last_used, cell_w, cell_h);
    let padding = 12;
    if width == 0 || height == 0 || width <= (padding * 2) as u32 || height <= (padding * 2) as u32
    {
        return;
    }
    let line_h = glyphs.cell_height() as i32 + 4;
    let panel_w = (width as i32 * 3 / 4).max(240);
    let max_y = height.saturating_sub(10) as i32;
    let palette_h = (cell_h as i32).max(line_h);
    let panel_h = (line_h * lines.len() as i32 + palette_h + padding * 2 + 6).min(max_y);
    let (x0, y0, x1, y1) = center_panel_rect(width, height, panel_w, panel_h, padding);

    // Opaque background so preview glyphs are not ghosted beneath the panel.
    fill_rect(
        frame,
        width,
        height,
        x0,
        y0,
        x1,
        y1,
        rgba_bytes(OVERLAY_PANEL),
    );
    fill_rect(
        frame,
        width,
        height,
        x0 + 1,
        y0 + 1,
        x1 - 1,
        y1 - 1,
        rgba_bytes(OVERLAY_PANEL_INNER),
    );

    let mut y = y0 + 10;
    for (line, color) in &lines {
        if y + line_h > y1 {
            break;
        }
        draw_text_line_clipped(
            glyphs,
            frame,
            width,
            height,
            x0 + 10,
            y,
            line,
            color_bytes(*color),
            x1 - 10,
        );
        y += line_h;
    }
    if y + palette_h + 4 < y1 {
        y += 4;
        draw_palette_row(frame, width, height, x0 + 10, y, palette_h);
    }
}

pub fn draw_debug_overlay_cpu(
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    width: u32,
    height: u32,
    overlay: &DebugOverlay,
    last_used: Option<DebugRendererUsed>,
    cell_w: u32,
    cell_h: u32,
) {
    fill_rect(
        frame,
        width,
        height,
        0,
        0,
        width as i32,
        height as i32,
        rgba_bytes(OVERLAY_DIM),
    );
    draw_debug_panel(
        glyphs, frame, width, height, overlay, last_used, cell_w, cell_h,
    );
}

pub fn build_debug_overlay_gpu(
    rects: &mut Vec<RectVertex>,
    glyphs_out: &mut Vec<GlyphVertex>,
    atlas: &mut GlyphAtlas,
    queue: &wgpu::Queue,
    glyphs: &mut GlyphCache,
    width: u32,
    height: u32,
    overlay: &DebugOverlay,
) {
    if width == 0 || height == 0 {
        return;
    }
    push_overlay_rect(
        rects,
        OverlayRect {
            x0: 0.0,
            y0: 0.0,
            x1: width as f32,
            y1: height as f32,
            color: OVERLAY_DIM,
        },
    );
    let (cell_w, cell_h) = glyphs.cell_size();
    let lines = debug_overlay_lines(overlay, overlay.last_used_renderer(), cell_w, cell_h);
    let padding = 12.0;
    if width as f32 <= padding * 2.0 || height as f32 <= padding * 2.0 {
        return;
    }
    let max_h = height as f32 - 10.0;
    if max_h <= 0.0 {
        return;
    }
    let panel_w = (width as f32 * 0.75).max(240.0);
    let line_h = glyphs.cell_height() as f32 + 4.0;
    let palette_h = (cell_h as f32).max(line_h);
    let panel_h = (line_h * lines.len() as f32 + palette_h + padding * 2.0 + 6.0).min(max_h);
    let (x0_i, y0_i, x1_i, y1_i) = center_panel_rect(
        width,
        height,
        panel_w as i32,
        panel_h as i32,
        padding as i32,
    );
    let x0 = x0_i as f32;
    let y0 = y0_i as f32;
    let x1 = x1_i as f32;
    let y1 = y1_i as f32;

    push_overlay_rect(
        rects,
        OverlayRect {
            x0,
            y0,
            x1,
            y1,
            color: OVERLAY_PANEL,
        },
    );
    push_overlay_rect(
        rects,
        OverlayRect {
            x0: x0 + 1.0,
            y0: y0 + 1.0,
            x1: x1 - 1.0,
            y1: y1 - 1.0,
            color: OVERLAY_PANEL_INNER,
        },
    );

    let mut y = y0 + 10.0;
    for (line, color) in &lines {
        if y + line_h > y1 {
            break;
        }
        GpuRenderer::push_text_line(glyphs, atlas, glyphs_out, line, x0 + 12.0, y, *color, queue);
        y += line_h;
    }
    if y + palette_h + 4.0 < y1 {
        y += 4.0;
        push_palette_row(rects, x0 + 12.0, y, palette_h, glyphs.cell_size().0 as f32);
    }
}

/// CPU debug scene (background + panel + border).
pub fn draw_debug_scene(
    overlay: &DebugOverlay,
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    cell_w: u32,
    cell_h: u32,
    start_time: Instant,
    focused: bool,
    last_used: Option<DebugRendererUsed>,
) {
    let bg = overlay.preview().default_bg();
    draw_background(frame, frame_width, frame_height, start_time, bg);
    fill_rect(
        frame,
        frame_width,
        frame_height,
        0,
        0,
        frame_width as i32,
        frame_height as i32,
        rgba_bytes(OVERLAY_DIM),
    );
    draw_debug_panel(
        glyphs,
        frame,
        frame_width,
        frame_height,
        overlay,
        last_used,
        cell_w,
        cell_h,
    );
    // Border drawing stays in the caller for now.
    let _ = (frame_width, frame_height, start_time, focused);
}
use pixels::wgpu;
use term_core::render::{draw_background, draw_text_line_clipped};
