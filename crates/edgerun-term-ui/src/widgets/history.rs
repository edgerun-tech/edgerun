#![allow(clippy::too_many_arguments)]

use crate::widgets::{
    MODAL_PANEL_H_FRAC, MODAL_PANEL_MIN_H, MODAL_PANEL_MIN_W, MODAL_PANEL_W_FRAC, modal_panel_rect,
};
use pixels::wgpu;
use term_core::gpu::{GlyphAtlas, GlyphVertex, GpuRenderer, RectVertex};
use term_core::render::{
    GlyphCache, OVERLAY_DIM, OVERLAY_PANEL, OVERLAY_PANEL_INNER, OVERLAY_TEXT_MUTED, rgba_bytes,
};
use term_core::terminal::Rgba;

#[derive(Clone)]
pub struct MenuEntry {
    pub label: String,
    pub command: String,
}

pub struct MenuColumn {
    pub title: &'static str,
    pub accent: Rgba,
    pub entries: Vec<MenuEntry>,
}

pub struct HistoryMenu {
    pub open: bool,
    pub entries: Vec<String>,
    pub columns: Vec<MenuColumn>,
    pub flat: Vec<(usize, usize)>,
    pub selected: usize,
    pub rect: Option<(i32, i32, i32, i32)>,
    pub visible_start: usize,
    pub item_height: i32,
    pub header_height: i32,
    pub padding: i32,
    pub column_bounds: Vec<(i32, i32)>,
    pub max_visible_rows: usize,
    pub bookmarks: Vec<MenuEntry>,
    pub active_column: usize,
    pub column_rects: Vec<(i32, i32, i32, i32)>,
    pub column_rows: Vec<usize>,
}

pub struct HistoryMenuLayout {
    pub rect: (i32, i32, i32, i32),
    pub padding: i32,
    pub header_height: i32,
    pub item_height: i32,
    pub column_bounds: Vec<(i32, i32)>,
    pub max_rows: usize,
}

impl Default for HistoryMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryMenu {
    pub fn new() -> Self {
        Self {
            open: false,
            entries: Vec::new(),
            columns: Vec::new(),
            flat: Vec::new(),
            selected: 0,
            rect: None,
            visible_start: 0,
            item_height: 0,
            header_height: 0,
            padding: 0,
            column_bounds: Vec::new(),
            max_visible_rows: 0,
            bookmarks: vec![
                MenuEntry {
                    label: "Edit hyprland.conf".to_string(),
                    command: "${EDITOR:-nvim} ~/.config/hypr/hyprland.conf".to_string(),
                },
                MenuEntry {
                    label: "Edit waybar config".to_string(),
                    command: "${EDITOR:-nvim} ~/.config/waybar/config.jsonc".to_string(),
                },
                MenuEntry {
                    label: "Edit zshrc".to_string(),
                    command: "${EDITOR:-nvim} ~/.zshrc".to_string(),
                },
                MenuEntry {
                    label: "Git status".to_string(),
                    command: "git status".to_string(),
                },
                MenuEntry {
                    label: "ls -lah".to_string(),
                    command: "ls -lah".to_string(),
                },
                MenuEntry {
                    label: "Tail system errors".to_string(),
                    command: "journalctl -p err -b --no-pager | tail -n 40".to_string(),
                },
            ],
            active_column: 0,
            column_rects: Vec::new(),
            column_rows: Vec::new(),
        }
    }

    fn rebuild_flat(&mut self) {
        self.flat.clear();
        let rows = self
            .columns
            .iter()
            .map(|c| c.entries.len())
            .max()
            .unwrap_or(0);
        for row in 0..rows {
            for (col_idx, col) in self.columns.iter().enumerate() {
                if row < col.entries.len() {
                    self.flat.push((col_idx, row));
                }
            }
        }
    }

    pub fn row_count(&self) -> usize {
        self.columns
            .iter()
            .map(|c| c.entries.len())
            .max()
            .unwrap_or(0)
    }

