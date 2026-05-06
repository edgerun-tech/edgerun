use std::time::Instant;

use crate::render::primitives::rainbow;
use crate::terminal::Rgba;

pub fn draw_border_cpu(
    frame: &mut [u8],
    width: u32,
    height: u32,
    thickness: u32,
    radius: u32,
    start_time: Instant,
    focused: bool,
    inset: u32,
) {
    if width == 0 || height == 0 || thickness == 0 {
        return;
    }
    if width <= inset * 2 || height <= inset * 2 {
        return;
    }
    let t = start_time.elapsed().as_secs_f32() * 1.2;
    let alpha = if focused { 255u16 } else { 128u16 };
    let r_outer = radius as f32;
    let r_inner = radius.saturating_sub(thickness) as f32;
    let inner_w = width - inset * 2;
    let inner_h = height - inset * 2;

    for y in 0..inner_h {
        for x in 0..inner_w {
            let on_border = x < thickness
                || y < thickness
                || x >= inner_w.saturating_sub(thickness)
                || y >= inner_h.saturating_sub(thickness);
            if !on_border {
                continue;
            }

            if radius > 0 {
                let corner_x = if x < radius {
                    radius as f32 - 0.5
                } else if x >= inner_w.saturating_sub(radius) {
                    inner_w as f32 - radius as f32 + 0.5
                } else {
                    f32::INFINITY
                };
                let corner_y = if y < radius {
                    radius as f32 - 0.5
                } else if y >= inner_h.saturating_sub(radius) {
                    inner_h as f32 - radius as f32 + 0.5
                } else {
                    f32::INFINITY
                };

                if corner_x.is_finite() && corner_y.is_finite() {
                    let dx = (corner_x - x as f32).abs();
                    let dy = (corner_y - y as f32).abs();
                    let dist2 = dx * dx + dy * dy;
                    if dist2 > r_outer * r_outer || dist2 < r_inner * r_inner {
                        continue;
                    }
                }
            }

            let pos = (x as f32 / width.max(1) as f32) + (y as f32 / height.max(1) as f32);
            let [r, g, b] = rainbow(t + pos * std::f32::consts::TAU);
            let px = x + inset;
            let py = y + inset;
            let idx = ((py * width + px) * 4) as usize;
            let inv = 255u16.saturating_sub(alpha);
            frame[idx] = ((frame[idx] as u16 * inv + r as u16 * alpha) / 255) as u8;
            frame[idx + 1] = ((frame[idx + 1] as u16 * inv + g as u16 * alpha) / 255) as u8;
            frame[idx + 2] = ((frame[idx + 2] as u16 * inv + b as u16 * alpha) / 255) as u8;
            frame[idx + 3] = 255;
        }
    }
}

pub fn build_border_gpu(
    rects: &mut Vec<crate::gpu::RectVertex>,
    width: u32,
    height: u32,
    thickness: u32,
    _radius: u32,
    start_time: Instant,
    focused: bool,
    inset: u32,
) {
    if width == 0 || height == 0 || thickness == 0 {
        return;
    }
    if width <= inset * 2 || height <= inset * 2 {
        return;
    }
    let t = start_time.elapsed().as_secs_f32() * 1.2;
    let alpha = if focused { 255 } else { 128 };
    let inner_w = width.saturating_sub(inset * 2);
    let inner_h = height.saturating_sub(inset * 2);
    let thickness_f = thickness as f32;
    let width_f = width as f32;
    let height_f = height as f32;
    let inset_f = inset as f32;
    let push_colored_rect =
        |rects: &mut Vec<crate::gpu::RectVertex>, x0: f32, y0: f32, x1: f32, y1: f32, pos: f32| {
            let [r, g, b] = rainbow(t + pos * std::f32::consts::TAU);
            crate::gpu::GpuRenderer::push_rect(rects, x0, y0, x1, y1, Rgba { r, g, b, a: alpha });
        };

    // Top and bottom edges, 1px wide strips to preserve the animated rainbow.
    for x in 0..inner_w {
        let px = inset + x;
        let pos_top = (px as f32 / width_f) + (inset_f / height_f);
        push_colored_rect(
            rects,
            px as f32,
            inset_f,
            px as f32 + 1.0,
            inset_f + thickness_f,
            pos_top,
        );

        let py_bottom = inset + inner_h - thickness;
        let pos_bottom = (px as f32 / width_f) + (py_bottom as f32 / height_f);
        push_colored_rect(
            rects,
            px as f32,
            py_bottom as f32,
            px as f32 + 1.0,
            py_bottom as f32 + thickness_f,
            pos_bottom,
        );
    }

    // Left and right edges; skip the corners because they are already filled above.
    let vertical_span = inner_h.saturating_sub(thickness * 2);
    for y in 0..vertical_span {
        let py = inset as u32 + thickness + y;
        let pos_left = (inset_f / width_f) + (py as f32 / height_f);
        push_colored_rect(
            rects,
            inset_f,
            py as f32,
            inset_f + thickness_f,
            py as f32 + 1.0,
            pos_left,
        );

        let px_right = inset as u32 + inner_w as u32 - thickness;
        let pos_right = (px_right as f32 / width_f) + (py as f32 / height_f);
        push_colored_rect(
            rects,
            px_right as f32,
            py as f32,
            px_right as f32 + thickness_f,
            py as f32 + 1.0,
            pos_right,
        );
    }
}
