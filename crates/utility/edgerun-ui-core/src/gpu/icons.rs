use super::{Color4, GpuRect, GpuScene, UiRect, spacing};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiIconSet {
    #[default]
    Tabler,
    Lucide,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiIconTone {
    #[default]
    Default,
    Muted,
    Disabled,
    Destructive,
    Warning,
    Success,
    Selected,
}

impl UiIconTone {
    pub const ALL: &'static [Self] = &[
        Self::Default,
        Self::Muted,
        Self::Disabled,
        Self::Destructive,
        Self::Warning,
        Self::Success,
        Self::Selected,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Muted => "muted",
            Self::Disabled => "disabled",
            Self::Destructive => "destructive",
            Self::Warning => "warning",
            Self::Success => "success",
            Self::Selected => "selected",
        }
    }

    pub const fn color(self, theme: super::UiResolvedTheme) -> Color4 {
        match self {
            Self::Default => theme.colors.text,
            Self::Muted => theme.colors.muted,
            Self::Disabled => theme.colors.muted.with_alpha(0.42),
            Self::Destructive => theme.colors.danger,
            Self::Warning => theme.colors.warning,
            Self::Success => theme.colors.success,
            Self::Selected => theme.colors.accent,
        }
    }
}

pub const ICON_VISUAL_MAX: f32 = 22.0;
pub const ROW_ICON_VISUAL: f32 = 20.0;
pub const STATUS_BADGE_ICON_VISUAL: f32 = 14.0;

pub const fn center_icon_rect(bounds: UiRect, size: f32) -> UiRect {
    UiRect::new(
        bounds.x + (bounds.w - size) * 0.5,
        bounds.y + (bounds.h - size) * 0.5,
        size,
        size,
    )
}

pub fn icon_only_button_icon_rect(bounds: UiRect) -> UiRect {
    center_icon_rect(bounds, bounds.w.min(bounds.h).min(ICON_VISUAL_MAX))
}

pub fn icon_hit_rect(visual: UiRect) -> UiRect {
    let w = visual.w.max(spacing::MIN_TOUCH_TARGET);
    let h = visual.h.max(spacing::MIN_TOUCH_TARGET);
    UiRect::new(
        visual.x + (visual.w - w) * 0.5,
        visual.y + (visual.h - h) * 0.5,
        w,
        h,
    )
}

pub fn row_icon_rect(row: UiRect) -> UiRect {
    center_icon_rect(
        UiRect::new(row.x + spacing::ROW_PAD_X, row.y, spacing::ROW_ICON, row.h),
        ROW_ICON_VISUAL,
    )
}

