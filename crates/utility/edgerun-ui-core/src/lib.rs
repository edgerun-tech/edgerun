#![cfg_attr(not(feature = "std"), no_std)]

//! Tiny software UI primitives for EdgeRun.
//!
//! This is not a browser, DOM, CSS engine, GTK/Qt wrapper, or Wayland toolkit.
//! It draws a small system UI kit into a caller-owned XRGB8888 / ARGB8888
//! pixel buffer so compositor, terminal, framebuffer, browser canvas, and
//! remote renderers can share one visual language.

pub mod components;
pub mod icons;
pub mod visual;
#[cfg(feature = "fontdue-text")]
pub mod font;
#[cfg(feature = "fontdue-text")]
pub mod tabler_font_generated;
#[cfg(feature = "tabler-icons")]
pub mod tabler_generated;
#[cfg(feature = "tabler-icons")]
pub mod tabler;
#[cfg(feature = "tabler-svg-atlas")]
pub mod tabler_svg_atlas_generated;


#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Color {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { b, g, r, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { b, g, r, a }
    }

    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    pub const fn inset(self, amount: i32) -> Self {
        let shrink = if amount <= 0 { 0 } else { (amount as u32).saturating_mul(2) };
        Self {
            x: self.x + amount,
            y: self.y + amount,
            w: self.w.saturating_sub(shrink),
            h: self.h.saturating_sub(shrink),
        }
    }

    pub fn contains(self, x: i32, y: i32) -> bool {
        let right = self.x.saturating_add(self.w as i32);
        let bottom = self.y.saturating_add(self.h as i32);
        x >= self.x && y >= self.y && x < right && y < bottom
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub bg: Color,
    pub panel: Color,
    pub panel_2: Color,
    pub text: Color,
    pub muted: Color,
    pub border: Color,
    pub accent: Color,
    pub accent_text: Color,
    pub danger: Color,
    pub shadow: Color,
}

pub const EDGERUN_DARK: Theme = Theme {
    bg: Color::rgb(0x02, 0x06, 0x17),          // slate-950
    panel: Color::rgb(0x0f, 0x17, 0x2a),       // slate-900
    panel_2: Color::rgb(0x1e, 0x29, 0x3b),     // slate-800
    text: Color::rgb(0xf8, 0xfa, 0xfc),        // slate-50
    muted: Color::rgb(0x94, 0xa3, 0xb8),       // slate-400
    border: Color::rgb(0x33, 0x41, 0x55),      // slate-700
    accent: Color::rgb(0x0e, 0x9f, 0xd1),      // cyan/sky
    accent_text: Color::rgb(0xf0, 0xf9, 0xff), // sky-50
    danger: Color::rgb(0xe1, 0x1d, 0x48),      // rose-600
    shadow: Color::rgba(0x00, 0x00, 0x00, 34),
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextSize {
    Small,
    Body,
    Title,
    Hero,
}

impl TextSize {
    pub const fn scale(self) -> u8 {
        match self {
            Self::Small => 1,
            Self::Body => 2,
            Self::Title => 3,
            Self::Hero => 4,
        }
    }

    pub const fn line_height(self) -> u32 {
        (self.scale() as u32 * 8) + (self.scale() as u32 * 3)
    }
}

pub struct Painter<'a> {
    pub pixels: &'a mut [u8],
    pub width: u32,
    pub height: u32,
    /// Bytes per row.
    pub pitch: u32,
}

impl<'a> Painter<'a> {
    pub fn clear(&mut self, color: Color) {
        let row_bytes = self.width as usize * 4;
        let pitch = self.pitch as usize;
        if row_bytes == 0 || pitch < row_bytes {
            return;
        }

        for y in 0..self.height as usize {
            let row = y * pitch;
            let Some(dst) = self.pixels.get_mut(row..row.saturating_add(row_bytes)) else {
                break;
            };
            for px in dst.chunks_exact_mut(4) {
                px[0] = color.b;
                px[1] = color.g;
                px[2] = color.r;
                px[3] = 0xff;
            }
        }
    }

    pub fn rect(&mut self, rect: Rect, color: Color) {
        let Some((x0, y0, x1, y1)) = self.clip_rect(rect) else {
            return;
        };
        for y in y0..y1 {
            for x in x0..x1 {
                self.put_pixel(x, y, color);
            }
        }
    }

    pub fn rect_alpha(&mut self, rect: Rect, color: Color) {
        if color.a == 0 {
            return;
        }
        if color.a == 255 {
            self.rect(rect, color);
            return;
        }
        let Some((x0, y0, x1, y1)) = self.clip_rect(rect) else {
            return;
        };
        for y in y0..y1 {
            for x in x0..x1 {
                self.blend_pixel(x, y, color);
            }
        }
    }

    pub fn border(&mut self, rect: Rect, color: Color) {
        if rect.w == 0 || rect.h == 0 {
            return;
        }
        self.rect(Rect::new(rect.x, rect.y, rect.w, 1), color);
        self.rect(Rect::new(rect.x, rect.y + rect.h as i32 - 1, rect.w, 1), color);
        self.rect(Rect::new(rect.x, rect.y, 1, rect.h), color);
        self.rect(Rect::new(rect.x + rect.w as i32 - 1, rect.y, 1, rect.h), color);
    }

