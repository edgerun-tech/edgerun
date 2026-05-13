//! Shared GPU UI scene primitives and a small native OpenGL renderer.
//!
//! The scene types are platform neutral and are intended to be consumed by both
//! native OpenGL/EGL/SDL hosts and browser WebGL hosts. The native GL renderer
//! below is only one backend for the scene.

use std::string::String;
use std::vec::Vec;

#[cfg(test)]
use crate::gpu_ui;

#[cfg(not(feature = "fontdue-text"))]
use core::marker::PhantomData;

mod app_registry;
mod apps;
mod bitmap_font;
pub mod components;
#[cfg(all(feature = "gpu-gl", not(target_arch = "wasm32")))]
pub mod gl;
mod icons;
mod node;
mod paint;
pub mod palette;
mod primitives;
mod runtime;
mod scene;
mod shell;
pub mod style;
#[cfg(feature = "fontdue-text")]
mod text;
pub mod webgl2;
mod workspace;
pub use app_registry::{
    CAPABILITY_REQUEST_APP_ID, CHAT_APP_ID, COMPONENT_GALLERY_APP_ID, EDGERUN_APP_REGISTRY,
    LAUNCH_CAPABILITY_REQUEST_ITEM_ID, LAUNCH_CHAT_ITEM_ID, LAUNCH_COMPONENT_GALLERY_ITEM_ID,
    LAUNCH_LOCK_SCREEN_ITEM_ID, LAUNCH_STORAGE_ITEM_ID, LAUNCH_TRUST_MANAGER_ITEM_ID,
    LOCK_SCREEN_APP_ID, SHELL_LAUNCHER_BUTTON_ID, STORAGE_APP_ID, TRUST_MANAGER_APP_ID,
    UiAppPlacement, UiAppSpec, app_spec, app_spec_for_launch_id,
};
#[cfg(test)]
use apps::render_component_gallery_app;
pub use apps::{
    CAPABILITY_ALLOW_BUTTON_ID, CAPABILITY_DENY_BUTTON_ID, CAPABILITY_DETAILS_BUTTON_ID,
    LOCK_UNLOCK_BUTTON_ID, LOCK_UNLOCK_FIELD_ID, build_edgerun_fullscreen_app_with_font,
    build_edgerun_shell_overlay_with_font, build_edgerun_workspace_shell_with_font,
    build_edgerun_workspace_with_shell_with_font, build_unified_chat_shell,
    build_unified_chat_shell_with_font, build_unified_chat_shell_with_font_and_runtime,
};
pub use components::{
    BarChart, ControlAccessory, ControlRow, Field, MenuItem, MetricCard, PanelHeader, Slider,
    TextArea, TransactionRow, UiGrid, UiStack, bar_chart, control_row, field, menu_item,
    metric_card, panel_header, slider, text_area, transaction_row,
};
pub use icons::{UiIcon, UiIconAtlasRect, UiIconSet};
#[cfg(feature = "tabler-svg-atlas")]
pub use icons::{UiIconAtlas, tabler_svg_icon_atlas};
use icons::{draw_canonical_icon, icon_circle, icon_line};
pub use node::{
    UiNode, UiNodeKind, app_launcher_item, attachment_preview, avatar_node, badge,
    bar_chart_labels, bar_chart_node, breadcrumb, button, capability_grant_row, card, checkbox,
    column, command_palette, contact_card, control_row_node, dialog, divider, empty_state,
    field_node, grid, grid_auto, grid_auto_for_width, header, icon, icon_button, identity_card,
    list_row_node, menu_item_node, metric, package_card, progress_bar_node, progress_ring,
    proof_event_row, radio, receipt_row, route_path, row, scroll_area, section, select_node,
    skeleton, slider_node, spacer, tab_labels, table_labels, tabs_node, text, text_area_node,
    thread_row, toast, toggle_node, tooltip, transaction_node, tree_item,
};
use paint::{
    component_label_width, contact_initial, draw_contact_row, draw_message, draw_pill,
    estimate_message_height, panel, push_bounded_label, push_label, soft_card,
};
pub use primitives::{
    ButtonStyle, UiControlAccessory, UiRect, UnifiedChatState, UnifiedContact, UnifiedContactKind,
    UnifiedMessage,
};
pub use runtime::{GpuHit, HitKind, UiAction, UiEvent, UiKey, UiRuntimeState};
pub use scene::{Color4, GpuClip, GpuRect, GpuScene, RectMode, UiColorScheme};
use shell::render_edgerun_shell_overlay;
pub use shell::{UiShellAction, UiShellState};
pub use style::{AlignItems, Axis, JustifyContent, UiStyle};
#[cfg(feature = "fontdue-text")]
pub use text::{FontAtlas, TextQuad};
#[cfg(test)]
use workspace::WORKSPACE_CHROME_H;
pub use workspace::{
    UiAppKind, UiAppSurface, UiTileAxis, UiTileNode, UiWorkspace, UiWorkspaceAction,
};

#[derive(Clone, Copy, Debug)]
pub struct ChatShellMetrics {
    pub sidebar_w: f32,
    pub topbar_h: f32,
    pub composer_h: f32,
    pub pad: f32,
}

impl Default for ChatShellMetrics {
    fn default() -> Self {
        Self {
            sidebar_w: 284.0,
            topbar_h: 58.0,
            composer_h: 86.0,
            pad: 18.0,
        }
    }
}

pub struct UiPainter<'a, 'font> {
    scene: &'a mut GpuScene,
    #[cfg(feature = "fontdue-text")]
    atlas: Option<&'font FontAtlas>,
    #[cfg(not(feature = "fontdue-text"))]
    _font: PhantomData<&'font ()>,
}

impl<'a, 'font> UiPainter<'a, 'font> {
    pub fn new(scene: &'a mut GpuScene) -> Self {
        Self {
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas: None,
            #[cfg(not(feature = "fontdue-text"))]
            _font: PhantomData,
        }
    }

    #[cfg(feature = "fontdue-text")]
    pub fn with_font(scene: &'a mut GpuScene, atlas: &'font FontAtlas) -> Self {
        Self {
            scene,
            atlas: Some(atlas),
        }
    }

    pub fn label(&mut self, x: f32, y: f32, text: &str, scale: f32, color: Color4) {
        push_label(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            text,
            scale,
            color,
        );
    }

    pub fn bounded_label(
        &mut self,
        x: f32,
        y: f32,
        max_w: f32,
        text: &str,
        scale: f32,
        color: Color4,
    ) {
        push_bounded_label(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            max_w,
            text,
            scale,
            color,
        );
    }

    pub fn panel(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) {
        panel(self.scene, x, y, w, h, radius, color);
    }

    pub fn card(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) {
        soft_card(self.scene, x, y, w, h, radius, color);
    }

    pub fn fill_rect(&mut self, rect: UiRect, radius: f32, color: Color4) {
        self.scene
            .push_rect(GpuRect::fill(rect.x, rect.y, rect.w, rect.h, radius, color));
    }

    pub fn border_rect(&mut self, rect: UiRect, radius: f32, color: Color4) {
        self.scene.push_rect(GpuRect::border(
            rect.x, rect.y, rect.w, rect.h, radius, color,
        ));
    }

    pub fn pill(&mut self, x: f32, y: f32, w: f32, label: &str, color: Color4) {
        draw_pill(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            w,
            label,
            color,
        );
    }

    pub fn hit(&mut self, kind: HitKind, id: u32, x: f32, y: f32, w: f32, h: f32) {
        self.scene.push_hit(GpuHit::new(kind, id, x, y, w, h));
    }

    pub fn divider(&mut self, x: f32, y: f32, w: f32, axis: Axis) {
        match axis {
            Axis::Horizontal => {
                self.scene
                    .push_rect(GpuRect::fill(x, y, w, 1.0, 0.0, palette::BORDER));
            }
            Axis::Vertical => {
                self.scene
                    .push_rect(GpuRect::fill(x, y, 1.0, w, 0.0, palette::BORDER));
            }
        }
    }

