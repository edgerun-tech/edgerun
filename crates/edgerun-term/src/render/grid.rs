use crate::render::GlyphCache;
use crate::terminal::{Rgba, SELECTION, Terminal, ensure_contrast};
use crate::text::GlyphMetrics;

#[allow(clippy::too_many_arguments)]
pub fn draw_grid(
    term: &Terminal,
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    width: u32,
    height: u32,
    cell_w: u32,
    cell_h: u32,
    origin_x: u32,
    origin_y: u32,
    selection: Option<((usize, usize), (usize, usize))>,
    hover: Option<(usize, usize)>,
    blink_on: bool,
) {
    let baseline = glyphs.baseline();
    let default_bg = term.default_bg();
    let selected_bounds = selection.map(|(a, b)| {
        let (mut c0, mut r0) = a;
        let (mut c1, mut r1) = b;
        if r0 > r1 {
            std::mem::swap(&mut r0, &mut r1);
        }
        if c0 > c1 {
            std::mem::swap(&mut c0, &mut c1);
        }
        (c0, r0, c1, r1)
    });

    for row in 0..term.rows {
        // Background + selection pass.
        for col in 0..term.cols {
            let cell = term.display_cell(col, row);
            let base_x = origin_x as i32 + col as i32 * cell_w as i32;
            let base_y = origin_y as i32 + row as i32 * cell_h as i32;

            let bg_is_default = cell.bg.r == default_bg.r
                && cell.bg.g == default_bg.g
                && cell.bg.b == default_bg.b
                && cell.bg.a == default_bg.a;
            if !bg_is_default && cell.bg.a > 0 {
                let a = cell.bg.a as u16;
                for y in 0..cell_h.min(height.saturating_sub(origin_y)) {
                    let py = base_y + y as i32;
                    if py < 0 || py >= height as i32 {
                        continue;
                    }
                    for x in 0..cell_w.min(width.saturating_sub(origin_x)) {
                        let px = base_x + x as i32;
                        if px < 0 || px >= width as i32 {
                            continue;
                        }
                        let idx = ((py as u32 * width + px as u32) * 4) as usize;
                        frame[idx] =
                            ((frame[idx] as u16 * (255 - a) + cell.bg.r as u16 * a) / 255) as u8;
                        frame[idx + 1] = ((frame[idx + 1] as u16 * (255 - a)
                            + cell.bg.g as u16 * a)
                            / 255) as u8;
                        frame[idx + 2] = ((frame[idx + 2] as u16 * (255 - a)
                            + cell.bg.b as u16 * a)
                            / 255) as u8;
                    }
                }
            }

            let hovered = hover.map(|(c, r)| c == col && r == row).unwrap_or(false);

            if hovered {
                for y in 0..cell_h.min(height.saturating_sub(origin_y)) {
                    let py = base_y + y as i32;
                    if py < 0 || py >= height as i32 {
                        continue;
                    }
                    for x in 0..cell_w.min(width.saturating_sub(origin_x)) {
                        let px = base_x + x as i32;
                        if px < 0 || px >= width as i32 {
                            continue;
                        }
                        let idx = ((py as u32 * width + px as u32) * 4) as usize;
                        let a = 60u16;
                        frame[idx] = ((frame[idx] as u16 * (255 - a) + 120 * a) / 255) as u8;
                        frame[idx + 1] =
                            ((frame[idx + 1] as u16 * (255 - a) + 180 * a) / 255) as u8;
                        frame[idx + 2] =
                            ((frame[idx + 2] as u16 * (255 - a) + 255 * a) / 255) as u8;
                    }
                }
            }
            if cell.hyperlink.is_some() {
                let line_y = base_y + cell_h as i32 - 2;
                if line_y >= 0 && line_y < height as i32 {
                    for x in 0..cell_w.min(width.saturating_sub(origin_x)) {
                        let px = base_x + x as i32;
                        if px < 0 || px >= width as i32 {
                            continue;
                        }
                        let idx = ((line_y as u32 * width + px as u32) * 4) as usize;
                        frame[idx] = frame[idx].saturating_add(20);
                        frame[idx + 1] = frame[idx + 1].saturating_add(60);
                        frame[idx + 2] = frame[idx + 2].saturating_add(120);
                    }
                }
            }

            if let Some((c0, r0, c1, r1)) = selected_bounds
                && row >= r0
                && row <= r1
                && col >= c0
                && col <= c1
            {
                for y in 0..cell_h.min(height.saturating_sub(origin_y)) {
                    let py = base_y + y as i32;
                    if py < 0 || py >= height as i32 {
                        continue;
                    }
                    for x in 0..cell_w.min(width.saturating_sub(origin_x)) {
                        let px = base_x + x as i32;
                        if px < 0 || px >= width as i32 {
                            continue;
                        }
                        let idx = ((py as u32 * width + px as u32) * 4) as usize;
                        let a = SELECTION[3] as u16;
                        frame[idx] =
                            ((frame[idx] as u16 * (255 - a) + SELECTION[0] as u16 * a) / 255) as u8;
                        frame[idx + 1] = ((frame[idx + 1] as u16 * (255 - a)
                            + SELECTION[1] as u16 * a)
                            / 255) as u8;
                        frame[idx + 2] = ((frame[idx + 2] as u16 * (255 - a)
                            + SELECTION[2] as u16 * a)
                            / 255) as u8;
                    }
                }
            }
        }

        draw_row_glyphs(
            term, glyphs, frame, width, height, cell_w, cell_h, origin_x, origin_y, row, baseline, blink_on,
        );
    }

    // Sixel overlay
    if !term.sixels.is_empty() {
        for sprite in &term.sixels {
            let x0 = origin_x as i32 + sprite.col as i32 * cell_w as i32;
            let y0 = origin_y as i32 + sprite.row as i32 * cell_h as i32;
            for y in 0..sprite.height as u32 {
                let py = y0 + y as i32;
                if py < 0 || py >= height as i32 {
                    continue;
                }
                for x in 0..sprite.width as u32 {
                    let px = x0 + x as i32;
                    if px < 0 || px >= width as i32 {
                        continue;
                    }
                    let idx_src = ((y * sprite.width as u32 + x) * 4) as usize;
                    let a = sprite.pixels.get(idx_src + 3).copied().unwrap_or(0) as u16;
                    if a == 0 {
                        continue;
                    }
                    let inv = 255 - a;
                    let idx_dst = ((py as u32 * width + px as u32) * 4) as usize;
                    let r = sprite.pixels.get(idx_src).copied().unwrap_or(0) as u16;
                    let g = sprite.pixels.get(idx_src + 1).copied().unwrap_or(0) as u16;
                    let b = sprite.pixels.get(idx_src + 2).copied().unwrap_or(0) as u16;
                    frame[idx_dst] = ((frame[idx_dst] as u16 * inv + r * a) / 255) as u8;
                    frame[idx_dst + 1] =
                        ((frame[idx_dst + 1] as u16 * inv + g * a) / 255) as u8;
                    frame[idx_dst + 2] =
                        ((frame[idx_dst + 2] as u16 * inv + b * a) / 255) as u8;
                    frame[idx_dst + 3] = 255;
                }
            }
        }
    }

    if term.view_offset == 0 && term.cursor_row < term.rows && term.cursor_col < term.cols {
        let mut cursor_col = term.cursor_col;
        let cursor_row = term.cursor_row;
        let mut cell = term.display_cell(cursor_col, cursor_row);
        if cell.wide_continuation && cursor_col > 0 {
            cursor_col -= 1;
            cell = term.display_cell(cursor_col, cursor_row);
        }
        cell.fg = ensure_contrast(cell.fg, cell.bg);

        let cursor_selected = selected_bounds
            .map(|(c0, r0, c1, r1)| {
                cursor_row >= r0 && cursor_row <= r1 && cursor_col >= c0 && cursor_col <= c1
            })
            .unwrap_or(false);

        let span = if cell.wide { 2u32 } else { 1u32 };
        let cursor_x = origin_x
            .saturating_add(cursor_col as u32 * cell_w)
            .min(width.saturating_sub(1));
        if !cursor_selected && term.cursor_visible() {
            let cursor = term.cursor_color();
            let (cursor_w, cursor_h, cursor_y, cursor_x) = match term.cursor_shape() {
                crate::terminal::CursorShape::Underline => {
                    let cursor_thickness = cell_h.max(1).min(2);
                    let base_y = origin_y as i32 + cursor_row as i32 * cell_h as i32;
                    let cursor_y = (base_y + cell_h as i32 - cursor_thickness as i32)
                        .clamp(0, height.saturating_sub(1) as i32) as u32;
                    let cursor_w = (span * cell_w).min(width.saturating_sub(cursor_x));
                    let cursor_h = cursor_thickness.min(height.saturating_sub(cursor_y));
                    (cursor_w, cursor_h, cursor_y, cursor_x)
                }
                crate::terminal::CursorShape::Bar => {
                    let bar_w = (cell_w / 6).max(1);
                    let cursor_w = bar_w.min(width.saturating_sub(cursor_x));
                    let cursor_h = cell_h.min(height.saturating_sub(
                        origin_y.saturating_add(cursor_row as u32 * cell_h),
                    ));
                    let cursor_y = origin_y.saturating_add(cursor_row as u32 * cell_h);
                    (cursor_w, cursor_h, cursor_y, cursor_x)
                }
                crate::terminal::CursorShape::Block => {
                    let cursor_w = (span * cell_w).min(width.saturating_sub(cursor_x));
                    let cursor_h = cell_h.min(height.saturating_sub(
                        origin_y.saturating_add(cursor_row as u32 * cell_h),
                    ));
                    let cursor_y = origin_y.saturating_add(cursor_row as u32 * cell_h);
                    (cursor_w, cursor_h, cursor_y, cursor_x)
                }
            };

            for y in cursor_y..cursor_y.saturating_add(cursor_h) {
                for x in cursor_x..cursor_x.saturating_add(cursor_w) {
                    let idx = ((y * width + x) * 4) as usize;
                    let a = cursor.a as u16;
                    let inv = 255 - a;
                    frame[idx] = ((frame[idx] as u16 * inv + cursor.r as u16 * a) / 255) as u8;
                    frame[idx + 1] =
                        ((frame[idx + 1] as u16 * inv + cursor.g as u16 * a) / 255) as u8;
                    frame[idx + 2] =
                        ((frame[idx + 2] as u16 * inv + cursor.b as u16 * a) / 255) as u8;
                    frame[idx + 3] = 255;
                }
            }
        }

        // Cursor overlay draws underline only; glyphs stay from the main pass to avoid double
        // rasterization and any clipping differences.
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_row_glyphs(
    term: &Terminal,
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    width: u32,
    height: u32,
    cell_w: u32,
    cell_h: u32,
    origin_x: u32,
    origin_y: u32,
    row: usize,
    baseline: i32,
    blink_on: bool,
) {
    for col in 0..term.cols {
        let mut cell = term.display_cell(col, row);
        if cell.is_blank() || cell.wide_continuation {
            continue;
        }
        let blink_alpha = if cell.blink && !blink_on && !cell.concealed {
            128u8
        } else {
            255u8
        };
        cell.fg = ensure_contrast(cell.fg, cell.bg);
        let mut base_x = origin_x as i32 + col as i32 * cell_w as i32;
        let base_y = origin_y as i32 + row as i32 * cell_h as i32;

        for ch in cell.text.chars() {
            let mut fg = cell.fg;
            if cell.faint {
                fg = crate::terminal::faintened(fg);
            }
            let (metrics, bitmap, is_color) = glyphs.rasterize(ch);
            if metrics.width == 0 || metrics.height == 0 {
                draw_missing_glyph_bar(
                    frame, width, height, base_x, base_y, cell_h, cell_w, cell.fg,
                );
                base_x += glyphs.advance_width(ch);
                continue;
            }

            let mut drew_pixel = false;
            let glyph_x = base_x + metrics.xmin;
            let glyph_y = glyph_top(base_y, cell_h, baseline, &metrics);

            for y in 0..metrics.height {
                let italic_shift = if cell.italic {
                    ((metrics.height.saturating_sub(y) as i32) + 3) / 4
                } else {
                    0
                };
                for x in 0..metrics.width {
                    let src_idx = (y * metrics.width + x) as usize * 4;
                    let alpha = bitmap[src_idx + 3];
                    if alpha == 0 {
                        continue;
                    }

                    let px = glyph_x + x as i32 + italic_shift;
                    let py = glyph_y + y as i32;

                    if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                        continue;
                    }

                    let idx = ((py as u32 * width + px as u32) * 4) as usize;
                    let a = (alpha as u16 * blink_alpha as u16 / 255).min(255);
                    let inv = 255u16.saturating_sub(a);
                    let mut fg = cell.fg;
                    if cell.concealed {
                        fg = cell.bg;
                    } else if cell.faint {
                        fg = crate::terminal::faintened(fg);
                    }
                    let (sr, sg, sb) = if is_color {
                        (
                            bitmap[src_idx] as u16,
                            bitmap[src_idx + 1] as u16,
                            bitmap[src_idx + 2] as u16,
                        )
                    } else if cell.bold {
                        (
                            crate::terminal::brightened(fg).r as u16,
                            crate::terminal::brightened(fg).g as u16,
                            crate::terminal::brightened(fg).b as u16,
                        )
                    } else {
                        (fg.r as u16, fg.g as u16, fg.b as u16)
                    };
                    frame[idx] = ((frame[idx] as u16 * inv + sr * a) / 255) as u8;
                    frame[idx + 1] = ((frame[idx + 1] as u16 * inv + sg * a) / 255) as u8;
                    frame[idx + 2] = ((frame[idx + 2] as u16 * inv + sb * a) / 255) as u8;
                    frame[idx + 3] = 255;
                    drew_pixel = true;

                    if cell.bold && px + 1 < width as i32 {
                        let idx_b = ((py as u32 * width + (px as u32 + 1)) * 4) as usize;
                        frame[idx_b] = ((frame[idx_b] as u16 * inv + sr * a) / 255) as u8;
                        frame[idx_b + 1] = ((frame[idx_b + 1] as u16 * inv + sg * a) / 255) as u8;
                        frame[idx_b + 2] = ((frame[idx_b + 2] as u16 * inv + sb * a) / 255) as u8;
                        frame[idx_b + 3] = 255;
                        drew_pixel = true;
                    }
                }
            }

            if !drew_pixel {
                // Safety net: if bitmap had no coverage (e.g., bad glyph), draw fallback bar.
                draw_missing_glyph_bar(
                    frame, width, height, base_x, base_y, cell_h, cell_w, cell.fg,
                );
            }

            if cell.underline {
                let line_y = (base_y + cell_h as i32 - 2).min(base_y + cell_h as i32 - 1);
                if line_y >= 0 && line_y < height as i32 {
                    for x in 0..cell_w.min(width.saturating_sub(origin_x)) {
                        let px = base_x + x as i32;
                        if px < 0 || px >= width as i32 {
                            continue;
                        }
                        let idx = ((line_y as u32 * width + px as u32) * 4) as usize;
                        frame[idx] = fg.r;
                        frame[idx + 1] = fg.g;
                        frame[idx + 2] = fg.b;
                        frame[idx + 3] = 255;
                    }
                }
            }

            if cell.overline {
                let line_y = base_y.max(0);
                if line_y < height as i32 {
                    for x in 0..cell_w.min(width.saturating_sub(origin_x)) {
                        let px = base_x + x as i32;
                        if px < 0 || px >= width as i32 {
                            continue;
                        }
                        let idx = ((line_y as u32 * width + px as u32) * 4) as usize;
                        frame[idx] = fg.r;
                        frame[idx + 1] = fg.g;
                        frame[idx + 2] = fg.b;
                        frame[idx + 3] = 255;
                    }
                }
            }

            if cell.strike {
                let line_y =
                    (base_y + (cell_h as i32 / 2)).clamp(0, height.saturating_sub(1) as i32);
                for x in 0..cell_w.min(width.saturating_sub(origin_x)) {
                    let px = base_x + x as i32;
                    if px < 0 || px >= width as i32 {
                        continue;
                    }
                    let idx = ((line_y as u32 * width + px as u32) * 4) as usize;
                    frame[idx] = fg.r;
                    frame[idx + 1] = fg.g;
                    frame[idx + 2] = fg.b;
                    frame[idx + 3] = 255;
                }
            }

            base_x += glyphs.advance_width(ch);
        }
    }
}

fn draw_missing_glyph_bar(
    frame: &mut [u8],
    width: u32,
    height: u32,
    base_x: i32,
    base_y: i32,
    cell_h: u32,
    cell_w: u32,
    fg: Rgba,
) {
    let bar_y = base_y + cell_h.saturating_sub(3) as i32;
    let bar_h = 2;
    let bar_x0 = base_x;
    let bar_x1 = base_x + cell_w as i32;
    for y in 0..bar_h {
        let py = bar_y + y;
        if py < 0 || py >= height as i32 {
            continue;
        }
        for px in bar_x0..bar_x1 {
            if px < 0 || px >= width as i32 {
                continue;
            }
            let idx = ((py as u32 * width + px as u32) * 4) as usize;
            frame[idx] = fg.r;
            frame[idx + 1] = fg.g;
            frame[idx + 2] = fg.b;
            frame[idx + 3] = 255;
        }
    }
}

fn glyph_top(base_y: i32, cell_h: u32, baseline: i32, metrics: &GlyphMetrics) -> i32 {
    let desired = base_y + baseline - metrics.ymin;
    clamp_to_cell(base_y, cell_h, metrics.height, desired) as i32
}

fn clamp_to_cell(base_y: i32, cell_h: u32, glyph_h: u32, desired_top: i32) -> u32 {
    let glyph_h = glyph_h.max(1);
    let max_top = base_y + cell_h.saturating_sub(glyph_h) as i32;
    if max_top < base_y {
        return base_y.max(0) as u32;
    }
    desired_top.clamp(base_y, max_top).max(0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_to_cell_bounds_y_inside_row() {
        let base_y = 10;
        let cell_h = 5;
        let glyph_h = 4;

        // Too low clamps to row top.
        assert_eq!(clamp_to_cell(base_y, cell_h, glyph_h, 0), 10);
        // Too high clamps to max top that still fits height.
        assert_eq!(clamp_to_cell(base_y, cell_h, glyph_h, 50), 11);
    }
}