    pub fn shadow_card(&mut self, rect: Rect, theme: Theme) {
        self.rect_alpha(Rect::new(rect.x + 5, rect.y + 6, rect.w, rect.h), theme.shadow);
        self.rect(rect, theme.panel);
        self.border(rect, theme.border);
        self.rect(Rect::new(rect.x + 1, rect.y + 1, rect.w.saturating_sub(2), 1), theme.panel_2);
    }

    pub fn button(&mut self, rect: Rect, label: &str, theme: Theme, active: bool) {
        let fill = if active { theme.accent } else { theme.panel_2 };
        let fg = if active { theme.accent_text } else { theme.text };
        self.rect(rect, fill);
        self.border(rect, if active { theme.accent } else { theme.border });
        let scale = TextSize::Body.scale() as i32;
        let tw = text_width(label, TextSize::Body) as i32;
        let tx = rect.x + ((rect.w as i32 - tw) / 2).max(6);
        let ty = rect.y + ((rect.h as i32 - 8 * scale) / 2).max(2);
        self.text(tx, ty, label, fg, TextSize::Body);
    }

    pub fn badge(&mut self, rect: Rect, label: &str, color: Color, theme: Theme) {
        self.rect_alpha(rect, color.with_alpha(80));
        self.border(rect, color);
        self.text(rect.x + 7, rect.y + 5, label, theme.text, TextSize::Small);
    }

    pub fn progress(&mut self, rect: Rect, value_permille: u16, theme: Theme) {
        self.rect(rect, theme.panel_2);
        self.border(rect, theme.border);
        let inner = rect.inset(2);
        let fill_w = inner.w.saturating_mul(value_permille.min(1000) as u32) / 1000;
        self.rect(Rect::new(inner.x, inner.y, fill_w, inner.h), theme.accent);
    }

    pub fn text(&mut self, mut x: i32, y: i32, text: &str, color: Color, size: TextSize) {
        let scale = size.scale() as i32;
        let start_x = x;
        let mut cy = y;
        for byte in text.bytes() {
            match byte {
                b'\n' => {
                    x = start_x;
                    cy += size.line_height() as i32;
                }
                b'\r' => {}
                b'\t' => x += 4 * 6 * scale,
                _ => {
                    self.glyph5x7(x, cy, byte, color, scale);
                    x += 6 * scale;
                }
            }
        }
    }

    fn clip_rect(&self, rect: Rect) -> Option<(u32, u32, u32, u32)> {
        let x0 = rect.x.max(0) as u32;
        let y0 = rect.y.max(0) as u32;
        let x1 = rect.x.saturating_add(rect.w as i32).max(0) as u32;
        let y1 = rect.y.saturating_add(rect.h as i32).max(0) as u32;
        let x1 = x1.min(self.width);
        let y1 = y1.min(self.height);
        if x0 >= x1 || y0 >= y1 { None } else { Some((x0, y0, x1, y1)) }
    }

    fn put_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x >= self.width || y >= self.height || self.pitch < self.width.saturating_mul(4) {
            return;
        }
        let off = y as usize * self.pitch as usize + x as usize * 4;
        let Some(px) = self.pixels.get_mut(off..off + 4) else { return; };
        px[0] = color.b;
        px[1] = color.g;
        px[2] = color.r;
        px[3] = 0xff;
    }

    fn blend_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x >= self.width || y >= self.height || self.pitch < self.width.saturating_mul(4) {
            return;
        }
        let off = y as usize * self.pitch as usize + x as usize * 4;
        let Some(px) = self.pixels.get_mut(off..off + 4) else { return; };
        let a = color.a as u32;
        let inv = 255 - a;
        px[0] = ((color.b as u32 * a + px[0] as u32 * inv) / 255) as u8;
        px[1] = ((color.g as u32 * a + px[1] as u32 * inv) / 255) as u8;
        px[2] = ((color.r as u32 * a + px[2] as u32 * inv) / 255) as u8;
        px[3] = 0xff;
    }

    fn glyph5x7(&mut self, x: i32, y: i32, byte: u8, color: Color, scale: i32) {
        let glyph = glyph5x7(byte);
        for (row, bits) in glyph.iter().copied().enumerate() {
            for col in 0..5 {
                if ((bits >> (4 - col)) & 1) == 0 { continue; }
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = x + col as i32 * scale + sx;
                        let py = y + row as i32 * scale + sy;
                        if px >= 0 && py >= 0 {
                            self.put_pixel(px as u32, py as u32, color);
                        }
                    }
                }
            }
        }
    }
}

pub fn text_width(text: &str, size: TextSize) -> u32 {
    text.bytes().filter(|b| *b != b'\n' && *b != b'\r').count() as u32 * 6 * size.scale() as u32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DashboardState<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub node_status: &'a str,
    pub cpu_permille: u16,
    pub memory_mb: u32,
    pub network_status: &'a str,
}

