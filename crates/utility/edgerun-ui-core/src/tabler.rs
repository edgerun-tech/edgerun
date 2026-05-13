//! Tiny runtime renderer for generated Tabler icon path data.
//!
//! This module is behind `tabler-icons` and expects `tabler_generated.rs` to be
//! generated locally by `tools/import_tabler_icons.py`. It intentionally handles
//! only the small SVG path command subset used by Tabler outline icons and draws
//! them as stroked polylines into the existing CPU buffer.

#![cfg(feature = "tabler-icons")]

use crate::{Color, Painter, Rect, tabler_generated::TABLER_ICONS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TablerIconName {
    Activity,
    BrandTabler,
    Cpu,
    Server,
    ShieldCheck,
    Network,
    Database,
    Terminal2,
    Wallet,
    Key,
    Lock,
    Sparkles,
    Check,
    AlertTriangle,
}

impl TablerIconName {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Activity => "activity",
            Self::BrandTabler => "brand-tabler",
            Self::Cpu => "cpu",
            Self::Server => "server",
            Self::ShieldCheck => "shield-check",
            Self::Network => "network",
            Self::Database => "database",
            Self::Terminal2 => "terminal-2",
            Self::Wallet => "wallet",
            Self::Key => "key",
            Self::Lock => "lock",
            Self::Sparkles => "sparkles",
            Self::Check => "check",
            Self::AlertTriangle => "alert-triangle",
        }
    }
}

impl<'a> Painter<'a> {
    pub fn tabler_icon(&mut self, name: TablerIconName, rect: Rect, color: Color) {
        let Some(icon) = TABLER_ICONS.iter().find(|icon| icon.name == name.as_str()) else {
            return;
        };

        let scale = rect.w.min(rect.h) as f32 / 24.0;
        let ox = rect.x as f32 + (rect.w as f32 - 24.0 * scale) * 0.5;
        let oy = rect.y as f32 + (rect.h as f32 - 24.0 * scale) * 0.5;
        let stroke = round_f32(scale * 2.0).max(1);

        for path in icon.paths {
            draw_svg_path(self, path, ox, oy, scale, stroke, color);
        }
    }
}

