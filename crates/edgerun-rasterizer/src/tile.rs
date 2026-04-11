//! Tile-based rendering support — `no_std` compatible.
//!
//! Provides tile decomposition and command filtering for parallel rendering.
//! Callers with `std` can spawn threads per tile; `no_std` callers can
//! render tiles sequentially.
//!
//! ## Parallel usage (with std):
//! ```ignore
//! use edgerun_rasterizer::tile::compute_tiles;
//! use std::thread::scope;
//!
//! let tiles = compute_tiles(width, height, stride, num_threads);
//! scope(|s| {
//!     for tile in &tiles {
//!         s.spawn(|| render_tile(&fb.pixels, tile, &commands));
//!     }
//! });
//! ```
//!
//! ## Sequential usage (no_std):
//! ```ignore
//! for tile in compute_tiles(width, height, stride, 4) {
//!     render_tile_into(&mut local_fb, &commands, &tile);
//! }
//! ```

extern crate alloc;
use alloc::vec::Vec;

use crate::scanline::RasterCommand;

/// A tile region within a framebuffer.
#[derive(Clone, Copy, Debug)]
pub struct Tile {
    /// X offset within parent framebuffer.
    pub x: u32,
    /// Y offset within parent framebuffer.
    pub y: u32,
    /// Tile width (always full framebuffer width).
    pub w: u32,
    /// Tile height.
    pub h: u32,
    /// Byte offset of the first row in the parent framebuffer.
    pub fb_byte_offset: usize,
    /// Stride of the parent framebuffer in bytes.
    pub fb_stride: u32,
}

impl Tile {
    /// Number of bytes per row in this tile.
    pub fn tile_stride(&self) -> usize {
        (self.w * 4) as usize
    }

    /// Total byte size of the tile's local buffer.
    pub fn buffer_size(&self) -> usize {
        self.tile_stride() * self.h as usize
    }
}

/// Compute tile regions for the given framebuffer dimensions.
///
/// Divides the framebuffer horizontally into `num_tiles` stripes.
/// If `num_tiles` exceeds the framebuffer height, it's clamped to `height`.
pub fn compute_tiles(width: u32, height: u32, stride: u32, num_tiles: usize) -> Vec<Tile> {
    let num_tiles = num_tiles.min(height as usize).max(1);
    let tile_h = height / num_tiles as u32;
    let remainder = height % num_tiles as u32;

    let mut tiles = Vec::with_capacity(num_tiles);
    let mut y = 0;
    for i in 0..num_tiles {
        let h = tile_h + if (i as u32) < remainder { 1 } else { 0 };
        if h == 0 { continue; }
        tiles.push(Tile {
            x: 0,
            y,
            w: width,
            h,
            fb_byte_offset: (y * stride) as usize,
            fb_stride: stride,
        });
        y += h;
    }
    tiles
}

/// Filter and adjust raster commands for a tile region.
///
/// Commands that don't intersect the tile are dropped.
/// Commands that partially intersect are clipped to the tile bounds
/// with coordinates made relative to the tile's top-left.
pub fn filter_commands_for_tile(commands: &[RasterCommand], tile: &Tile) -> Vec<RasterCommand> {
    let mut result = Vec::with_capacity(commands.len());

    for cmd in commands {
        if let Some(adjusted) = adjust_command(cmd, tile) {
            result.push(adjusted);
        }
    }

    result
}

fn adjust_command(cmd: &RasterCommand, tile: &Tile) -> Option<RasterCommand> {
    match cmd {
        RasterCommand::FillRect { x, y, w, h, r, g, b, a } => {
            let x0 = (*x).max(tile.x);
            let y0 = (*y).max(tile.y);
            let x1 = (*x + *w).min(tile.x + tile.w);
            let y1 = (*y + *h).min(tile.y + tile.h);
            if x0 >= x1 || y0 >= y1 { return None; }
            Some(RasterCommand::FillRect {
                x: x0 - tile.x, y: y0 - tile.y,
                w: x1 - x0, h: y1 - y0,
                r: *r, g: *g, b: *b, a: *a,
            })
        }
        RasterCommand::StrokeRect { x, y, w, h, r, g, b, style, thickness } => {
            if *x + *w <= tile.x || *y + *h <= tile.y
                || *x >= tile.x + tile.w || *y >= tile.y + tile.h {
                return None;
            }
            Some(RasterCommand::StrokeRect {
                x: x.saturating_sub(tile.x),
                y: y.saturating_sub(tile.y),
                w: *w, h: *h,
                r: *r, g: *g, b: *b,
                style: *style, thickness: *thickness,
            })
        }
        RasterCommand::Text { x, y, text, r, g, b } => {
            let text_w = (text.len() as u32) * 8;
            let tx1 = *x + text_w;
            let ty1 = *y + 8;
            if tx1 <= tile.x || *y >= tile.y + tile.h { return None; }
            Some(RasterCommand::Text {
                x: x.saturating_sub(tile.x),
                y: y.saturating_sub(tile.y),
                text: text.clone(),
                r: *r, g: *g, b: *b,
            })
        }
        RasterCommand::LinearGradient { x, y, w, h, angle, stops } => {
            let x0 = (*x).max(tile.x);
            let y0 = (*y).max(tile.y);
            let x1 = (*x + *w).min(tile.x + tile.w);
            let y1 = (*y + *h).min(tile.y + tile.h);
            if x0 >= x1 || y0 >= y1 { return None; }
            Some(RasterCommand::LinearGradient {
                x: x0 - tile.x, y: y0 - tile.y,
                w: x1 - x0, h: y1 - y0,
                angle: *angle, stops: stops.clone(),
            })
        }
        RasterCommand::RadialGradient { x, y, w, h, cx, cy, stops } => {
            let x0 = (*x).max(tile.x);
            let y0 = (*y).max(tile.y);
            let x1 = (*x + *w).min(tile.x + tile.w);
            let y1 = (*y + *h).min(tile.y + tile.h);
            if x0 >= x1 || y0 >= y1 { return None; }
            Some(RasterCommand::RadialGradient {
                x: x0 - tile.x, y: y0 - tile.y,
                w: x1 - x0, h: y1 - y0,
                cx: *cx, cy: *cy, stops: stops.clone(),
            })
        }
        RasterCommand::ConicGradient { x, y, w, h, from_angle, cx, cy, stops } => {
            let x0 = (*x).max(tile.x);
            let y0 = (*y).max(tile.y);
            let x1 = (*x + *w).min(tile.x + tile.w);
            let y1 = (*y + *h).min(tile.y + tile.h);
            if x0 >= x1 || y0 >= y1 { return None; }
            Some(RasterCommand::ConicGradient {
                x: x0 - tile.x, y: y0 - tile.y,
                w: x1 - x0, h: y1 - y0,
                from_angle: *from_angle,
                cx: *cx, cy: *cy, stops: stops.clone(),
            })
        }
        // Clip/opacity are global — skip in tiled mode
        RasterCommand::PushClip { .. } | RasterCommand::PopClip
        | RasterCommand::PushOpacity { .. } | RasterCommand::PopOpacity => None,
    }
}

/// Determine a reasonable tile count for the given framebuffer height.
pub fn suggested_tile_count(height: u32) -> usize {
    // Default to 4-8 tiles; caller with std should use available_parallelism()
    let n = height / 64; // roughly 64px tall tiles
    n.max(1).min(16) as usize
}
