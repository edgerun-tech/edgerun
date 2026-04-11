//! Rectangle rendering — solid fill, border stroke with patterns.
//! DO NOT EDIT. Regenerate with: scripts/generate_rasterizer.py
use crate::framebuffer::Framebuffer;
use crate::border_lut;
/// Draw a filled rectangle with optional alpha.
pub fn fill_rect(fb: &mut Framebuffer<'_>, x: u32, y: u32, w: u32, h: u32, r: u8, g: u8, b: u8, alpha: u8) {
    if alpha == 0 { return; }
    if alpha == 255 {
        fb.fill_solid(x, y, w, h, r, g, b);
    } else {
        fb.fill_alpha(x, y, w, h, r, g, b, alpha);
    }
}

/// Draw a rectangle border with a specific style.
pub fn stroke_rect(fb: &mut Framebuffer<'_>, x: u32, y: u32, w: u32, h: u32, r: u8, g: u8, b: u8, style: u8, thickness: u32) {
    if thickness == 0 { return; }
    let pattern = border_lut::border_pattern(style);

    // Top border
    for t in 0..thickness {
        border_lut::draw_horizontal_line(&mut fb.pixels, fb.stride, x, y + t, w, r, g, b, pattern);
    }
    // Bottom border
    for t in 0..thickness {
        let by = y + h - t - 1;
        if by < fb.height {
            border_lut::draw_horizontal_line(&mut fb.pixels, fb.stride, x, by, w, r, g, b, pattern);
        }
    }
    // Left border (vertical)
    for t in 0..thickness {
        let lx = x + t;
        if lx < fb.width {
            for dy in 0..h {
                let cy = y + dy;
                if cy < fb.height && pattern[(dy % 8) as usize] != 0 {
                    let i = (cy * fb.stride + lx * 4) as usize;
                    if i + 2 < fb.pixels.len() {
                        fb.pixels[i] = b; fb.pixels[i+1] = g; fb.pixels[i+2] = r;
                    }
                }
            }
        }
    }
    // Right border
    for t in 0..thickness {
        let rx = x + w - t - 1;
        if rx < fb.width {
            for dy in 0..h {
                let cy = y + dy;
                if cy < fb.height && pattern[(dy % 8) as usize] != 0 {
                    let i = (cy * fb.stride + rx * 4) as usize;
                    if i + 2 < fb.pixels.len() {
                        fb.pixels[i] = b; fb.pixels[i+1] = g; fb.pixels[i+2] = r;
                    }
                }
            }
        }
    }
}
