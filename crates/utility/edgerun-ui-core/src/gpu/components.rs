//! Reusable GPU UI components for the shared native/WASM scene.
//!
//! Components are semantic Rust structs rendered through `UiPainter`. Browser
//! and native hosts only consume the resulting `GpuScene`, keeping application
//! layout and state out of JavaScript, SDL, and OpenGL glue.

use super::{Axis, ButtonStyle, Color4, HitKind, UiPainter, UiRect, palette};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelHeader<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub action: Option<(&'a str, u32)>,
}

impl<'a> PanelHeader<'a> {
    pub const fn new(title: &'a str) -> Self {
        Self {
            title,
            subtitle: "",
            action: None,
        }
    }

    pub const fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = subtitle;
        self
    }

    pub const fn action(mut self, label: &'a str, id: u32) -> Self {
        self.action = Some((label, id));
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetricCard<'a> {
    pub title: &'a str,
    pub value: &'a str,
    pub detail: &'a str,
    pub progress: Option<f32>,
    pub accent: Color4,
}

impl<'a> MetricCard<'a> {
    pub const fn new(title: &'a str, value: &'a str) -> Self {
        Self {
            title,
            value,
            detail: "",
            progress: None,
            accent: palette::ACCENT,
        }
    }

    pub const fn detail(mut self, detail: &'a str) -> Self {
        self.detail = detail;
        self
    }

    pub const fn progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress);
        self
    }

    pub const fn accent(mut self, accent: Color4) -> Self {
        self.accent = accent;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Field<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub helper: &'a str,
    pub focused: bool,
    pub id: Option<u32>,
}

impl<'a> Field<'a> {
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            value: "",
            helper: "",
            focused: false,
            id: None,
        }
    }

    pub const fn value(mut self, value: &'a str) -> Self {
        self.value = value;
        self
    }

    pub const fn helper(mut self, helper: &'a str) -> Self {
        self.helper = helper;
        self
    }

    pub const fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub const fn id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextArea<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub focused: bool,
    pub id: Option<u32>,
}

impl<'a> TextArea<'a> {
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            value: "",
            focused: false,
            id: None,
        }
    }

    pub const fn value(mut self, value: &'a str) -> Self {
        self.value = value;
        self
    }

    pub const fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub const fn id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slider<'a> {
    pub label: &'a str,
    pub value: f32,
    pub min_label: &'a str,
    pub max_label: &'a str,
    pub accent: Color4,
    pub id: u32,
}

impl<'a> Slider<'a> {
    pub const fn new(label: &'a str, value: f32, id: u32) -> Self {
        Self {
            label,
            value,
            min_label: "",
            max_label: "",
            accent: palette::ACCENT,
            id,
        }
    }

    pub const fn range_labels(mut self, min_label: &'a str, max_label: &'a str) -> Self {
        self.min_label = min_label;
        self.max_label = max_label;
        self
    }

    pub const fn accent(mut self, accent: Color4) -> Self {
        self.accent = accent;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarChart<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub labels: &'a [&'a str],
    pub values: &'a [f32],
    pub accent: Color4,
}

impl<'a> BarChart<'a> {
    pub const fn new(title: &'a str, labels: &'a [&'a str], values: &'a [f32]) -> Self {
        Self {
            title,
            subtitle: "",
            labels,
            values,
            accent: palette::ACCENT,
        }
    }

    pub const fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = subtitle;
        self
    }

    pub const fn accent(mut self, accent: Color4) -> Self {
        self.accent = accent;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransactionRow<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub date: &'a str,
    pub amount: &'a str,
    pub positive: bool,
    pub id: u32,
}

impl<'a> TransactionRow<'a> {
    pub const fn new(title: &'a str, amount: &'a str, id: u32) -> Self {
        Self {
            title,
            subtitle: "",
            date: "",
            amount,
            positive: false,
            id,
        }
    }

    pub const fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = subtitle;
        self
    }

    pub const fn date(mut self, date: &'a str) -> Self {
        self.date = date;
        self
    }

