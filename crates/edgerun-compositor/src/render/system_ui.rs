//! EdgeRun system UI overlay for the compositor CPU scanout path.
//!
//! This deliberately stays above Wayland surface composition and below the
//! cursor. It is the first bridge from the compositor into `edgerun-ui-core`.

use edgerun_ui_core::{Color, Painter, Rect, TextSize, EDGERUN_DARK};

/// Draw the EdgeRun native command-center overlay into the caller-owned scanout
/// buffer.
///
/// The CPU renderer already composites Wayland surfaces into an XRGB8888 dumb
/// buffer. This function paints a compact native UI pass directly into the same
/// buffer before the cursor is drawn. It intentionally does not clear the
/// framebuffer, so normal Wayland clients remain visible underneath.
pub fn draw_system_ui_overlay(pixels: &mut [u8], width: u32, height: u32, pitch: u32) {
    if pixels.is_empty() || width == 0 || height == 0 || pitch < width.saturating_mul(4) {
        return;
    }

    let theme = EDGERUN_DARK;
    let mut painter = Painter {
        pixels,
        width,
        height,
        pitch,
    };

    let margin = 18;
    let panel_w = width.saturating_sub((margin * 2) as u32).min(620);
    let panel_h = 190;
    let panel = Rect::new(margin, margin, panel_w, panel_h);

    painter.rect_alpha(Rect::new(0, 0, width, 84), Color::rgba(0, 0, 0, 72));
    painter.shadow_card(panel, theme);

    painter.text(
        panel.x + 20,
        panel.y + 18,
        "EdgeRun",
        theme.text,
        TextSize::Hero,
    );
    painter.text(
        panel.x + 22,
        panel.y + 66,
        "native compositor UI / no DOM / no CSS / no browser chrome",
        theme.muted,
        TextSize::Body,
    );

    let badge = Rect::new(panel.x + panel.w as i32 - 110, panel.y + 22, 86, 24);
    painter.badge(badge, "native", theme.accent, theme);

    let chip_y = panel.y + 108;
    stat_chip(
        &mut painter,
        Rect::new(panel.x + 22, chip_y, 132, 48),
        "runtime",
        "ready",
        theme.accent,
    );
    stat_chip(
        &mut painter,
        Rect::new(panel.x + 166, chip_y, 132, 48),
        "renderer",
        "cpu",
        Color::rgb(0x8b, 0xe9, 0xfd),
    );
    stat_chip(
        &mut painter,
        Rect::new(panel.x + 310, chip_y, 132, 48),
        "mesh",
        "online",
        Color::rgb(0x50, 0xfa, 0x7b),
    );

    let button_y = panel.y + panel.h as i32 - 48;
    painter.button(
        Rect::new(panel.x + panel.w as i32 - 230, button_y, 98, 32),
        "Trust",
        theme,
        false,
    );
    painter.button(
        Rect::new(panel.x + panel.w as i32 - 120, button_y, 96, 32),
        "Open",
        theme,
        true,
    );
}

fn stat_chip(painter: &mut Painter<'_>, rect: Rect, label: &str, value: &str, accent: Color) {
    let theme = EDGERUN_DARK;
    painter.rect_alpha(rect, theme.panel_2.with_alpha(224));
    painter.border(rect, theme.border);
    painter.rect(Rect::new(rect.x, rect.y, 3, rect.h), accent);
    painter.text(rect.x + 12, rect.y + 8, label, theme.muted, TextSize::Small);
    painter.text(rect.x + 12, rect.y + 24, value, theme.text, TextSize::Body);
}
