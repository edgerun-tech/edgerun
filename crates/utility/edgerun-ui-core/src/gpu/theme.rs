use super::{Color4, UiColorScheme, palette, preset_recipe_for_style_family};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiStyleAuthority {
    User,
    AuthorVision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiRadiusPreset {
    None,
    Compact,
    Default,
    Soft,
}

impl UiRadiusPreset {
    pub const fn next(self) -> Self {
        match self {
            Self::None => Self::Compact,
            Self::Compact => Self::Default,
            Self::Default => Self::Soft,
            Self::Soft => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiAccentPreset {
    Neutral,
    Cyan,
    Blue,
    Green,
    Violet,
    Amber,
}

impl UiAccentPreset {
    pub const fn next(self) -> Self {
        match self {
            Self::Neutral => Self::Cyan,
            Self::Cyan => Self::Blue,
            Self::Blue => Self::Green,
            Self::Green => Self::Violet,
            Self::Violet => Self::Amber,
            Self::Amber => Self::Neutral,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiStyleFamily {
    Vega,
    Nova,
    Maia,
    Lyra,
    Mira,
    Luma,
    Sera,
}

pub const UI_STYLE_FAMILIES: [UiStyleFamily; 7] = [
    UiStyleFamily::Vega,
    UiStyleFamily::Nova,
    UiStyleFamily::Maia,
    UiStyleFamily::Lyra,
    UiStyleFamily::Mira,
    UiStyleFamily::Luma,
    UiStyleFamily::Sera,
];

impl UiStyleFamily {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Vega => "Vega",
            Self::Nova => "Nova",
            Self::Maia => "Maia",
            Self::Lyra => "Lyra",
            Self::Mira => "Mira",
            Self::Luma => "Luma",
            Self::Sera => "Sera",
        }
    }

    pub const fn preset_code(self) -> &'static str {
        match self {
            Self::Vega => "bIkeymG",
            Self::Nova => "b2fA",
            Self::Maia => "bbVKFP6",
            Self::Lyra => "buFznsW",
            Self::Mira => "b1D0eCA4",
            Self::Luma => "b1VlIttI",
            Self::Sera => "b4xFeBLg4O",
        }
    }

    pub const fn base_color(self) -> &'static str {
        match self {
            Self::Vega => "Neutral",
            Self::Nova => "Neutral",
            Self::Maia => "Neutral",
            Self::Lyra => "Neutral",
            Self::Mira => "Neutral",
            Self::Luma => "Neutral",
            Self::Sera => "Taupe",
        }
    }

    pub const fn accent(self) -> UiAccentPreset {
        match self {
            Self::Vega => UiAccentPreset::Blue,
            Self::Nova => UiAccentPreset::Cyan,
            Self::Maia => UiAccentPreset::Green,
            Self::Lyra => UiAccentPreset::Violet,
            Self::Mira => UiAccentPreset::Neutral,
            Self::Luma => UiAccentPreset::Amber,
            Self::Sera => UiAccentPreset::Neutral,
        }
    }

    pub const fn radius(self) -> UiRadiusPreset {
        match self {
            Self::Vega => UiRadiusPreset::Compact,
            Self::Nova => UiRadiusPreset::Default,
            Self::Maia => UiRadiusPreset::Default,
            Self::Lyra => UiRadiusPreset::None,
            Self::Mira => UiRadiusPreset::Default,
            Self::Luma => UiRadiusPreset::Soft,
            Self::Sera => UiRadiusPreset::None,
        }
    }

    pub const fn theme_color(self) -> &'static str {
        match self {
            Self::Sera => "Taupe",
            _ => "Neutral",
        }
    }

    pub const fn chart_color(self) -> &'static str {
        match self {
            Self::Sera => "Taupe",
            _ => "Neutral",
        }
    }

    pub const fn font(self) -> &'static str {
        match self {
            Self::Nova => "Geist",
            Self::Maia => "Figtree",
            Self::Lyra => "JetBrains Mono",
            Self::Sera => "Noto Sans",
            _ => "Inter",
        }
    }

    pub const fn font_heading(self) -> &'static str {
        match self {
            Self::Sera => "Playfair Display",
            _ => "Inherit",
        }
    }

    pub const fn icon_library(self) -> &'static str {
        match self {
            Self::Maia | Self::Mira => "HugeIcons",
            Self::Lyra => "Phosphor Icons",
            _ => "Lucide",
        }
    }

    pub const fn menu_color(self) -> &'static str {
        "Default / Solid"
    }

    pub const fn menu_accent(self) -> &'static str {
        "Subtle"
    }

    pub const fn encoded_radius(self) -> &'static str {
        match self {
            Self::Lyra | Self::Sera => "Default",
            Self::Vega | Self::Nova | Self::Maia | Self::Mira | Self::Luma => "Default",
        }
    }

    pub const fn preset(self) -> UiStylePreset {
        let recipe = preset_recipe_for_style_family(self);
        UiStylePreset {
            name: self.name(),
            preset_code: self.preset_code(),
            scheme: UiColorScheme::Dark,
            base_color: self.base_color(),
            theme_color: self.theme_color(),
            chart_color: self.chart_color(),
            accent: self.accent(),
            radius: self.radius(),
            encoded_radius: recipe.encoded_radius,
            icon_set: self.icon_library(),
            font: self.font(),
            font_heading: self.font_heading(),
            menu_color: self.menu_color(),
            menu_accent: self.menu_accent(),
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Vega => Self::Nova,
            Self::Nova => Self::Maia,
            Self::Maia => Self::Lyra,
            Self::Lyra => Self::Mira,
            Self::Mira => Self::Luma,
            Self::Luma => Self::Sera,
            Self::Sera => Self::Vega,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiStylePreset {
    pub name: &'static str,
    pub preset_code: &'static str,
    pub scheme: UiColorScheme,
    pub base_color: &'static str,
    pub theme_color: &'static str,
    pub chart_color: &'static str,
    pub accent: UiAccentPreset,
    pub radius: UiRadiusPreset,
    pub encoded_radius: &'static str,
    pub icon_set: &'static str,
    pub font: &'static str,
    pub font_heading: &'static str,
    pub menu_color: &'static str,
    pub menu_accent: &'static str,
}

impl UiStylePreset {
    pub const fn user_default() -> Self {
        UiStyleFamily::Mira.preset()
    }

    pub const fn from_family(family: UiStyleFamily) -> Self {
        family.preset()
    }

    pub const fn vega() -> Self {
        UiStyleFamily::Vega.preset()
    }

    pub const fn nova() -> Self {
        UiStyleFamily::Nova.preset()
    }

    pub const fn maia() -> Self {
        UiStyleFamily::Maia.preset()
    }

    pub const fn lyra() -> Self {
        UiStyleFamily::Lyra.preset()
    }

    pub const fn luma() -> Self {
        UiStyleFamily::Luma.preset()
    }

    pub const fn sera() -> Self {
        UiStyleFamily::Sera.preset()
    }

    pub const fn author_vision() -> Self {
        Self {
            name: "Author Vision",
            preset_code: "",
            scheme: UiColorScheme::Terminal,
            base_color: "Zinc",
            theme_color: "Zinc",
            chart_color: "Green",
            accent: UiAccentPreset::Green,
            radius: UiRadiusPreset::Compact,
            encoded_radius: "compact",
            icon_set: "Tabler",
            font: "Inter",
            font_heading: "Inter",
            menu_color: "Default / Solid",
            menu_accent: "Subtle",
        }
    }

    pub const fn scheme_label(self) -> &'static str {
        match self.scheme {
            UiColorScheme::Dark => "Dark",
            UiColorScheme::Light => "Light",
            UiColorScheme::Terminal => "Terminal",
        }
    }

    pub const fn accent_label(self) -> &'static str {
        match self.accent {
            UiAccentPreset::Neutral => "Neutral",
            UiAccentPreset::Cyan => "Cyan",
            UiAccentPreset::Blue => "Blue",
            UiAccentPreset::Green => "Green",
            UiAccentPreset::Violet => "Violet",
            UiAccentPreset::Amber => "Amber",
        }
    }

    pub const fn radius_label(self) -> &'static str {
        match self.radius {
            UiRadiusPreset::None => "None",
            UiRadiusPreset::Compact => "Compact",
            UiRadiusPreset::Default => "Default",
            UiRadiusPreset::Soft => "Soft",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiSemanticColors {
    pub bg: Color4,
    pub sidebar: Color4,
    pub topbar: Color4,
    pub panel: Color4,
    pub row: Color4,
    pub active: Color4,
    pub composer: Color4,
    pub text: Color4,
    pub muted: Color4,
    pub border: Color4,
    pub accent: Color4,
    pub accent_text: Color4,
    pub success: Color4,
    pub warning: Color4,
    pub danger: Color4,
    pub info: Color4,
}

impl UiSemanticColors {
    pub const fn edgerun_dark() -> Self {
        Self {
            bg: palette::BG,
            sidebar: palette::SIDEBAR,
            topbar: palette::TOPBAR,
            panel: palette::PANEL,
            row: palette::ROW,
            active: palette::ACTIVE_ROW,
            composer: palette::COMPOSER,
            text: palette::TEXT,
            muted: palette::MUTED,
            border: palette::BORDER,
            accent: palette::ACCENT,
            accent_text: palette::ACCENT_TEXT,
            success: palette::GREEN,
            warning: palette::AMBER,
            danger: palette::DANGER,
            info: palette::VIOLET,
        }
    }

    pub const fn with_accent(mut self, accent: Color4) -> Self {
        self.accent = accent;
        self.active = accent.with_alpha(0.42);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiRadiusScale {
    pub control: f32,
    pub card: f32,
    pub panel: f32,
    pub pill: f32,
}

impl UiRadiusScale {
    pub const fn from_preset(preset: UiRadiusPreset) -> Self {
        match preset {
            UiRadiusPreset::None => Self {
                control: 0.0,
                card: 0.0,
                panel: 0.0,
                pill: 0.0,
            },
            UiRadiusPreset::Compact => Self {
                control: 6.0,
                card: 6.0,
                panel: 8.0,
                pill: 999.0,
            },
            UiRadiusPreset::Default => Self {
                control: 10.0,
                card: 8.0,
                panel: 12.0,
                pill: 999.0,
            },
            UiRadiusPreset::Soft => Self {
                control: 14.0,
                card: 14.0,
                panel: 18.0,
                pill: 999.0,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiDensity {
    Compact,
    Comfortable,
    Spacious,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiResolvedTheme {
    pub authority: UiStyleAuthority,
    pub preset: UiStylePreset,
    pub colors: UiSemanticColors,
    pub radius: UiRadiusScale,
    pub density: UiDensity,
}

impl UiResolvedTheme {
    pub const fn from_preset(authority: UiStyleAuthority, preset: UiStylePreset) -> Self {
        let family = style_family_from_name(preset.name);
        if matches_preset_name(preset.name, family.name()) {
            return Self::style_family(
                authority,
                preset,
                super::style_family::colors_for_style_family(family),
            );
        }
        Self {
            authority,
            preset,
            colors: semantic_colors_for_scheme(preset.scheme)
                .with_accent(accent_color(preset.accent)),
            radius: UiRadiusScale::from_preset(preset.radius),
            density: UiDensity::Comfortable,
        }
    }

    pub const fn user_default() -> Self {
        Self::from_preset(UiStyleAuthority::User, UiStylePreset::user_default())
    }

    pub const fn style_family(
        authority: UiStyleAuthority,
        preset: UiStylePreset,
        colors: UiSemanticColors,
    ) -> Self {
        Self {
            authority,
            preset,
            colors: apply_accent_override(colors, preset.accent),
            radius: UiRadiusScale::from_preset(preset.radius),
            density: UiDensity::Comfortable,
        }
    }

    pub const fn authority_label(self) -> &'static str {
        match self.authority {
            UiStyleAuthority::User => "User style",
            UiStyleAuthority::AuthorVision => "Author vision",
        }
    }
}

const fn apply_accent_override(
    colors: UiSemanticColors,
    accent: UiAccentPreset,
) -> UiSemanticColors {
    match accent {
        UiAccentPreset::Neutral => colors,
        _ => colors.with_accent(accent_color(accent)),
    }
}

const fn matches_preset_name(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

pub const fn next_color_scheme(scheme: UiColorScheme) -> UiColorScheme {
    match scheme {
        UiColorScheme::Dark => UiColorScheme::Terminal,
        UiColorScheme::Terminal => UiColorScheme::Light,
        UiColorScheme::Light => UiColorScheme::Dark,
    }
}

pub const fn semantic_colors_for_scheme(scheme: UiColorScheme) -> UiSemanticColors {
    match scheme {
        UiColorScheme::Dark => UiSemanticColors::edgerun_dark(),
        UiColorScheme::Terminal => UiSemanticColors {
            bg: Color4::from_color(crate::TAILWIND.black),
            sidebar: Color4::from_color_alpha(crate::TAILWIND.black, 0.98),
            topbar: Color4::from_color_alpha(crate::TAILWIND.slate_950, 0.96),
            panel: Color4::from_color_alpha(crate::TAILWIND.slate_950, 0.94),
            row: Color4::from_color_alpha(crate::TAILWIND.slate_900, 0.78),
            active: palette::ACTIVE_ROW,
            composer: Color4::from_color_alpha(crate::TAILWIND.slate_950, 0.98),
            text: Color4::from_color(crate::TAILWIND.slate_50),
            muted: Color4::from_color(crate::TAILWIND.slate_400),
            border: Color4::from_color_alpha(crate::TAILWIND.emerald_500, 0.34),
            accent: palette::GREEN,
            accent_text: palette::ACCENT_TEXT,
            success: palette::GREEN,
            warning: palette::AMBER,
            danger: palette::DANGER,
            info: palette::VIOLET,
        },
        UiColorScheme::Light => UiSemanticColors {
            bg: Color4::from_color(crate::TAILWIND.slate_50),
            sidebar: Color4::from_color_alpha(crate::TAILWIND.slate_50, 0.98),
            topbar: Color4::from_color_alpha(crate::TAILWIND.slate_50, 0.96),
            panel: Color4::from_color_alpha(crate::TAILWIND.slate_50, 0.94),
            row: Color4::from_color_alpha(crate::TAILWIND.slate_400, 0.22),
            active: palette::ACTIVE_ROW,
            composer: Color4::from_color_alpha(crate::TAILWIND.slate_50, 0.98),
            text: Color4::from_color(crate::TAILWIND.slate_950),
            muted: Color4::from_color(crate::TAILWIND.slate_700),
            border: Color4::from_color_alpha(crate::TAILWIND.slate_400, 0.52),
            accent: palette::ACCENT,
            accent_text: palette::ACCENT_TEXT,
            success: palette::GREEN,
            warning: palette::AMBER,
            danger: palette::DANGER,
            info: palette::VIOLET,
        },
    }
}

impl Default for UiResolvedTheme {
    fn default() -> Self {
        Self::user_default()
    }
}

pub const fn accent_color(accent: UiAccentPreset) -> Color4 {
    match accent {
        UiAccentPreset::Neutral => palette::MUTED,
        UiAccentPreset::Cyan => palette::ACCENT,
        UiAccentPreset::Blue => Color4::from_color(crate::TAILWIND.cyan_600),
        UiAccentPreset::Green => palette::GREEN,
        UiAccentPreset::Violet => palette::VIOLET,
        UiAccentPreset::Amber => palette::AMBER,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiComponentPreviewState {
    pub authority: UiStyleAuthority,
    pub user_preset: UiStylePreset,
    pub author_preset: UiStylePreset,
}

impl UiComponentPreviewState {
    pub const fn user_owned() -> Self {
        Self {
            authority: UiStyleAuthority::User,
            user_preset: UiStylePreset::user_default(),
            author_preset: UiStylePreset::author_vision(),
        }
    }

    pub const fn active_preset(self) -> UiStylePreset {
        match self.authority {
            UiStyleAuthority::User => self.user_preset,
            UiStyleAuthority::AuthorVision => self.author_preset,
        }
    }

    pub const fn resolved_theme(self) -> UiResolvedTheme {
        UiResolvedTheme::from_preset(self.authority, self.active_preset())
    }

    pub const fn active_authority_label(self) -> &'static str {
        match self.authority {
            UiStyleAuthority::User => "User style",
            UiStyleAuthority::AuthorVision => "Author vision",
        }
    }

    pub const fn author_available(self) -> bool {
        !matches!(self.authority, UiStyleAuthority::AuthorVision)
    }

    pub fn cycle_user_scheme(&mut self) {
        self.authority = UiStyleAuthority::User;
        self.user_preset.scheme = next_color_scheme(self.user_preset.scheme);
    }

    pub fn cycle_user_accent(&mut self) {
        self.authority = UiStyleAuthority::User;
        self.user_preset.accent = self.user_preset.accent.next();
    }

    pub fn cycle_user_style_family(&mut self) {
        self.authority = UiStyleAuthority::User;
        self.user_preset =
            UiStylePreset::from_family(style_family_from_name(self.user_preset.name).next());
    }

    pub fn cycle_user_radius(&mut self) {
        self.authority = UiStyleAuthority::User;
        self.user_preset.radius = self.user_preset.radius.next();
    }
}

pub const fn style_family_from_name(name: &str) -> UiStyleFamily {
    if matches_preset_name(name, "Vega") {
        return UiStyleFamily::Vega;
    }
    if matches_preset_name(name, "Nova") {
        return UiStyleFamily::Nova;
    }
    if matches_preset_name(name, "Maia") {
        return UiStyleFamily::Maia;
    }
    if matches_preset_name(name, "Lyra") {
        return UiStyleFamily::Lyra;
    }
    if matches_preset_name(name, "Luma") {
        return UiStyleFamily::Luma;
    }
    if matches_preset_name(name, "Sera") {
        return UiStyleFamily::Sera;
    }
    UiStyleFamily::Mira
}

impl Default for UiComponentPreviewState {
    fn default() -> Self {
        Self::user_owned()
    }
}
