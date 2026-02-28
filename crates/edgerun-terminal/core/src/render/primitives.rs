// SPDX-License-Identifier: Apache-2.0
use crate::render::cpu::msdf_alpha;
use crate::terminal::Rgba;
use crate::text::{GlyphCache, ShapedGlyph};
use std::time::Instant;

/// Basic rectangular overlay primitive to support higher-level UI drawing.
#[derive(Clone, Copy, Debug)]
pub struct OverlayRect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub color: Rgba,
}

/// Push an overlay rectangle into the GPU rect buffer.
#[cfg(not(target_arch = "wasm32"))]
pub fn push_overlay_rect(rects: &mut Vec<crate::gpu::RectVertex>, rect: OverlayRect) {
    crate::gpu::GpuRenderer::push_rect(rects, rect.x0, rect.y0, rect.x1, rect.y1, rect.color);
}

/// CPU-side rectangle fill helper.
#[allow(clippy::too_many_arguments)]
pub fn fill_rect(
    frame: &mut [u8],
    width: u32,
    height: u32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 4],
) {
    if color[3] == 0 {
        return;
    }
    let a = color[3] as u16;
    let inv = 255 - a;
    let x_start = x0.max(0) as u32;
    let y_start = y0.max(0) as u32;
    let x_end = x1.min(width as i32) as u32;
    let y_end = y1.min(height as i32) as u32;
    for y in y_start..y_end {
        let row_start = (y * width * 4) as usize;
        for x in x_start..x_end {
            let idx = row_start + (x * 4) as usize;
            if a == 255 {
                frame[idx] = color[0];
                frame[idx + 1] = color[1];
                frame[idx + 2] = color[2];
            } else {
                frame[idx] = ((frame[idx] as u16 * inv + color[0] as u16 * a) / 255) as u8;
                frame[idx + 1] = ((frame[idx + 1] as u16 * inv + color[1] as u16 * a) / 255) as u8;
                frame[idx + 2] = ((frame[idx + 2] as u16 * inv + color[2] as u16 * a) / 255) as u8;
            }
            frame[idx + 3] = 255;
        }
    }
}

/// Rainbow color helper used in borders/animations.
pub fn rainbow(phase: f32) -> [u8; 3] {
    let r = ((phase).sin() * 0.5 + 0.5) * 255.0;
    let g = ((phase + 2.094395_f32).sin() * 0.5 + 0.5) * 255.0;
    let b = ((phase + 4.18879_f32).sin() * 0.5 + 0.5) * 255.0;
    [r as u8, g as u8, b as u8]
}

/// Truncate a label to a bounded length, appending ellipsis if needed.
pub fn truncate_label(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else if max <= 3 {
        ".".repeat(max)
    } else {
        format!("{}...", &text[..max - 3])
    }
}