    pub fn selected_cell(&self) -> Option<(usize, usize)> {
        self.flat.get(self.selected).copied()
    }

    fn index_for_cell(&self, col: usize, row: usize) -> Option<usize> {
        self.flat.iter().position(|&(c, r)| c == col && r == row)
    }

    fn ensure_visible(&mut self) {
        let max_rows = self.max_visible_rows.max(1);
        if let Some((_, row)) = self.selected_cell() {
            if row < self.visible_start {
                self.visible_start = row;
            } else if row >= self.visible_start + max_rows {
                self.visible_start = row + 1 - max_rows;
            }
        }
    }

    pub fn open(&mut self, columns: Vec<MenuColumn>) {
        self.columns = columns;
        self.rebuild_flat();
        self.entries = self
            .flat
            .iter()
            .filter_map(|(c, r)| {
                let col = self.columns.get(*c)?;
                let entry = col.entries.get(*r)?;
                Some(entry.label.clone())
            })
            .collect();
        self.selected = 0;
        self.active_column = 0;
        self.visible_start = 0;
        self.item_height = 0;
        self.header_height = 0;
        self.padding = 0;
        self.column_bounds.clear();
        self.column_rects.clear();
        self.column_rows.clear();
        self.max_visible_rows = 0;
        self.open = !self.flat.is_empty();
        self.rect = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.entries.clear();
        self.columns.clear();
        self.flat.clear();
        self.rect = None;
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.flat.is_empty() {
            return;
        }
        let len = self.flat.len() as i32;
        let mut idx = self.selected as i32 + delta;
        if idx < 0 {
            idx = 0;
        } else if idx >= len {
            idx = len - 1;
        }
        self.selected = idx as usize;
        self.ensure_visible();
    }

    pub fn move_dir(&mut self, dx: i32, dy: i32) {
        if self.columns.is_empty() {
            return;
        }
        let len = self.columns.len();
        let (mut col, mut row) = self.selected_cell().unwrap_or((0, 0));

        // If current column is empty, jump to first non-empty.
        if self
            .columns
            .get(col)
            .map(|c| c.entries.is_empty())
            .unwrap_or(true)
        {
            if let Some((idx, _)) = self
                .columns
                .iter()
                .enumerate()
                .find(|(_, c)| !c.entries.is_empty())
            {
                col = idx;
                row = 0;
            } else {
                return;
            }
        }

        if dx != 0 {
            // Wrap to first/last column when tabbing past edges, skipping empties.
            for step in 1..=len {
                let next =
                    ((col as i32 + dx.signum() * step as i32).rem_euclid(len as i32)) as usize;
                if self
                    .columns
                    .get(next)
                    .map(|c| !c.entries.is_empty())
                    .unwrap_or(false)
                {
                    col = next;
                    self.active_column = col;
                    let target_len = self.columns[col].entries.len();
                    row = row.min(target_len.saturating_sub(1));
                    break;
                }
            }
        }

        if dy != 0
            && let Some(target_len) = self.columns.get(col).map(|c| c.entries.len())
            && target_len > 0
        {
            let new_row = (row as i32 + dy).clamp(0, target_len.saturating_sub(1) as i32) as usize;
            row = new_row;
        }

        if let Some(idx) = self.index_for_cell(col, row) {
            self.selected = idx;
            self.ensure_visible();
        }
    }

    pub fn selected_entry(&self) -> Option<&MenuEntry> {
        self.selected_cell()
            .and_then(|(c, r)| self.columns.get(c)?.entries.get(r))
    }

