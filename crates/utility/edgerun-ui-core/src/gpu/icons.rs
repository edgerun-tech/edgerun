use super::{Color4, GpuRect, GpuScene, UiRect};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiIconSet {
    #[default]
    Tabler,
    Lucide,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiIconAtlasRect {
    pub name: &'static str,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

#[cfg(feature = "tabler-svg-atlas")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiIconAtlas {
    pub width: u32,
    pub height: u32,
    pub alpha: &'static [u8],
}

#[cfg(feature = "tabler-svg-atlas")]
pub fn tabler_svg_icon_atlas() -> UiIconAtlas {
    UiIconAtlas {
        width: crate::tabler_svg_atlas_generated::TABLER_SVG_ATLAS_W,
        height: crate::tabler_svg_atlas_generated::TABLER_SVG_ATLAS_H,
        alpha: crate::tabler_svg_atlas_generated::TABLER_SVG_ATLAS_ALPHA,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiIcon {
    Activity,
    App,
    Bell,
    Chat,
    Check,
    ChevronRight,
    Code,
    Cpu,
    Database,
    Eye,
    File,
    Key,
    Lock,
    Menu,
    Network,
    Route,
    Search,
    Send,
    Server,
    Settings,
    Shield,
    Sparkles,
    Storage,
    Terminal,
    Trust,
    User,
    Wallet,
    Warning,
    X,
}

impl UiIcon {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Activity => "activity",
            Self::App => "app",
            Self::Bell => "bell",
            Self::Chat => "chat",
            Self::Check => "check",
            Self::ChevronRight => "chevron-right",
            Self::Code => "code",
            Self::Cpu => "cpu",
            Self::Database => "database",
            Self::Eye => "eye",
            Self::File => "file",
            Self::Key => "key",
            Self::Lock => "lock",
            Self::Menu => "menu",
            Self::Network => "network",
            Self::Route => "route",
            Self::Search => "search",
            Self::Send => "send",
            Self::Server => "server",
            Self::Settings => "settings",
            Self::Shield => "shield",
            Self::Sparkles => "sparkles",
            Self::Storage => "storage",
            Self::Terminal => "terminal",
            Self::Trust => "trust",
            Self::User => "user",
            Self::Wallet => "wallet",
            Self::Warning => "warning",
            Self::X => "x",
        }
    }

    pub const fn provider_name(self, set: UiIconSet) -> &'static str {
        match set {
            UiIconSet::Tabler => self.tabler_name(),
            UiIconSet::Lucide => self.lucide_name(),
        }
    }

    #[cfg(feature = "tabler-svg-atlas")]
    pub fn tabler_svg_atlas_rect(self) -> Option<UiIconAtlasRect> {
        let name = self.tabler_name();
        let atlas_w = crate::tabler_svg_atlas_generated::TABLER_SVG_ATLAS_W as f32;
        let atlas_h = crate::tabler_svg_atlas_generated::TABLER_SVG_ATLAS_H as f32;
        crate::tabler_svg_atlas_generated::TABLER_SVG_ICONS
            .iter()
            .find(|icon| icon.name == name)
            .map(|icon| UiIconAtlasRect {
                name: icon.name,
                x: icon.x,
                y: icon.y,
                w: icon.w,
                h: icon.h,
                u0: icon.x as f32 / atlas_w,
                v0: icon.y as f32 / atlas_h,
                u1: (icon.x + icon.w) as f32 / atlas_w,
                v1: (icon.y + icon.h) as f32 / atlas_h,
            })
    }

    pub const fn tabler_name(self) -> &'static str {
        match self {
            Self::Activity => "activity",
            Self::App => "apps",
            Self::Bell => "bell",
            Self::Chat => "message-circle",
            Self::Check => "check",
            Self::ChevronRight => "chevron-right",
            Self::Code => "code",
            Self::Cpu => "cpu",
            Self::Database | Self::Storage => "database",
            Self::Eye => "eye",
            Self::File => "file",
            Self::Key => "key",
            Self::Lock => "lock",
            Self::Menu => "menu-2",
            Self::Network => "network",
            Self::Route => "route",
            Self::Search => "search",
            Self::Send => "send",
            Self::Server => "server",
            Self::Settings => "settings",
            Self::Shield | Self::Trust => "shield-check",
            Self::Sparkles => "sparkles",
            Self::Terminal => "terminal-2",
            Self::User => "user",
            Self::Wallet => "wallet",
            Self::Warning => "alert-triangle",
            Self::X => "x",
        }
    }

    pub const fn lucide_name(self) -> &'static str {
        match self {
            Self::Activity => "activity",
            Self::App => "app-window",
            Self::Bell => "bell",
            Self::Chat => "message-circle",
            Self::Check => "check",
            Self::ChevronRight => "chevron-right",
            Self::Code => "code",
            Self::Cpu => "cpu",
            Self::Database | Self::Storage => "database",
            Self::Eye => "eye",
            Self::File => "file",
            Self::Key => "key",
            Self::Lock => "lock",
            Self::Menu => "menu",
            Self::Network => "network",
            Self::Route => "route",
            Self::Search => "search",
            Self::Send => "send",
            Self::Server => "server",
            Self::Settings => "settings",
            Self::Shield | Self::Trust => "shield-check",
            Self::Sparkles => "sparkles",
            Self::Terminal => "square-terminal",
            Self::User => "user",
            Self::Wallet => "wallet",
            Self::Warning => "triangle-alert",
            Self::X => "x",
        }
    }
}

