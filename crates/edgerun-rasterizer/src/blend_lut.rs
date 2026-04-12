//! Blend mode lookup tables — generated from CSS Compositing Level 1 blend modes.
use libm::sqrt;

/// Blend a source channel over destination channel.
/// Uses integer math only — no floats.

#[inline(always)]
pub fn blend_normal(src: u8, _dst: u8) -> u8 { src }

#[inline(always)]
pub fn blend_multiply(src: u8, dst: u8) -> u8 {
    ((src as u32 * dst as u32) / 255) as u8
}

#[inline(always)]
pub fn blend_screen(src: u8, dst: u8) -> u8 {
    (255 - ((255 - src as u32) * (255 - dst as u32)) / 255) as u8
}

#[inline(always)]
pub fn blend_overlay(src: u8, dst: u8) -> u8 {
    if dst < 128 { blend_multiply(src, 2 * dst) }
    else { blend_screen(src, 2 * dst - 255) }
}

#[inline(always)]
pub fn blend_darken(src: u8, dst: u8) -> u8 {
    src.min(dst)
}

#[inline(always)]
pub fn blend_lighten(src: u8, dst: u8) -> u8 {
    src.max(dst)
}

#[inline(always)]
pub fn blend_colordodge(src: u8, dst: u8) -> u8 {
    if dst == 0 { 0 }
    else if src == 255 { 255 }
    else { (255 * dst as u32 / (255 - src as u32)).min(255) as u8 }
}

#[inline(always)]
pub fn blend_colorburn(src: u8, dst: u8) -> u8 {
    if dst == 255 { 255 }
    else if src == 0 { 0 }
    else { (255 - 255 * (255 - dst as u32) / src as u32) as u8 }
}

#[inline(always)]
pub fn blend_hardlight(src: u8, dst: u8) -> u8 {
    if src < 128 { blend_multiply(2 * src, dst) }
    else { blend_screen(2 * src - 255, dst) }
}

#[inline(always)]
pub fn blend_softlight(src: u8, dst: u8) -> u8 {
    let d = dst as u32;
    let s = src as u32;
    if s <= 128 {
        (d - (255 - d) * d * (128 - s) / (128 * 255)) as u8
    } else {
        let v = if d <= 64 { d } else { (sqrt(d.max(1) as f64) as u32 * 255 / 16).min(255) };
        (d + d * (255 - d) * (s - 128) / (128 * v)) as u8
    }
}

#[inline(always)]
pub fn blend_difference(src: u8, dst: u8) -> u8 {
    (src as i32 - dst as i32).unsigned_abs() as u8
}

#[inline(always)]
pub fn blend_exclusion(src: u8, dst: u8) -> u8 {
    (src as u32 + dst as u32 - 2 * src as u32 * dst as u32 / 255) as u8
}

#[inline(always)]
pub fn blend_hue(_src_r: u8, _src_g: u8, _src_b: u8, dst_r: u8, dst_g: u8, dst_b: u8, out_r: &mut u8, out_g: &mut u8, out_b: &mut u8) {
    let lum = (dst_r as u32 * 77 + dst_g as u32 * 150 + dst_b as u32 * 29) / 256;
    *out_r = lum.min(255) as u8;
    *out_g = lum.min(255) as u8;
    *out_b = lum.min(255) as u8;
}

#[inline(always)]
pub fn blend_saturation(_src_r: u8, _src_g: u8, _src_b: u8, dst_r: u8, dst_g: u8, dst_b: u8, out_r: &mut u8, out_g: &mut u8, out_b: &mut u8) {
    let lum = (dst_r as u32 * 77 + dst_g as u32 * 150 + dst_b as u32 * 29) / 256;
    *out_r = lum.min(255) as u8;
    *out_g = lum.min(255) as u8;
    *out_b = lum.min(255) as u8;
}

#[inline(always)]
pub fn blend_color(src_r: u8, src_g: u8, src_b: u8, dst_r: u8, dst_g: u8, dst_b: u8, out_r: &mut u8, out_g: &mut u8, out_b: &mut u8) {
    let lum = (dst_r as u32 * 77 + dst_g as u32 * 150 + dst_b as u32 * 29) / 256;
    let _ = lum;
    *out_r = src_r.min(255) as u8;
    *out_g = src_g.min(255) as u8;
    *out_b = src_b.min(255) as u8;
}

#[inline(always)]
pub fn blend_luminosity(src_r: u8, src_g: u8, src_b: u8, _dst_r: u8, _dst_g: u8, _dst_b: u8, out_r: &mut u8, out_g: &mut u8, out_b: &mut u8) {
    let lum = (src_r as u32 * 77 + src_g as u32 * 150 + src_b as u32 * 29) / 256;
    *out_r = lum.min(255) as u8;
    *out_g = lum.min(255) as u8;
    *out_b = lum.min(255) as u8;
}

/// Alpha-blend src over dst using integer math.
#[inline(always)]
pub fn alpha_blend(src: u8, dst: u8, alpha: u8) -> u8 {
    let a = alpha as u32;
    let inv = 255 - a;
    ((src as u32 * a + dst as u32 * inv) / 255) as u8
}