    pub fn update_hover(&mut self, x: f64, y: f64) {
        let (x0, y0, x1, y1) = match self.rect {
            Some(r) => r,
            None => return,
        };
        if x < x0 as f64 || x > x1 as f64 || y < y0 as f64 || y > y1 as f64 {
            return;
        }
        if self.item_height <= 0 {
            return;
        }

        for (idx, rect) in self.column_rects.iter().enumerate() {
            let (cx0, cy0, cx1, cy1) = *rect;
            if x < cx0 as f64 || x > cx1 as f64 || y < cy0 as f64 || y > cy1 as f64 {
                continue;
            }
            let list_start = cy0 + self.header_height;
            if y < list_start as f64 {
                continue;
            }
            let local_y = y as i32 - list_start;
            if self.item_height <= 0 {
                continue;
            }
            let row = (local_y / self.item_height) as usize;
            if let Some(col) = self.columns.get(idx)
                && row >= col.entries.len()
            {
                continue;
            }
            if let Some(idx_flat) = self.index_for_cell(idx, row) {
                self.selected = idx_flat;
                self.active_column = idx;
                self.ensure_visible();
                break;
            }
        }
    }

    pub fn layout(
        &mut self,
        glyphs: &mut GlyphCache,
        width: u32,
        height: u32,
    ) -> Option<HistoryMenuLayout> {
        if !self.open || self.columns.is_empty() {
            return None;
        }
        if self.row_count() == 0 {
            self.close();
            return None;
        }

        let (x0, y0, x1, y1) = modal_panel_rect(
            width,
            height,
            MODAL_PANEL_W_FRAC,
            MODAL_PANEL_H_FRAC,
            MODAL_PANEL_MIN_W,
            MODAL_PANEL_MIN_H,
        );
        let w = (x1 - x0).max(1);
        let h = (y1 - y0).max(1);

        let padding = 12;
        let header_height = glyphs.cell_height() as i32 + 6;
        let item_height = glyphs.cell_height() as i32 + 6;
        let section_spacing = 10;
        let cols = self.columns.len().max(1) as i32;

        let usable_w = (w - padding * 2).max(cols);
        let col_w = usable_w.max(1);
        let mut bounds = Vec::with_capacity(cols as usize);
        for _ in 0..cols {
            bounds.push((x0 + padding, x0 + padding + col_w.min(x1 - x0 - padding)));
        }

        let available_h = (h - padding * 2 - section_spacing * (cols - 1)).max(item_height);
        let base_rows =
            ((available_h / cols - header_height).max(item_height) / item_height).max(1) as usize;

        self.column_rects.clear();
        self.column_rows.clear();
        let mut y_cursor = y0 + padding;
        for col in &self.columns {
            let desired_rows = col.entries.len().max(1);
            let rows_here = desired_rows.min(base_rows);
            let rect_y0 = y_cursor;
            let rect_y1 = rect_y0 + header_height + rows_here as i32 * item_height;
            self.column_rects
                .push((x0 + padding, rect_y0, x1 - padding, rect_y1));
            self.column_rows.push(rows_here);
            y_cursor = rect_y1 + section_spacing;
        }

        self.max_visible_rows = self.column_rows.iter().copied().max().unwrap_or(0);
        self.padding = padding;
        self.header_height = header_height;
        self.item_height = item_height;
        self.column_bounds = bounds.clone();
        self.ensure_visible();
        self.rect = Some((x0, y0, x1, y1));

        Some(HistoryMenuLayout {
            rect: (x0, y0, x1, y1),
            padding,
            header_height,
            item_height,
            column_bounds: bounds,
            max_rows: self.max_visible_rows,
        })
    }