pub(super) fn draw_canonical_icon(scene: &mut GpuScene, rect: UiRect, icon: UiIcon, color: Color4) {
    let size = rect.w.min(rect.h).max(1.0);
    let ox = rect.x + (rect.w - size) * 0.5;
    let oy = rect.y + (rect.h - size) * 0.5;
    let p = |x: f32, y: f32| -> (f32, f32) { (ox + x * size / 24.0, oy + y * size / 24.0) };
    let stroke = (size / 12.0).clamp(1.25, 2.4);

    match icon {
        UiIcon::X => {
            icon_line(scene, p(6.0, 6.0), p(18.0, 18.0), stroke, color);
            icon_line(scene, p(18.0, 6.0), p(6.0, 18.0), stroke, color);
        }
        UiIcon::Check => {
            icon_line(scene, p(5.0, 12.5), p(10.0, 17.0), stroke, color);
            icon_line(scene, p(10.0, 17.0), p(20.0, 7.0), stroke, color);
        }
        UiIcon::ChevronRight | UiIcon::Send => {
            icon_line(scene, p(8.0, 5.0), p(16.0, 12.0), stroke, color);
            icon_line(scene, p(16.0, 12.0), p(8.0, 19.0), stroke, color);
            if icon == UiIcon::Send {
                icon_line(scene, p(4.0, 12.0), p(16.0, 12.0), stroke, color);
            }
        }
        UiIcon::Menu => {
            for y in [7.0, 12.0, 17.0] {
                icon_line(scene, p(5.0, y), p(19.0, y), stroke, color);
            }
        }
        UiIcon::Search => {
            icon_circle(scene, p(10.5, 10.5), size * 0.22, stroke, color);
            icon_line(scene, p(15.5, 15.5), p(20.0, 20.0), stroke, color);
        }
        UiIcon::Lock | UiIcon::Key => {
            if icon == UiIcon::Lock {
                scene.push_rect(GpuRect::border(
                    ox + size * 5.0 / 24.0,
                    oy + size * 10.0 / 24.0,
                    size * 14.0 / 24.0,
                    size * 10.0 / 24.0,
                    size * 2.0 / 24.0,
                    color,
                ));
                icon_arc_top(
                    scene,
                    p(12.0, 11.0),
                    size * 0.25,
                    size * 0.29,
                    stroke,
                    color,
                );
            } else {
                icon_circle(scene, p(8.0, 10.0), size * 0.17, stroke, color);
                icon_line(scene, p(11.0, 13.0), p(20.0, 22.0), stroke, color);
                icon_line(scene, p(16.0, 18.0), p(19.0, 15.0), stroke, color);
                icon_line(scene, p(18.0, 20.0), p(21.0, 17.0), stroke, color);
            }
        }
        UiIcon::Shield | UiIcon::Trust => {
            for (a, b) in [
                (p(12.0, 3.0), p(20.0, 6.0)),
                (p(20.0, 6.0), p(18.0, 15.0)),
                (p(18.0, 15.0), p(12.0, 21.0)),
                (p(12.0, 21.0), p(6.0, 15.0)),
                (p(6.0, 15.0), p(4.0, 6.0)),
                (p(4.0, 6.0), p(12.0, 3.0)),
            ] {
                icon_line(scene, a, b, stroke, color);
            }
            icon_line(scene, p(8.0, 12.0), p(11.0, 15.0), stroke, color);
            icon_line(scene, p(11.0, 15.0), p(17.0, 9.0), stroke, color);
        }
        UiIcon::Warning => {
            icon_line(scene, p(12.0, 3.0), p(22.0, 21.0), stroke, color);
            icon_line(scene, p(22.0, 21.0), p(2.0, 21.0), stroke, color);
            icon_line(scene, p(2.0, 21.0), p(12.0, 3.0), stroke, color);
            icon_line(scene, p(12.0, 9.0), p(12.0, 15.0), stroke, color);
            scene.push_rect(GpuRect::fill(
                p(12.0, 18.0).0 - 1.0,
                p(12.0, 18.0).1 - 1.0,
                2.0,
                2.0,
                1.0,
                color,
            ));
        }
        UiIcon::Database | UiIcon::Storage => {
            icon_ellipse(scene, p(12.0, 6.0), size * 0.33, size * 0.12, stroke, color);
            icon_line(scene, p(4.0, 6.0), p(4.0, 18.0), stroke, color);
            icon_line(scene, p(20.0, 6.0), p(20.0, 18.0), stroke, color);
            icon_ellipse(
                scene,
                p(12.0, 18.0),
                size * 0.33,
                size * 0.12,
                stroke,
                color,
            );
            icon_line(
                scene,
                p(4.0, 12.0),
                p(20.0, 12.0),
                stroke * 0.65,
                color.with_alpha(0.68),
            );
        }
        UiIcon::Server | UiIcon::App | UiIcon::Terminal | UiIcon::Code => {
            let box_rect = UiRect::new(
                ox + size * 4.0 / 24.0,
                oy + size * 5.0 / 24.0,
                size * 16.0 / 24.0,
                size * 14.0 / 24.0,
            );
            scene.push_rect(GpuRect::border(
                box_rect.x,
                box_rect.y,
                box_rect.w,
                box_rect.h,
                size * 2.0 / 24.0,
                color,
            ));
            if matches!(icon, UiIcon::Terminal | UiIcon::Code) {
                icon_line(scene, p(8.0, 9.0), p(11.0, 12.0), stroke, color);
                icon_line(scene, p(11.0, 12.0), p(8.0, 15.0), stroke, color);
                icon_line(scene, p(13.0, 16.0), p(17.0, 16.0), stroke, color);
            } else {
                icon_line(scene, p(7.0, 10.0), p(17.0, 10.0), stroke, color);
                icon_line(scene, p(7.0, 15.0), p(17.0, 15.0), stroke, color);
            }
        }
        UiIcon::Network | UiIcon::Route => {
            let pts = [
                p(6.0, 7.0),
                p(18.0, 7.0),
                p(18.0, 17.0),
                p(6.0, 17.0),
                p(12.0, 12.0),
            ];
            for (a, b) in [(0, 4), (1, 4), (2, 4), (3, 4)] {
                icon_line(scene, pts[a], pts[b], stroke * 0.65, color);
            }
            for pt in pts {
                scene.push_rect(GpuRect::fill(pt.0 - 2.0, pt.1 - 2.0, 4.0, 4.0, 2.0, color));
            }
        }
        UiIcon::Cpu => {
            let r = UiRect::new(
                ox + size * 6.0 / 24.0,
                oy + size * 6.0 / 24.0,
                size * 12.0 / 24.0,
                size * 12.0 / 24.0,
            );
            scene.push_rect(GpuRect::border(r.x, r.y, r.w, r.h, 2.0, color));
            for i in [4.0, 8.0, 12.0, 16.0, 20.0] {
                icon_line(scene, p(i, 3.0), p(i, 6.0), stroke * 0.65, color);
                icon_line(scene, p(i, 18.0), p(i, 21.0), stroke * 0.65, color);
            }
        }
        UiIcon::User => {
            scene.push_rect(GpuRect::border(
                ox + size * 8.0 / 24.0,
                oy + size * 4.0 / 24.0,
                size * 8.0 / 24.0,
                size * 8.0 / 24.0,
                size * 4.0 / 24.0,
                color,
            ));
            icon_arc_top(
                scene,
                p(12.0, 22.0),
                size * 0.32,
                size * 0.28,
                stroke,
                color,
            );
        }
        UiIcon::Wallet
        | UiIcon::File
        | UiIcon::Bell
        | UiIcon::Eye
        | UiIcon::Settings
        | UiIcon::Activity
        | UiIcon::Chat
        | UiIcon::Sparkles => {
            draw_symbolic_icon(scene, rect, icon, color, stroke);
        }
    }
}

