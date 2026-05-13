//! Tiny built-in vector icon set.
//!
//! These are not font glyphs. They are immediate-mode line icons drawn into the
//! same XRGB8888/ARGB8888 pixel buffer as the rest of the UI, which keeps the
//! core renderer independent from icon fonts, SVG, XML, CSS, and font shaping.

use crate::{Color, Painter, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Node,
    Shield,
    Mesh,
    Cpu,
    Memory,
    Network,
    Storage,
    Lock,
    Key,
    Terminal,
    Wallet,
    Check,
    Warning,
    Spark,
}

impl<'a> Painter<'a> {
    pub fn icon(&mut self, icon: Icon, rect: Rect, color: Color) {
        let s = rect.w.min(rect.h).max(1) as i32;
        let ox = rect.x + ((rect.w as i32 - s) / 2);
        let oy = rect.y + ((rect.h as i32 - s) / 2);
        let p = |x: i32, y: i32| -> (i32, i32) { (ox + x * s / 24, oy + y * s / 24) };

        match icon {
            Icon::Node => {
                self.icon_circle(p(12, 12), s / 4, color);
                self.icon_circle(p(5, 5), s / 7, color);
                self.icon_circle(p(19, 5), s / 7, color);
                self.icon_circle(p(5, 19), s / 7, color);
                self.icon_circle(p(19, 19), s / 7, color);
                self.icon_line(p(7, 7), p(10, 10), 2, color);
                self.icon_line(p(17, 7), p(14, 10), 2, color);
                self.icon_line(p(7, 17), p(10, 14), 2, color);
                self.icon_line(p(17, 17), p(14, 14), 2, color);
            }
            Icon::Shield => {
                self.icon_line(p(12, 2), p(20, 6), 2, color);
                self.icon_line(p(20, 6), p(19, 14), 2, color);
                self.icon_line(p(19, 14), p(12, 22), 2, color);
                self.icon_line(p(12, 22), p(5, 14), 2, color);
                self.icon_line(p(5, 14), p(4, 6), 2, color);
                self.icon_line(p(4, 6), p(12, 2), 2, color);
                self.icon_line(p(8, 12), p(11, 15), 2, color);
                self.icon_line(p(11, 15), p(17, 9), 2, color);
            }
            Icon::Mesh => {
                for &(a, b) in &[(0, 1), (1, 2), (2, 3), (3, 0), (0, 4), (1, 4), (2, 4), (3, 4)] {
                    let pts = [p(5, 6), p(19, 6), p(19, 18), p(5, 18), p(12, 12)];
                    self.icon_line(pts[a], pts[b], 1, color);
                }
                for &(x, y) in &[(5, 6), (19, 6), (19, 18), (5, 18), (12, 12)] {
                    self.icon_circle(p(x, y), s / 9, color);
                }
            }
            Icon::Cpu => {
                self.icon_rect(Rect::new(ox + s * 6 / 24, oy + s * 6 / 24, (s * 12 / 24) as u32, (s * 12 / 24) as u32), color);
                for i in [4, 8, 12, 16, 20] {
                    self.icon_line(p(i, 3), p(i, 6), 1, color);
                    self.icon_line(p(i, 18), p(i, 21), 1, color);
                    self.icon_line(p(3, i), p(6, i), 1, color);
                    self.icon_line(p(18, i), p(21, i), 1, color);
                }
            }
            Icon::Memory => {
                self.icon_rect(Rect::new(ox + s * 4 / 24, oy + s * 7 / 24, (s * 16 / 24) as u32, (s * 10 / 24) as u32), color);
                for i in 0..5 {
                    let x = 6 + i * 3;
                    self.icon_line(p(x, 17), p(x, 21), 1, color);
                }
            }
            Icon::Network => {
                self.icon_arc_wifi(p(12, 18), s * 15 / 24, color);
                self.icon_arc_wifi(p(12, 18), s * 10 / 24, color);
                self.icon_arc_wifi(p(12, 18), s * 5 / 24, color);
                self.icon_circle(p(12, 19), s / 14, color);
            }
            Icon::Storage => {
                self.icon_ellipse(p(12, 6), s * 8 / 24, s * 3 / 24, color);
                self.icon_line(p(4, 6), p(4, 18), 2, color);
                self.icon_line(p(20, 6), p(20, 18), 2, color);
                self.icon_ellipse(p(12, 18), s * 8 / 24, s * 3 / 24, color);
                self.icon_line(p(4, 12), p(20, 12), 1, color.with_alpha(140));
            }
            Icon::Lock => {
                self.icon_rect(Rect::new(ox + s * 5 / 24, oy + s * 10 / 24, (s * 14 / 24) as u32, (s * 10 / 24) as u32), color);
                self.icon_arc_top(p(12, 11), s * 6 / 24, s * 7 / 24, color);
            }
            Icon::Key => {
                self.icon_circle(p(8, 10), s / 5, color);
                self.icon_line(p(11, 13), p(20, 22), 2, color);
                self.icon_line(p(16, 18), p(19, 15), 2, color);
                self.icon_line(p(18, 20), p(21, 17), 2, color);
            }
            Icon::Terminal => {
                self.icon_rect(Rect::new(ox + s * 3 / 24, oy + s * 5 / 24, (s * 18 / 24) as u32, (s * 14 / 24) as u32), color);
                self.icon_line(p(7, 9), p(10, 12), 2, color);
                self.icon_line(p(10, 12), p(7, 15), 2, color);
                self.icon_line(p(12, 16), p(17, 16), 2, color);
            }
            Icon::Wallet => {
                self.icon_rect(Rect::new(ox + s * 3 / 24, oy + s * 7 / 24, (s * 18 / 24) as u32, (s * 11 / 24) as u32), color);
                self.icon_rect(Rect::new(ox + s * 13 / 24, oy + s * 10 / 24, (s * 7 / 24) as u32, (s * 5 / 24) as u32), color);
                self.icon_circle(p(16, 12), s / 20, color);
            }
            Icon::Check => {
                self.icon_line(p(5, 13), p(10, 18), 3, color);
                self.icon_line(p(10, 18), p(20, 7), 3, color);
            }
            Icon::Warning => {
                self.icon_line(p(12, 3), p(22, 21), 2, color);
                self.icon_line(p(22, 21), p(2, 21), 2, color);
                self.icon_line(p(2, 21), p(12, 3), 2, color);
                self.icon_line(p(12, 9), p(12, 15), 2, color);
                self.icon_circle(p(12, 18), s / 18, color);
            }
            Icon::Spark => {
                self.icon_line(p(12, 2), p(12, 22), 2, color);
                self.icon_line(p(2, 12), p(22, 12), 2, color);
                self.icon_line(p(5, 5), p(19, 19), 1, color);
                self.icon_line(p(19, 5), p(5, 19), 1, color);
            }
        }
    }