    pub const fn positive(mut self, positive: bool) -> Self {
        self.positive = positive;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuItem<'a> {
    pub label: &'a str,
    pub detail: &'a str,
    pub badge: &'a str,
    pub selected: bool,
    pub accent: Color4,
    pub id: u32,
}

impl<'a> MenuItem<'a> {
    pub const fn new(label: &'a str, id: u32) -> Self {
        Self {
            label,
            detail: "",
            badge: "",
            selected: false,
            accent: palette::ACCENT,
            id,
        }
    }

    pub const fn detail(mut self, detail: &'a str) -> Self {
        self.detail = detail;
        self
    }

    pub const fn badge(mut self, badge: &'a str) -> Self {
        self.badge = badge;
        self
    }

    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub const fn accent(mut self, accent: Color4) -> Self {
        self.accent = accent;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControlAccessory<'a> {
    None,
    Value(&'a str),
    Badge(&'a str, Color4),
    Toggle {
        on: bool,
        id: u32,
    },
    Button {
        label: &'a str,
        id: u32,
        style: ButtonStyle,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlRow<'a> {
    pub label: &'a str,
    pub detail: &'a str,
    pub accessory: ControlAccessory<'a>,
    pub id: Option<u32>,
}

impl<'a> ControlRow<'a> {
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            detail: "",
            accessory: ControlAccessory::None,
            id: None,
        }
    }

    pub const fn detail(mut self, detail: &'a str) -> Self {
        self.detail = detail;
        self
    }

    pub const fn id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }

    pub const fn value(mut self, value: &'a str) -> Self {
        self.accessory = ControlAccessory::Value(value);
        self
    }

    pub const fn badge(mut self, label: &'a str, color: Color4) -> Self {
        self.accessory = ControlAccessory::Badge(label, color);
        self
    }

    pub const fn toggle(mut self, on: bool, id: u32) -> Self {
        self.accessory = ControlAccessory::Toggle { on, id };
        self
    }

    pub const fn button(mut self, label: &'a str, id: u32, style: ButtonStyle) -> Self {
        self.accessory = ControlAccessory::Button { label, id, style };
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiGrid {
    pub rect: UiRect,
    pub columns: u16,
    pub gap: f32,
}

impl UiGrid {
    pub const fn new(rect: UiRect, columns: u16, gap: f32) -> Self {
        Self { rect, columns, gap }
    }

    pub fn cell(&self, column: u16, span: u16, y: f32, height: f32) -> UiRect {
        let columns_u = self.columns.max(1);
        let column_u = column.min(columns_u - 1);
        let span_u = span.max(1).min(columns_u - column_u);
        let columns = columns_u as f32;
        let column = column_u as f32;
        let span = span_u as f32;
        let track = (self.rect.w - self.gap * (columns - 1.0)).max(0.0) / columns;
        UiRect::new(
            self.rect.x + column * (track + self.gap),
            self.rect.y + y,
            track * span + self.gap * (span - 1.0),
            height,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiStack {
    pub rect: UiRect,
    pub axis: Axis,
    pub gap: f32,
    cursor: f32,
}

impl UiStack {
    pub const fn vertical(rect: UiRect, gap: f32) -> Self {
        Self {
            rect,
            axis: Axis::Vertical,
            gap,
            cursor: rect.y,
        }
    }

    pub const fn horizontal(rect: UiRect, gap: f32) -> Self {
        Self {
            rect,
            axis: Axis::Horizontal,
            gap,
            cursor: rect.x,
        }
    }

    pub fn next(&mut self, size: f32) -> UiRect {
        match self.axis {
            Axis::Vertical => {
                let y = self.cursor;
                self.cursor += size + self.gap;
                UiRect::new(self.rect.x, y, self.rect.w, size)
            }
            Axis::Horizontal => {
                let x = self.cursor;
                self.cursor += size + self.gap;
                UiRect::new(x, self.rect.y, size, self.rect.h)
            }
        }
    }

    pub fn remaining(&self) -> UiRect {
        match self.axis {
            Axis::Vertical => UiRect::new(
                self.rect.x,
                self.cursor,
                self.rect.w,
                (self.rect.y + self.rect.h - self.cursor).max(0.0),
            ),
            Axis::Horizontal => UiRect::new(
                self.cursor,
                self.rect.y,
                (self.rect.x + self.rect.w - self.cursor).max(0.0),
                self.rect.h,
            ),
        }
    }
}

pub fn panel_header(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: PanelHeader<'_>) {
    let colors = ui.theme().colors;
    ui.bounded_label(
        rect.x,
        rect.y,
        action_reserved_width(rect, spec.action),
        spec.title,
        2.0,
        colors.text,
    );
    if !spec.subtitle.is_empty() {
        ui.bounded_label(
            rect.x,
            rect.y + 22.0,
            action_reserved_width(rect, spec.action),
            spec.subtitle,
            2.0,
            colors.muted,
        );
    }
    if let Some((label, id)) = spec.action {
        let w = (label.chars().count() as f32 * 9.0 + 26.0).clamp(72.0, 136.0);
        ui.button(
            UiRect::new(rect.x + rect.w - w, rect.y, w, 32.0),
            label,
            ButtonStyle::Secondary,
            id,
            true,
        );
    }
}

pub fn metric_card(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: MetricCard<'_>) {
    let theme = ui.theme();
    let colors = theme.colors;
    ui.card(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        theme.radius.card,
        colors.panel,
    );
    ui.bounded_label(
        rect.x + 16.0,
        rect.y + 15.0,
        rect.w - 32.0,
        spec.title,
        2.0,
        colors.muted,
    );
    ui.bounded_label(
        rect.x + 16.0,
        rect.y + 48.0,
        rect.w - 32.0,
        spec.value,
        4.0,
        colors.text,
    );
    if !spec.detail.is_empty() {
        ui.bounded_label(
            rect.x + 16.0,
            rect.y + rect.h - 30.0,
            rect.w - 32.0,
            spec.detail,
            2.0,
            colors.muted,
        );
    }
    if let Some(progress) = spec.progress {
        ui.progress_bar(
            UiRect::new(rect.x + 16.0, rect.y + rect.h - 47.0, rect.w - 32.0, 6.0),
            progress,
            resolve_component_accent(ui, spec.accent),
        );
    }
}

pub fn field(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: Field<'_>) {
    let colors = ui.theme().colors;
    ui.bounded_label(rect.x, rect.y, rect.w, spec.label, 2.0, colors.muted);
    let field_rect = UiRect::new(rect.x, rect.y + 25.0, rect.w, 40.0);
    if let Some(id) = spec.id {
        ui.hit(
            HitKind::Input,
            id,
            field_rect.x,
            field_rect.y,
            field_rect.w,
            field_rect.h,
        );
    }
    ui.input_field(field_rect, spec.value, spec.focused);
    if !spec.helper.is_empty() {
        ui.bounded_label(
            rect.x,
            rect.y + 72.0,
            rect.w,
            spec.helper,
            2.0,
            colors.muted,
        );
    }
}

pub fn text_area(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: TextArea<'_>) {
    let theme = ui.theme();
    let colors = theme.colors;
    let field_rect = if spec.label.is_empty() {
        rect
    } else {
        ui.bounded_label(rect.x, rect.y, rect.w, spec.label, 2.0, colors.muted);
        UiRect::new(rect.x, rect.y + 25.0, rect.w, rect.h - 25.0)
    };
    if let Some(id) = spec.id {
        ui.hit(
            HitKind::TextArea,
            id,
            field_rect.x,
            field_rect.y,
            field_rect.w,
            field_rect.h,
        );
    }
    ui.fill_rect(field_rect, theme.radius.card, colors.composer);
    ui.border_rect(
        field_rect,
        theme.radius.card,
        if spec.focused {
            colors.accent
        } else {
            colors.border
        },
    );
    let text_x = field_rect.x + 14.0;
    let text_y = field_rect.y + 14.0;
    let text_w = field_rect.w - 28.0;
    let max_lines = ((field_rect.h - 24.0) / 22.0).floor().max(1.0) as usize;
    ui.wrapped_label(
        text_x,
        text_y,
        text_w,
        spec.value,
        max_lines,
        22.0,
        if spec.value.is_empty() {
            colors.muted
        } else {
            colors.text
        },
    );
}

pub fn slider(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: Slider<'_>) {
    let colors = ui.theme().colors;
    ui.hit(HitKind::Slider, spec.id, rect.x, rect.y, rect.w, rect.h);
    ui.bounded_label(rect.x, rect.y, rect.w, spec.label, 2.0, colors.text);
    let track = UiRect::new(rect.x, rect.y + 34.0, rect.w, 6.0);
    ui.progress_bar(track, spec.value, resolve_component_accent(ui, spec.accent));
    let x = track.x + track.w * spec.value.clamp(0.0, 1.0);
    ui.fill_rect(
        UiRect::new(x - 7.0, track.y - 6.0, 14.0, 18.0),
        4.0,
        colors.text,
    );
    if !spec.min_label.is_empty() {
        ui.bounded_label(
            rect.x,
            rect.y + 51.0,
            rect.w * 0.5,
            spec.min_label,
            2.0,
            colors.muted,
        );
    }
    if !spec.max_label.is_empty() {
        ui.bounded_label(
            rect.x + rect.w * 0.5,
            rect.y + 51.0,
            rect.w * 0.5,
            spec.max_label,
            2.0,
            colors.muted,
        );
    }
}

pub fn bar_chart(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: BarChart<'_>) {
    let theme = ui.theme();
    let colors = theme.colors;
    ui.card(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        theme.radius.card,
        colors.panel,
    );
    panel_header(
        ui,
        UiRect::new(rect.x + 16.0, rect.y + 15.0, rect.w - 32.0, 44.0),
        PanelHeader {
            title: spec.title,
            subtitle: spec.subtitle,
            action: None,
        },
    );

    let count = spec.values.len().min(spec.labels.len()).min(12);
    if count == 0 {
        return;
    }
    let chart = UiRect::new(rect.x + 22.0, rect.y + 70.0, rect.w - 44.0, rect.h - 104.0);
    let max = spec.values.iter().copied().fold(0.0_f32, f32::max).max(1.0);
    let gap = 12.0_f32.min(chart.w / count as f32 * 0.32);
    let bar_w = ((chart.w - gap * (count.saturating_sub(1) as f32)) / count as f32).max(3.0);
    for index in 0..count {
        let value = spec.values[index].max(0.0);
        let bar_h = (chart.h * (value / max)).clamp(3.0, chart.h);
        let x = chart.x + index as f32 * (bar_w + gap);
        let y = chart.y + chart.h - bar_h;
        ui.fill_rect(
            UiRect::new(x, y, bar_w, bar_h),
            3.0,
            resolve_component_accent(ui, spec.accent),
        );
        ui.bounded_label(
            x,
            chart.y + chart.h + 11.0,
            bar_w + gap,
            spec.labels[index],
            2.0,
            colors.muted,
        );
    }
}

pub fn transaction_row(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: TransactionRow<'_>) {
    let colors = ui.theme().colors;
    ui.hit(
        HitKind::TransactionRow,
        spec.id,
        rect.x,
        rect.y,
        rect.w,
        rect.h,
    );
    ui.fill_rect(rect, 0.0, colors.panel);
    ui.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);
    let icon = UiRect::new(rect.x + 12.0, rect.y + 12.0, 34.0, 34.0);
    let amount_color = if spec.positive {
        colors.success
    } else {
        colors.text
    };
    ui.fill_rect(icon, 4.0, colors.row);
    ui.border_rect(icon, 4.0, colors.border.with_alpha(0.68));
    ui.bounded_label(
        rect.x + 58.0,
        rect.y + 12.0,
        rect.w * 0.36,
        spec.title,
        2.0,
        colors.text,
    );
    ui.bounded_label(
        rect.x + 58.0,
        rect.y + 34.0,
        rect.w * 0.36,
        spec.subtitle,
        2.0,
        colors.muted,
    );
    ui.bounded_label(
        rect.x + rect.w * 0.54,
        rect.y + 22.0,
        rect.w * 0.22,
        spec.date,
        2.0,
        colors.muted,
    );
    ui.bounded_label(
        rect.x + rect.w - 146.0,
        rect.y + 22.0,
        130.0,
        spec.amount,
        2.0,
        amount_color,
    );
}

pub fn menu_item(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: MenuItem<'_>) {
    let theme = ui.theme();
    let colors = theme.colors;
    ui.hit(HitKind::MenuItem, spec.id, rect.x, rect.y, rect.w, rect.h);
    if spec.selected {
        ui.fill_rect(rect, theme.radius.card, colors.active);
        let accent = resolve_component_accent(ui, spec.accent);
        ui.border_rect(rect, theme.radius.card, accent.with_alpha(0.42));
        ui.fill_rect(
            UiRect::new(rect.x, rect.y + 10.0, 3.0, rect.h - 20.0),
            2.0,
            accent,
        );
    }
    ui.bounded_label(
        rect.x + 14.0,
        rect.y + 10.0,
        rect.w - 28.0,
        spec.label,
        2.0,
        if spec.selected {
            colors.text
        } else {
            colors.muted
        },
    );
    if !spec.detail.is_empty() {
        ui.bounded_label(
            rect.x + 14.0,
            rect.y + 31.0,
            rect.w - 28.0,
            spec.detail,
            2.0,
            colors.muted,
        );
    }
    if !spec.badge.is_empty() {
        let badge_w = (spec.badge.chars().count() as f32 * 9.0 + 18.0).clamp(34.0, 92.0);
        ui.badge(
            rect.x + rect.w - badge_w - 10.0,
            rect.y + 11.0,
            spec.badge,
            resolve_component_accent(ui, spec.accent),
        );
    }
}

pub fn control_row(ui: &mut UiPainter<'_, '_>, rect: UiRect, spec: ControlRow<'_>) {
    let colors = ui.theme().colors;
    if let Some(id) = spec.id {
        ui.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
    }
    ui.fill_rect(rect, 0.0, colors.panel);
    ui.divider(rect.x, rect.y + rect.h - 1.0, rect.w, Axis::Horizontal);

    let accessory_w = match spec.accessory {
        ControlAccessory::None => 0.0,
        ControlAccessory::Value(value) => {
            (value.chars().count() as f32 * 8.0 + 8.0).clamp(48.0, 150.0)
        }
        ControlAccessory::Badge(label, _) => {
            (label.chars().count() as f32 * 9.0 + 18.0).clamp(34.0, 96.0)
        }
        ControlAccessory::Toggle { .. } => 54.0,
        ControlAccessory::Button { label, .. } => {
            (label.chars().count() as f32 * 9.0 + 26.0).clamp(72.0, 136.0)
        }
    };
    let text_w = (rect.w - accessory_w - 38.0).max(0.0);
    ui.bounded_label(
        rect.x + 14.0,
        rect.y + 10.0,
        text_w,
        spec.label,
        2.0,
        colors.text,
    );
    if !spec.detail.is_empty() {
        ui.bounded_label(
            rect.x + 14.0,
            rect.y + 31.0,
            text_w,
            spec.detail,
            2.0,
            colors.muted,
        );
    }

    let right = rect.x + rect.w - 14.0;
    match spec.accessory {
        ControlAccessory::None => {}
        ControlAccessory::Value(value) => ui.bounded_label(
            right - accessory_w,
            rect.y + 21.0,
            accessory_w,
            value,
            2.0,
            colors.muted,
        ),
        ControlAccessory::Badge(label, color) => {
            ui.badge(right - accessory_w, rect.y + 18.0, label, color);
        }
        ControlAccessory::Toggle { on, id } => {
            ui.toggle(right - 46.0, rect.y + (rect.h - 24.0) * 0.5, on, id);
        }
        ControlAccessory::Button { label, id, style } => {
            ui.button(
                UiRect::new(right - accessory_w, rect.y + 13.0, accessory_w, 32.0),
                label,
                style,
                id,
                true,
            );
        }
    }
}

fn action_reserved_width(rect: UiRect, action: Option<(&str, u32)>) -> f32 {
    if action.is_some() {
        (rect.w - 156.0).max(0.0)
    } else {
        rect.w
    }
}

fn resolve_component_accent(ui: &UiPainter<'_, '_>, accent: Color4) -> Color4 {
    if accent == palette::ACCENT {
        ui.theme().colors.accent
    } else {
        accent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::{GpuScene, palette};

    #[test]
    fn stack_lays_out_vertical_children() {
        let mut stack = UiStack::vertical(UiRect::new(10.0, 20.0, 300.0, 400.0), 8.0);

        assert_eq!(stack.next(40.0), UiRect::new(10.0, 20.0, 300.0, 40.0));
        assert_eq!(stack.next(60.0), UiRect::new(10.0, 68.0, 300.0, 60.0));
        assert_eq!(stack.remaining(), UiRect::new(10.0, 136.0, 300.0, 284.0));
    }

    #[test]
    fn grid_returns_predictable_dashboard_cells() {
        let grid = UiGrid::new(UiRect::new(20.0, 40.0, 1000.0, 800.0), 4, 16.0);

        assert_eq!(
            grid.cell(0, 1, 0.0, 120.0),
            UiRect::new(20.0, 40.0, 238.0, 120.0)
        );
        assert_eq!(
            grid.cell(1, 2, 140.0, 240.0),
            UiRect::new(274.0, 180.0, 492.0, 240.0)
        );
    }

    #[test]
    fn dashboard_components_render_into_gpu_scene() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            metric_card(
                &mut ui,
                UiRect::new(16.0, 16.0, 240.0, 132.0),
                MetricCard::new("Balance", "$0.00")
                    .detail("pending setup")
                    .progress(0.42),
            );
            field(
                &mut ui,
                UiRect::new(280.0, 16.0, 260.0, 92.0),
                Field::new("Endpoint")
                    .value("nodes.edgerun.tech")
                    .focused(true)
                    .id(6),
            );
            slider(
                &mut ui,
                UiRect::new(280.0, 126.0, 260.0, 78.0),
                Slider::new("Budget", 0.64, 7).range_labels("$0", "$100"),
            );
            let labels = ["Jan", "Feb", "Mar"];
            let values = [0.4, 0.9, 0.62];
            bar_chart(
                &mut ui,
                UiRect::new(16.0, 170.0, 240.0, 180.0),
                BarChart::new("Activity", &labels, &values),
            );
            transaction_row(
                &mut ui,
                UiRect::new(280.0, 230.0, 320.0, 58.0),
                TransactionRow::new("Relay receipt", "+$4.20", 8)
                    .subtitle("Income")
                    .date("Today")
                    .positive(true),
            );
            menu_item(
                &mut ui,
                UiRect::new(620.0, 16.0, 220.0, 58.0),
                MenuItem::new("Payments", 9)
                    .detail("proof-backed receipts")
                    .badge("new")
                    .selected(true),
            );
            control_row(
                &mut ui,
                UiRect::new(620.0, 92.0, 300.0, 58.0),
                ControlRow::new("Admission policy")
                    .detail("personal relay budget")
                    .toggle(true, 10)
                    .id(11),
            );
        }

        assert!(scene.rects().len() > 20);
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Input && hit.id == 6)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Slider && hit.id == 7)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::TransactionRow && hit.id == 8)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::MenuItem && hit.id == 9)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::Toggle && hit.id == 10)
        );
        assert!(
            scene
                .hits()
                .iter()
                .any(|hit| hit.kind == HitKind::ListRow && hit.id == 11)
        );
    }
}