    pub fn draw_cpu(&mut self, glyphs: &mut GlyphCache, frame: &mut [u8], width: u32, height: u32) {
        let Some(layout) = self.layout(glyphs, width, height) else {
            return;
        };

        let (x0, y0, x1, y1) = layout.rect;
        term_core::render::primitives::fill_rect(
            frame,
            width,
            height,
            0,
            0,
            width as i32,
            height as i32,
            rgba_bytes(OVERLAY_DIM),
        );
        term_core::render::primitives::fill_rect(
            frame,
            width,
            height,
            x0,
            y0,
            x1,
            y1,
            rgba_bytes(OVERLAY_PANEL),
        );
        term_core::render::primitives::fill_rect(
            frame,
            width,
            height,
            x0 + 1,
            y0 + 1,
            x1 - 1,
            y1 - 1,
            rgba_bytes(OVERLAY_PANEL_INNER),
        );

        for (col_idx, col) in self.columns.iter().enumerate() {
            let (cx0, cy0, cx1, _cy1) = match self.column_rects.get(col_idx) {
                Some(r) => *r,
                None => continue,
            };
            let header_y = cy0;
            let list_y = cy0 + layout.header_height;
            let visible_rows = *self.column_rows.get(col_idx).unwrap_or(&0);
            let header_color = [col.accent.r, col.accent.g, col.accent.b, 70];
            term_core::render::primitives::fill_rect(
                frame,
                width,
                height,
                cx0,
                header_y,
                cx1,
                header_y + layout.header_height,
                header_color,
            );
            term_core::render::draw_text_line_clipped(
                glyphs,
                frame,
                width,
                height,
                cx0 + 6,
                header_y,
                col.title,
                rgba_bytes(OVERLAY_TEXT_MUTED),
                cx1 - 6,
            );

            for row in 0..visible_rows.min(col.entries.len()) {
                let row_y = list_y + row as i32 * layout.item_height;
                let selected =
                    matches!(self.selected_cell(), Some((c, r)) if c == col_idx && r == row);
                if selected {
                    term_core::render::primitives::fill_rect(
                        frame,
                        width,
                        height,
                        cx0 + 2,
                        row_y - 2,
                        cx1 - 2,
                        row_y + layout.item_height - 2,
                        [90, 140, 255, 64],
                    );
                }
                let text = &col.entries[row].label;
                term_core::render::draw_text_line_clipped(
                    glyphs,
                    frame,
                    width,
                    height,
                    cx0 + 6,
                    row_y,
                    text,
                    rgba_bytes(OVERLAY_TEXT_MUTED),
                    cx1 - 6,
                );
            }
        }
    }

    pub fn draw_gpu(
        &mut self,
        rects: &mut Vec<RectVertex>,
        glyphs_out: &mut Vec<GlyphVertex>,
        atlas: &mut GlyphAtlas,
        queue: &wgpu::Queue,
        glyphs: &mut GlyphCache,
        width: u32,
        height: u32,
    ) {
        let Some(layout) = self.layout(glyphs, width, height) else {
            return;
        };

        let (x0, y0, x1, y1) = layout.rect;
        GpuRenderer::push_rect(rects, 0.0, 0.0, width as f32, height as f32, OVERLAY_DIM);
        GpuRenderer::push_rect(
            rects,
            x0 as f32,
            y0 as f32,
            x1 as f32,
            y1 as f32,
            OVERLAY_PANEL,
        );
        GpuRenderer::push_rect(
            rects,
            x0 as f32 + 1.0,
            y0 as f32 + 1.0,
            x1 as f32 - 1.0,
            y1 as f32 - 1.0,
            OVERLAY_PANEL_INNER,
        );

        for (col_idx, col) in self.columns.iter().enumerate() {
            let (cx0, cy0, cx1, _cy1) = match self.column_rects.get(col_idx) {
                Some(r) => *r,
                None => continue,
            };
            let header_y = cy0 as f32;
            let list_y = header_y + layout.header_height as f32;
            let visible_rows = *self.column_rows.get(col_idx).unwrap_or(&0);
            GpuRenderer::push_rect(
                rects,
                cx0 as f32,
                header_y,
                cx1 as f32,
                header_y + layout.header_height as f32,
                Rgba {
                    r: col.accent.r,
                    g: col.accent.g,
                    b: col.accent.b,
                    a: 70,
                },
            );
            GpuRenderer::push_text_line(
                glyphs,
                atlas,
                glyphs_out,
                col.title,
                cx0 as f32 + 6.0,
                header_y,
                OVERLAY_TEXT_MUTED,
                queue,
            );

            for row in 0..visible_rows.min(col.entries.len()) {
                let row_y = list_y + row as f32 * layout.item_height as f32;
                let selected =
                    matches!(self.selected_cell(), Some((c, r)) if c == col_idx && r == row);
                if selected {
                    GpuRenderer::push_rect(
                        rects,
                        cx0 as f32 + 2.0,
                        row_y - 2.0,
                        cx1 as f32 - 2.0,
                        row_y + layout.item_height as f32 - 2.0,
                        Rgba {
                            r: 90,
                            g: 140,
                            b: 255,
                            a: 64,
                        },
                    );
                }
                let text = &col.entries[row].label;
                GpuRenderer::push_text_line(
                    glyphs,
                    atlas,
                    glyphs_out,
                    text,
                    cx0 as f32 + 6.0,
                    row_y,
                    OVERLAY_TEXT_MUTED,
                    queue,
                );
            }
        }
    }