    fn icon_rect(&mut self, rect: Rect, color: Color) {
        self.rounded_border(rect, 3, 2, color);
    }

    fn icon_circle(&mut self, center: (i32, i32), radius: i32, color: Color) {
        self.rounded_rect(Rect::new(center.0 - radius, center.1 - radius, (radius * 2) as u32, (radius * 2) as u32), radius as u32, color);
    }

    fn icon_line(&mut self, a: (i32, i32), b: (i32, i32), thickness: i32, color: Color) {
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let steps = dx.abs().max(dy.abs()).max(1);
        let r = thickness.max(1);
        for i in 0..=steps {
            let x = a.0 + dx * i / steps;
            let y = a.1 + dy * i / steps;
            self.rounded_rect(Rect::new(x - r / 2, y - r / 2, r as u32, r as u32), r as u32, color);
        }
    }

    fn icon_ellipse(&mut self, center: (i32, i32), rx: i32, ry: i32, color: Color) {
        if rx <= 0 || ry <= 0 {
            return;
        }
        for y in -ry..=ry {
            for x in -rx..=rx {
                let lhs = x * x * ry * ry + y * y * rx * rx;
                let rhs = rx * rx * ry * ry;
                let edge = (lhs - rhs).abs() < rhs / 4;
                if edge {
                    let px = center.0 + x;
                    let py = center.1 + y;
                    if px >= 0 && py >= 0 {
                        self.rect(Rect::new(px, py, 1, 1), color);
                    }
                }
            }
        }
    }

    fn icon_arc_wifi(&mut self, center: (i32, i32), radius: i32, color: Color) {
        for deg in (215..=325).step_by(3) {
            let (s, c) = sin_cos_deg(deg);
            let x = center.0 + (c * radius) / 1024;
            let y = center.1 + (s * radius) / 1024;
            self.rounded_rect(Rect::new(x - 1, y - 1, 3, 3), 2, color);
        }
    }

    fn icon_arc_top(&mut self, center: (i32, i32), rx: i32, ry: i32, color: Color) {
        for deg in (200..=340).step_by(3) {
            let (s, c) = sin_cos_deg(deg);
            let x = center.0 + (c * rx) / 1024;
            let y = center.1 + (s * ry) / 1024;
            self.rounded_rect(Rect::new(x - 1, y - 1, 3, 3), 2, color);
        }
    }
}

fn sin_cos_deg(deg: i32) -> (i32, i32) {
    // no_std friendly lookup. Values are scaled by 1024.
    const TABLE: [(i32, i32); 24] = [
        (0, 1024),
        (265, 989),
        (512, 887),
        (724, 724),
        (887, 512),
        (989, 265),
        (1024, 0),
        (989, -265),
        (887, -512),
        (724, -724),
        (512, -887),
        (265, -989),
        (0, -1024),
        (-265, -989),
        (-512, -887),
        (-724, -724),
        (-887, -512),
        (-989, -265),
        (-1024, 0),
        (-989, 265),
        (-887, 512),
        (-724, 724),
        (-512, 887),
        (-265, 989),
    ];

    let mut d = deg % 360;
    if d < 0 {
        d += 360;
    }
    let idx = ((d + 7) / 15) as usize % 24;
    TABLE[idx]
}
