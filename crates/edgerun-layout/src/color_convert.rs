//! Color space conversions — generated from CSS Colors spec data.
//! DO NOT EDIT. Regenerate with: scripts/generate_renderer.py
use crate::render_object::Color;
use core::f64::consts::PI;
use libm::{cos, sin, pow};

pub fn hsl_to_rgba(h: f64, s: f64, l: f64, a: f64) -> Color {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let s = s.min(100.0).max(0.0) / 100.0;
    let l = l.min(100.0).max(0.0) / 100.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = if h < 60.0 { (c, x, 0.0) } else if h < 120.0 { (x, c, 0.0) }
        else if h < 180.0 { (0.0, c, x) } else if h < 240.0 { (0.0, x, c) }
        else if h < 300.0 { (x, 0.0, c) } else { (c, 0.0, x) };
    Color { r: ((r + m) * 255.0) as f32 / 255.0, g: ((g + m) * 255.0) as f32 / 255.0, b: ((b + m) * 255.0) as f32 / 255.0, a: a as f32 }
}

pub fn oklch_to_rgba(l: f64, c: f64, h: f64, a: f64) -> Color {
    let h_rad = h * PI / 180.0;
    let a_l = c * cos(h_rad); let b_l = c * sin(h_rad);
    let l_ = l + 0.3963377774 * a_l + 0.2158037573 * b_l;
    let m_ = l - 0.1055613458 * a_l - 0.0638541728 * b_l;
    let s_ = l - 0.0894841775 * a_l - 1.2914855480 * b_l;
    let ll = l_*l_*l_; let mm = m_*m_*m_; let ss = s_*s_*s_;
    let r = 4.0767416621 * ll - 3.3077115913 * mm + 0.2309699292 * ss;
    let g = -1.2684380046 * ll + 2.6097574011 * mm - 0.3413193965 * ss;
    let b = -0.0041960863 * ll - 0.7034186147 * mm + 1.7076147010 * ss;
    Color { r: linear_to_srgb(r) as f32, g: linear_to_srgb(g) as f32, b: linear_to_srgb(b) as f32, a: a as f32 }
}

fn linear_to_srgb(c: f64) -> f64 { if c <= 0.0031308 { 12.92 * c } else { 1.055 * pow(c, 1.0 / 2.4) - 0.055 } }