    pub fn badge(&mut self, x: f32, y: f32, label: &str, color: Color4) -> f32 {
        let w = component_label_width(
            label,
            2.0,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
        ) + 18.0;
        self.scene
            .push_rect(GpuRect::fill(x, y, w, 22.0, 11.0, color.with_alpha(0.16)));
        self.scene
            .push_rect(GpuRect::border(x, y, w, 22.0, 11.0, color.with_alpha(0.42)));
        self.bounded_label(x + 9.0, y + 5.0, w - 18.0, label, 2.0, color);
        w
    }

    pub fn avatar(&mut self, x: f32, y: f32, size: f32, label: &str, color: Color4, online: bool) {
        let radius = size * 0.5;
        self.scene.push_rect(GpuRect::fill(
            x,
            y,
            size,
            size,
            radius,
            color.with_alpha(0.22),
        ));
        self.scene.push_rect(GpuRect::border(
            x,
            y,
            size,
            size,
            radius,
            color.with_alpha(0.54),
        ));
        self.bounded_label(
            x + size * 0.34,
            y + size * 0.30,
            size * 0.42,
            contact_initial(label),
            2.0,
            color,
        );
        self.status_dot(x + size - 8.0, y + size - 8.0, online);
    }

    pub fn status_dot(&mut self, x: f32, y: f32, online: bool) {
        self.scene.push_rect(GpuRect::fill(
            x,
            y,
            8.0,
            8.0,
            4.0,
            if online {
                palette::GREEN
            } else {
                palette::MUTED
            },
        ));
    }

    pub fn button(&mut self, rect: UiRect, label: &str, style: ButtonStyle, id: u32, active: bool) {
        self.hit(HitKind::Button, id, rect.x, rect.y, rect.w, rect.h);
        let (fill, border, text) = match style {
            ButtonStyle::Primary => (palette::ACCENT, palette::ACCENT, palette::ACCENT_TEXT),
            ButtonStyle::Secondary => (palette::ROW, palette::BORDER, palette::TEXT),
            ButtonStyle::Ghost => (
                palette::PANEL.with_alpha(0.0),
                palette::BORDER,
                palette::MUTED,
            ),
            ButtonStyle::Danger => (
                palette::DANGER.with_alpha(0.18),
                palette::DANGER,
                palette::DANGER,
            ),
        };
        let fill = if active {
            fill
        } else {
            fill.with_alpha(fill.a * 0.74)
        };
        self.scene
            .push_rect(GpuRect::fill(rect.x, rect.y, rect.w, rect.h, 10.0, fill));
        self.scene.push_rect(GpuRect::border(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            10.0,
            border.with_alpha(if active { 0.72 } else { 0.42 }),
        ));
        let label_w = component_label_width(
            label,
            2.0,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
        );
        self.bounded_label(
            rect.x + ((rect.w - label_w) * 0.5).max(10.0),
            rect.y + (rect.h - 14.0) * 0.5,
            (rect.w - 20.0).max(0.0),
            label,
            2.0,
            text,
        );
    }

    pub fn icon_button(&mut self, rect: UiRect, icon: UiIcon, id: u32, active: bool) {
        self.hit(HitKind::Button, id, rect.x, rect.y, rect.w, rect.h);
        let fill = if active {
            palette::ROW
        } else {
            palette::ROW.with_alpha(0.58)
        };
        self.fill_rect(rect, 10.0, fill);
        self.border_rect(
            rect,
            10.0,
            palette::BORDER.with_alpha(if active { 0.72 } else { 0.42 }),
        );
        let size = rect.w.min(rect.h).min(22.0);
        self.icon(
            UiRect::new(
                rect.x + (rect.w - size) * 0.5,
                rect.y + (rect.h - size) * 0.5,
                size,
                size,
            ),
            icon,
            if active {
                palette::TEXT
            } else {
                palette::MUTED
            },
        );
    }

    pub fn icon(&mut self, rect: UiRect, icon: UiIcon, color: Color4) {
        draw_canonical_icon(self.scene, rect, icon, color);
    }

    pub fn checkbox(&mut self, rect: UiRect, label: &str, checked: bool, id: u32) {
        self.hit(HitKind::Checkbox, id, rect.x, rect.y, rect.w, rect.h);
        let box_rect = UiRect::new(rect.x, rect.y + (rect.h - 22.0) * 0.5, 22.0, 22.0);
        self.fill_rect(box_rect, 6.0, palette::ROW);
        self.border_rect(
            box_rect,
            6.0,
            if checked {
                palette::ACCENT
            } else {
                palette::BORDER
            },
        );
        if checked {
            self.icon(box_rect.inset(4.0, 4.0), UiIcon::Check, palette::ACCENT);
        }
        self.bounded_label(
            rect.x + 32.0,
            rect.y + (rect.h - 14.0) * 0.5,
            (rect.w - 32.0).max(0.0),
            label,
            2.0,
            palette::TEXT,
        );
    }

    pub fn radio(&mut self, rect: UiRect, label: &str, selected: bool, id: u32) {
        self.hit(HitKind::Radio, id, rect.x, rect.y, rect.w, rect.h);
        let dot_rect = UiRect::new(rect.x, rect.y + (rect.h - 22.0) * 0.5, 22.0, 22.0);
        self.fill_rect(dot_rect, 11.0, palette::ROW);
        self.border_rect(
            dot_rect,
            11.0,
            if selected {
                palette::ACCENT
            } else {
                palette::BORDER
            },
        );
        if selected {
            self.fill_rect(dot_rect.inset(6.0, 6.0), 5.0, palette::ACCENT);
        }
        self.bounded_label(
            rect.x + 32.0,
            rect.y + (rect.h - 14.0) * 0.5,
            (rect.w - 32.0).max(0.0),
            label,
            2.0,
            palette::TEXT,
        );
    }

    pub fn select_trigger(&mut self, rect: UiRect, label: &str, value: &str, id: u32) {
        self.hit(HitKind::Select, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 10.0, palette::COMPOSER);
        self.border_rect(rect, 10.0, palette::BORDER);
        self.bounded_label(
            rect.x + 14.0,
            rect.y + 8.0,
            (rect.w - 48.0).max(0.0),
            label,
            2.0,
            palette::MUTED,
        );
        self.bounded_label(
            rect.x + 14.0,
            rect.y + 29.0,
            (rect.w - 48.0).max(0.0),
            value,
            2.0,
            palette::TEXT,
        );
        self.icon(
            UiRect::new(
                rect.x + rect.w - 31.0,
                rect.y + (rect.h - 18.0) * 0.5,
                18.0,
                18.0,
            ),
            UiIcon::ChevronRight,
            palette::MUTED,
        );
    }

    pub fn tooltip(&mut self, rect: UiRect, text: &str) {
        self.fill_rect(rect, 8.0, palette::TOPBAR);
        self.border_rect(rect, 8.0, palette::BORDER.with_alpha(0.72));
        self.bounded_label(
            rect.x + 10.0,
            rect.y + (rect.h - 14.0) * 0.5,
            (rect.w - 20.0).max(0.0),
            text,
            2.0,
            palette::TEXT,
        );
    }

    pub fn dialog(&mut self, rect: UiRect, title: &str, body: &str, icon: UiIcon) {
        self.card(rect.x, rect.y, rect.w, rect.h, 12.0, palette::PANEL);
        self.icon(
            UiRect::new(rect.x + 18.0, rect.y + 18.0, 34.0, 34.0),
            icon,
            palette::ACCENT,
        );
        self.bounded_label(
            rect.x + 64.0,
            rect.y + 18.0,
            rect.w - 84.0,
            title,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 64.0,
            rect.y + 42.0,
            rect.w - 84.0,
            body,
            2.0,
            palette::MUTED,
        );
        self.divider(
            rect.x + 18.0,
            rect.y + 74.0,
            rect.w - 36.0,
            Axis::Horizontal,
        );
    }

