//! CSS Gradient rasterization — linear, radial, conic.
use libm::{sin, cos, sqrt};
//! DO NOT EDIT. Regenerate with: scripts/generate_rasterizer.py
use libm::atan2;
use core::f64::consts::PI;

/// Color stop for gradients.
#[derive(Clone, Copy)]
pub struct GradientStop {
    pub r: u8, pub g: u8, pub b: u8, pub a: u8,
    pub position: f64, // 0.0 to 1.0
}

/// Interpolate between two color stops at position t.
#[inline]
fn lerp_stops(a: &GradientStop, b: &GradientStop, t: f64) -> (u8, u8, u8, u8) {
    let span = b.position - a.position;
    if span <= 0.0 { return (a.r, a.g, a.b, a.a); }
    let local = (t - a.position) / span;
    let local = local.max(0.0).min(1.0);
    (
        (a.r as f64 + (b.r as f64 - a.r as f64) * local) as u8,
        (a.g as f64 + (b.g as f64 - a.g as f64) * local) as u8,
        (a.b as f64 + (b.b as f64 - a.b as f64) * local) as u8,
        (a.a as f64 + (b.a as f64 - a.a as f64) * local) as u8,
    )
}

/// Find the two stops surrounding position t.
fn find_stops(stops: &[GradientStop], t: f64) -> (usize, usize) {
    let mut i = 0;
    while i + 1 < stops.len() && stops[i + 1].position < t {
        i += 1;
    }
    (i, (i + 1).min(stops.len() - 1))
}

/// Linear gradient: color varies along an axis.
/// angle is in radians, 0 = right, PI/2 = down.
pub fn linear_gradient(
    pixels: &mut [u8],
    stride: u32,
    fb_width: u32,
    fb_height: u32,
    x: u32, y: u32, w: u32, h: u32,
    angle: f64,
    stops: &[GradientStop],
) {
    if stops.len() < 2 { return; }
    let cos_a = cos(angle) as f64;
    let sin_a = sin(angle) as f64;

    for cy in y..(y + h).min(fb_height) {
        for cx in x..(x + w).min(fb_width) {
            let rel_x = (cx - x) as f64;
            let rel_y = (cy - y) as f64;
            // Project onto gradient axis
            let t = (rel_x * cos_a + rel_y * sin_a) / (w as f64 + h as f64) * 2.0;
            let t = t.max(0.0).min(1.0);
            let (i0, i1) = find_stops(stops, t);
            let (r, g, b, a) = lerp_stops(&stops[i0], &stops[i1], t);
            let pi = (cy * stride + cx * 4) as usize;
            if pi + 2 < pixels.len() {
                pixels[pi] = blend_alpha(pixels[pi], b, a);
                pixels[pi+1] = blend_alpha(pixels[pi+1], g, a);
                pixels[pi+2] = blend_alpha(pixels[pi+2], r, a);
            }
        }
    }
}

/// Radial gradient: color varies by distance from center.
pub fn radial_gradient(
    pixels: &mut [u8],
    stride: u32,
    fb_width: u32,
    fb_height: u32,
    x: u32, y: u32, w: u32, h: u32,
    cx: f64, cy: f64, // center as fraction of rect
    stops: &[GradientStop],
) {
    if stops.len() < 2 { return; }
    let center_x = x as f64 + w as f64 * cx;
    let center_y = y as f64 + h as f64 * cy;
    let max_dist = sqrt(w as f64 * w as f64 + h as f64 * h as f64) * 0.5;

    for py in y..(y + h).min(fb_height) {
        for px in x..(x + w).min(fb_width) {
            let dx = px as f64 - center_x;
            let dy = py as f64 - center_y;
            let dist = sqrt(dx * dx + dy * dy) / max_dist;
            let t = dist.max(0.0).min(1.0);
            let (i0, i1) = find_stops(stops, t);
            let (r, g, b, a) = lerp_stops(&stops[i0], &stops[i1], t);
            let pi = (py * stride + px * 4) as usize;
            if pi + 2 < pixels.len() {
                pixels[pi] = blend_alpha(pixels[pi], b, a);
                pixels[pi+1] = blend_alpha(pixels[pi+1], g, a);
                pixels[pi+2] = blend_alpha(pixels[pi+2], r, a);
            }
        }
    }
}

/// Conic gradient: color varies by angle around center.
pub fn conic_gradient(
    pixels: &mut [u8],
    stride: u32,
    fb_width: u32,
    fb_height: u32,
    x: u32, y: u32, w: u32, h: u32,
    from_angle: f64,
    cx: f64, cy: f64,
    stops: &[GradientStop],
) {
    if stops.len() < 2 { return; }
    let center_x = x as f64 + w as f64 * cx;
    let center_y = y as f64 + h as f64 * cy;

    for py in y..(y + h).min(fb_height) {
        for px in x..(x + w).min(fb_width) {
            let dx = px as f64 - center_x;
            let dy = py as f64 - center_y;
            let mut angle = atan2(dy, dx) + PI - from_angle;
            if angle < 0.0 { angle += 2.0 * PI; }
            let t = angle / (2.0 * PI);
            let (i0, i1) = find_stops(stops, t);
            let (r, g, b, a) = lerp_stops(&stops[i0], &stops[i1], t);
            let pi = (py * stride + px * 4) as usize;
            if pi + 2 < pixels.len() {
                pixels[pi] = blend_alpha(pixels[pi], b, a);
                pixels[pi+1] = blend_alpha(pixels[pi+1], g, a);
                pixels[pi+2] = blend_alpha(pixels[pi+2], r, a);
            }
        }
    }
}

#[inline]
fn blend_alpha(dst: u8, src: u8, alpha: u8) -> u8 {
    let a = alpha as u32;
    let inv = 255 - a;
    ((src as u32 * a + dst as u32 * inv) / 255) as u8
}
