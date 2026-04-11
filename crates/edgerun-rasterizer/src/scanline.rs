//! Scanline renderer with SIMD auto-dispatch.
//! DO NOT EDIT. Regenerate with: scripts/generate_rasterizer.py
extern crate alloc;
use alloc::vec::Vec;

use crate::framebuffer::Framebuffer;
use crate::rect;
use crate::color_lut;
use crate::text_bitmap;
use crate::gradient::{self, GradientStop};

#[derive(Clone)]
pub enum RasterCommand {
    FillRect { x: u32, y: u32, w: u32, h: u32, r: u8, g: u8, b: u8, a: u8 },
    StrokeRect { x: u32, y: u32, w: u32, h: u32, r: u8, g: u8, b: u8, style: u8, thickness: u32 },
    Text { x: u32, y: u32, text: alloc::string::String, r: u8, g: u8, b: u8 },
    LinearGradient { x: u32, y: u32, w: u32, h: u32, angle: f64, stops: Vec<GradientStop> },
    RadialGradient { x: u32, y: u32, w: u32, h: u32, cx: f64, cy: f64, stops: Vec<GradientStop> },
    ConicGradient { x: u32, y: u32, w: u32, h: u32, from_angle: f64, cx: f64, cy: f64, stops: Vec<GradientStop> },
    PushClip { x: u32, y: u32, w: u32, h: u32 },
    PopClip,
    PushOpacity { alpha: u8 },
    PopOpacity,
}

pub fn rasterize(fb: &mut Framebuffer, commands: &[RasterCommand]) {
    let fb_u32 = fb.pixels.as_mut_ptr() as *mut u32;
    let stride = (fb.stride / 4) as usize;
    let w = fb.width as usize;
    let h = fb.height as usize;

    for cmd in commands {
        match cmd {
            RasterCommand::FillRect { x, y, w: width, h: height, r, g, b, a } => {
                let color = ((*r as u32) << 16) | ((*g as u32) << 8) | (*b as u32) | ((*a as u32) << 24);
                let x0 = (*x as usize).min(w.saturating_sub(1));
                let y0 = (*y as usize).min(h.saturating_sub(1));
                let x1 = ((*x + *width) as usize).min(w);
                let y1 = ((*y + *height) as usize).min(h);
                if x0 >= x1 || y0 >= y1 { continue; }

                #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
                unsafe { crate::simd_blend::fill_rect_avx2(fb_u32, stride, x0, y0, x1 - x0, y1 - y0, color); }

                #[cfg(not(all(target_arch = "x86_64", target_feature = "avx2")))]
                unsafe {
                    for row in y0..y1 {
                        let base = row * stride + x0;
                        for col in 0..(x1 - x0) {
                            *fb_u32.add(base + col) = color;
                        }
                    }
                }
            }
            RasterCommand::StrokeRect { x, y, w: width, h: height, r, g, b, style, thickness } => {
                rect::stroke_rect(fb, *x, *y, *width, *height, *r, *g, *b, *style, *thickness);
            }
            RasterCommand::Text { x, y, text, r, g, b } => {
                text_bitmap::draw_text(&mut fb.pixels, fb.stride, fb.width, fb.height, text, *x, *y, *r, *g, *b);
            }
            RasterCommand::LinearGradient { x, y, w: width, h: height, angle, stops } => {
                gradient::linear_gradient(&mut fb.pixels, fb.stride, fb.width, fb.height, *x, *y, *width, *height, *angle, stops);
            }
            RasterCommand::RadialGradient { x, y, w: width, h: height, cx, cy, stops } => {
                gradient::radial_gradient(&mut fb.pixels, fb.stride, fb.width, fb.height, *x, *y, *width, *height, *cx, *cy, stops);
            }
            RasterCommand::ConicGradient { x, y, w: width, h: height, from_angle, cx, cy, stops } => {
                gradient::conic_gradient(&mut fb.pixels, fb.stride, fb.width, fb.height, *x, *y, *width, *height, *from_angle, *cx, *cy, stops);
            }
            _ => {}
        }
    }
}

pub fn cmd_fill(x: u32, y: u32, w: u32, h: u32, r: u8, g: u8, b: u8) -> RasterCommand {
    RasterCommand::FillRect { x, y, w, h, r, g, b, a: 255 }
}
pub fn cmd_text(x: u32, y: u32, text: &str, r: u8, g: u8, b: u8) -> RasterCommand {
    RasterCommand::Text { x, y, text: text.into(), r, g, b }
}

/// Rasterize into a tile-local framebuffer.
/// Used by the tile module for per-tile rendering.
pub fn rasterize_tile(fb: &mut Framebuffer, commands: &[RasterCommand]) {
    rasterize(fb, commands);
}