    pub fn toast(&mut self, rect: UiRect, message: &str, icon: UiIcon, accent: Color4) {
        self.fill_rect(rect, 10.0, palette::TOPBAR);
        self.border_rect(rect, 10.0, accent.with_alpha(0.54));
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + (rect.h - 22.0) * 0.5, 22.0, 22.0),
            icon,
            accent,
        );
        self.bounded_label(
            rect.x + 44.0,
            rect.y + (rect.h - 14.0) * 0.5,
            (rect.w - 56.0).max(0.0),
            message,
            2.0,
            palette::TEXT,
        );
    }

    pub fn empty_state(&mut self, rect: UiRect, title: &str, body: &str, icon: UiIcon) {
        self.fill_rect(rect, 10.0, palette::PANEL);
        self.border_rect(rect, 10.0, palette::BORDER.with_alpha(0.64));
        let icon_size = rect.h.min(rect.w).min(54.0);
        let icon_rect = UiRect::new(
            rect.x + (rect.w - icon_size) * 0.5,
            rect.y + 22.0,
            icon_size,
            icon_size,
        );
        self.icon(icon_rect, icon, palette::ACCENT);
        self.bounded_label(
            rect.x + 20.0,
            icon_rect.y + icon_rect.h + 16.0,
            rect.w - 40.0,
            title,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 20.0,
            icon_rect.y + icon_rect.h + 40.0,
            rect.w - 40.0,
            body,
            2.0,
            palette::MUTED,
        );
    }

    pub fn skeleton(&mut self, rect: UiRect) {
        self.fill_rect(rect, 8.0, palette::ROW.with_alpha(0.74));
        let shine_w = (rect.w * 0.28).max(18.0).min(rect.w);
        self.fill_rect(
            UiRect::new(rect.x + rect.w * 0.18, rect.y, shine_w, rect.h),
            8.0,
            palette::BORDER.with_alpha(0.34),
        );
    }

    pub fn progress_ring(&mut self, rect: UiRect, value: f32, color: Color4) {
        let size = rect.w.min(rect.h);
        let center = (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5);
        let radius = size * 0.38;
        icon_circle(self.scene, center, radius, 2.0, palette::BORDER);
        let steps = (32.0 * value.clamp(0.0, 1.0)).ceil().max(1.0) as u32;
        let mut prev = None;
        for i in 0..=steps {
            let angle = -core::f32::consts::FRAC_PI_2 + (i as f32 / 32.0) * core::f32::consts::TAU;
            let pt = (
                center.0 + angle.cos() * radius,
                center.1 + angle.sin() * radius,
            );
            if let Some(prev) = prev {
                icon_line(self.scene, prev, pt, 3.0, color);
            }
            prev = Some(pt);
        }
    }

    pub fn table(&mut self, rect: UiRect, headers: &[String], rows: &[Vec<String>], id_base: u32) {
        self.fill_rect(rect, 8.0, palette::PANEL);
        self.border_rect(rect, 8.0, palette::BORDER);
        let cols = headers.len().max(1);
        let col_w = (rect.w - 24.0).max(0.0) / cols as f32;
        let mut y = rect.y + 12.0;
        for (index, header) in headers.iter().enumerate() {
            self.bounded_label(
                rect.x + 12.0 + index as f32 * col_w,
                y,
                col_w - 10.0,
                header,
                2.0,
                palette::MUTED,
            );
        }
        y += 28.0;
        self.divider(rect.x + 12.0, y - 8.0, rect.w - 24.0, Axis::Horizontal);
        for (row_index, row) in rows.iter().enumerate() {
            let row_rect = UiRect::new(rect.x + 6.0, y - 7.0, rect.w - 12.0, 34.0);
            self.hit(
                HitKind::ListRow,
                id_base + row_index as u32,
                row_rect.x,
                row_rect.y,
                row_rect.w,
                row_rect.h,
            );
            if row_index % 2 == 1 {
                self.fill_rect(row_rect, 6.0, palette::ROW.with_alpha(0.48));
            }
            for col in 0..cols {
                let value = row.get(col).map(String::as_str).unwrap_or("");
                self.bounded_label(
                    rect.x + 12.0 + col as f32 * col_w,
                    y,
                    col_w - 10.0,
                    value,
                    2.0,
                    palette::TEXT,
                );
            }
            y += 34.0;
            if y > rect.y + rect.h - 20.0 {
                break;
            }
        }
    }

    pub fn breadcrumb(&mut self, rect: UiRect, items: &[String], selected: usize, base_id: u32) {
        let mut x = rect.x;
        for (index, item) in items.iter().enumerate() {
            let w = (component_label_width(
                item,
                2.0,
                #[cfg(feature = "fontdue-text")]
                self.atlas,
            ) + 24.0)
                .clamp(46.0, 150.0);
            let item_rect = UiRect::new(x, rect.y, w, rect.h.min(32.0));
            self.hit(
                HitKind::Breadcrumb,
                base_id + index as u32,
                item_rect.x,
                item_rect.y,
                item_rect.w,
                item_rect.h,
            );
            self.fill_rect(
                item_rect,
                8.0,
                if index == selected {
                    palette::ACTIVE_ROW
                } else {
                    palette::ROW.with_alpha(0.38)
                },
            );
            self.bounded_label(
                item_rect.x + 10.0,
                item_rect.y + 8.0,
                item_rect.w - 20.0,
                item,
                2.0,
                if index == selected {
                    palette::TEXT
                } else {
                    palette::MUTED
                },
            );
            x += w + 6.0;
            if index + 1 < items.len() {
                self.icon(
                    UiRect::new(x, rect.y + 7.0, 16.0, 16.0),
                    UiIcon::ChevronRight,
                    palette::MUTED,
                );
                x += 22.0;
            }
        }
    }

    pub fn command_palette(&mut self, rect: UiRect, placeholder: &str, id: u32) {
        self.hit(HitKind::Input, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 12.0, palette::COMPOSER);
        self.border_rect(rect, 12.0, palette::BORDER);
        self.icon(
            UiRect::new(rect.x + 14.0, rect.y + (rect.h - 20.0) * 0.5, 20.0, 20.0),
            UiIcon::Search,
            palette::MUTED,
        );
        self.bounded_label(
            rect.x + 44.0,
            rect.y + (rect.h - 14.0) * 0.5,
            rect.w - 58.0,
            placeholder,
            2.0,
            palette::MUTED,
        );
    }

    pub fn tree_item(
        &mut self,
        rect: UiRect,
        label: &str,
        detail: &str,
        depth: u8,
        expanded: bool,
        id: u32,
    ) {
        self.hit(HitKind::TreeItem, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 6.0, palette::PANEL);
        let indent = 12.0 + depth as f32 * 18.0;
        self.icon(
            UiRect::new(rect.x + indent, rect.y + (rect.h - 16.0) * 0.5, 16.0, 16.0),
            if expanded {
                UiIcon::ChevronRight
            } else {
                UiIcon::File
            },
            palette::MUTED,
        );
        self.bounded_label(
            rect.x + indent + 24.0,
            rect.y + 9.0,
            rect.w * 0.48,
            label,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + rect.w * 0.58,
            rect.y + 9.0,
            rect.w * 0.36,
            detail,
            2.0,
            palette::MUTED,
        );
    }

    pub fn section_header(&mut self, rect: UiRect, title: &str, detail: &str) {
        self.bounded_label(rect.x, rect.y, rect.w * 0.55, title, 2.0, palette::TEXT);
        self.bounded_label(
            rect.x + rect.w * 0.58,
            rect.y,
            rect.w * 0.42,
            detail,
            2.0,
            palette::MUTED,
        );
        self.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);
    }

    pub fn identity_card(&mut self, rect: UiRect, name: &str, node: &str, policy: &str, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.card(rect.x, rect.y, rect.w, rect.h, 10.0, palette::PANEL);
        self.icon(
            UiRect::new(rect.x + 16.0, rect.y + 18.0, 34.0, 34.0),
            UiIcon::Trust,
            palette::ACCENT,
        );
        self.bounded_label(
            rect.x + 62.0,
            rect.y + 16.0,
            rect.w - 82.0,
            name,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 62.0,
            rect.y + 39.0,
            rect.w - 82.0,
            node,
            2.0,
            palette::MUTED,
        );
        self.badge(
            rect.x + 16.0,
            rect.y + rect.h - 34.0,
            policy,
            palette::ACCENT,
        );
    }

    pub fn contact_card(&mut self, rect: UiRect, name: &str, detail: &str, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 8.0, palette::PANEL);
        self.border_rect(rect, 8.0, palette::BORDER);
        self.avatar(
            rect.x + 12.0,
            rect.y + 12.0,
            36.0,
            name,
            palette::ACCENT,
            true,
        );
        self.bounded_label(
            rect.x + 58.0,
            rect.y + 13.0,
            rect.w - 72.0,
            name,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 58.0,
            rect.y + 35.0,
            rect.w - 72.0,
            detail,
            2.0,
            palette::MUTED,
        );
    }

    pub fn thread_row(
        &mut self,
        rect: UiRect,
        title: &str,
        last_message: &str,
        unread: bool,
        id: u32,
    ) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(
            rect,
            6.0,
            if unread {
                palette::ACTIVE_ROW
            } else {
                palette::PANEL
            },
        );
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 15.0, 24.0, 24.0),
            UiIcon::Chat,
            if unread {
                palette::ACCENT
            } else {
                palette::MUTED
            },
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 10.0,
            rect.w - 62.0,
            title,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 32.0,
            rect.w - 62.0,
            last_message,
            2.0,
            palette::MUTED,
        );
    }

    pub fn attachment_preview(&mut self, rect: UiRect, name: &str, kind: &str, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 8.0, palette::ROW);
        self.border_rect(rect, 8.0, palette::BORDER.with_alpha(0.68));
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 12.0, 28.0, 28.0),
            UiIcon::File,
            palette::ACCENT,
        );
        self.bounded_label(
            rect.x + 52.0,
            rect.y + 11.0,
            rect.w - 66.0,
            name,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 52.0,
            rect.y + 33.0,
            rect.w - 66.0,
            kind,
            2.0,
            palette::MUTED,
        );
    }

    pub fn capability_grant_row(
        &mut self,
        rect: UiRect,
        app: &str,
        capability: &str,
        state: &str,
        id: u32,
    ) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 0.0, palette::PANEL);
        self.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 17.0, 24.0, 24.0),
            UiIcon::Shield,
            palette::VIOLET,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 10.0,
            rect.w * 0.34,
            app,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 32.0,
            rect.w * 0.34,
            capability,
            2.0,
            palette::MUTED,
        );
        self.badge(
            rect.x + rect.w - 96.0,
            rect.y + 18.0,
            state,
            palette::ACCENT,
        );
    }

    pub fn proof_event_row(
        &mut self,
        rect: UiRect,
        title: &str,
        hash: &str,
        status: &str,
        id: u32,
    ) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 0.0, palette::PANEL);
        self.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 17.0, 24.0, 24.0),
            UiIcon::Check,
            palette::GREEN,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 10.0,
            rect.w * 0.36,
            title,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 32.0,
            rect.w * 0.48,
            hash,
            2.0,
            palette::MUTED,
        );
        self.badge(
            rect.x + rect.w - 96.0,
            rect.y + 18.0,
            status,
            palette::GREEN,
        );
    }

    pub fn route_path(&mut self, rect: UiRect, label: &str, hops: &[String]) {
        self.fill_rect(rect, 8.0, palette::PANEL);
        self.border_rect(rect, 8.0, palette::BORDER);
        self.bounded_label(
            rect.x + 14.0,
            rect.y + 10.0,
            rect.w - 28.0,
            label,
            2.0,
            palette::TEXT,
        );
        let mut x = rect.x + 16.0;
        let y = rect.y + 45.0;
        for (index, hop) in hops.iter().enumerate() {
            self.icon(
                UiRect::new(x, y, 22.0, 22.0),
                UiIcon::Route,
                palette::ACCENT,
            );
            self.bounded_label(x + 28.0, y + 4.0, 78.0, hop, 2.0, palette::MUTED);
            x += 112.0;
            if index + 1 < hops.len() {
                self.icon(
                    UiRect::new(x - 22.0, y + 3.0, 16.0, 16.0),
                    UiIcon::ChevronRight,
                    palette::MUTED,
                );
            }
            if x > rect.x + rect.w - 80.0 {
                break;
            }
        }
    }

    pub fn package_card(&mut self, rect: UiRect, name: &str, policy: &str, hash: &str, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.card(rect.x, rect.y, rect.w, rect.h, 10.0, palette::PANEL);
        self.icon(
            UiRect::new(rect.x + 16.0, rect.y + 18.0, 30.0, 30.0),
            UiIcon::App,
            palette::ACCENT,
        );
        self.bounded_label(
            rect.x + 58.0,
            rect.y + 16.0,
            rect.w - 76.0,
            name,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 58.0,
            rect.y + 39.0,
            rect.w - 76.0,
            hash,
            2.0,
            palette::MUTED,
        );
        self.badge(
            rect.x + 16.0,
            rect.y + rect.h - 34.0,
            policy,
            palette::VIOLET,
        );
    }

    pub fn receipt_row(&mut self, rect: UiRect, label: &str, amount: &str, status: &str, id: u32) {
        self.hit(HitKind::TransactionRow, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 0.0, palette::PANEL);
        self.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 17.0, 24.0, 24.0),
            UiIcon::Wallet,
            palette::GREEN,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 18.0,
            rect.w * 0.38,
            label,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + rect.w - 150.0,
            rect.y + 18.0,
            70.0,
            status,
            2.0,
            palette::MUTED,
        );
        self.bounded_label(
            rect.x + rect.w - 76.0,
            rect.y + 18.0,
            64.0,
            amount,
            2.0,
            palette::GREEN,
        );
    }

    pub fn input_field(&mut self, rect: UiRect, placeholder: &str, focused: bool) {
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            12.0,
            palette::COMPOSER,
        ));
        self.scene.push_rect(GpuRect::border(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            12.0,
            if focused {
                palette::ACCENT
            } else {
                palette::BORDER
            },
        ));
        self.bounded_label(
            rect.x + 16.0,
            rect.y + 13.0,
            (rect.w - 32.0).max(0.0),
            placeholder,
            2.0,
            palette::MUTED,
        );
    }

    pub fn app_launcher_item(
        &mut self,
        rect: UiRect,
        title: &str,
        detail: &str,
        icon: UiIcon,
        id: u32,
    ) {
        self.hit(HitKind::AppLauncherItem, id, rect.x, rect.y, rect.w, rect.h);
        self.fill_rect(rect, 8.0, palette::ROW.with_alpha(0.72));
        self.border_rect(rect, 8.0, palette::BORDER.with_alpha(0.58));
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 15.0, 26.0, 26.0),
            icon,
            palette::ACCENT,
        );
        self.bounded_label(
            rect.x + 50.0,
            rect.y + 10.0,
            (rect.w - 84.0).max(0.0),
            title,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 50.0,
            rect.y + 32.0,
            (rect.w - 84.0).max(0.0),
            detail,
            2.0,
            palette::MUTED,
        );
        self.icon(
            UiRect::new(rect.x + rect.w - 30.0, rect.y + 20.0, 16.0, 16.0),
            UiIcon::ChevronRight,
            palette::MUTED,
        );
    }

    pub fn toggle(&mut self, x: f32, y: f32, on: bool, id: u32) {
        self.hit(HitKind::Toggle, id, x, y, 46.0, 24.0);
        let color = if on { palette::GREEN } else { palette::MUTED };
        self.scene.push_rect(GpuRect::fill(
            x,
            y,
            46.0,
            24.0,
            12.0,
            color.with_alpha(0.18),
        ));
        self.scene.push_rect(GpuRect::border(
            x,
            y,
            46.0,
            24.0,
            12.0,
            color.with_alpha(0.54),
        ));
        let knob_x = if on { x + 24.0 } else { x + 4.0 };
        self.scene
            .push_rect(GpuRect::fill(knob_x, y + 4.0, 16.0, 16.0, 8.0, color));
    }

    pub fn progress_bar(&mut self, rect: UiRect, fraction: f32, color: Color4) {
        let value = fraction.clamp(0.0, 1.0);
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            rect.h * 0.5,
            palette::ROW,
        ));
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w * value,
            rect.h,
            rect.h * 0.5,
            color,
        ));
    }

    pub fn segmented_tabs(&mut self, rect: UiRect, labels: &[&str], selected: usize, base_id: u32) {
        if labels.is_empty() {
            return;
        }
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            12.0,
            palette::ROW,
        ));
        self.scene.push_rect(GpuRect::border(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            12.0,
            palette::BORDER,
        ));
        let item_w = rect.w / labels.len() as f32;
        for (index, label) in labels.iter().enumerate() {
            let x = rect.x + index as f32 * item_w;
            let active = index == selected;
            self.hit(
                HitKind::Tab,
                base_id + index as u32,
                x,
                rect.y,
                item_w,
                rect.h,
            );
            if active {
                self.scene.push_rect(GpuRect::fill(
                    x + 3.0,
                    rect.y + 3.0,
                    item_w - 6.0,
                    rect.h - 6.0,
                    9.0,
                    palette::ACTIVE_ROW,
                ));
            }
            let label_w = component_label_width(
                label,
                2.0,
                #[cfg(feature = "fontdue-text")]
                self.atlas,
            );
            self.bounded_label(
                x + ((item_w - label_w) * 0.5).max(8.0),
                rect.y + (rect.h - 14.0) * 0.5,
                (item_w - 16.0).max(0.0),
                label,
                2.0,
                if active {
                    palette::TEXT
                } else {
                    palette::MUTED
                },
            );
        }
    }

    pub fn list_row(&mut self, rect: UiRect, title: &str, detail: &str, accent: Color4, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.card(rect.x, rect.y, rect.w, rect.h, 10.0, palette::ROW);
        self.scene
            .push_rect(GpuRect::fill(rect.x, rect.y, 3.0, rect.h, 2.0, accent));
        self.bounded_label(
            rect.x + 16.0,
            rect.y + 10.0,
            (rect.w - 32.0).max(0.0),
            title,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 16.0,
            rect.y + 31.0,
            (rect.w - 32.0).max(0.0),
            detail,
            2.0,
            palette::MUTED,
        );
    }

    pub fn scrollbar(&mut self, rect: UiRect, visible_fraction: f32, offset_fraction: f32) {
        let visible = visible_fraction.clamp(0.08, 1.0);
        let offset = offset_fraction.clamp(0.0, 1.0);
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            rect.w * 0.5,
            palette::ROW.with_alpha(0.42),
        ));
        let thumb_h = rect.h * visible;
        let thumb_y = rect.y + (rect.h - thumb_h) * offset;
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            thumb_y,
            rect.w,
            thumb_h,
            rect.w * 0.5,
            palette::MUTED.with_alpha(0.74),
        ));
    }

    pub fn contact_row(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        contact: &UnifiedContact<'_>,
        selected: bool,
        id: u32,
    ) {
        self.hit(HitKind::Contact, id, x, y, w, 56.0);
        draw_contact_row(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            w,
            contact,
            selected,
        );
    }

    pub fn message_bubble(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        role: &str,
        body: &str,
        fill: Color4,
        accent: Color4,
    ) -> f32 {
        draw_message(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            w,
            role,
            body,
            fill,
            accent,
        )
    }

    pub fn composer(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        placeholder: &str,
        active: bool,
        chips: &[(&str, Color4)],
    ) {
        self.hit(HitKind::Composer, 0, x, y, w, h);
        self.card(x, y, w, h, 16.0, palette::COMPOSER);
        self.scene.push_rect(GpuRect::border(
            x,
            y,
            w,
            h,
            16.0,
            if active {
                palette::ACCENT
            } else {
                palette::BORDER
            },
        ));
        self.bounded_label(
            x + 22.0,
            y + 24.0,
            (w - 96.0).max(0.0),
            placeholder,
            2.0,
            palette::MUTED,
        );

        let mut chip_x = x + 20.0;
        for (label, color) in chips.iter().copied() {
            let chip_w = component_label_width(
                label,
                2.0,
                #[cfg(feature = "fontdue-text")]
                self.atlas,
            ) + 24.0;
            if chip_x + chip_w > x + w - 78.0 {
                break;
            }
            self.pill(chip_x, y + h - 34.0, chip_w, label, color);
            chip_x += chip_w + 10.0;
        }

        self.hit(HitKind::Send, 0, x + w - 58.0, y + h - 56.0, 40.0, 38.0);
        self.scene.push_rect(GpuRect::fill(
            x + w - 58.0,
            y + h - 56.0,
            40.0,
            38.0,
            12.0,
            palette::ACCENT,
        ));
        self.bounded_label(
            x + w - 47.0,
            y + h - 45.0,
            24.0,
            ">",
            3.0,
            palette::ACCENT_TEXT,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_node_builder_renders_component_tree() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-panel border rounded-md p-4 gap-3")
                .children([
                    tab_labels(&["Routes", "Proofs", "Files"], 1, 80).class("h-9"),
                    row("gap-2 h-10 items-center")
                        .child(avatar_node("EdgeRun", palette::ACCENT).online(true))
                        .child(text("Dashboard").class("flex-1 text-text truncate"))
                        .child(badge("sealed", palette::GREEN)),
                    metric("Relay balance", "$0.00")
                        .detail("pending setup")
                        .progress(0.32)
                        .class("h-32"),
                    field_node("Endpoint", "nodes.edgerun.tech")
                        .detail("identity-routed relay")
                        .focused(true),
                    list_row_node("Admission route", "policy-bound relay", 83)
                        .accent(palette::GREEN),
                    control_row_node("Relay enabled")
                        .detail("use admitted identity route")
                        .control_toggle(true, 84)
                        .hit_id(85),
                    menu_item_node("Payments", 42)
                        .detail("proof-backed receipts")
                        .badge_text("new")
                        .selected(true),
                    button("Open", 43, ButtonStyle::Secondary).class("h-8"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 360.0, 620.0));
        }

        assert!(scene.rects().len() > 20);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Tab && hit.id == 81)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 83)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Toggle && hit.id == 84)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 85)
        );
    }

    #[test]
    fn ui_node_builder_renders_dashboard_primitives() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            let months = ["Jan", "Feb", "Mar", "Apr"];
            let values = [0.42, 0.72, 0.55, 0.91];
            column("bg-panel border rounded-md p-4 gap-3")
                .children([
                    header("Contribution History")
                        .detail("Last 4 months")
                        .action("View", 50),
                    bar_chart_labels("Relay Receipts", &months, &values).class("h-44"),
                    slider_node("Payout threshold", 0.62, 51)
                        .range_labels("$50", "$10,000")
                        .accent(palette::GREEN),
                    text_area_node("Notes", "Route budget and admission notes").focused(true),
                    transaction_node("Stripe payout", "+$4,200.00", 52)
                        .detail("Income")
                        .date("Today")
                        .positive(true),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 420.0, 620.0));
        }

        assert!(scene.rects().len() > 35);
        assert!(scene.hits().len() >= 3);
    }

    #[test]
    fn gpu_ui_macro_composes_nested_trees() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            gpu_ui!(
                column("bg-panel border rounded-md p-4 gap-3"),
                [
                    gpu_ui!(
                        row("gap-2 h-10 items-center"),
                        [
                            avatar_node("EdgeRun", palette::ACCENT).online(true),
                            text("Components").class("flex-1 text-text truncate"),
                            badge("rust", palette::ACCENT),
                            icon_button(UiIcon::ChevronRight, 69).class("size-8")
                        ]
                    ),
                    metric("Storage", "128 MB").detail("verified cache"),
                    progress_bar_node(0.58, palette::GREEN).class("h-2"),
                    toggle_node(true, 68),
                    button("Run", 70, ButtonStyle::Primary).class("h-8")
                ]
            )
            .render(&mut ui, UiRect::new(0.0, 0.0, 320.0, 320.0));
        }

        assert!(scene.rects().len() > 25);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Toggle && hit.id == 68)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 69)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 70)
        );
    }

    #[test]
    fn disabled_and_loading_nodes_suppress_interaction_hits() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("gap-2")
                .children([
                    button("Disabled", 71, ButtonStyle::Secondary)
                        .class("h-8")
                        .disabled(true),
                    button("Loading", 72, ButtonStyle::Primary)
                        .class("h-8")
                        .loading(true),
                    button("Ready", 73, ButtonStyle::Primary).class("h-8"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 240.0, 130.0));
        }

        assert!(!scene.hits().iter().any(|hit| hit.id == 71));
        assert!(!scene.hits().iter().any(|hit| hit.id == 72));
        assert!(scene.hits().iter().any(|hit| hit.id == 73));
        assert!(scene.rects().len() > 10);
    }

    #[test]
    fn keyboard_focus_cycles_and_activates_controls() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("gap-2")
                .children([
                    button("Run", 81, ButtonStyle::Primary).class("h-8"),
                    checkbox("Cache verified bytes", false, 82).class("h-8"),
                    field_node("Filter", "").hit_id(83).class("h-16"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 260.0, 150.0));
        }

        let mut runtime = UiRuntimeState::default();
        assert!(matches!(
            runtime.focus_first(&scene),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::Button && hit.id == 81
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Enter }),
            UiAction::Activated(hit) if hit.kind == HitKind::Button && hit.id == 81
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Tab }),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::Checkbox && hit.id == 82
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Enter }),
            UiAction::Toggled { id: 82, on: true }
        ));
        assert!(matches!(
            runtime.handle_event(&scene, UiEvent::KeyDown { key: UiKey::Tab }),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::Input && hit.id == 83
        ));
    }

    #[test]
    fn scroll_area_only_renders_visible_rows() {
        let rows = (0..10).map(|index| {
            list_row_node(
                &format!("Route {}", index + 1),
                "identity-routed relay",
                100 + index,
            )
        });

        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            scroll_area("bg-panel border rounded-md p-2 gap-2", 1.0)
                .scroll_id(99)
                .children(rows)
                .render(&mut ui, UiRect::new(0.0, 0.0, 320.0, 160.0));
        }

        let visible_rows = scene
            .hits()
            .iter()
            .filter(|hit| hit.kind == HitKind::ListRow)
            .count();
        assert!((1..10).contains(&visible_rows));
        assert!(
            !scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 100)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 109)
        );
        assert!(
            scene
                .hits()
                .iter()
                .filter(|hit| hit.kind == HitKind::ListRow)
                .all(|hit| hit.y >= 8.0 && hit.y + hit.h <= 152.0)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Scrollbar && hit.id == 99)
        );
    }

    #[test]
    fn scene_clip_stack_clips_rects_and_hits() {
        let mut scene = GpuScene::new(palette::BG);
        assert!(scene.push_clip(GpuClip::new(10.0, 20.0, 80.0, 40.0)));
        scene.push_rect(GpuRect::fill(0.0, 0.0, 40.0, 40.0, 8.0, palette::ACCENT));
        scene.push_hit(GpuHit::new(HitKind::Button, 7, 0.0, 0.0, 40.0, 40.0));
        scene.push_rect(GpuRect::fill(120.0, 0.0, 20.0, 20.0, 0.0, palette::ACCENT));
        scene.pop_clip();

        assert_eq!(scene.rects().len(), 1);
        assert_eq!(
            (
                scene.rects()[0].x,
                scene.rects()[0].y,
                scene.rects()[0].w,
                scene.rects()[0].h
            ),
            (10.0, 20.0, 30.0, 20.0)
        );
        assert_eq!(scene.hits().len(), 1);
        assert_eq!(
            (
                scene.hits()[0].x,
                scene.hits()[0].y,
                scene.hits()[0].w,
                scene.hits()[0].h
            ),
            (10.0, 20.0, 30.0, 20.0)
        );
    }

    #[test]
    fn ui_runtime_state_emits_semantic_actions() {
        let mut scene = GpuScene::new(palette::BG);
        scene.push_hit(GpuHit::new(HitKind::Toggle, 4, 10.0, 10.0, 46.0, 24.0));
        scene.push_hit(GpuHit::new(HitKind::Input, 7, 10.0, 44.0, 160.0, 34.0));
        scene.push_hit(GpuHit::new(HitKind::Scrollbar, 9, 180.0, 10.0, 8.0, 120.0));
        let mut state = UiRuntimeState::default();

        assert_eq!(
            state.handle_event(&scene, UiEvent::PointerDown { x: 12.0, y: 12.0 }),
            UiAction::Activated(GpuHit::new(HitKind::Toggle, 4, 10.0, 10.0, 46.0, 24.0))
        );
        assert_eq!(
            state.handle_event(&scene, UiEvent::PointerUp { x: 12.0, y: 12.0 }),
            UiAction::Toggled { id: 4, on: true }
        );

        assert!(matches!(
            state.handle_event(&scene, UiEvent::PointerDown { x: 20.0, y: 50.0 }),
            UiAction::Focused(Some(hit)) if hit.kind == HitKind::Input && hit.id == 7
        ));
        assert_eq!(
            state.handle_event(&scene, UiEvent::TextInput("ok".to_string())),
            UiAction::TextChanged {
                id: 7,
                value: "ok".to_string()
            }
        );
        assert_eq!(
            state.handle_event(
                &scene,
                UiEvent::KeyDown {
                    key: UiKey::Backspace,
                },
            ),
            UiAction::TextChanged {
                id: 7,
                value: "o".to_string()
            }
        );

        assert_eq!(
            state.handle_event(
                &scene,
                UiEvent::Wheel {
                    x: 182.0,
                    y: 20.0,
                    delta_y: 180.0,
                },
            ),
            UiAction::ScrollChanged { id: 9, offset: 0.2 }
        );
    }

    #[test]
    fn ui_node_render_consumes_runtime_scroll_state() {
        let rows = (0..8).map(|index| {
            list_row_node(
                &format!("Runtime row {}", index + 1),
                "state owned by rust",
                130 + index,
            )
        });
        let mut runtime = UiRuntimeState::default();
        runtime.set_scroll_offset(77, 1.0);

        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            scroll_area("bg-panel border rounded-md p-2 gap-2", 0.0)
                .scroll_id(77)
                .children(rows)
                .render_with_state(&mut ui, UiRect::new(0.0, 0.0, 320.0, 150.0), Some(&runtime));
        }

        assert!(
            !scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 130)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 137)
        );
    }

    #[test]
    fn workspace_tiles_apps_without_overlap_and_clips_surfaces() {
        let mut workspace = UiWorkspace::split(
            UiTileAxis::Horizontal,
            UiAppSurface::new(1, "Chat"),
            UiAppSurface::new(2, "Trust"),
        );
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            workspace.render(
                &mut ui,
                UiRect::new(0.0, 0.0, 640.0, 360.0),
                |ui, bounds, app| {
                    ui.fill_rect(bounds, 0.0, palette::PANEL);
                    ui.bounded_label(
                        bounds.x + 12.0,
                        bounds.y + 14.0,
                        bounds.w - 24.0,
                        &app.title,
                        2.0,
                        palette::TEXT,
                    );
                    ui.hit(
                        HitKind::Button,
                        app.id + 100,
                        bounds.x,
                        bounds.y,
                        bounds.w,
                        bounds.h,
                    );
                },
            );
        }

        let first = workspace.app(1).and_then(UiAppSurface::bounds).unwrap();
        let second = workspace.app(2).and_then(UiAppSurface::bounds).unwrap();
        assert!(first.x + first.w <= second.x);
        assert!(first.h <= 360.0 - WORKSPACE_CHROME_H);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::WorkspaceTab && hit.id == 1)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::WorkspaceClose && hit.id == 2)
        );
        assert!(
            scene
                .hits()
                .iter()
                .filter(|hit| hit.kind == HitKind::Button)
                .all(|hit| hit.y >= WORKSPACE_CHROME_H)
        );
    }

    #[test]
    fn workspace_routes_pointer_events_to_app_runtime() {
        let mut workspace = UiWorkspace::single(UiAppSurface::new(7, "Chat"));
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            workspace.render(
                &mut ui,
                UiRect::new(0.0, 0.0, 320.0, 220.0),
                |ui, bounds, _app| {
                    ui.hit(
                        HitKind::Toggle,
                        55,
                        bounds.x + 20.0,
                        bounds.y + 20.0,
                        46.0,
                        24.0,
                    );
                    ui.toggle(bounds.x + 20.0, bounds.y + 20.0, false, 55);
                },
            );
        }

        let down = workspace.handle_event(&scene, UiEvent::PointerDown { x: 24.0, y: 58.0 });
        assert!(matches!(
            down,
            UiWorkspaceAction::AppAction {
                app_id: 7,
                action: UiAction::Activated(_)
            }
        ));
        let up = workspace.handle_event(&scene, UiEvent::PointerUp { x: 24.0, y: 58.0 });
        assert_eq!(
            up,
            UiWorkspaceAction::AppAction {
                app_id: 7,
                action: UiAction::Toggled { id: 55, on: true }
            }
        );
        assert!(
            workspace
                .app(7)
                .is_some_and(|app| app.runtime.toggle_value(55, false))
        );
    }

    #[test]
    fn shell_overlay_toggles_launcher_and_opens_apps() {
        let mut shell = UiShellState::default();
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            render_edgerun_shell_overlay(&mut ui, UiRect::new(0.0, 0.0, 900.0, 600.0), &mut shell);
        }
        let launcher_hit = scene
            .hits()
            .iter()
            .find(|hit| hit.kind == HitKind::ShellLauncher)
            .copied()
            .expect("shell launcher hit");
        let action = shell.handle_event(
            &scene,
            UiEvent::PointerDown {
                x: launcher_hit.x + 4.0,
                y: launcher_hit.y + 4.0,
            },
        );
        assert_eq!(action, UiShellAction::ToggledLauncher(true));
        assert!(shell.launcher_open);

        scene.clear_rects();
        {
            let mut ui = UiPainter::new(&mut scene);
            render_edgerun_shell_overlay(&mut ui, UiRect::new(0.0, 0.0, 900.0, 600.0), &mut shell);
        }
        let gallery_hit = scene
            .hits()
            .iter()
            .find(|hit| hit.id == LAUNCH_COMPONENT_GALLERY_ITEM_ID)
            .copied()
            .expect("gallery hit");
        let action = shell.handle_event(
            &scene,
            UiEvent::PointerDown {
                x: gallery_hit.x + 4.0,
                y: gallery_hit.y + 4.0,
            },
        );
        assert_eq!(
            action,
            UiShellAction::OpenApp {
                app_id: COMPONENT_GALLERY_APP_ID,
                kind: UiAppKind::ComponentGallery,
            }
        );
        assert!(!shell.launcher_open);
    }

    #[test]
    fn full_screen_system_app_replaces_workspace_chrome() {
        let mut workspace = UiWorkspace::full_screen(UiAppSurface::lock_screen(10));
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            workspace.render(
                &mut ui,
                UiRect::new(0.0, 0.0, 640.0, 360.0),
                |ui, bounds, app| {
                    ui.fill_rect(bounds, 0.0, palette::PANEL);
                    ui.hit(
                        HitKind::Button,
                        app.id + 100,
                        bounds.x,
                        bounds.y,
                        bounds.w,
                        bounds.h,
                    );
                },
            );
        }

        let bounds = workspace.app(10).and_then(UiAppSurface::bounds).unwrap();
        assert_eq!(bounds, UiRect::new(0.0, 0.0, 640.0, 360.0));
        assert!(
            !scene
                .hits()
                .iter()
                .any(|hit| matches!(hit.kind, HitKind::WorkspaceTab | HitKind::WorkspaceClose))
        );
    }

    #[test]
    fn lock_and_capability_apps_render_expected_actions() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            let app = UiAppSurface::lock_screen(10);
            render_lock_screen_app(&mut ui, UiRect::new(0.0, 0.0, 800.0, 520.0), &app);
        }
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == LOCK_UNLOCK_BUTTON_ID)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Input && hit.id == LOCK_UNLOCK_FIELD_ID)
        );

        scene.clear_rects();
        {
            let mut ui = UiPainter::new(&mut scene);
            let app = UiAppSurface::capability_request(11);
            render_capability_request_app(&mut ui, UiRect::new(0.0, 0.0, 900.0, 620.0), &app);
        }
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == CAPABILITY_ALLOW_BUTTON_ID)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == CAPABILITY_DENY_BUTTON_ID)
        );
    }

    #[test]
    fn component_gallery_renders_reusable_primitives() {
        let mut scene = GpuScene::new(palette::BG);
        let app = UiAppSurface::component_gallery(12);
        {
            let mut ui = UiPainter::new(&mut scene);
            render_component_gallery_app(&mut ui, UiRect::new(0.0, 0.0, 900.0, 1100.0), &app);
        }

        assert!(scene.rects().len() > 80);
        assert!(scene.hits().iter().any(|hit| hit.id == 761));
        assert!(!scene.hits().iter().any(|hit| hit.id == 762));
        assert!(scene.hits().iter().any(|hit| hit.id == 780));
        assert!(scene.hits().iter().any(|hit| hit.id == 800));
    }

    #[cfg(feature = "tabler-svg-atlas")]
    #[test]
    fn canonical_icons_resolve_tabler_svg_atlas_rects() {
        let atlas = tabler_svg_icon_atlas();
        assert_eq!(
            atlas.width,
            crate::tabler_svg_atlas_generated::TABLER_SVG_ATLAS_W
        );
        assert_eq!(atlas.alpha.len(), (atlas.width * atlas.height) as usize);
        let rect = UiIcon::Trust
            .tabler_svg_atlas_rect()
            .expect("trust icon atlas rect");
        assert_eq!(rect.name, "shield-check");
        assert!(rect.u0 >= 0.0 && rect.u1 <= 1.0 && rect.u0 < rect.u1);
        assert!(rect.v0 >= 0.0 && rect.v1 <= 1.0 && rect.v0 < rect.v1);
    }

    #[test]
    fn grid_node_places_spanned_dashboard_cards() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            let labels = ["A", "B", "C"];
            let values = [0.25, 0.5, 0.75];
            grid("bg-panel border rounded-md p-3 gap-3", 4)
                .children([
                    metric("Balance", "$42.00")
                        .progress(0.4)
                        .span(2)
                        .class("h-32"),
                    field_node("Relay", "nodes.edgerun.tech")
                        .focused(true)
                        .span(2),
                    bar_chart_labels("Activity", &labels, &values)
                        .span(4)
                        .class("h-44"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 640.0, 420.0));
        }

        assert!(scene.rects().len() > 30);
        assert!(scene.hits().is_empty());
    }

    #[test]
    fn grid_auto_reads_columns_from_classes() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            grid_auto("grid grid-cols-3 bg-panel border rounded-md p-3 gap-3")
                .children([
                    metric("One", "1").class("h-24"),
                    metric("Two", "2").class("h-24"),
                    metric("Three", "3").class("h-24"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 480.0, 160.0));
        }

        assert!(scene.rects().len() > 20);
    }

    #[test]
    fn grid_auto_for_width_applies_responsive_columns() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            grid_auto_for_width(
                "grid grid-cols-1 md:grid-cols-3 bg-panel p-3 gap-2 md:gap-3",
                900.0,
            )
            .children([
                metric("One", "1").class("h-24"),
                metric("Two", "2").class("h-24"),
                metric("Three", "3").class("h-24"),
            ])
            .render(&mut ui, UiRect::new(0.0, 0.0, 480.0, 160.0));
        }

        assert!(scene.rects().len() > 20);
    }

    #[test]
    fn alignment_classes_control_child_placement() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            row("bg-panel border rounded-md p-2 items-center justify-between")
                .children([
                    text("Left").class("w-12"),
                    button("Right", 80, ButtonStyle::Secondary).class("w-20 h-8"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 320.0, 80.0));
        }

        let hit = scene
            .hits()
            .iter()
            .find(|hit| hit.id == 80)
            .expect("button hit");
        assert!(hit.x > 220.0);
        assert!((hit.y - 24.0).abs() < 0.1);
    }

    #[test]
    fn column_children_stretch_by_default() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-panel border rounded-md p-2 gap-2")
                .child(button("Wide", 81, ButtonStyle::Secondary).class("h-8"))
                .render(&mut ui, UiRect::new(0.0, 0.0, 220.0, 80.0));
        }

        let hit = scene
            .hits()
            .iter()
            .find(|hit| hit.id == 81)
            .expect("button hit");
        assert!(hit.w > 190.0);
    }

    #[test]
    fn conditional_children_do_not_allocate_placeholder_layout() {
        let mut hidden = row("gap-2")
            .when(false, text("hidden"))
            .child(text("visible"));
        let shown = row("gap-2").when(true, text("visible"));

        assert_eq!(hidden.children.len(), 1);
        assert_eq!(shown.children.len(), 1);
        hidden = hidden.child(text("second"));
        assert_eq!(hidden.children.len(), 2);
    }

    #[test]
    fn canonical_icons_map_to_replaceable_provider_names() {
        assert_eq!(UiIconSet::default(), UiIconSet::Tabler);
        assert_eq!(UiIcon::Trust.name(), "trust");
        assert_eq!(
            UiIcon::Trust.provider_name(UiIconSet::Tabler),
            "shield-check"
        );
        assert_eq!(
            UiIcon::Trust.provider_name(UiIconSet::Lucide),
            "shield-check"
        );
        assert_eq!(
            UiIcon::Terminal.provider_name(UiIconSet::Tabler),
            "terminal-2"
        );
        assert_eq!(
            UiIcon::Terminal.provider_name(UiIconSet::Lucide),
            "square-terminal"
        );
    }

    #[test]
    fn canonical_icon_node_renders_gpu_geometry() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            row("row gap-2 items-center")
                .child(icon(UiIcon::Lock).accent(palette::ACCENT).class("size-8"))
                .child(icon_button(UiIcon::X, 91).class("size-8"))
                .render(&mut ui, UiRect::new(0.0, 0.0, 120.0, 48.0));
        }

        assert!(scene.rects().len() > 8);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Button && hit.id == 91)
        );
    }

    #[test]
    fn foundation_form_controls_render_and_emit_state() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-panel border rounded-md p-3 gap-2")
                .child(checkbox("Cache verified bytes", false, 201))
                .child(radio("Personal admission", false, 202))
                .child(select_node("Relay endpoint", "assigned by admission", 203))
                .child(tooltip("Only the Trust Container can decrypt."))
                .render(&mut ui, UiRect::new(0.0, 0.0, 360.0, 190.0));
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Checkbox && hit.id == 201)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Radio && hit.id == 202)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Select && hit.id == 203)
        );

        let mut runtime = UiRuntimeState::default();
        let down = runtime.handle_event(&scene, UiEvent::PointerDown { x: 18.0, y: 18.0 });
        assert!(matches!(down, UiAction::Activated(hit) if hit.kind == HitKind::Checkbox));
        assert_eq!(
            runtime.handle_event(&scene, UiEvent::PointerUp { x: 18.0, y: 18.0 }),
            UiAction::Toggled { id: 201, on: true }
        );

        let _ = runtime.handle_event(&scene, UiEvent::PointerDown { x: 18.0, y: 54.0 });
        assert_eq!(
            runtime.handle_event(&scene, UiEvent::PointerUp { x: 18.0, y: 54.0 }),
            UiAction::Toggled { id: 202, on: true }
        );
    }

    #[test]
    fn feedback_components_render_without_custom_surfaces() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            grid("grid grid-cols-2 bg-bg p-3 gap-3", 2)
                .child(
                    dialog(
                        "Capability request",
                        "Review the admitted action before signing.",
                        UiIcon::Shield,
                    )
                    .class("h-44"),
                )
                .child(empty_state(
                    "No proofs yet",
                    "Runtime events will appear here.",
                    UiIcon::File,
                ))
                .child(toast("Route admitted", UiIcon::Check, palette::GREEN).span(2))
                .child(skeleton().class("h-6"))
                .child(progress_ring(0.64, palette::ACCENT).class("size-12"))
                .render(&mut ui, UiRect::new(0.0, 0.0, 720.0, 420.0));
        }

        assert!(scene.rects().len() > 40);
        assert!(scene.hits().is_empty());
    }

    #[test]
    fn data_navigation_components_render_semantic_hits() {
        let rows: &[&[&str]] = &[
            &["admission", "policy", "ready"],
            &["relay", "route", "pending"],
        ];
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-bg p-3 gap-3")
                .child(breadcrumb(&["Trust", "Routes", "Relay"], 2, 300))
                .child(command_palette("Search commands, apps, proofs...", 310))
                .child(table_labels(&["Node", "Kind", "State"], rows, 320).class("h-32"))
                .child(section("Node instances", "identity + role + policy"))
                .child(tree_item(
                    "admission:personal",
                    "local policy",
                    0,
                    true,
                    330,
                ))
                .child(tree_item("relay:nodes", "websocket", 1, false, 331))
                .render(&mut ui, UiRect::new(0.0, 0.0, 640.0, 420.0));
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Breadcrumb && hit.id == 302)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Input && hit.id == 310)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 320)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::TreeItem && hit.id == 331)
        );
    }

    #[test]
    fn edgerun_domain_components_render_semantic_rows() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            grid("grid grid-cols-2 bg-bg p-3 gap-3", 2)
                .child(identity_card("Ken", "browser-node", "personal policy", 400))
                .child(package_card("Chat", "free-run", "b3f2...a91", 401))
                .child(route_path("Admitted route", &["app", "device", "relay", "user"]).span(2))
                .child(contact_card("Codex client", "app contact", 402))
                .child(thread_row("Alice", "Encrypted message", true, 403))
                .child(attachment_preview("photo.jpg", "image payload", 404))
                .child(capability_grant_row(
                    "Chat",
                    "decrypt message",
                    "single use",
                    405,
                ))
                .child(proof_event_row("Package verified", "hash:b3f2", "ok", 406))
                .child(receipt_row("Relay delivery", "+$0.01", "settled", 407))
                .render(&mut ui, UiRect::new(0.0, 0.0, 760.0, 620.0));
        }

        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 400)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 406)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::TransactionRow && hit.id == 407)
        );
    }
}
