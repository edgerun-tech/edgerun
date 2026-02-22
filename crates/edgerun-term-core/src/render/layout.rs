// SPDX-License-Identifier: Apache-2.0
#[derive(Clone, Copy)]
pub struct LayoutMetrics {
    pub content_x: u32,
    pub content_y: u32,
    pub usable_width: u32,
    pub usable_height: u32,
    pub cols: usize,
    pub rows: usize,
}

#[allow(clippy::too_many_arguments)]
pub fn compute_layout(
    width: u32,
    height: u32,
    cell_w: u32,
    cell_h: u32,
    tab_bar_height: u32,
    border_thickness: u32,
    border_inset: u32,
    padding_x: u32,
    padding_y: u32,
) -> LayoutMetrics {
    let inset = border_inset;
    let left_pad = border_thickness + inset + padding_x;
    let right_pad = border_thickness + inset + padding_x;
    let top_pad = border_thickness + inset + tab_bar_height + padding_y;
    let bottom_pad = border_thickness + inset + padding_y;

    let usable_width = width.saturating_sub(left_pad + right_pad);
    let usable_height = height.saturating_sub(top_pad + bottom_pad);

    let cols = ((usable_width / cell_w.max(1)) as usize).max(1);
    let rows = ((usable_height / cell_h.max(1)) as usize).max(1);

    LayoutMetrics {
        content_x: left_pad,
        content_y: top_pad,
        usable_width,
        usable_height,
        cols,
        rows,
    }
}