pub fn status_badge_icon_rect(badge: UiRect) -> UiRect {
    center_icon_rect(badge, badge.w.min(badge.h).min(STATUS_BADGE_ICON_VISUAL))
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

#[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
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

#[cfg(feature = "lucide-svg-atlas")]
pub fn lucide_svg_icon_atlas() -> UiIconAtlas {
    UiIconAtlas {
        width: crate::lucide_svg_atlas_generated::LUCIDE_SVG_ATLAS_W,
        height: crate::lucide_svg_atlas_generated::LUCIDE_SVG_ATLAS_H,
        alpha: crate::lucide_svg_atlas_generated::LUCIDE_SVG_ATLAS_ALPHA,
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
    MessagePlus,
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
    Trash,
    User,
    Wallet,
    Warning,
    X,
}

impl UiIcon {
    pub const ALL: &'static [Self] = &[
        Self::Activity,
        Self::App,
        Self::Bell,
        Self::Chat,
        Self::Check,
        Self::ChevronRight,
        Self::Code,
        Self::Cpu,
        Self::Database,
        Self::Eye,
        Self::File,
        Self::Key,
        Self::Lock,
        Self::Menu,
        Self::MessagePlus,
        Self::Network,
        Self::Route,
        Self::Search,
        Self::Send,
        Self::Server,
        Self::Settings,
        Self::Shield,
        Self::Sparkles,
        Self::Storage,
        Self::Terminal,
        Self::Trust,
        Self::Trash,
        Self::User,
        Self::Wallet,
        Self::Warning,
        Self::X,
    ];

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
            Self::MessagePlus => "message-plus",
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
            Self::Trash => "trash",
            Self::User => "user",
            Self::Wallet => "wallet",
            Self::Warning => "warning",
            Self::X => "x",
        }
    }

    pub const fn tooltip_label(self) -> &'static str {
        match self {
            Self::Activity => "View activity",
            Self::App => "Open app",
            Self::Bell => "Notifications",
            Self::Chat => "Open chat",
            Self::Check => "Confirm",
            Self::ChevronRight => "Next",
            Self::Code => "View code",
            Self::Cpu => "Compute",
            Self::Database => "Database",
            Self::Eye => "View",
            Self::File => "Open file",
            Self::Key => "Keys",
            Self::Lock => "Lock",
            Self::Menu => "Menu",
            Self::MessagePlus => "New message",
            Self::Network => "Network",
            Self::Route => "Route",
            Self::Search => "Search",
            Self::Send => "Send",
            Self::Server => "Server",
            Self::Settings => "Settings",
            Self::Shield => "Trust",
            Self::Sparkles => "Enhance",
            Self::Storage => "Storage",
            Self::Terminal => "Terminal",
            Self::Trust => "Trust",
            Self::Trash => "Delete",
            Self::User => "Profile",
            Self::Wallet => "Wallet",
            Self::Warning => "Warning",
            Self::X => "Close",
        }
    }

    pub const fn provider_name(self, set: UiIconSet) -> &'static str {
        match set {
            UiIconSet::Tabler => self.tabler_name(),
            UiIconSet::Lucide => self.lucide_name(),
        }
    }

    #[cfg(feature = "tabler-svg-atlas")]
    pub fn tabler_svg_atlas_rect(self) -> UiIconAtlasRect {
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
            .expect("compiled Tabler SVG atlas must contain every canonical UiIcon")
    }

    #[cfg(feature = "lucide-svg-atlas")]
    pub fn lucide_svg_atlas_rect(self) -> UiIconAtlasRect {
        let name = self.lucide_name();
        let atlas_w = crate::lucide_svg_atlas_generated::LUCIDE_SVG_ATLAS_W as f32;
        let atlas_h = crate::lucide_svg_atlas_generated::LUCIDE_SVG_ATLAS_H as f32;
        crate::lucide_svg_atlas_generated::LUCIDE_SVG_ICONS
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
            .expect("compiled Lucide SVG atlas must contain every canonical UiIcon")
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
            Self::MessagePlus => "message-plus",
            Self::Network => "network",
            Self::Route => "route",
            Self::Search => "search",
            Self::Send => "arrow-up",
            Self::Server => "server",
            Self::Settings => "settings",
            Self::Shield | Self::Trust => "shield-check",
            Self::Sparkles => "sparkles",
            Self::Terminal => "terminal-2",
            Self::User => "user",
            Self::Trash => "trash",
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
            Self::MessagePlus => "message-circle-plus",
            Self::Network => "network",
            Self::Route => "route",
            Self::Search => "search",
            Self::Send => "arrow-up",
            Self::Server => "server",
            Self::Settings => "settings",
            Self::Shield | Self::Trust => "shield-check",
            Self::Sparkles => "sparkles",
            Self::Terminal => "square-terminal",
            Self::User => "user",
            Self::Trash => "trash-2",
            Self::Wallet => "wallet",
            Self::Warning => "triangle-alert",
            Self::X => "x",
        }
    }
}

