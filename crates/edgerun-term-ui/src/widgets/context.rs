use crate::widgets::{PanelLayout, clamp_panel_to_view, list_panel_cpu, list_panel_gpu};
use term_core::gpu::{GlyphAtlas, GlyphVertex, RectVertex};
use term_core::render::{GlyphCache, OVERLAY_PANEL_INNER, rgba_bytes};
use term_core::render::cpu::text_width;
use pixels::wgpu;

pub struct ContextItem {
    pub label: &'static str,
    pub action: ContextAction,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextAction {
    Copy,
    Paste,
}

pub struct ContextMenu {
    pub open: bool,
    pub items: Vec<ContextItem>,
    pub rect: Option<(i32, i32, i32, i32)>,
    pub hovered: Option<usize>,
    pub anchor: (f64, f64),
    pub item_height: i32,
    pub padding: i32,
}

impl ContextMenu {
    pub fn new() -> Self {
        Self {
            open: false,
            items: Vec::new(),
            rect: None,
            hovered: None,
            anchor: (0.0, 0.0),
            item_height: 0,
            padding: 10,
        }
    }

    pub fn open(&mut self, x: f64, y: f64, can_copy: bool) {
        self.items = vec![
            ContextItem {
                label: "Copy",
                action: ContextAction::Copy,
                enabled: can_copy,
            },
            ContextItem {
                label: "Paste",
                action: ContextAction::Paste,
                enabled: true,
            },
        ];
        self.anchor = (x, y);
        self.rect = None;
        self.hovered = None;
        self.item_height = 0;
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.rect = None;
        self.hovered = None;
    }

    pub fn update_hover(&mut self, x: f64, y: f64) {
        if let Some((x0, y0, x1, y1)) = self.rect {
            if x < x0 as f64 || x > x1 as f64 || y < y0 as f64 || y > y1 as f64 {
                self.hovered = None;
                return;
            }
            let local_y = y as i32 - y0 - self.padding;
            if local_y < 0 || self.item_height <= 0 {
                self.hovered = None;
                return;
            }
            let idx = (local_y / self.item_height) as usize;
            self.hovered = self.items.get(idx).map(|_| idx);
        } else {
            self.hovered = None;
        }
    }

    pub fn click(&mut self, x: f64, y: f64) -> Option<ContextAction> {
        self.update_hover(x, y);
        if let Some(idx) = self.hovered
            && let Some(item) = self.items.get(idx)
            && item.enabled
        {
            return Some(item.action);
        }
        None
    }

    pub fn layout(
        &mut self,
        glyphs: &mut GlyphCache,
        width: u32,
        height: u32,
    ) -> Option<PanelLayout> {
        if !self.open || self.items.is_empty() {
            return None;
        }
        let padding = self.padding;
        let item_height = glyphs.cell_height() as i32 + 6;
        self.item_height = item_height;

        let max_label = self
            .items
            .iter()
            .map(|i| text_width(glyphs, i.label))
            .max()
            .unwrap_or(0);
        let w = (max_label + padding * 2).max(96);
        let h = item_height * self.items.len() as i32 + padding * 2;

        let rect = clamp_panel_to_view(
            (self.anchor.0 as i32, self.anchor.1 as i32),
            (w, h),
            width,
            height,
            4,
        );
        self.rect = Some(rect);
        Some(PanelLayout {
            rect,
            padding,
            item_height,
        })
    }

    pub fn draw_cpu(&mut self, glyphs: &mut GlyphCache, frame: &mut [u8], width: u32, height: u32) {
        let Some(layout) = self.layout(glyphs, width, height) else {
            return;
        };
        let items: Vec<(String, bool)> = self
            .items
            .iter()
            .map(|i| (i.label.to_string(), i.enabled))
            .collect();
        list_panel_cpu(
            glyphs,
            frame,
            width,
            height,
            layout,
            rgba_bytes(OVERLAY_PANEL_INNER),
            &items,
            self.hovered,
            None,
        );
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
        let items: Vec<(String, bool)> = self
            .items
            .iter()
            .map(|i| (i.label.to_string(), i.enabled))
            .collect();
        list_panel_gpu(
            rects,
            glyphs_out,
            atlas,
            queue,
            glyphs,
            layout,
            OVERLAY_PANEL_INNER,
            &items,
            self.hovered,
            None,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use term_core::render::FONT_DATA;

    fn glyphs() -> GlyphCache {
        GlyphCache::new(std::sync::Arc::new(FONT_DATA.to_vec()), 16.0)
    }

    #[test]
    fn open_sets_items_and_enabled_state() {
        let mut menu = ContextMenu::new();
        menu.open(10.0, 20.0, false);
        assert!(menu.open);
        assert_eq!(menu.items.len(), 2);
        assert!(!menu.items[0].enabled, "copy should be disabled");
        assert!(menu.items[1].enabled, "paste should stay enabled");
    }

    #[test]
    fn layout_populates_rect_and_hover() {
        let mut menu = ContextMenu::new();
        menu.open(5.0, 5.0, true);
        let mut glyphs = glyphs();
        let width = 200;
        let height = 120;
        let mut frame = vec![0u8; width as usize * height as usize * 4];
        menu.draw_cpu(&mut glyphs, &mut frame, width, height);
        let rect = menu.rect.expect("layout should set rect");
        let (x0, y0, _x1, _y1) = rect;
        // Hover inside first item.
        let hover_y = y0 + menu.padding + menu.item_height / 2;
        menu.update_hover(x0 as f64 + 2.0, hover_y as f64);
        assert_eq!(menu.hovered, Some(0));
        // Click should return Copy when enabled.
        assert_eq!(
            menu.click(x0 as f64 + 2.0, hover_y as f64),
            Some(ContextAction::Copy)
        );
    }
}
