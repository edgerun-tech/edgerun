// SPDX-License-Identifier: Apache-2.0
use edgerun_term_core::render::FONT_DATA;
use edgerun_term_core::text::GlyphCache;
use std::sync::Arc;
fn main() {
    let mut cache = GlyphCache::new(Arc::new(FONT_DATA.to_vec()), 16.0);
    let (m, _, _) = cache.rasterize('M');
    let baseline = cache.baseline();
    let (cell_w, cell_h) = cache.cell_size();
    println!(
        "baseline {baseline} cell {cell_w}x{cell_h} metrics w{} h{} xmin{} ymin{} adv{}",
        m.width, m.height, m.xmin, m.ymin, m.advance_width
    );
}