    pub fn click(&mut self, x: f64, y: f64) -> Option<usize> {
        if let Some((x0, y0, x1, y1)) = self.rect
            && x >= x0 as f64
            && x <= x1 as f64
            && y >= y0 as f64
            && y <= y1 as f64
        {
            self.update_hover(x, y);
            return Some(self.selected);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use term_core::render::FONT_DATA;

    fn glyphs() -> GlyphCache {
        GlyphCache::new(std::sync::Arc::new(FONT_DATA.to_vec()), 16.0)
    }

    fn sample_columns() -> Vec<MenuColumn> {
        vec![
            MenuColumn {
                title: "A",
                accent: Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                entries: vec![
                    MenuEntry {
                        label: "one".into(),
                        command: "1".into(),
                    },
                    MenuEntry {
                        label: "two".into(),
                        command: "2".into(),
                    },
                ],
            },
            MenuColumn {
                title: "B",
                accent: Rgba {
                    r: 0,
                    g: 255,
                    b: 0,
                    a: 255,
                },
                entries: vec![MenuEntry {
                    label: "three".into(),
                    command: "3".into(),
                }],
            },
        ]
    }

    #[test]
    fn open_flattens_and_selects_first() {
        let mut menu = HistoryMenu::new();
        menu.open(sample_columns());
        assert!(menu.open);
        assert_eq!(menu.entries.len(), 3);
        assert_eq!(menu.selected_cell(), Some((0, 0)));
        assert_eq!(menu.active_column, 0);
    }

    #[test]
    fn move_selection_clamps() {
        let mut menu = HistoryMenu::new();
        menu.open(sample_columns());
        menu.move_selection(10);
        assert_eq!(menu.selected, menu.flat.len().saturating_sub(1));
        menu.move_selection(-50);
        assert_eq!(menu.selected, 0);
    }

    #[test]
    fn move_dir_wraps_columns_and_rows() {
        let mut menu = HistoryMenu::new();
        menu.open(sample_columns());
        menu.move_dir(1, 0);
        assert_eq!(menu.selected_cell(), Some((1, 0)));
        menu.move_dir(0, 1);
        assert_eq!(menu.selected_cell(), Some((1, 0)));
        menu.move_dir(-1, 0);
        assert_eq!(menu.selected_cell(), Some((0, 0)));
        menu.move_dir(0, 1);
        menu.move_dir(0, 1);
        assert_eq!(menu.selected_cell(), Some((0, 1)));
    }

    #[test]
    fn hover_updates_selection_when_inside_rect() {
        let mut menu = HistoryMenu::new();
        menu.open(sample_columns());
        let mut glyphs = glyphs();
        let width = 400;
        let height = 400;
        let mut frame = vec![0u8; width as usize * height as usize * 4];
        menu.draw_cpu(&mut glyphs, &mut frame, width, height);
        let (cx0, cy0, _cx1, _cy1) = menu.column_rects[0];
        let y = cy0 + menu.header_height + menu.item_height + 1;
        menu.update_hover((cx0 + 5) as f64, y as f64);
        assert_eq!(menu.selected_cell(), Some((0, 1)));
        assert_eq!(menu.active_column, 0);
    }
}