fn draw_svg_path(
    p: &mut Painter<'_>,
    path: &str,
    ox: f32,
    oy: f32,
    scale: f32,
    stroke: i32,
    color: Color,
) {
    let bytes = path.as_bytes();
    let mut i = 0usize;
    let mut cmd = b'M';
    let mut cur = Pt { x: 0.0, y: 0.0 };
    let mut start = cur;

    while i < bytes.len() {
        skip_sep(bytes, &mut i);
        if i >= bytes.len() {
            break;
        }
        let b = bytes[i];
        if is_cmd(b) {
            cmd = b;
            i += 1;
            if cmd == b'Z' || cmd == b'z' {
                stroke_line(p, cur, start, ox, oy, scale, stroke, color);
                cur = start;
            }
            continue;
        }

        match cmd {
            b'M' | b'm' => {
                let Some(x) = read_num(bytes, &mut i) else {
                    break;
                };
                let Some(y) = read_num(bytes, &mut i) else {
                    break;
                };
                let pt = if cmd == b'm' {
                    Pt {
                        x: cur.x + x,
                        y: cur.y + y,
                    }
                } else {
                    Pt { x, y }
                };
                cur = pt;
                start = pt;
                cmd = if cmd == b'm' { b'l' } else { b'L' };
            }
            b'L' | b'l' => {
                let Some(x) = read_num(bytes, &mut i) else {
                    break;
                };
                let Some(y) = read_num(bytes, &mut i) else {
                    break;
                };
                let next = if cmd == b'l' {
                    Pt {
                        x: cur.x + x,
                        y: cur.y + y,
                    }
                } else {
                    Pt { x, y }
                };
                stroke_line(p, cur, next, ox, oy, scale, stroke, color);
                cur = next;
            }
            b'H' | b'h' => {
                let Some(x) = read_num(bytes, &mut i) else {
                    break;
                };
                let next = if cmd == b'h' {
                    Pt {
                        x: cur.x + x,
                        y: cur.y,
                    }
                } else {
                    Pt { x, y: cur.y }
                };
                stroke_line(p, cur, next, ox, oy, scale, stroke, color);
                cur = next;
            }
            b'V' | b'v' => {
                let Some(y) = read_num(bytes, &mut i) else {
                    break;
                };
                let next = if cmd == b'v' {
                    Pt {
                        x: cur.x,
                        y: cur.y + y,
                    }
                } else {
                    Pt { x: cur.x, y }
                };
                stroke_line(p, cur, next, ox, oy, scale, stroke, color);
                cur = next;
            }
            b'C' | b'c' => {
                let Some(x1) = read_num(bytes, &mut i) else {
                    break;
                };
                let Some(y1) = read_num(bytes, &mut i) else {
                    break;
                };
                let Some(x2) = read_num(bytes, &mut i) else {
                    break;
                };
                let Some(y2) = read_num(bytes, &mut i) else {
                    break;
                };
                let Some(x3) = read_num(bytes, &mut i) else {
                    break;
                };
                let Some(y3) = read_num(bytes, &mut i) else {
                    break;
                };
                let c1 = rel(cmd, cur, x1, y1);
                let c2 = rel(cmd, cur, x2, y2);
                let end = rel(cmd, cur, x3, y3);
                stroke_cubic(p, cur, c1, c2, end, ox, oy, scale, stroke, color);
                cur = end;
            }
            b'Q' | b'q' => {
                let Some(x1) = read_num(bytes, &mut i) else {
                    break;
                };
                let Some(y1) = read_num(bytes, &mut i) else {
                    break;
                };
                let Some(x2) = read_num(bytes, &mut i) else {
                    break;
                };
                let Some(y2) = read_num(bytes, &mut i) else {
                    break;
                };
                let c = rel(cmd, cur, x1, y1);
                let end = rel(cmd, cur, x2, y2);
                stroke_quad(p, cur, c, end, ox, oy, scale, stroke, color);
                cur = end;
            }
            // Most Tabler icons do not need these. Treat smooth/arc commands as
            // endpoint lines so the icon still shows instead of disappearing.
            b'S' | b's' => {
                let _ = read_num(bytes, &mut i);
                let _ = read_num(bytes, &mut i);
                let Some(x) = read_num(bytes, &mut i) else {
                    break;
                };
                let Some(y) = read_num(bytes, &mut i) else {
                    break;
                };
                let next = rel(cmd, cur, x, y);
                stroke_line(p, cur, next, ox, oy, scale, stroke, color);
                cur = next;
            }
            b'A' | b'a' => {
                for _ in 0..5 {
                    let _ = read_num(bytes, &mut i);
                }
                let Some(x) = read_num(bytes, &mut i) else {
                    break;
                };
                let Some(y) = read_num(bytes, &mut i) else {
                    break;
                };
                let next = rel(cmd, cur, x, y);
                stroke_line(p, cur, next, ox, oy, scale, stroke, color);
                cur = next;
            }
            _ => i += 1,
        }
    }
}

#[derive(Clone, Copy)]
struct Pt {
    x: f32,
    y: f32,
}

fn rel(cmd: u8, cur: Pt, x: f32, y: f32) -> Pt {
    if cmd.is_ascii_lowercase() {
        Pt {
            x: cur.x + x,
            y: cur.y + y,
        }
    } else {
        Pt { x, y }
    }
}