pub(super) fn icon_line(
    scene: &mut GpuScene,
    a: (f32, f32),
    b: (f32, f32),
    thickness: f32,
    color: Color4,
) {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as u32;
    let t = thickness.max(1.0);
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let x = a.0 + dx * f;
        let y = a.1 + dy * f;
        scene.push_rect(GpuRect::fill(
            x - t * 0.5,
            y - t * 0.5,
            t,
            t,
            t * 0.5,
            color,
        ));
    }
}

fn draw_symbolic_icon(
    scene: &mut GpuScene,
    rect: UiRect,
    icon: UiIcon,
    color: Color4,
    stroke: f32,
) {
    let size = rect.w.min(rect.h).max(1.0);
    let ox = rect.x + (rect.w - size) * 0.5;
    let oy = rect.y + (rect.h - size) * 0.5;
    let p = |x: f32, y: f32| -> (f32, f32) { (ox + x * size / 24.0, oy + y * size / 24.0) };
    match icon {
        UiIcon::Chat => {
            scene.push_rect(GpuRect::border(
                ox + size * 4.0 / 24.0,
                oy + size * 5.0 / 24.0,
                size * 16.0 / 24.0,
                size * 12.0 / 24.0,
                size * 4.0 / 24.0,
                color,
            ));
            icon_line(scene, p(9.0, 17.0), p(6.0, 21.0), stroke, color);
        }
        UiIcon::Eye => {
            icon_ellipse(
                scene,
                p(12.0, 12.0),
                size * 0.38,
                size * 0.20,
                stroke,
                color,
            );
            scene.push_rect(GpuRect::fill(
                p(12.0, 12.0).0 - 2.0,
                p(12.0, 12.0).1 - 2.0,
                4.0,
                4.0,
                2.0,
                color,
            ));
        }
        UiIcon::Bell => {
            icon_arc_top(
                scene,
                p(12.0, 14.0),
                size * 0.28,
                size * 0.34,
                stroke,
                color,
            );
            icon_line(scene, p(6.0, 16.0), p(18.0, 16.0), stroke, color);
            scene.push_rect(GpuRect::fill(
                p(12.0, 19.0).0 - 2.0,
                p(12.0, 19.0).1 - 2.0,
                4.0,
                4.0,
                2.0,
                color,
            ));
        }
        UiIcon::Settings => {
            icon_circle(scene, p(12.0, 12.0), size * 0.18, stroke, color);
            for (a, b) in [
                (12.0, 3.0),
                (12.0, 21.0),
                (3.0, 12.0),
                (21.0, 12.0),
                (5.5, 5.5),
                (18.5, 18.5),
                (18.5, 5.5),
                (5.5, 18.5),
            ] {
                icon_line(
                    scene,
                    p(12.0, 12.0),
                    p(a, b),
                    stroke * 0.55,
                    color.with_alpha(0.72),
                );
            }
        }
        UiIcon::Sparkles | UiIcon::Activity => {
            icon_line(scene, p(12.0, 3.0), p(12.0, 21.0), stroke, color);
            icon_line(scene, p(3.0, 12.0), p(21.0, 12.0), stroke, color);
            icon_line(scene, p(6.0, 6.0), p(18.0, 18.0), stroke * 0.65, color);
            icon_line(scene, p(18.0, 6.0), p(6.0, 18.0), stroke * 0.65, color);
        }
        UiIcon::Wallet | UiIcon::File => {
            scene.push_rect(GpuRect::border(
                ox + size * 5.0 / 24.0,
                oy + size * 4.0 / 24.0,
                size * 14.0 / 24.0,
                size * 16.0 / 24.0,
                size * 2.0 / 24.0,
                color,
            ));
            if icon == UiIcon::Wallet {
                icon_line(scene, p(13.0, 12.0), p(19.0, 12.0), stroke, color);
            } else {
                icon_line(scene, p(9.0, 9.0), p(15.0, 9.0), stroke * 0.65, color);
                icon_line(scene, p(9.0, 13.0), p(15.0, 13.0), stroke * 0.65, color);
            }
        }
        _ => {}
    }
}