#[cfg(not(feature = "tabler-svg-atlas"))]
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
        UiIcon::ChevronRight => {
            icon_line(scene, p(8.0, 5.0), p(16.0, 12.0), stroke, color);
            icon_line(scene, p(16.0, 12.0), p(8.0, 19.0), stroke, color);
        }
        UiIcon::Send => {
            icon_line(scene, p(12.0, 5.0), p(12.0, 19.0), stroke, color);
            icon_line(scene, p(12.0, 5.0), p(6.5, 10.5), stroke, color);
            icon_line(scene, p(12.0, 5.0), p(17.5, 10.5), stroke, color);
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
        | UiIcon::MessagePlus
        | UiIcon::Sparkles => {
            draw_symbolic_icon(scene, rect, icon, color, stroke);
        }
        UiIcon::Trash => {
            icon_line(scene, p(5.0, 7.0), p(19.0, 7.0), stroke, color);
            icon_line(scene, p(10.0, 4.0), p(14.0, 4.0), stroke, color);
            icon_line(scene, p(9.0, 4.0), p(15.0, 4.0), stroke, color);
            scene.push_rect(GpuRect::border(
                ox + size * 6.0 / 24.0,
                oy + size * 7.0 / 24.0,
                size * 12.0 / 24.0,
                size * 13.0 / 24.0,
                size * 1.5 / 24.0,
                color,
            ));
            icon_line(scene, p(10.0, 11.0), p(10.0, 17.0), stroke * 0.65, color);
            icon_line(scene, p(14.0, 11.0), p(14.0, 17.0), stroke * 0.65, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_icons_have_provider_names() {
        for icon in UiIcon::ALL {
            assert!(!icon.name().is_empty());
            assert!(!icon.provider_name(UiIconSet::Tabler).is_empty());
            assert!(!icon.provider_name(UiIconSet::Lucide).is_empty());
        }
    }

    #[test]
    fn icon_tones_resolve_to_theme_color_tokens() {
        let theme = super::super::UiResolvedTheme::user_default();
        for tone in UiIconTone::ALL {
            assert!(!tone.name().is_empty());
        }
        assert_eq!(UiIconTone::Default.color(theme), theme.colors.text);
        assert_eq!(UiIconTone::Muted.color(theme), theme.colors.muted);
        assert_eq!(
            UiIconTone::Disabled.color(theme),
            theme.colors.muted.with_alpha(0.42)
        );
        assert_eq!(UiIconTone::Destructive.color(theme), theme.colors.danger);
        assert_eq!(UiIconTone::Warning.color(theme), theme.colors.warning);
        assert_eq!(UiIconTone::Success.color(theme), theme.colors.success);
        assert_eq!(UiIconTone::Selected.color(theme), theme.colors.accent);
    }

    #[test]
    fn icon_alignment_helpers_center_visual_rects_and_preserve_size_contracts() {
        let button = UiRect::new(10.0, 20.0, 40.0, 32.0);
        assert_eq!(
            icon_only_button_icon_rect(button),
            UiRect::new(19.0, 25.0, 22.0, 22.0)
        );

        let row = UiRect::new(4.0, 8.0, 320.0, spacing::ROW_H);
        assert_eq!(
            row_icon_rect(row),
            UiRect::new(25.0, 27.0, ROW_ICON_VISUAL, ROW_ICON_VISUAL)
        );

        let badge = UiRect::new(100.0, 12.0, 34.0, 22.0);
        assert_eq!(
            status_badge_icon_rect(badge),
            UiRect::new(
                110.0,
                16.0,
                STATUS_BADGE_ICON_VISUAL,
                STATUS_BADGE_ICON_VISUAL
            )
        );
    }

    #[test]
    fn icon_hit_rect_expands_small_visuals_to_minimum_touch_target() {
        let visual = UiRect::new(20.0, 30.0, 20.0, 20.0);
        assert_eq!(
            icon_hit_rect(visual),
            UiRect::new(
                14.0,
                24.0,
                spacing::MIN_TOUCH_TARGET,
                spacing::MIN_TOUCH_TARGET
            )
        );

        let large = UiRect::new(5.0, 6.0, 48.0, 40.0);
        assert_eq!(icon_hit_rect(large), large);
    }

    #[cfg(feature = "tabler-svg-atlas")]
    #[test]
    fn canonical_icons_have_tabler_atlas_rects() {
        for icon in UiIcon::ALL {
            let rect = icon.tabler_svg_atlas_rect();
            assert_eq!(rect.name, icon.tabler_name());
            assert!(rect.w > 0);
            assert!(rect.h > 0);
        }
    }

    #[cfg(feature = "lucide-svg-atlas")]
    #[test]
    fn canonical_icons_have_lucide_atlas_rects() {
        for icon in UiIcon::ALL {
            let rect = icon.lucide_svg_atlas_rect();
            assert_eq!(rect.name, icon.lucide_name());
            assert!(rect.w > 0);
            assert!(rect.h > 0);
        }
    }

    #[cfg(feature = "tabler-svg-atlas")]
    #[test]
    fn tabler_atlas_alpha_matches_texture_upload_contract() {
        let atlas = tabler_svg_icon_atlas();
        assert_eq!(
            atlas.alpha.len(),
            atlas.width as usize * atlas.height as usize
        );
        assert!(atlas.alpha.iter().any(|alpha| *alpha > 0));
        for icon in UiIcon::ALL {
            let rect = icon.tabler_svg_atlas_rect();
            assert!(rect.x + rect.w <= atlas.width);
            assert!(rect.y + rect.h <= atlas.height);
            assert!(rect.u0 >= 0.0 && rect.u0 < rect.u1 && rect.u1 <= 1.0);
            assert!(rect.v0 >= 0.0 && rect.v0 < rect.v1 && rect.v1 <= 1.0);
        }
    }

    #[cfg(feature = "lucide-svg-atlas")]
    #[test]
    fn lucide_atlas_alpha_matches_texture_upload_contract() {
        let atlas = lucide_svg_icon_atlas();
        assert_eq!(
            atlas.alpha.len(),
            atlas.width as usize * atlas.height as usize
        );
        assert!(atlas.alpha.iter().any(|alpha| *alpha > 0));
        for icon in UiIcon::ALL {
            let rect = icon.lucide_svg_atlas_rect();
            assert!(rect.x + rect.w <= atlas.width);
            assert!(rect.y + rect.h <= atlas.height);
            assert!(rect.u0 >= 0.0 && rect.u0 < rect.u1 && rect.u1 <= 1.0);
            assert!(rect.v0 >= 0.0 && rect.v0 < rect.v1 && rect.v1 <= 1.0);
        }
    }

    #[cfg(feature = "tabler-svg-atlas")]
    #[test]
    fn tabler_icons_emit_stable_quads_at_common_ui_sizes() {
        assert_common_icon_sizes_emit_stable_quads(UiIconSet::Tabler);
    }

    #[cfg(feature = "lucide-svg-atlas")]
    #[test]
    fn lucide_icons_emit_stable_quads_at_common_ui_sizes() {
        assert_common_icon_sizes_emit_stable_quads(UiIconSet::Lucide);
    }

    #[cfg(any(feature = "tabler-svg-atlas", feature = "lucide-svg-atlas"))]
    fn assert_common_icon_sizes_emit_stable_quads(icon_set: UiIconSet) {
        for size in [12.0, 16.0, 20.0, 24.0, 32.0, 40.0] {
            let mut scene = GpuScene::new(Color4::rgba(0.0, 0.0, 0.0, 0.0));
            let mut painter = super::super::UiPainter::new(&mut scene).with_icon_set(icon_set);
            painter.icon(
                super::super::UiRect::new(8.0, 12.0, size, size),
                UiIcon::Shield,
                Color4::rgba(0.1, 0.2, 0.3, 1.0),
            );

            assert_eq!(scene.icon_quads().len(), 1, "size {size}");
            let quad = scene.icon_quads()[0];
            assert_eq!(quad.x, 8.0);
            assert_eq!(quad.y, 12.0);
            assert_eq!(quad.w, size);
            assert_eq!(quad.h, size);
            assert!(quad.u0 >= 0.0 && quad.u0 < quad.u1 && quad.u1 <= 1.0);
            assert!(quad.v0 >= 0.0 && quad.v0 < quad.v1 && quad.v1 <= 1.0);
            assert_eq!(quad.color, Color4::rgba(0.1, 0.2, 0.3, 1.0));
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

#[cfg(not(feature = "tabler-svg-atlas"))]
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
        UiIcon::MessagePlus => {
            scene.push_rect(GpuRect::border(
                ox + size * 4.0 / 24.0,
                oy + size * 5.0 / 24.0,
                size * 16.0 / 24.0,
                size * 12.0 / 24.0,
                size * 4.0 / 24.0,
                color,
            ));
            icon_line(scene, p(9.0, 17.0), p(6.0, 21.0), stroke, color);
            icon_line(scene, p(12.0, 8.5), p(12.0, 13.5), stroke, color);
            icon_line(scene, p(9.5, 11.0), p(14.5, 11.0), stroke, color);
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

#[cfg(not(feature = "tabler-svg-atlas"))]
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

#[cfg(not(feature = "tabler-svg-atlas"))]
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