/// Fill entire frame with the provided background color.
pub fn draw_background(frame: &mut [u8], width: u32, height: u32, _start_time: Instant, bg: Rgba) {
    if width == 0 || height == 0 {
        return;
    }
    let mut bg = bg;
    if bg.a == 0 {
        bg.a = 255;
    }

    for chunk in frame.chunks_exact_mut(4) {
        chunk[0] = bg.r;
        chunk[1] = bg.g;
        chunk[2] = bg.b;
        chunk[3] = bg.a;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text_line(
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    width: u32,
    height: u32,
    mut x: i32,
    y: i32,
    text: &str,
    color: [u8; 4],
) {
    if let Some(run) = glyphs.shape_text(text) {
        draw_shaped_run(glyphs, frame, width, height, x as f32, y, &run, color);
        return;
    }

    let baseline = glyphs.baseline();
    let use_sdf = glyphs.use_sdf();
    let msdf_min_width = glyphs.msdf_min_width();
    for ch in text.chars() {
        let (metrics, bitmap, is_color) = glyphs.rasterize(ch);
        let gx = x + metrics.xmin;
        let gy = y + baseline - metrics.ymin;
        for py in 0..metrics.height {
            for px in 0..metrics.width {
                let src_idx = (py * metrics.width + px) as usize * 4;
                let alpha = if !is_color && use_sdf {
                    msdf_alpha(
                        bitmap[src_idx],
                        bitmap[src_idx + 1],
                        bitmap[src_idx + 2],
                        msdf_min_width,
                    )
                } else {
                    bitmap[src_idx + 3]
                };
                if alpha == 0 {
                    continue;
                }
                let sr = if is_color { bitmap[src_idx] } else { color[0] };
                let sg = if is_color {
                    bitmap[src_idx + 1]
                } else {
                    color[1]
                };
                let sb = if is_color {
                    bitmap[src_idx + 2]
                } else {
                    color[2]
                };
                let tx = gx + px as i32;
                let ty = gy + py as i32;
                if tx < 0 || ty < 0 || tx >= width as i32 || ty >= height as i32 {
                    continue;
                }
                let idx = ((ty as u32 * width + tx as u32) * 4) as usize;
                let glyph_alpha = alpha as u16;
                let a = (glyph_alpha * color[3] as u16) / 255;
                if a == 0 {
                    continue;
                }
                let inv = 255u16.saturating_sub(a);
                frame[idx] = ((frame[idx] as u16 * inv + sr as u16 * a) / 255) as u8;
                frame[idx + 1] = ((frame[idx + 1] as u16 * inv + sg as u16 * a) / 255) as u8;
                frame[idx + 2] = ((frame[idx + 2] as u16 * inv + sb as u16 * a) / 255) as u8;
                frame[idx + 3] = 255;
            }
        }
        x += metrics.advance_width.ceil() as i32;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text_line_clipped(
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    text: &str,
    color: [u8; 4],
    max_x: i32,
) {
    if max_x <= x {
        return;
    }

    let available = max_x - x;
    let raw_width: i32 = text.chars().map(|ch| glyphs.advance_width(ch)).sum();
    if raw_width > available {
        draw_text_line(glyphs, frame, width, height, x, y, "...", color);
        return;
    }

    if let Some(run) = glyphs.shape_text(text) {
        let total_width: f32 = run.iter().map(|g| g.x_advance).sum();
        if total_width.ceil() as i32 > available {
            if let Some(ellipsis) = glyphs.shape_text("...") {
                draw_shaped_run(glyphs, frame, width, height, x as f32, y, &ellipsis, color);
            }
            return;
        }
        let mut cursor = x as f32;
        let mut clipped = Vec::with_capacity(run.len());
        for g in run {
            let next = cursor + g.x_advance;
            if next.ceil() as i32 > max_x {
                if let Some(ellipsis) = glyphs.shape_text("...") {
                    draw_shaped_run(glyphs, frame, width, height, x as f32, y, &ellipsis, color);
                }
                return;
            }
            clipped.push(g);
            cursor = next;
        }
        draw_shaped_run(glyphs, frame, width, height, x as f32, y, &clipped, color);
        return;
    }

    let mut acc = String::new();
    let mut cursor = x;
    for ch in text.chars() {
        let adv = glyphs.advance_width(ch);
        if cursor + adv > max_x {
            acc.clear();
            acc.push_str("...");
            break;
        }
        acc.push(ch);
        cursor += adv;
    }

    draw_text_line(glyphs, frame, width, height, x, y, &acc, color);
}

#[allow(clippy::too_many_arguments)]
fn draw_shaped_run(
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    width: u32,
    height: u32,
    mut pen_x: f32,
    y: i32,
    run: &[ShapedGlyph],
    color: [u8; 4],
) {
    let baseline = glyphs.baseline();
    let use_sdf = glyphs.use_sdf();
    let msdf_min_width = glyphs.msdf_min_width();
    for g in run {
        let (metrics, bitmap, is_color) =
            glyphs.rasterize_indexed_in_font(g.font_idx, g.glyph_id as usize);
        if metrics.width == 0 || metrics.height == 0 {
            pen_x += g.x_advance;
            continue;
        }

        let gx = (pen_x + g.x_offset + metrics.xmin as f32).round() as i32;
        let gy = (y as f32 + g.y_offset + (baseline - metrics.ymin) as f32).round() as i32;

        for py in 0..metrics.height {
            for px in 0..metrics.width {
                let src_idx = (py * metrics.width + px) as usize * 4;
                let alpha = if !is_color && use_sdf {
                    msdf_alpha(
                        bitmap[src_idx],
                        bitmap[src_idx + 1],
                        bitmap[src_idx + 2],
                        msdf_min_width,
                    )
                } else {
                    bitmap[src_idx + 3]
                };
                if alpha == 0 {
                    continue;
                }
                let tx = gx + px as i32;
                let ty = gy + py as i32;
                if tx < 0 || ty < 0 || tx >= width as i32 || ty >= height as i32 {
                    continue;
                }
                let idx = ((ty as u32 * width + tx as u32) * 4) as usize;
                let glyph_alpha = alpha as u16;
                let a = (glyph_alpha * color[3] as u16) / 255;
                if a == 0 {
                    continue;
                }
                let inv = 255u16.saturating_sub(a);
                let sr = if is_color { bitmap[src_idx] } else { color[0] };
                let sg = if is_color {
                    bitmap[src_idx + 1]
                } else {
                    color[1]
                };
                let sb = if is_color {
                    bitmap[src_idx + 2]
                } else {
                    color[2]
                };
                frame[idx] = ((frame[idx] as u16 * inv + sr as u16 * a) / 255) as u8;
                frame[idx + 1] = ((frame[idx + 1] as u16 * inv + sg as u16 * a) / 255) as u8;
                frame[idx + 2] = ((frame[idx + 2] as u16 * inv + sb as u16 * a) / 255) as u8;
                frame[idx + 3] = 255;
            }
        }

        pen_x += g.x_advance;
    }
}
