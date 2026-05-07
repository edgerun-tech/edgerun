use std::time::Instant;

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
    let t = start_time.elapsed().as_secs_f32();
    let base_alpha = if focused { 200u16 } else { 120u16 };
    let pulse = ((t * 0.2).sin() * 0.5 + 0.5) * 40.0;
    let alpha = (base_alpha as f32 + pulse).min(255.0) as u16;
    let r_outer = radius as f32;
    let r_inner = radius.saturating_sub(thickness) as f32;
    let inner_w = width - inset * 2;
    let inner_h = height - inset * 2;
    let color = Rgba {
        r: 76,
        g: 108,
        b: 132,
        a: 255,
    };

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

            let px = x + inset;
            let py = y + inset;
            let idx = ((py * width + px) * 4) as usize;
            let inv = 255u16.saturating_sub(alpha);
            frame[idx] = ((frame[idx] as u16 * inv + color.r as u16 * alpha) / 255) as u8;
            frame[idx + 1] = ((frame[idx + 1] as u16 * inv + color.g as u16 * alpha) / 255) as u8;
            frame[idx + 2] = ((frame[idx + 2] as u16 * inv + color.b as u16 * alpha) / 255) as u8;
            frame[idx + 3] = 255;
        }
    }
}
