//! Scanline renderer — the main entry point that rasterizes a display list.
//! DO NOT EDIT. Regenerate with: scripts/generate_rasterizer.py

extern crate alloc;
use alloc::vec::Vec;

use crate::framebuffer::Framebuffer;
use crate::rect;
use crate::color_lut;
use crate::text_bitmap;
use crate::gradient::{self, GradientStop};

/// A paint command the rasterizer can execute.
/// Derived from edgerun-layout's PaintCommand enum.
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

/// Rasterize a list of commands into a framebuffer.
pub fn rasterize(fb: &mut Framebuffer, commands: &[RasterCommand]) {
    for cmd in commands {
        match cmd {
            RasterCommand::FillRect { x, y, w, h, r, g, b, a } => {
                rect::fill_rect(fb, *x, *y, *w, *h, *r, *g, *b, *a);
            }
            RasterCommand::StrokeRect { x, y, w, h, r, g, b, style, thickness } => {
                rect::stroke_rect(fb, *x, *y, *w, *h, *r, *g, *b, *style, *thickness);
            }
            RasterCommand::Text { x, y, text, r, g, b } => {
                text_bitmap::draw_text(&mut fb.pixels, fb.stride, fb.width, fb.height, text, *x, *y, *r, *g, *b);
            }
            RasterCommand::LinearGradient { x, y, w, h, angle, stops } => {
                gradient::linear_gradient(&mut fb.pixels, fb.stride, fb.width, fb.height, *x, *y, *w, *h, *angle, stops);
            }
            RasterCommand::RadialGradient { x, y, w, h, cx, cy, stops } => {
                gradient::radial_gradient(&mut fb.pixels, fb.stride, fb.width, fb.height, *x, *y, *w, *h, *cx, *cy, stops);
            }
            RasterCommand::ConicGradient { x, y, w, h, from_angle, cx, cy, stops } => {
                gradient::conic_gradient(&mut fb.pixels, fb.stride, fb.width, fb.height, *x, *y, *w, *h, *from_angle, *cx, *cy, stops);
            }
            RasterCommand::PushClip { .. } | RasterCommand::PopClip => {
                // Clip and opacity stack would be implemented with a clipping region stack
            }
            RasterCommand::PushOpacity { .. } | RasterCommand::PopOpacity => {}
        }
    }
}

/// Helper: create a solid color fill command.
pub fn cmd_fill(x: u32, y: u32, w: u32, h: u32, r: u8, g: u8, b: u8) -> RasterCommand {
    RasterCommand::FillRect { x, y, w, h, r, g, b, a: 255 }
}

/// Helper: create a named color fill (from color LUT).
pub fn cmd_named_fill(x: u32, y: u32, w: u32, h: u32, color_index: usize) -> RasterCommand {
    let (r, g, b, a) = color_lut::named_color(color_index);
    RasterCommand::FillRect { x, y, w, h, r, g, b, a }
}

/// Helper: create a text command.
pub fn cmd_text(x: u32, y: u32, text: &str, r: u8, g: u8, b: u8) -> RasterCommand {
    RasterCommand::Text { x, y, text: text.into(), r, g, b }
}
