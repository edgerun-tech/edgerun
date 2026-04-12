//! Border style lookup tables — generated from CSS Box proto border patterns.

/// 8-bit dash patterns for each CSS border style.
/// Each pattern is 8 bytes (pixels), repeating.
/// 1 = draw, 0 = skip.

/// Dashed: 4 on, 4 off (4px dash, 4px gap)
pub static DASHED_PATTERN: [u8; 8] = [1,1,1,1, 0,0,0,0];

/// Dotted: 1 on, 3 off (1px dot, 3px gap)
pub static DOTTED_PATTERN: [u8; 8] = [1,0,0,0, 1,0,0,0];

/// Double: 2 on, 1 off, 2 on, 3 off (two thin lines)
pub static DOUBLE_PATTERN: [u8; 8] = [1,1,0,1, 1,0,0,0];

/// Groove/ridge/outset/inset use solid (3D effect simulated by color)
pub static SOLID_PATTERN: [u8; 8] = [1,1,1,1, 1,1,1,1];

/// Groove: 1 on, 1 off (thin groove)
pub static GROOVE_PATTERN: [u8; 8] = [1,0,1,0, 1,0,1,0];

/// Ridge: same as groove but different color
pub static RIDGE_PATTERN: [u8; 8] = [1,0,1,0, 1,0,1,0];

/// Inset/outset: solid
pub static INSET_PATTERN: [u8; 8] = [1,1,1,1, 1,1,1,1];
pub static OUTSET_PATTERN: [u8; 8] = [1,1,1,1, 1,1,1,1];

/// None: all off
pub static NONE_PATTERN: [u8; 8] = [0,0,0,0, 0,0,0,0];

/// Get the pattern for a border style index.
/// 0=none, 1=solid, 2=dashed, 3=dotted, 4=double, 5=groove, 6=ridge, 7=inset, 8=outset
#[inline(always)]
pub fn border_pattern(style: u8) -> &'static [u8; 8] {
    const PATTERNS: [&[u8; 8]; 9] = [
        &NONE_PATTERN, &SOLID_PATTERN, &DASHED_PATTERN, &DOTTED_PATTERN,
        &DOUBLE_PATTERN, &GROOVE_PATTERN, &RIDGE_PATTERN, &INSET_PATTERN, &OUTSET_PATTERN,
    ];
    PATTERNS[(style as usize).min(8)]
}

/// Draw a horizontal border line using the pattern.
/// Writes directly to the framebuffer scanline.
#[inline]
pub fn draw_horizontal_line(
    pixels: &mut [u8],
    stride: u32,
    x: u32, y: u32,
    width: u32,
    r: u8, g: u8, b: u8,
    pattern: &[u8; 8],
) {
    for dx in 0..width {
        if pattern[(dx % 8) as usize] == 0 { continue; }
        let cx = x + dx;
        if cx >= pixels.len() as u32 / 4 { break; }
        let i = (y * stride + cx * 4) as usize;
        if i + 2 < pixels.len() {
            pixels[i] = b; pixels[i+1] = g; pixels[i+2] = r;
        }
    }
}
