#[cfg(not(feature = "fontdue-text"))]
use core::marker::PhantomData;

use super::*;

pub struct UiPainter<'a, 'font> {
    pub(super) scene: &'a mut GpuScene,
    pub(super) theme: UiResolvedTheme,
    #[cfg(feature = "fontdue-text")]
    pub(super) atlas: Option<&'font FontAtlas>,
    #[cfg(not(feature = "fontdue-text"))]
    pub(super) _font: PhantomData<&'font ()>,
}

impl<'a, 'font> UiPainter<'a, 'font> {
    pub fn new(scene: &'a mut GpuScene) -> Self {
        Self {
            scene,
            theme: UiResolvedTheme::default(),
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
            theme: UiResolvedTheme::default(),
            atlas: Some(atlas),
        }
    }

    pub fn with_theme(mut self, theme: UiResolvedTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn theme(&self) -> UiResolvedTheme {
        self.theme
    }

    pub fn set_theme(&mut self, theme: UiResolvedTheme) {
        self.theme = theme;
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
                    .push_rect(GpuRect::fill(x, y, w, 1.0, 0.0, self.theme.colors.border));
            }
            Axis::Vertical => {
                self.scene
                    .push_rect(GpuRect::fill(x, y, 1.0, w, 0.0, self.theme.colors.border));
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
                self.theme.colors.success
            } else {
                self.theme.colors.muted
            },
        ));
    }

    pub fn button(&mut self, rect: UiRect, label: &str, style: ButtonStyle, id: u32, active: bool) {
        self.hit(HitKind::Button, id, rect.x, rect.y, rect.w, rect.h);
        let colors = self.theme.colors;
        let (fill, border, text) = match style {
            ButtonStyle::Primary => (colors.accent, colors.accent, colors.accent_text),
            ButtonStyle::Secondary => (colors.row, colors.border, colors.text),
            ButtonStyle::Ghost => (colors.panel.with_alpha(0.0), colors.border, colors.muted),
            ButtonStyle::Danger => (colors.danger.with_alpha(0.18), colors.danger, colors.danger),
        };
        let fill = if active {
            fill
        } else {
            fill.with_alpha(fill.a * 0.74)
        };
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            self.theme.radius.control,
            fill,
        ));
        self.scene.push_rect(GpuRect::border(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            self.theme.radius.control,
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
            self.theme.colors.row
        } else {
            self.theme.colors.row.with_alpha(0.58)
        };
        self.fill_rect(rect, self.theme.radius.control, fill);
        self.border_rect(
            rect,
            self.theme.radius.control,
            self.theme
                .colors
                .border
                .with_alpha(if active { 0.72 } else { 0.42 }),
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
                self.theme.colors.text
            } else {
                self.theme.colors.muted
            },
        );
    }

    pub fn icon(&mut self, rect: UiRect, icon: UiIcon, color: Color4) {
        draw_canonical_icon(self.scene, rect, icon, color);
    }

    pub fn checkbox(&mut self, rect: UiRect, label: &str, checked: bool, id: u32) {
        self.hit(HitKind::Checkbox, id, rect.x, rect.y, rect.w, rect.h);
        let colors = self.theme.colors;
        let radius = self.theme.radius.control.min(6.0);
        let box_rect = UiRect::new(rect.x, rect.y + (rect.h - 22.0) * 0.5, 22.0, 22.0);
        self.fill_rect(box_rect, radius, colors.row);
        self.border_rect(
            box_rect,
            radius,
            if checked {
                colors.accent
            } else {
                colors.border
            },
        );
        if checked {
            self.icon(box_rect.inset(4.0, 4.0), UiIcon::Check, colors.accent);
        }
        self.bounded_label(
            rect.x + 32.0,
            rect.y + (rect.h - 14.0) * 0.5,
            (rect.w - 32.0).max(0.0),
            label,
            2.0,
            colors.text,
        );
    }

    pub fn radio(&mut self, rect: UiRect, label: &str, selected: bool, id: u32) {
        self.hit(HitKind::Radio, id, rect.x, rect.y, rect.w, rect.h);
        let colors = self.theme.colors;
        let dot_rect = UiRect::new(rect.x, rect.y + (rect.h - 22.0) * 0.5, 22.0, 22.0);
        self.fill_rect(dot_rect, 11.0, colors.row);
        self.border_rect(
            dot_rect,
            11.0,
            if selected {
                colors.accent
            } else {
                colors.border
            },
        );
        if selected {
            self.fill_rect(dot_rect.inset(6.0, 6.0), 5.0, colors.accent);
        }
        self.bounded_label(
            rect.x + 32.0,
            rect.y + (rect.h - 14.0) * 0.5,
            (rect.w - 32.0).max(0.0),
            label,
            2.0,
            colors.text,
        );
    }

    pub fn select_trigger(&mut self, rect: UiRect, label: &str, value: &str, id: u32) {
        self.hit(HitKind::Select, id, rect.x, rect.y, rect.w, rect.h);
        let colors = self.theme.colors;
        let radius = self.theme.radius.control;
        self.fill_rect(rect, radius, colors.composer);
        self.border_rect(rect, radius, colors.border);
        self.bounded_label(
            rect.x + 14.0,
            rect.y + 8.0,
            (rect.w - 48.0).max(0.0),
            label,
            2.0,
            colors.muted,
        );
        self.bounded_label(
            rect.x + 14.0,
            rect.y + 29.0,
            (rect.w - 48.0).max(0.0),
            value,
            2.0,
            colors.text,
        );
        self.icon(
            UiRect::new(
                rect.x + rect.w - 31.0,
                rect.y + (rect.h - 18.0) * 0.5,
                18.0,
                18.0,
            ),
            UiIcon::ChevronRight,
            colors.muted,
        );
    }

    pub fn tooltip(&mut self, rect: UiRect, text: &str) {
        let colors = self.theme.colors;
        self.fill_rect(rect, self.theme.radius.card, colors.topbar);
        self.border_rect(rect, self.theme.radius.card, colors.border.with_alpha(0.72));
        self.bounded_label(
            rect.x + 10.0,
            rect.y + (rect.h - 14.0) * 0.5,
            (rect.w - 20.0).max(0.0),
            text,
            2.0,
            colors.text,
        );
    }

    pub fn dialog(&mut self, rect: UiRect, title: &str, body: &str, icon: UiIcon) {
        let colors = self.theme.colors;
        self.card(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            self.theme.radius.panel,
            colors.panel,
        );
        self.icon(
            UiRect::new(rect.x + 18.0, rect.y + 18.0, 34.0, 34.0),
            icon,
            colors.accent,
        );
        self.bounded_label(
            rect.x + 64.0,
            rect.y + 18.0,
            rect.w - 84.0,
            title,
            2.0,
            colors.text,
        );
        self.bounded_label(
            rect.x + 64.0,
            rect.y + 42.0,
            rect.w - 84.0,
            body,
            2.0,
            colors.muted,
        );
        self.divider(
            rect.x + 18.0,
            rect.y + 74.0,
            rect.w - 36.0,
            Axis::Horizontal,
        );
    }

    pub fn toast(&mut self, rect: UiRect, message: &str, icon: UiIcon, accent: Color4) {
        self.fill_rect(rect, self.theme.radius.control, self.theme.colors.topbar);
        self.border_rect(rect, self.theme.radius.control, accent.with_alpha(0.54));
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
            self.theme.colors.text,
        );
    }

    pub fn empty_state(&mut self, rect: UiRect, title: &str, body: &str, icon: UiIcon) {
        let colors = self.theme.colors;
        self.fill_rect(rect, self.theme.radius.card, colors.panel);
        self.border_rect(rect, self.theme.radius.card, colors.border.with_alpha(0.64));
        let icon_size = rect.h.min(rect.w).min(54.0);
        let icon_rect = UiRect::new(
            rect.x + (rect.w - icon_size) * 0.5,
            rect.y + 22.0,
            icon_size,
            icon_size,
        );
        self.icon(icon_rect, icon, colors.accent);
        self.bounded_label(
            rect.x + 20.0,
            icon_rect.y + icon_rect.h + 16.0,
            rect.w - 40.0,
            title,
            2.0,
            colors.text,
        );
        self.bounded_label(
            rect.x + 20.0,
            icon_rect.y + icon_rect.h + 40.0,
            rect.w - 40.0,
            body,
            2.0,
            colors.muted,
        );
    }

    pub fn skeleton(&mut self, rect: UiRect) {
        let colors = self.theme.colors;
        self.fill_rect(rect, self.theme.radius.card, colors.row.with_alpha(0.74));
        let shine_w = (rect.w * 0.28).max(18.0).min(rect.w);
        self.fill_rect(
            UiRect::new(rect.x + rect.w * 0.18, rect.y, shine_w, rect.h),
            self.theme.radius.card,
            colors.border.with_alpha(0.34),
        );
    }

    pub fn progress_ring(&mut self, rect: UiRect, value: f32, color: Color4) {
        let size = rect.w.min(rect.h);
        let center = (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5);
        let radius = size * 0.38;
        icon_circle(self.scene, center, radius, 2.0, self.theme.colors.border);
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
        let colors = self.theme.colors;
        self.fill_rect(rect, self.theme.radius.card, colors.panel);
        self.border_rect(rect, self.theme.radius.card, colors.border);
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
                colors.muted,
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
                self.fill_rect(
                    row_rect,
                    self.theme.radius.card,
                    colors.row.with_alpha(0.48),
                );
            }
            for col in 0..cols {
                let value = row.get(col).map(String::as_str).unwrap_or("");
                self.bounded_label(
                    rect.x + 12.0 + col as f32 * col_w,
                    y,
                    col_w - 10.0,
                    value,
                    2.0,
                    colors.text,
                );
            }
            y += 34.0;
            if y > rect.y + rect.h - 20.0 {
                break;
            }
        }
    }

    pub fn breadcrumb(&mut self, rect: UiRect, items: &[String], selected: usize, base_id: u32) {
        let colors = self.theme.colors;
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
                self.theme.radius.card,
                if index == selected {
                    colors.active
                } else {
                    colors.row.with_alpha(0.38)
                },
            );
            self.bounded_label(
                item_rect.x + 10.0,
                item_rect.y + 8.0,
                item_rect.w - 20.0,
                item,
                2.0,
                if index == selected {
                    colors.text
                } else {
                    colors.muted
                },
            );
            x += w + 6.0;
            if index + 1 < items.len() {
                self.icon(
                    UiRect::new(x, rect.y + 7.0, 16.0, 16.0),
                    UiIcon::ChevronRight,
                    colors.muted,
                );
                x += 22.0;
            }
        }
    }

    pub fn command_palette(&mut self, rect: UiRect, placeholder: &str, id: u32) {
        self.hit(HitKind::Input, id, rect.x, rect.y, rect.w, rect.h);
        let colors = self.theme.colors;
        self.fill_rect(rect, self.theme.radius.control, colors.composer);
        self.border_rect(rect, self.theme.radius.control, colors.border);
        self.icon(
            UiRect::new(rect.x + 14.0, rect.y + (rect.h - 20.0) * 0.5, 20.0, 20.0),
            UiIcon::Search,
            colors.muted,
        );
        self.bounded_label(
            rect.x + 44.0,
            rect.y + (rect.h - 14.0) * 0.5,
            rect.w - 58.0,
            placeholder,
            2.0,
            colors.muted,
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
        let colors = self.theme.colors;
        self.fill_rect(rect, self.theme.radius.card, colors.panel);
        let indent = 12.0 + depth as f32 * 18.0;
        self.icon(
            UiRect::new(rect.x + indent, rect.y + (rect.h - 16.0) * 0.5, 16.0, 16.0),
            if expanded {
                UiIcon::ChevronRight
            } else {
                UiIcon::File
            },
            colors.muted,
        );
        self.bounded_label(
            rect.x + indent + 24.0,
            rect.y + 9.0,
            rect.w * 0.48,
            label,
            2.0,
            colors.text,
        );
        self.bounded_label(
            rect.x + rect.w * 0.58,
            rect.y + 9.0,
            rect.w * 0.36,
            detail,
            2.0,
            colors.muted,
        );
    }

    pub fn section_header(&mut self, rect: UiRect, title: &str, detail: &str) {
        let colors = self.theme.colors;
        self.bounded_label(rect.x, rect.y, rect.w * 0.55, title, 2.0, colors.text);
        self.bounded_label(
            rect.x + rect.w * 0.58,
            rect.y,
            rect.w * 0.42,
            detail,
            2.0,
            colors.muted,
        );
        self.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);
    }

    pub fn identity_card(&mut self, rect: UiRect, name: &str, node: &str, policy: &str, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        let colors = self.theme.colors;
        self.card(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            self.theme.radius.card,
            colors.panel,
        );
        self.icon(
            UiRect::new(rect.x + 16.0, rect.y + 18.0, 34.0, 34.0),
            UiIcon::Trust,
            colors.accent,
        );
        self.bounded_label(
            rect.x + 62.0,
            rect.y + 16.0,
            rect.w - 82.0,
            name,
            2.0,
            colors.text,
        );
        self.bounded_label(
            rect.x + 62.0,
            rect.y + 39.0,
            rect.w - 82.0,
            node,
            2.0,
            colors.muted,
        );
        self.badge(rect.x + 16.0, rect.y + rect.h - 34.0, policy, colors.accent);
    }

    pub fn contact_card(&mut self, rect: UiRect, name: &str, detail: &str, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        let colors = self.theme.colors;
        self.fill_rect(rect, self.theme.radius.card, colors.panel);
        self.border_rect(rect, self.theme.radius.card, colors.border);
        self.avatar(
            rect.x + 12.0,
            rect.y + 12.0,
            36.0,
            name,
            colors.accent,
            true,
        );
        self.bounded_label(
            rect.x + 58.0,
            rect.y + 13.0,
            rect.w - 72.0,
            name,
            2.0,
            colors.text,
        );
        self.bounded_label(
            rect.x + 58.0,
            rect.y + 35.0,
            rect.w - 72.0,
            detail,
            2.0,
            colors.muted,
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
        let colors = self.theme.colors;
        self.fill_rect(
            rect,
            self.theme.radius.card,
            if unread { colors.active } else { colors.panel },
        );
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 15.0, 24.0, 24.0),
            UiIcon::Chat,
            if unread { colors.accent } else { colors.muted },
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 10.0,
            rect.w - 62.0,
            title,
            2.0,
            colors.text,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 32.0,
            rect.w - 62.0,
            last_message,
            2.0,
            colors.muted,
        );
    }

    pub fn attachment_preview(&mut self, rect: UiRect, name: &str, kind: &str, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        let colors = self.theme.colors;
        self.fill_rect(rect, self.theme.radius.card, colors.row);
        self.border_rect(rect, self.theme.radius.card, colors.border.with_alpha(0.68));
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 12.0, 28.0, 28.0),
            UiIcon::File,
            colors.accent,
        );
        self.bounded_label(
            rect.x + 52.0,
            rect.y + 11.0,
            rect.w - 66.0,
            name,
            2.0,
            colors.text,
        );
        self.bounded_label(
            rect.x + 52.0,
            rect.y + 33.0,
            rect.w - 66.0,
            kind,
            2.0,
            colors.muted,
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
        let colors = self.theme.colors;
        self.fill_rect(rect, 0.0, colors.panel);
        self.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 17.0, 24.0, 24.0),
            UiIcon::Shield,
            colors.info,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 10.0,
            rect.w * 0.34,
            app,
            2.0,
            colors.text,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 32.0,
            rect.w * 0.34,
            capability,
            2.0,
            colors.muted,
        );
        self.badge(rect.x + rect.w - 96.0, rect.y + 18.0, state, colors.accent);
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
        let colors = self.theme.colors;
        self.fill_rect(rect, 0.0, colors.panel);
        self.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 17.0, 24.0, 24.0),
            UiIcon::Check,
            colors.success,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 10.0,
            rect.w * 0.36,
            title,
            2.0,
            colors.text,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 32.0,
            rect.w * 0.48,
            hash,
            2.0,
            colors.muted,
        );
        self.badge(
            rect.x + rect.w - 96.0,
            rect.y + 18.0,
            status,
            colors.success,
        );
    }

    pub fn route_path(&mut self, rect: UiRect, label: &str, hops: &[String]) {
        let colors = self.theme.colors;
        self.fill_rect(rect, self.theme.radius.card, colors.panel);
        self.border_rect(rect, self.theme.radius.card, colors.border);
        self.bounded_label(
            rect.x + 14.0,
            rect.y + 10.0,
            rect.w - 28.0,
            label,
            2.0,
            colors.text,
        );
        let mut x = rect.x + 16.0;
        let y = rect.y + 45.0;
        for (index, hop) in hops.iter().enumerate() {
            self.icon(UiRect::new(x, y, 22.0, 22.0), UiIcon::Route, colors.accent);
            self.bounded_label(x + 28.0, y + 4.0, 78.0, hop, 2.0, colors.muted);
            x += 112.0;
            if index + 1 < hops.len() {
                self.icon(
                    UiRect::new(x - 22.0, y + 3.0, 16.0, 16.0),
                    UiIcon::ChevronRight,
                    colors.muted,
                );
            }
            if x > rect.x + rect.w - 80.0 {
                break;
            }
        }
    }

    pub fn package_card(&mut self, rect: UiRect, name: &str, policy: &str, hash: &str, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        let colors = self.theme.colors;
        self.card(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            self.theme.radius.card,
            colors.panel,
        );
        self.icon(
            UiRect::new(rect.x + 16.0, rect.y + 18.0, 30.0, 30.0),
            UiIcon::App,
            colors.accent,
        );
        self.bounded_label(
            rect.x + 58.0,
            rect.y + 16.0,
            rect.w - 76.0,
            name,
            2.0,
            colors.text,
        );
        self.bounded_label(
            rect.x + 58.0,
            rect.y + 39.0,
            rect.w - 76.0,
            hash,
            2.0,
            colors.muted,
        );
        self.badge(rect.x + 16.0, rect.y + rect.h - 34.0, policy, colors.info);
    }

    pub fn receipt_row(&mut self, rect: UiRect, label: &str, amount: &str, status: &str, id: u32) {
        self.hit(HitKind::TransactionRow, id, rect.x, rect.y, rect.w, rect.h);
        let colors = self.theme.colors;
        self.fill_rect(rect, 0.0, colors.panel);
        self.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 17.0, 24.0, 24.0),
            UiIcon::Wallet,
            colors.success,
        );
        self.bounded_label(
            rect.x + 48.0,
            rect.y + 18.0,
            rect.w * 0.38,
            label,
            2.0,
            colors.text,
        );
        self.bounded_label(
            rect.x + rect.w - 150.0,
            rect.y + 18.0,
            70.0,
            status,
            2.0,
            colors.muted,
        );
        self.bounded_label(
            rect.x + rect.w - 76.0,
            rect.y + 18.0,
            64.0,
            amount,
            2.0,
            colors.success,
        );
    }

    pub fn input_field(&mut self, rect: UiRect, placeholder: &str, focused: bool) {
        let colors = self.theme.colors;
        let radius = self.theme.radius.control;
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            radius,
            colors.composer,
        ));
        self.scene.push_rect(GpuRect::border(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            radius,
            if focused {
                colors.accent
            } else {
                colors.border
            },
        ));
        self.bounded_label(
            rect.x + 16.0,
            rect.y + 13.0,
            (rect.w - 32.0).max(0.0),
            placeholder,
            2.0,
            colors.muted,
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
        let colors = self.theme.colors;
        self.fill_rect(rect, self.theme.radius.card, colors.row.with_alpha(0.72));
        self.border_rect(rect, self.theme.radius.card, colors.border.with_alpha(0.58));
        self.icon(
            UiRect::new(rect.x + 12.0, rect.y + 15.0, 26.0, 26.0),
            icon,
            colors.accent,
        );
        self.bounded_label(
            rect.x + 50.0,
            rect.y + 10.0,
            (rect.w - 84.0).max(0.0),
            title,
            2.0,
            colors.text,
        );
        self.bounded_label(
            rect.x + 50.0,
            rect.y + 32.0,
            (rect.w - 84.0).max(0.0),
            detail,
            2.0,
            colors.muted,
        );
        self.icon(
            UiRect::new(rect.x + rect.w - 30.0, rect.y + 20.0, 16.0, 16.0),
            UiIcon::ChevronRight,
            colors.muted,
        );
    }

    pub fn toggle(&mut self, x: f32, y: f32, on: bool, id: u32) {
        self.hit(HitKind::Toggle, id, x, y, 46.0, 24.0);
        let colors = self.theme.colors;
        let color = if on { colors.success } else { colors.muted };
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
        let colors = self.theme.colors;
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            rect.h * 0.5,
            colors.row,
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
        let colors = self.theme.colors;
        let radius = self.theme.radius.control;
        self.scene.push_rect(GpuRect::fill(
            rect.x, rect.y, rect.w, rect.h, radius, colors.row,
        ));
        self.scene.push_rect(GpuRect::border(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            radius,
            colors.border,
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
                    radius.min(9.0),
                    colors.active,
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
                if active { colors.text } else { colors.muted },
            );
        }
    }

    pub fn list_row(&mut self, rect: UiRect, title: &str, detail: &str, accent: Color4, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        let colors = self.theme.colors;
        self.card(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            self.theme.radius.card,
            colors.row,
        );
        self.scene
            .push_rect(GpuRect::fill(rect.x, rect.y, 3.0, rect.h, 2.0, accent));
        self.bounded_label(
            rect.x + 16.0,
            rect.y + 10.0,
            (rect.w - 32.0).max(0.0),
            title,
            2.0,
            colors.text,
        );
        self.bounded_label(
            rect.x + 16.0,
            rect.y + 31.0,
            (rect.w - 32.0).max(0.0),
            detail,
            2.0,
            colors.muted,
        );
    }

    pub fn scrollbar(&mut self, rect: UiRect, visible_fraction: f32, offset_fraction: f32) {
        let visible = visible_fraction.clamp(0.08, 1.0);
        let offset = offset_fraction.clamp(0.0, 1.0);
        let colors = self.theme.colors;
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            rect.w * 0.5,
            colors.row.with_alpha(0.42),
        ));
        let thumb_h = rect.h * visible;
        let thumb_y = rect.y + (rect.h - thumb_h) * offset;
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            thumb_y,
            rect.w,
            thumb_h,
            rect.w * 0.5,
            colors.muted.with_alpha(0.74),
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
        let colors = self.theme.colors;
        self.card(x, y, w, h, self.theme.radius.panel, colors.composer);
        self.scene.push_rect(GpuRect::border(
            x,
            y,
            w,
            h,
            self.theme.radius.panel,
            if active { colors.accent } else { colors.border },
        ));
        self.bounded_label(
            x + 22.0,
            y + 24.0,
            (w - 96.0).max(0.0),
            placeholder,
            2.0,
            colors.muted,
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
            colors.accent,
        ));
        self.bounded_label(
            x + w - 47.0,
            y + h - 45.0,
            24.0,
            ">",
            3.0,
            colors.accent_text,
        );
    }
}
