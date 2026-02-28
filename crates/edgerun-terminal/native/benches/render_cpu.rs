// SPDX-License-Identifier: Apache-2.0
use std::sync::Arc;
use std::time::Instant;

use criterion::{Criterion, criterion_group, criterion_main};
use term_core::render::{FONT_DATA, FONT_SIZE, GlyphCache, draw_background, draw_grid};
use term_core::terminal::Terminal;

fn bench_cpu_draw(c: &mut Criterion) {
    let mut glyphs = GlyphCache::new(Arc::new(FONT_DATA.to_vec()), FONT_SIZE);
    let (cell_w, cell_h) = glyphs.cell_size();

    let mut term = Terminal::new(160, 48);
    // Fill with visible glyphs to exercise background + glyph raster paths.
    for _ in 0..(term.rows * term.cols) {
        term.put_char('A');
    }
    term.set_cursor(0, 0);

    let width = cell_w * term.cols as u32;
    let height = cell_h * term.rows as u32;
    let mut frame = vec![0u8; (width * height * 4) as usize];

    c.bench_function("cpu_draw_grid", |b| {
        b.iter(|| {
            draw_background(&mut frame, width, height, Instant::now(), term.default_bg());
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
                None,
                None,
                true,
                None,
                None,
            );
        })
    });
}

criterion_group!(benches, bench_cpu_draw);
criterion_main!(benches);
