//! edgerun-rasterizer — Scanline renderer generated from CSS proto data.
//!
//! Generated from: css_colors, css_images, css_text, css_box, css_ui, css_transforms,
//! css_fonts, css_sizing, css_display, css_values, css_cascade, css_contain
#![no_std]

extern crate alloc;

pub mod framebuffer;
pub mod scanline;
pub mod blend_lut;
pub mod border_lut;
pub mod color_lut;
pub mod text_bitmap;
pub mod rect;
pub mod gradient;
pub mod simd_blend;
pub mod tile;
