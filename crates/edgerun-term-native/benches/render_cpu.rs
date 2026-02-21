use criterion::{Criterion, criterion_group, criterion_main};
use fontdue::Font;
use term_core::render::GlyphCache;
use term_core::terminal::{DEFAULT_BG, Terminal};

const FONT_DATA: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");
const FONT_SIZE: f32 = 16.0;

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
) {
    for col in 0..term.cols {
        let cell = term.display_cell(col, row);
        if cell.ch == ' ' || cell.wide_continuation {
            continue;
        }
        let base_x = origin_x as i32 + col as i32 * cell_w as i32;
        let base_y = origin_y as i32 + row as i32 * cell_h as i32;

        let (metrics, bitmap) = glyphs.rasterize(cell.ch);
        if metrics.width == 0 || metrics.height == 0 {
            continue;
        }

        let glyph_x = base_x + metrics.xmin;
        let glyph_y = base_y + baseline - (metrics.height as i32 + metrics.ymin);

        for y in 0..metrics.height {
            let italic_shift = if cell.italic {
                ((metrics.height.saturating_sub(y) as i32) + 3) / 4
            } else {
                0
            };
            for x in 0..metrics.width {
                let alpha = bitmap[y * metrics.width + x];
                if alpha == 0 {
                    continue;
                }

                let px = glyph_x + x as i32 + italic_shift;
                let py = glyph_y + y as i32;

                if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                    continue;
                }

                let idx = ((py as u32 * width + px as u32) * 4) as usize;
                let a = alpha as u16;
                let inv = 255u16.saturating_sub(a);
                frame[idx] = ((frame[idx] as u16 * inv + cell.fg.r as u16 * a) / 255) as u8;
                frame[idx + 1] = ((frame[idx + 1] as u16 * inv + cell.fg.g as u16 * a) / 255) as u8;
                frame[idx + 2] = ((frame[idx + 2] as u16 * inv + cell.fg.b as u16 * a) / 255) as u8;
                frame[idx + 3] = 255;

                if cell.bold && px + 1 < width as i32 {
                    let idx_b = ((py as u32 * width + (px as u32 + 1)) * 4) as usize;
                    frame[idx_b] = ((frame[idx_b] as u16 * inv + cell.fg.r as u16 * a) / 255) as u8;
                    frame[idx_b + 1] =
                        ((frame[idx_b + 1] as u16 * inv + cell.fg.g as u16 * a) / 255) as u8;
                    frame[idx_b + 2] =
                        ((frame[idx_b + 2] as u16 * inv + cell.fg.b as u16 * a) / 255) as u8;
                    frame[idx_b + 3] = 255;
                }
            }
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
                    frame[idx] = cell.fg.r;
                    frame[idx + 1] = cell.fg.g;
                    frame[idx + 2] = cell.fg.b;
                    frame[idx + 3] = 255;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_grid(
    term: &Terminal,
    glyphs: &mut GlyphCache,
    frame: &mut [u8],
    width: u32,
    height: u32,
    cell_w: u32,
    cell_h: u32,
    origin_x: u32,
    origin_y: u32,
) {
    let baseline = glyphs.baseline();
    let default_bg = DEFAULT_BG;

    for row in 0..term.rows {
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
        }

        draw_row_glyphs(
            term, glyphs, frame, width, height, cell_w, cell_h, origin_x, origin_y, row, baseline,
        );
    }
}

fn bench_cpu_draw(c: &mut Criterion) {
    let font = Font::from_bytes(FONT_DATA, fontdue::FontSettings::default()).unwrap();
    let mut glyphs = GlyphCache::new(font, FONT_SIZE);
    let (cell_w, cell_h) = glyphs.cell_size();

    let mut term = Terminal::new(160, 48);
    // Fill with visible glyphs and style variety
    for _ in 0..(term.rows * term.cols) {
        term.put_char('A');
    }
    term.set_cursor(0, 0);

    let width = cell_w * term.cols as u32;
    let height = cell_h * term.rows as u32;
    let mut frame = vec![0u8; (width * height * 4) as usize];

    c.bench_function("cpu_draw_grid", |b| {
        b.iter(|| {
            // clear
            for px in frame.chunks_exact_mut(4) {
                px[0] = DEFAULT_BG.r;
                px[1] = DEFAULT_BG.g;
                px[2] = DEFAULT_BG.b;
                px[3] = DEFAULT_BG.a;
            }
            draw_grid(
                &term,
                &mut glyphs,
                &mut frame,
                width,
                height,
                cell_w,
                cell_h,
                0,
                0,
            );
        })
    });
}

criterion_group!(benches, bench_cpu_draw);
criterion_main!(benches);