impl<'a> Default for DashboardState<'a> {
    fn default() -> Self {
        Self {
            title: "EdgeRun",
            subtitle: "local first runtime / compositor UI",
            node_status: "ready",
            cpu_permille: 170,
            memory_mb: 42,
            network_status: "online",
        }
    }
}

pub fn demo_dashboard(p: &mut Painter<'_>, state: DashboardState<'_>, theme: Theme) {
    p.clear(theme.bg);
    let margin = 24;
    let hero = Rect::new(margin, margin, p.width.saturating_sub((margin * 2) as u32), 178);
    p.shadow_card(hero, theme);
    p.text(hero.x + 22, hero.y + 22, state.title, theme.text, TextSize::Hero);
    p.text(hero.x + 24, hero.y + 70, state.subtitle, theme.muted, TextSize::Body);
    let badge = Rect::new(hero.x + hero.w as i32 - 116, hero.y + 22, 88, 24);
    p.badge(badge, state.node_status, theme.accent, theme);
    p.button(Rect::new(hero.x + 24, hero.y + 120, 168, 38), "Open node", theme, true);
    p.button(Rect::new(hero.x + 204, hero.y + 120, 168, 38), "Trust root", theme, false);

    let stats_y = hero.y + hero.h as i32 + 18;
    let gap = 14;
    let stat_w = hero.w.saturating_sub((gap * 2) as u32) / 3;
    let r1 = Rect::new(margin, stats_y, stat_w, 126);
    let r2 = Rect::new(margin + stat_w as i32 + gap, stats_y, stat_w, 126);
    let r3 = Rect::new(margin + (stat_w as i32 + gap) * 2, stats_y, stat_w, 126);

    p.shadow_card(r1, theme);
    p.text(r1.x + 18, r1.y + 18, "CPU", theme.muted, TextSize::Body);
    draw_permille(p, r1.x + 18, r1.y + 52, state.cpu_permille, theme.text);
    p.progress(Rect::new(r1.x + 18, r1.y + 96, r1.w.saturating_sub(36), 12), state.cpu_permille, theme);

    p.shadow_card(r2, theme);
    p.text(r2.x + 18, r2.y + 18, "RAM", theme.muted, TextSize::Body);
    draw_u32_suffix(p, r2.x + 18, r2.y + 52, state.memory_mb, " MB", theme.text);

    p.shadow_card(r3, theme);
    p.text(r3.x + 18, r3.y + 18, "NET", theme.muted, TextSize::Body);
    p.text(r3.x + 18, r3.y + 56, state.network_status, theme.text, TextSize::Title);
}

fn draw_permille(p: &mut Painter<'_>, x: i32, y: i32, value: u16, color: Color) {
    let pct = value.min(1000) as u32 / 10;
    draw_u32_suffix(p, x, y, pct, "%", color);
}

fn draw_u32_suffix(p: &mut Painter<'_>, x: i32, y: i32, mut value: u32, suffix: &str, color: Color) {
    let mut buf = [0u8; 10];
    let mut n = 0usize;
    if value == 0 { buf[0] = b'0'; n = 1; } else {
        while value > 0 && n < buf.len() { buf[n] = b'0' + (value % 10) as u8; value /= 10; n += 1; }
    }
    let mut cx = x;
    for i in (0..n).rev() {
        let ch = [buf[i]];
        if let Ok(s) = core::str::from_utf8(&ch) { p.text(cx, y, s, color, TextSize::Title); }
        cx += 6 * TextSize::Title.scale() as i32;
    }
    p.text(cx, y + 6, suffix, color, TextSize::Body);
}

fn glyph5x7(byte: u8) -> [u8; 7] {
    let c = if byte.is_ascii_lowercase() { byte - 32 } else { byte };
    match c {
        b' ' => [0, 0, 0, 0, 0, 0, 0],
        b'!' => [0b00100, 0b00100, 0b00100, 0b00100, 0, 0b00100, 0],
        b'%' => [0b11001, 0b11010, 0b00100, 0b01000, 0b10110, 0b00110, 0],
        b'-' => [0, 0, 0, 0b11111, 0, 0, 0],
        b'.' => [0, 0, 0, 0, 0, 0b00100, 0],
        b'/' => [0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0, 0],
        b'0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        b'1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        b'2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        b'3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        b'4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        b'5' => [0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110],
        b'6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        b'7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        b'8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        b'9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        b':' => [0, 0b00100, 0, 0, 0b00100, 0, 0],
        b'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        b'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        b'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        b'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        b'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        b'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        b'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        b'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        b'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        b'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100],
        b'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        b'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        b'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        b'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        b'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        b'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        b'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        b'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        b'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        b'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        b'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        b'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        b'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
        b'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        b'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        b'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        b'_' => [0, 0, 0, 0, 0, 0, 0b11111],
        _ => [0b11111, 0b10001, 0b00101, 0b01001, 0b10001, 0b10001, 0b11111],
    }
}
