//! Small reusable EdgeRun UI component kit.
//!
//! This layer is intentionally semantic and tiny. Components describe meaning
//! and visual intent, not CSS.
//!
//! The canonical EdgeRun UI surface is the GPU scene builder in `gpu.rs`.
//! This CPU painter path remains as a compatibility adapter for older previews
//! while primitives migrate to the shared GPU component model.

use crate::{Color, Painter, Rect, TextSize, Theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputState {
    Idle,
    Focused,
    Error,
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListDensity {
    Compact,
    Comfortable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComponentMetrics {
    pub gap: i32,
    pub pad: i32,
    pub radius_card: u32,
    pub radius_control: u32,
    pub topbar_h: u32,
    pub control_h: u32,
}

impl Default for ComponentMetrics {
    fn default() -> Self {
        Self {
            gap: 14,
            pad: 18,
            radius_card: 18,
            radius_control: 11,
            topbar_h: 58,
            control_h: 40,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Card<'a> {
    pub rect: Rect,
    pub title: Option<&'a str>,
    pub subtitle: Option<&'a str>,
    pub elevated: bool,
}

impl<'a> Card<'a> {
    pub const fn new(rect: Rect) -> Self {
        Self {
            rect,
            title: None,
            subtitle: None,
            elevated: true,
        }
    }

    pub const fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub const fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    pub const fn flat(mut self) -> Self {
        self.elevated = false;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Button<'a> {
    pub rect: Rect,
    pub label: &'a str,
    pub kind: ButtonKind,
    pub pressed: bool,
    pub disabled: bool,
}

impl<'a> Button<'a> {
    pub const fn new(rect: Rect, label: &'a str) -> Self {
        Self {
            rect,
            label,
            kind: ButtonKind::Secondary,
            pressed: false,
            disabled: false,
        }
    }

    pub const fn primary(mut self) -> Self {
        self.kind = ButtonKind::Primary;
        self
    }

    pub const fn ghost(mut self) -> Self {
        self.kind = ButtonKind::Ghost;
        self
    }

    pub const fn danger(mut self) -> Self {
        self.kind = ButtonKind::Danger;
        self
    }

    pub const fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input<'a> {
    pub rect: Rect,
    pub label: Option<&'a str>,
    pub value: &'a str,
    pub placeholder: &'a str,
    pub state: InputState,
}

impl<'a> Input<'a> {
    pub const fn new(rect: Rect) -> Self {
        Self {
            rect,
            label: None,
            value: "",
            placeholder: "",
            state: InputState::Idle,
        }
    }

    pub const fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub const fn value(mut self, value: &'a str) -> Self {
        self.value = value;
        self
    }

    pub const fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub const fn state(mut self, state: InputState) -> Self {
        self.state = state;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListItem<'a> {
    pub label: &'a str,
    pub detail: Option<&'a str>,
    pub badge: Option<&'a str>,
    pub selected: bool,
}

impl<'a> ListItem<'a> {
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            detail: None,
            badge: None,
            selected: false,
        }
    }

    pub const fn detail(mut self, detail: &'a str) -> Self {
        self.detail = Some(detail);
        self
    }

    pub const fn badge(mut self, badge: &'a str) -> Self {
        self.badge = Some(badge);
        self
    }

    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct List<'a> {
    pub rect: Rect,
    pub items: &'a [ListItem<'a>],
    pub density: ListDensity,
}

impl<'a> List<'a> {
    pub const fn new(rect: Rect, items: &'a [ListItem<'a>]) -> Self {
        Self {
            rect,
            items,
            density: ListDensity::Comfortable,
        }
    }

    pub const fn compact(mut self) -> Self {
        self.density = ListDensity::Compact;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TopBar<'a> {
    pub rect: Rect,
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub left: Option<&'a str>,
    pub right: Option<&'a str>,
}

impl<'a> TopBar<'a> {
    pub const fn new(rect: Rect, title: &'a str) -> Self {
        Self {
            rect,
            title,
            subtitle: None,
            left: None,
            right: None,
        }
    }

    pub const fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    pub const fn left(mut self, left: &'a str) -> Self {
        self.left = Some(left);
        self
    }

    pub const fn right(mut self, right: &'a str) -> Self {
        self.right = Some(right);
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ComponentKit {
    pub theme: Theme,
    pub metrics: ComponentMetrics,
}

impl ComponentKit {
    pub const fn new(theme: Theme) -> Self {
        Self {
            theme,
            metrics: ComponentMetrics {
                gap: 14,
                pad: 18,
                radius_card: 18,
                radius_control: 11,
                topbar_h: 58,
                control_h: 40,
            },
        }
    }

    pub const fn with_metrics(mut self, metrics: ComponentMetrics) -> Self {
        self.metrics = metrics;
        self
    }

    pub fn draw_card(&self, p: &mut Painter<'_>, card: Card<'_>) {
        if card.elevated {
            p.soft_shadow(
                card.rect,
                self.metrics.radius_card,
                self.theme.shadow.with_alpha(42),
                7,
                0,
                3,
            );
        }
        p.rounded_rect(
            card.rect,
            self.metrics.radius_card,
            self.theme.panel.with_alpha(242),
        );
        p.rounded_border(
            card.rect,
            self.metrics.radius_card,
            1,
            self.theme.border.with_alpha(190),
        );
        p.rounded_rect(
            Rect::new(
                card.rect.x + 2,
                card.rect.y + 2,
                card.rect.w.saturating_sub(4),
                22,
            ),
            self.metrics.radius_card.saturating_sub(2),
            self.theme.panel_2.with_alpha(46),
        );

        let mut y = card.rect.y + self.metrics.pad;
        if let Some(title) = card.title {
            p.text(
                card.rect.x + self.metrics.pad,
                y,
                title,
                self.theme.text,
                TextSize::Title,
            );
            y += TextSize::Title.line_height() as i32 + 2;
        }
        if let Some(subtitle) = card.subtitle {
            p.text(
                card.rect.x + self.metrics.pad,
                y,
                subtitle,
                self.theme.muted,
                TextSize::Body,
            );
        }
    }

    pub fn draw_button(&self, p: &mut Painter<'_>, button: Button<'_>) {
        let (fill, border, fg) = match button.kind {
            ButtonKind::Primary => (self.theme.accent, self.theme.accent, self.theme.accent_text),
            ButtonKind::Secondary => (self.theme.panel_2, self.theme.border, self.theme.text),
            ButtonKind::Ghost => (
                Color::rgba(0, 0, 0, 0),
                self.theme.border.with_alpha(120),
                self.theme.text,
            ),
            ButtonKind::Danger => (self.theme.danger, self.theme.danger, self.theme.text),
        };

        let alpha = if button.disabled {
            92
        } else if button.pressed {
            220
        } else {
            255
        };
        let fill = fill.with_alpha(alpha);
        let fg = fg.with_alpha(if button.disabled { 130 } else { 255 });
        let radius = self.metrics.radius_control;

        if matches!(button.kind, ButtonKind::Primary) && !button.disabled {
            p.soft_shadow(
                button.rect,
                radius,
                self.theme.accent.with_alpha(28),
                4,
                0,
                2,
            );
        }

        if button.kind != ButtonKind::Ghost {
            p.rounded_rect(button.rect, radius, fill);
        }
        p.rounded_border(
            button.rect,
            radius,
            1,
            border.with_alpha(if button.disabled { 90 } else { 210 }),
        );

        let tw = crate::text_width(button.label, TextSize::Body) as i32;
        let scale = TextSize::Body.scale() as i32;
        let tx = button.rect.x + ((button.rect.w as i32 - tw) / 2).max(6);
        let ty = button.rect.y + ((button.rect.h as i32 - 8 * scale) / 2).max(2);
        p.text(tx, ty, button.label, fg, TextSize::Body);
    }

    pub fn draw_input(&self, p: &mut Painter<'_>, input: Input<'_>) {
        let label_h = if input.label.is_some() {
            TextSize::Small.line_height() as i32 + 5
        } else {
            0
        };
        if let Some(label) = input.label {
            p.text(
                input.rect.x,
                input.rect.y,
                label,
                self.theme.muted,
                TextSize::Small,
            );
        }

        let field = Rect::new(
            input.rect.x,
            input.rect.y + label_h,
            input.rect.w,
            input.rect.h.saturating_sub(label_h.max(0) as u32),
        );

        let border = match input.state {
            InputState::Idle => self.theme.border,
            InputState::Focused => self.theme.accent,
            InputState::Error => self.theme.danger,
            InputState::Disabled => self.theme.border.with_alpha(90),
        };
        let text = if input.value.is_empty() {
            input.placeholder
        } else {
            input.value
        };
        let text_color = if input.value.is_empty() || matches!(input.state, InputState::Disabled) {
            self.theme.muted
        } else {
            self.theme.text
        };

        p.rounded_rect(
            field,
            self.metrics.radius_control,
            self.theme
                .panel_2
                .with_alpha(if matches!(input.state, InputState::Disabled) {
                    92
                } else {
                    210
                }),
        );
        p.rounded_border(
            field,
            self.metrics.radius_control,
            if matches!(input.state, InputState::Focused) {
                2
            } else {
                1
            },
            border.with_alpha(220),
        );
        p.text(
            field.x + 13,
            field.y + ((field.h as i32 - TextSize::Body.line_height() as i32) / 2).max(6),
            text,
            text_color,
            TextSize::Body,
        );
    }

    pub fn draw_list(&self, p: &mut Painter<'_>, list: List<'_>) {
        let row_h = match list.density {
            ListDensity::Compact => 42,
            ListDensity::Comfortable => 58,
        };
        p.rounded_rect(
            list.rect,
            self.metrics.radius_card,
            self.theme.panel.with_alpha(180),
        );
        p.rounded_border(
            list.rect,
            self.metrics.radius_card,
            1,
            self.theme.border.with_alpha(150),
        );

        let mut y = list.rect.y + 8;
        for (idx, item) in list.items.iter().enumerate() {
            if y + row_h > list.rect.y + list.rect.h as i32 {
                break;
            }
            let row = Rect::new(
                list.rect.x + 8,
                y,
                list.rect.w.saturating_sub(16),
                row_h as u32,
            );
            if item.selected {
                p.rounded_rect(
                    row,
                    self.metrics.radius_control,
                    self.theme.accent.with_alpha(42),
                );
                p.rounded_border(
                    row,
                    self.metrics.radius_control,
                    1,
                    self.theme.accent.with_alpha(130),
                );
            } else if idx > 0 {
                p.rect_alpha(
                    Rect::new(list.rect.x + 18, y - 1, list.rect.w.saturating_sub(36), 1),
                    self.theme.border.with_alpha(80),
                );
            }

            p.text(
                row.x + 12,
                row.y + 10,
                item.label,
                self.theme.text,
                TextSize::Body,
            );
            if let Some(detail) = item.detail {
                p.text(
                    row.x + 12,
                    row.y + 30,
                    detail,
                    self.theme.muted,
                    TextSize::Small,
                );
            }
            if let Some(badge) = item.badge {
                let badge_w = crate::text_width(badge, TextSize::Small).saturating_add(18);
                let b = Rect::new(
                    row.x + row.w as i32 - badge_w as i32 - 10,
                    row.y + 12,
                    badge_w,
                    22,
                );
                p.rounded_rect(b, 11, self.theme.panel_2.with_alpha(210));
                p.rounded_border(b, 11, 1, self.theme.border.with_alpha(150));
                p.text(b.x + 9, b.y + 6, badge, self.theme.muted, TextSize::Small);
            }
            y += row_h;
        }
    }

    pub fn draw_topbar(&self, p: &mut Painter<'_>, bar: TopBar<'_>) {
        p.rect_alpha(bar.rect, self.theme.bg.with_alpha(235));
        p.rect_alpha(
            Rect::new(
                bar.rect.x,
                bar.rect.y + bar.rect.h as i32 - 1,
                bar.rect.w,
                1,
            ),
            self.theme.border.with_alpha(150),
        );

        let mut title_x = bar.rect.x + self.metrics.pad;
        if let Some(left) = bar.left {
            let left_w = crate::text_width(left, TextSize::Body).saturating_add(22);
            let b = Rect::new(
                bar.rect.x + 12,
                bar.rect.y + 10,
                left_w,
                self.metrics.control_h,
            );
            self.draw_button(p, Button::new(b, left).ghost());
            title_x = b.x + b.w as i32 + self.metrics.gap;
        }

        p.text(
            title_x,
            bar.rect.y + 11,
            bar.title,
            self.theme.text,
            TextSize::Body,
        );
        if let Some(subtitle) = bar.subtitle {
            p.text(
                title_x,
                bar.rect.y + 32,
                subtitle,
                self.theme.muted,
                TextSize::Small,
            );
        }

        if let Some(right) = bar.right {
            let right_w = crate::text_width(right, TextSize::Body).saturating_add(26);
            let b = Rect::new(
                bar.rect.x + bar.rect.w as i32 - right_w as i32 - 12,
                bar.rect.y + 10,
                right_w,
                self.metrics.control_h,
            );
            self.draw_button(p, Button::new(b, right).primary());
        }
    }
}

pub fn demo_component_kit(p: &mut Painter<'_>, theme: Theme) {
    let kit = ComponentKit::new(theme);
    p.clear(theme.bg);

    kit.draw_topbar(
        p,
        TopBar::new(Rect::new(0, 0, p.width, kit.metrics.topbar_h), "EdgeRun")
            .subtitle("component kit preview")
            .left("Back")
            .right("Connect"),
    );

    let y = kit.metrics.topbar_h as i32 + 24;
    kit.draw_card(
        p,
        Card::new(Rect::new(24, y, 420, 160))
            .title("Local runtime")
            .subtitle("Card, Button, Input, List and TopBar are now first-class UI primitives."),
    );

    kit.draw_button(
        p,
        Button::new(Rect::new(48, y + 100, 128, 40), "Open").primary(),
    );
    kit.draw_button(p, Button::new(Rect::new(190, y + 100, 128, 40), "Trust"));

    kit.draw_input(
        p,
        Input::new(Rect::new(470, y, 360, 72))
            .label("Node endpoint")
            .placeholder("admission.edgerun.tech")
            .state(InputState::Focused),
    );

    let items = [
        ListItem::new("Storage")
            .detail("content-addressed local blobs")
            .badge("ready")
            .selected(true),
        ListItem::new("Identity")
            .detail("local keys and trust root")
            .badge("sealed"),
        ListItem::new("Network")
            .detail("mesh relay / direct peer route")
            .badge("online"),
    ];
    kit.draw_list(p, List::new(Rect::new(470, y + 94, 360, 190), &items));
}