fn stroke_cubic(
    p: &mut Painter<'_>,
    a: Pt,
    b: Pt,
    c: Pt,
    d: Pt,
    ox: f32,
    oy: f32,
    scale: f32,
    stroke: i32,
    color: Color,
) {
    let mut prev = a;
    for step in 1..=10 {
        let t = step as f32 / 10.0;
        let nt = 1.0 - t;
        let next = Pt {
            x: nt * nt * nt * a.x
                + 3.0 * nt * nt * t * b.x
                + 3.0 * nt * t * t * c.x
                + t * t * t * d.x,
            y: nt * nt * nt * a.y
                + 3.0 * nt * nt * t * b.y
                + 3.0 * nt * t * t * c.y
                + t * t * t * d.y,
        };
        stroke_line(p, prev, next, ox, oy, scale, stroke, color);
        prev = next;
    }
}

fn stroke_quad(
    p: &mut Painter<'_>,
    a: Pt,
    b: Pt,
    c: Pt,
    ox: f32,
    oy: f32,
    scale: f32,
    stroke: i32,
    color: Color,
) {
    let mut prev = a;
    for step in 1..=8 {
        let t = step as f32 / 8.0;
        let nt = 1.0 - t;
        let next = Pt {
            x: nt * nt * a.x + 2.0 * nt * t * b.x + t * t * c.x,
            y: nt * nt * a.y + 2.0 * nt * t * b.y + t * t * c.y,
        };
        stroke_line(p, prev, next, ox, oy, scale, stroke, color);
        prev = next;
    }
}

fn stroke_line(
    p: &mut Painter<'_>,
    a: Pt,
    b: Pt,
    ox: f32,
    oy: f32,
    scale: f32,
    stroke: i32,
    color: Color,
) {
    let x0 = round_f32(ox + a.x * scale);
    let y0 = round_f32(oy + a.y * scale);
    let x1 = round_f32(ox + b.x * scale);
    let y1 = round_f32(oy + b.y * scale);
    let dx = x1 - x0;
    let dy = y1 - y0;
    let steps = dx.abs().max(dy.abs()).max(1);
    let r = stroke.max(1);
    for i in 0..=steps {
        let x = x0 + dx * i / steps;
        let y = y0 + dy * i / steps;
        p.rounded_rect(
            Rect::new(x - r / 2, y - r / 2, r as u32, r as u32),
            r as u32,
            color,
        );
    }
}

fn round_f32(value: f32) -> i32 {
    if value >= 0.0 {
        (value + 0.5) as i32
    } else {
        (value - 0.5) as i32
    }
}

fn is_cmd(b: u8) -> bool {
    matches!(
        b,
        b'M' | b'm'
            | b'L'
            | b'l'
            | b'H'
            | b'h'
            | b'V'
            | b'v'
            | b'C'
            | b'c'
            | b'S'
            | b's'
            | b'Q'
            | b'q'
            | b'A'
            | b'a'
            | b'Z'
            | b'z'
    )
}

fn skip_sep(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() && matches!(bytes[*i], b' ' | b',' | b'\n' | b'\r' | b'\t') {
        *i += 1;
    }
}

fn read_num(bytes: &[u8], i: &mut usize) -> Option<f32> {
    skip_sep(bytes, i);
    if *i >= bytes.len() {
        return None;
    }
    let start = *i;
    if matches!(bytes[*i], b'-' | b'+') {
        *i += 1;
    }
    while *i < bytes.len() && bytes[*i].is_ascii_digit() {
        *i += 1;
    }
    if *i < bytes.len() && bytes[*i] == b'.' {
        *i += 1;
        while *i < bytes.len() && bytes[*i].is_ascii_digit() {
            *i += 1;
        }
    }
    if *i < bytes.len() && matches!(bytes[*i], b'e' | b'E') {
        *i += 1;
        if *i < bytes.len() && matches!(bytes[*i], b'-' | b'+') {
            *i += 1;
        }
        while *i < bytes.len() && bytes[*i].is_ascii_digit() {
            *i += 1;
        }
    }
    core::str::from_utf8(&bytes[start..*i]).ok()?.parse().ok()
}