pub(super) fn icon_circle(
    scene: &mut GpuScene,
    center: (f32, f32),
    radius: f32,
    thickness: f32,
    color: Color4,
) {
    let steps = 28;
    let mut prev = None;
    for i in 0..=steps {
        let angle = i as f32 * core::f32::consts::TAU / steps as f32;
        let pt = (
            center.0 + angle.cos() * radius,
            center.1 + angle.sin() * radius,
        );
        if let Some(prev) = prev {
            icon_line(scene, prev, pt, thickness, color);
        }
        prev = Some(pt);
    }
}

fn icon_ellipse(
    scene: &mut GpuScene,
    center: (f32, f32),
    rx: f32,
    ry: f32,
    thickness: f32,
    color: Color4,
) {
    let steps = 32;
    let mut prev = None;
    for i in 0..=steps {
        let angle = i as f32 * core::f32::consts::TAU / steps as f32;
        let pt = (center.0 + angle.cos() * rx, center.1 + angle.sin() * ry);
        if let Some(prev) = prev {
            icon_line(scene, prev, pt, thickness, color);
        }
        prev = Some(pt);
    }
}

fn icon_arc_top(
    scene: &mut GpuScene,
    center: (f32, f32),
    rx: f32,
    ry: f32,
    thickness: f32,
    color: Color4,
) {
    let steps = 18;
    let mut prev = None;
    for i in 0..=steps {
        let angle = core::f32::consts::PI + i as f32 * core::f32::consts::PI / steps as f32;
        let pt = (center.0 + angle.cos() * rx, center.1 + angle.sin() * ry);
        if let Some(prev) = prev {
            icon_line(scene, prev, pt, thickness, color);
        }
        prev = Some(pt);
    }
}
