//! Higher-quality visual primitives built on top of the tiny pixel painter.
//!
//! These helpers keep the core renderer small while making the system UI feel
//! modern: antialiased rounded rectangles, soft shadows, and polished widgets.

use crate::{Color, Painter, Rect, TextSize, Theme};

const AA_SAMPLES: [(f32, f32); 4] = [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Corners {
    pub radius: u32,
}

impl Corners {
    pub const fn all(radius: u32) -> Self {
        Self { radius }
    }
}

impl<'a> Painter<'a> {
    /// Draw an antialiased rounded rectangle using 2x2 supersampling.
    pub fn rounded_rect(&mut self, rect: Rect, radius: u32, color: Color) {
        self.rounded_rect_inner(rect, radius, color, false, 0);
    }

    /// Draw an antialiased rounded rectangle outline.
    pub fn rounded_border(&mut self, rect: Rect, radius: u32, thickness: u32, color: Color) {
        if rect.w == 0 || rect.h == 0 || thickness == 0 {
            return;
        }
        let outer = rect;
        let inner = rect.inset(thickness as i32);
        let inner_radius = radius.saturating_sub(thickness);

        let x0 = outer.x.max(0) as u32;
        let y0 = outer.y.max(0) as u32;
        let x1 = outer.x.saturating_add(outer.w as i32).max(0) as u32;
        let y1 = outer.y.saturating_add(outer.h as i32).max(0) as u32;
        let x1 = x1.min(self.width);
        let y1 = y1.min(self.height);

        for y in y0..y1 {
            for x in x0..x1 {
                let outer_cov = rounded_coverage(x, y, outer, radius);
                if outer_cov == 0 {
                    continue;
                }
                let inner_cov = if inner.w == 0 || inner.h == 0 {
                    0
                } else {
                    rounded_coverage(x, y, inner, inner_radius)
                };
                let cov = outer_cov.saturating_sub(inner_cov);
                if cov == 0 {
                    continue;
                }
                self.blend_pixel_public(x, y, color.with_alpha(scale_alpha(color.a, cov)));
            }
        }
    }

    /// Draw a soft shadow with multiple rounded translucent passes.
    pub fn soft_shadow(&mut self, rect: Rect, radius: u32, color: Color, spread: u32, offset_x: i32, offset_y: i32) {
        if color.a == 0 || spread == 0 {
            return;
        }

        // Draw far-to-near so closer passes accumulate naturally.
        for i in (1..=spread).rev() {
            let t = i as i32;
            let alpha = ((color.a as u32 * (spread + 1 - i)) / (spread * 3 + 1)).min(120) as u8;
            let r = Rect::new(
                rect.x + offset_x - t,
                rect.y + offset_y - t,
                rect.w.saturating_add((t as u32).saturating_mul(2)),
                rect.h.saturating_add((t as u32).saturating_mul(2)),
            );
            self.rounded_rect(r, radius.saturating_add(i), color.with_alpha(alpha));
        }
    }

    /// Draw a modern card: soft shadow, rounded fill, subtle border and top glow.
    pub fn glass_card(&mut self, rect: Rect, theme: Theme) {
        self.soft_shadow(rect, 18, theme.shadow.with_alpha(130), 18, 0, 10);
        self.rounded_rect(rect, 18, theme.panel.with_alpha(242));
        self.rounded_border(rect, 18, 1, theme.border.with_alpha(220));
        self.rounded_rect(Rect::new(rect.x + 2, rect.y + 2, rect.w.saturating_sub(4), 24), 16, theme.panel_2.with_alpha(72));
    }

    /// Draw a polished pill/button.
    pub fn pill_button(&mut self, rect: Rect, label: &str, theme: Theme, active: bool) {
        let fill = if active { theme.accent } else { theme.panel_2 };
        let fg = if active { theme.accent_text } else { theme.text };
        let radius = (rect.h / 2).max(8);

        if active {
            self.soft_shadow(rect, radius, theme.accent.with_alpha(90), 10, 0, 4);
        }
        self.rounded_rect(rect, radius, fill.with_alpha(if active { 255 } else { 225 }));
        self.rounded_border(rect, radius, 1, if active { theme.accent.with_alpha(255) } else { theme.border.with_alpha(220) });

        let scale = TextSize::Body.scale() as i32;
        let tw = crate::text_width(label, TextSize::Body) as i32;
        let tx = rect.x + ((rect.w as i32 - tw) / 2).max(6);
        let ty = rect.y + ((rect.h as i32 - 8 * scale) / 2).max(2);
        self.text(tx, ty, label, fg, TextSize::Body);
    }

    pub fn pill_badge(&mut self, rect: Rect, label: &str, color: Color, theme: Theme) {
        let radius = (rect.h / 2).max(6);
        self.rounded_rect(rect, radius, color.with_alpha(70));
        self.rounded_border(rect, radius, 1, color.with_alpha(210));
        self.text(rect.x + 9, rect.y + 5, label, theme.text, TextSize::Small);
    }

    pub fn rounded_progress(&mut self, rect: Rect, value_permille: u16, theme: Theme) {
        let radius = (rect.h / 2).max(3);
        self.rounded_rect(rect, radius, theme.panel_2.with_alpha(230));
        self.rounded_border(rect, radius, 1, theme.border.with_alpha(200));
        let inner = rect.inset(2);
        let fill_w = inner.w.saturating_mul(value_permille.min(1000) as u32) / 1000;
        if fill_w > 0 {
            self.rounded_rect(Rect::new(inner.x, inner.y, fill_w, inner.h), radius.saturating_sub(2), theme.accent);
        }
    }

    fn rounded_rect_inner(&mut self, rect: Rect, radius: u32, color: Color, _border_only: bool, _thickness: u32) {
        if rect.w == 0 || rect.h == 0 || color.a == 0 {
            return;
        }

        let x0 = rect.x.max(0) as u32;
        let y0 = rect.y.max(0) as u32;
        let x1 = rect.x.saturating_add(rect.w as i32).max(0) as u32;
        let y1 = rect.y.saturating_add(rect.h as i32).max(0) as u32;
        let x1 = x1.min(self.width);
        let y1 = y1.min(self.height);

        for y in y0..y1 {
            for x in x0..x1 {
                let cov = rounded_coverage(x, y, rect, radius);
                if cov == 0 {
                    continue;
                }
                self.blend_pixel_public(x, y, color.with_alpha(scale_alpha(color.a, cov)));
            }
        }
    }

    fn blend_pixel_public(&mut self, x: u32, y: u32, color: Color) {
        if color.a == 0 || x >= self.width || y >= self.height || self.pitch < self.width.saturating_mul(4) {
            return;
        }
        let off = y as usize * self.pitch as usize + x as usize * 4;
        let Some(px) = self.pixels.get_mut(off..off + 4) else {
            return;
        };

        let a = color.a as u32;
        let inv = 255 - a;
        px[0] = ((color.b as u32 * a + px[0] as u32 * inv) / 255) as u8;
        px[1] = ((color.g as u32 * a + px[1] as u32 * inv) / 255) as u8;
        px[2] = ((color.r as u32 * a + px[2] as u32 * inv) / 255) as u8;
        px[3] = 0xff;
    }
}

pub fn demo_dashboard_polished(p: &mut Painter<'_>, state: crate::DashboardState<'_>, theme: Theme) {
    p.clear(theme.bg);

    // subtle background glows
    p.rounded_rect(Rect::new(-120, -100, 420, 260), 130, theme.accent.with_alpha(20));
    p.rounded_rect(Rect::new(p.width as i32 - 360, p.height as i32 - 260, 460, 320), 150, Color::rgb(0x50, 0xfa, 0x7b).with_alpha(12));

    let margin = 28;
    let hero = Rect::new(margin, margin, p.width.saturating_sub((margin * 2) as u32), 188);
    p.glass_card(hero, theme);
    p.text(hero.x + 26, hero.y + 24, state.title, theme.text, TextSize::Hero);
    p.text(hero.x + 28, hero.y + 76, state.subtitle, theme.muted, TextSize::Body);

    let badge = Rect::new(hero.x + hero.w as i32 - 126, hero.y + 24, 96, 28);
    p.pill_badge(badge, state.node_status, theme.accent, theme);

    p.pill_button(Rect::new(hero.x + 26, hero.y + 128, 170, 40), "Open node", theme, true);
    p.pill_button(Rect::new(hero.x + 210, hero.y + 128, 172, 40), "Trust root", theme, false);

    let stats_y = hero.y + hero.h as i32 + 22;
    let gap = 16;
    let stat_w = hero.w.saturating_sub((gap * 2) as u32) / 3;
    let r1 = Rect::new(margin, stats_y, stat_w, 134);
    let r2 = Rect::new(margin + stat_w as i32 + gap, stats_y, stat_w, 134);
    let r3 = Rect::new(margin + (stat_w as i32 + gap) * 2, stats_y, stat_w, 134);

    p.glass_card(r1, theme);
    p.text(r1.x + 20, r1.y + 20, "CPU", theme.muted, TextSize::Body);
    draw_permille_polished(p, r1.x + 20, r1.y + 58, state.cpu_permille, theme.text);
    p.rounded_progress(Rect::new(r1.x + 20, r1.y + 104, r1.w.saturating_sub(40), 14), state.cpu_permille, theme);

    p.glass_card(r2, theme);
    p.text(r2.x + 20, r2.y + 20, "RAM", theme.muted, TextSize::Body);
    draw_u32_suffix_polished(p, r2.x + 20, r2.y + 58, state.memory_mb, " MB", theme.text);
    p.rounded_progress(Rect::new(r2.x + 20, r2.y + 104, r2.w.saturating_sub(40), 14), 420, theme);

    p.glass_card(r3, theme);
    p.text(r3.x + 20, r3.y + 20, "NET", theme.muted, TextSize::Body);
    p.text(r3.x + 20, r3.y + 62, state.network_status, theme.text, TextSize::Title);
    p.pill_badge(Rect::new(r3.x + 20, r3.y + 100, 92, 24), "private", Color::rgb(0x50, 0xfa, 0x7b), theme);
}

fn rounded_coverage(x: u32, y: u32, rect: Rect, radius: u32) -> u8 {
    if radius == 0 {
        return 255;
    }

    let mut hits = 0u32;
    for (sx, sy) in AA_SAMPLES {
        if point_in_rounded_rect(x as f32 + sx, y as f32 + sy, rect, radius as f32) {
            hits += 1;
        }
    }
    (hits * 255 / AA_SAMPLES.len() as u32) as u8
}

fn point_in_rounded_rect(px: f32, py: f32, rect: Rect, radius: f32) -> bool {
    let x0 = rect.x as f32;
    let y0 = rect.y as f32;
    let x1 = rect.x as f32 + rect.w as f32;
    let y1 = rect.y as f32 + rect.h as f32;
    if px < x0 || py < y0 || px >= x1 || py >= y1 {
        return false;
    }

    let r = radius.min(rect.w as f32 * 0.5).min(rect.h as f32 * 0.5);
    let cx = if px < x0 + r { x0 + r } else if px > x1 - r { x1 - r } else { px };
    let cy = if py < y0 + r { y0 + r } else if py > y1 - r { y1 - r } else { py };
    let dx = px - cx;
    let dy = py - cy;
    dx * dx + dy * dy <= r * r
}

fn scale_alpha(alpha: u8, coverage: u8) -> u8 {
    ((alpha as u32 * coverage as u32) / 255) as u8
}

fn draw_permille_polished(p: &mut Painter<'_>, x: i32, y: i32, value: u16, color: Color) {
    let pct = value.min(1000) as u32 / 10;
    draw_u32_suffix_polished(p, x, y, pct, "%", color);
}

fn draw_u32_suffix_polished(p: &mut Painter<'_>, x: i32, y: i32, mut value: u32, suffix: &str, color: Color) {
    let mut buf = [0u8; 10];
    let mut n = 0usize;
    if value == 0 {
        buf[0] = b'0';
        n = 1;
    } else {
        while value > 0 && n < buf.len() {
            buf[n] = b'0' + (value % 10) as u8;
            value /= 10;
            n += 1;
        }
    }

    let mut cx = x;
    for i in (0..n).rev() {
        let ch = [buf[i]];
        if let Ok(s) = core::str::from_utf8(&ch) {
            p.text(cx, y, s, color, TextSize::Title);
        }
        cx += 6 * TextSize::Title.scale() as i32;
    }
    p.text(cx, y + 6, suffix, color, TextSize::Body);
}
