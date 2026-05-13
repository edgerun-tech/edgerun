//! Extracted style-family palettes and semantic token inventory.

use super::{Color4, UiSemanticColors, UiStyleFamily};

const MIRA_COLORS: UiSemanticColors = UiSemanticColors {
    bg: Color4::rgba(0.035, 0.035, 0.035, 1.0),
    sidebar: Color4::rgba(0.055, 0.055, 0.055, 0.98),
    topbar: Color4::rgba(0.048, 0.048, 0.048, 0.98),
    panel: Color4::rgba(0.091, 0.091, 0.091, 0.96),
    row: Color4::rgba(0.125, 0.125, 0.125, 0.92),
    active: Color4::rgba(0.245, 0.245, 0.245, 0.88),
    composer: Color4::rgba(0.118, 0.118, 0.118, 0.98),
    text: Color4::rgba(0.925, 0.925, 0.925, 1.0),
    muted: Color4::rgba(0.63, 0.63, 0.63, 1.0),
    border: Color4::rgba(0.235, 0.235, 0.235, 0.78),
    accent: Color4::rgba(0.82, 0.82, 0.82, 1.0),
    accent_text: Color4::rgba(0.055, 0.055, 0.055, 1.0),
    success: Color4::rgba(0.0, 0.74, 0.54, 1.0),
    warning: Color4::rgba(0.93, 0.62, 0.06, 1.0),
    danger: Color4::rgba(0.86, 0.15, 0.15, 1.0),
    info: Color4::rgba(0.55, 0.43, 0.95, 1.0),
};

const VEGA_COLORS: UiSemanticColors = UiSemanticColors {
    bg: Color4::rgba(0.026, 0.034, 0.046, 1.0),
    sidebar: Color4::rgba(0.035, 0.045, 0.062, 0.98),
    topbar: Color4::rgba(0.031, 0.04, 0.056, 0.98),
    panel: Color4::rgba(0.065, 0.079, 0.103, 0.96),
    row: Color4::rgba(0.094, 0.113, 0.145, 0.92),
    active: Color4::rgba(0.1, 0.22, 0.36, 0.78),
    composer: Color4::rgba(0.055, 0.068, 0.091, 0.98),
    text: Color4::rgba(0.92, 0.95, 0.98, 1.0),
    muted: Color4::rgba(0.59, 0.66, 0.74, 1.0),
    border: Color4::rgba(0.19, 0.25, 0.34, 0.78),
    accent: Color4::rgba(0.3, 0.63, 0.95, 1.0),
    accent_text: Color4::rgba(0.02, 0.04, 0.07, 1.0),
    success: Color4::rgba(0.0, 0.74, 0.54, 1.0),
    warning: Color4::rgba(0.93, 0.62, 0.06, 1.0),
    danger: Color4::rgba(0.86, 0.15, 0.15, 1.0),
    info: Color4::rgba(0.44, 0.58, 0.95, 1.0),
};

const NOVA_COLORS: UiSemanticColors = UiSemanticColors {
    bg: Color4::rgba(0.028, 0.028, 0.031, 1.0),
    sidebar: Color4::rgba(0.042, 0.042, 0.047, 0.98),
    topbar: Color4::rgba(0.039, 0.039, 0.043, 0.98),
    panel: Color4::rgba(0.083, 0.083, 0.092, 0.96),
    row: Color4::rgba(0.121, 0.121, 0.134, 0.92),
    active: Color4::rgba(0.02, 0.27, 0.32, 0.72),
    composer: Color4::rgba(0.105, 0.105, 0.115, 0.98),
    text: Color4::rgba(0.94, 0.95, 0.96, 1.0),
    muted: Color4::rgba(0.63, 0.65, 0.68, 1.0),
    border: Color4::rgba(0.23, 0.24, 0.26, 0.78),
    accent: Color4::rgba(0.08, 0.76, 0.86, 1.0),
    accent_text: Color4::rgba(0.02, 0.04, 0.05, 1.0),
    success: Color4::rgba(0.0, 0.74, 0.54, 1.0),
    warning: Color4::rgba(0.93, 0.62, 0.06, 1.0),
    danger: Color4::rgba(0.86, 0.15, 0.15, 1.0),
    info: Color4::rgba(0.55, 0.43, 0.95, 1.0),
};

const MAIA_COLORS: UiSemanticColors = UiSemanticColors {
    bg: Color4::rgba(0.028, 0.035, 0.031, 1.0),
    sidebar: Color4::rgba(0.037, 0.048, 0.042, 0.98),
    topbar: Color4::rgba(0.034, 0.044, 0.039, 0.98),
    panel: Color4::rgba(0.071, 0.091, 0.08, 0.96),
    row: Color4::rgba(0.103, 0.133, 0.116, 0.92),
    active: Color4::rgba(0.0, 0.32, 0.2, 0.72),
    composer: Color4::rgba(0.063, 0.083, 0.072, 0.98),
    text: Color4::rgba(0.92, 0.96, 0.93, 1.0),
    muted: Color4::rgba(0.6, 0.69, 0.63, 1.0),
    border: Color4::rgba(0.18, 0.29, 0.22, 0.78),
    accent: Color4::rgba(0.0, 0.74, 0.54, 1.0),
    accent_text: Color4::rgba(0.02, 0.05, 0.035, 1.0),
    success: Color4::rgba(0.0, 0.74, 0.54, 1.0),
    warning: Color4::rgba(0.93, 0.62, 0.06, 1.0),
    danger: Color4::rgba(0.86, 0.15, 0.15, 1.0),
    info: Color4::rgba(0.55, 0.43, 0.95, 1.0),
};

const LYRA_COLORS: UiSemanticColors = UiSemanticColors {
    bg: Color4::rgba(0.033, 0.03, 0.049, 1.0),
    sidebar: Color4::rgba(0.046, 0.041, 0.066, 0.98),
    topbar: Color4::rgba(0.042, 0.037, 0.061, 0.98),
    panel: Color4::rgba(0.084, 0.075, 0.119, 0.96),
    row: Color4::rgba(0.122, 0.108, 0.17, 0.92),
    active: Color4::rgba(0.28, 0.18, 0.52, 0.72),
    composer: Color4::rgba(0.073, 0.064, 0.103, 0.98),
    text: Color4::rgba(0.95, 0.93, 0.99, 1.0),
    muted: Color4::rgba(0.67, 0.62, 0.76, 1.0),
    border: Color4::rgba(0.25, 0.2, 0.36, 0.78),
    accent: Color4::rgba(0.55, 0.43, 0.95, 1.0),
    accent_text: Color4::rgba(0.03, 0.02, 0.05, 1.0),
    success: Color4::rgba(0.0, 0.74, 0.54, 1.0),
    warning: Color4::rgba(0.93, 0.62, 0.06, 1.0),
    danger: Color4::rgba(0.86, 0.15, 0.15, 1.0),
    info: Color4::rgba(0.55, 0.43, 0.95, 1.0),
};

const LUMA_COLORS: UiSemanticColors = UiSemanticColors {
    bg: Color4::rgba(0.047, 0.04, 0.032, 1.0),
    sidebar: Color4::rgba(0.066, 0.055, 0.042, 0.98),
    topbar: Color4::rgba(0.06, 0.051, 0.039, 0.98),
    panel: Color4::rgba(0.118, 0.097, 0.07, 0.96),
    row: Color4::rgba(0.166, 0.132, 0.09, 0.92),
    active: Color4::rgba(0.42, 0.26, 0.04, 0.72),
    composer: Color4::rgba(0.104, 0.085, 0.061, 0.98),
    text: Color4::rgba(0.98, 0.94, 0.88, 1.0),
    muted: Color4::rgba(0.75, 0.67, 0.56, 1.0),
    border: Color4::rgba(0.34, 0.25, 0.15, 0.78),
    accent: Color4::rgba(0.93, 0.62, 0.06, 1.0),
    accent_text: Color4::rgba(0.06, 0.04, 0.01, 1.0),
    success: Color4::rgba(0.0, 0.74, 0.54, 1.0),
    warning: Color4::rgba(0.93, 0.62, 0.06, 1.0),
    danger: Color4::rgba(0.86, 0.15, 0.15, 1.0),
    info: Color4::rgba(0.55, 0.43, 0.95, 1.0),
};

const SERA_COLORS: UiSemanticColors = UiSemanticColors {
    bg: Color4::rgba(0.047, 0.039, 0.036, 1.0),
    sidebar: Color4::rgba(0.114, 0.094, 0.086, 0.98),
    topbar: Color4::rgba(0.047, 0.039, 0.036, 0.98),
    panel: Color4::rgba(0.114, 0.094, 0.086, 0.96),
    row: Color4::rgba(0.169, 0.142, 0.134, 0.92),
    active: Color4::rgba(0.169, 0.142, 0.134, 0.88),
    composer: Color4::rgba(0.114, 0.094, 0.086, 0.98),
    text: Color4::rgba(0.985, 0.981, 0.976, 1.0),
    muted: Color4::rgba(0.67, 0.628, 0.612, 1.0),
    border: Color4::rgba(1.0, 1.0, 1.0, 0.1),
    accent: Color4::rgba(0.911, 0.894, 0.89, 1.0),
    accent_text: Color4::rgba(0.114, 0.094, 0.086, 1.0),
    success: Color4::rgba(0.0, 0.74, 0.54, 1.0),
    warning: Color4::rgba(0.93, 0.62, 0.06, 1.0),
    danger: Color4::rgba(0.86, 0.15, 0.15, 1.0),
    info: Color4::rgba(0.55, 0.43, 0.95, 1.0),
};

pub const fn colors_for_style_family(family: UiStyleFamily) -> UiSemanticColors {
    match family {
        UiStyleFamily::Vega => VEGA_COLORS,
        UiStyleFamily::Nova => NOVA_COLORS,
        UiStyleFamily::Maia => MAIA_COLORS,
        UiStyleFamily::Lyra => LYRA_COLORS,
        UiStyleFamily::Mira => MIRA_COLORS,
        UiStyleFamily::Luma => LUMA_COLORS,
        UiStyleFamily::Sera => SERA_COLORS,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiStyleFamilySpec {
    pub family: UiStyleFamily,
    pub name: &'static str,
    pub preset_code: &'static str,
    pub base_color: &'static str,
    pub role: &'static str,
}

impl UiStyleFamilySpec {
    pub const fn preset(self) -> super::UiStylePreset {
        self.family.preset()
    }

    pub const fn colors(self) -> UiSemanticColors {
        colors_for_style_family(self.family)
    }
}

pub const STYLE_FAMILY_SPECS: &[UiStyleFamilySpec] = &[
    style_family_spec(UiStyleFamily::Vega, "structured blue-black system surface"),
    style_family_spec(UiStyleFamily::Nova, "neutral graphite system surface"),
    style_family_spec(UiStyleFamily::Maia, "green policy and trust surface"),
    style_family_spec(UiStyleFamily::Lyra, "violet creative and agent surface"),
    style_family_spec(UiStyleFamily::Mira, "neutral high-contrast default surface"),
    style_family_spec(UiStyleFamily::Luma, "warm finance and publishing surface"),
    style_family_spec(UiStyleFamily::Sera, "warm rose collaboration surface"),
];

const fn style_family_spec(family: UiStyleFamily, role: &'static str) -> UiStyleFamilySpec {
    UiStyleFamilySpec {
        family,
        name: family.name(),
        preset_code: family.preset_code(),
        base_color: family.base_color(),
        role,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiExtractedStyleTokenKind {
    Surface,
    Text,
    Border,
    Action,
    Status,
}

pub const EXTRACTED_STYLE_TOKEN_KINDS: [UiExtractedStyleTokenKind; 5] = [
    UiExtractedStyleTokenKind::Surface,
    UiExtractedStyleTokenKind::Text,
    UiExtractedStyleTokenKind::Border,
    UiExtractedStyleTokenKind::Action,
    UiExtractedStyleTokenKind::Status,
];

impl UiExtractedStyleTokenKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Surface => "Surface",
            Self::Text => "Text",
            Self::Border => "Border",
            Self::Action => "Action",
            Self::Status => "Status",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiExtractedStyleToken {
    pub name: &'static str,
    pub kind: UiExtractedStyleTokenKind,
    pub css_var: &'static str,
    pub class_names: &'static [&'static str],
    pub role: &'static str,
}

impl UiExtractedStyleToken {
    pub const fn kind_label(self) -> &'static str {
        self.kind.label()
    }

    pub fn has_class(self, class_name: &str) -> bool {
        self.class_names
            .iter()
            .any(|candidate| *candidate == class_name)
    }
}

pub const EXTRACTED_STYLE_TOKENS: &[UiExtractedStyleToken] = &[
    style_token(
        "background",
        UiExtractedStyleTokenKind::Surface,
        "--background",
        &["bg-background", "bg-bg"],
        "page and canvas background",
    ),
    style_token(
        "card",
        UiExtractedStyleTokenKind::Surface,
        "--card",
        &["bg-card", "bg-panel"],
        "dashboard cards, modals, and framed tools",
    ),
    style_token(
        "card foreground",
        UiExtractedStyleTokenKind::Text,
        "--card-foreground",
        &["text-card-foreground"],
        "primary text on card surfaces",
    ),
    style_token(
        "popover",
        UiExtractedStyleTokenKind::Surface,
        "--popover",
        &["bg-popover"],
        "floating menus, tooltips, and anchored overlays",
    ),
    style_token(
        "popover foreground",
        UiExtractedStyleTokenKind::Text,
        "--popover-foreground",
        &["text-popover-foreground"],
        "primary text inside floating overlays",
    ),
    style_token(
        "muted",
        UiExtractedStyleTokenKind::Surface,
        "--muted",
        &["bg-muted", "bg-row"],
        "secondary rows, icon wells, subdued controls",
    ),
    style_token(
        "foreground",
        UiExtractedStyleTokenKind::Text,
        "--foreground",
        &["text-foreground", "text-text"],
        "primary text",
    ),
    style_token(
        "muted foreground",
        UiExtractedStyleTokenKind::Text,
        "--muted-foreground",
        &["text-muted-foreground", "text-muted"],
        "labels, descriptions, helper text",
    ),
    style_token(
        "border",
        UiExtractedStyleTokenKind::Border,
        "--border",
        &["border-border", "border"],
        "card, input, separator, and control outlines",
    ),
    style_token(
        "primary",
        UiExtractedStyleTokenKind::Action,
        "--primary",
        &["bg-primary", "text-primary"],
        "main action and high-emphasis state",
    ),
    style_token(
        "primary foreground",
        UiExtractedStyleTokenKind::Text,
        "--primary-foreground",
        &["text-primary-foreground"],
        "text on primary action surfaces",
    ),
    style_token(
        "secondary",
        UiExtractedStyleTokenKind::Surface,
        "--secondary",
        &["bg-secondary"],
        "low-emphasis button and row surfaces",
    ),
    style_token(
        "secondary foreground",
        UiExtractedStyleTokenKind::Text,
        "--secondary-foreground",
        &["text-secondary-foreground"],
        "text on low-emphasis secondary surfaces",
    ),
    style_token(
        "accent",
        UiExtractedStyleTokenKind::Action,
        "--accent",
        &["bg-accent", "text-accent"],
        "hover, selected, and emphasized interactive state",
    ),
    style_token(
        "accent foreground",
        UiExtractedStyleTokenKind::Text,
        "--accent-foreground",
        &["text-accent-foreground"],
        "text on accent hover and selected surfaces",
    ),
    style_token(
        "destructive",
        UiExtractedStyleTokenKind::Status,
        "--destructive",
        &["text-destructive", "aria-invalid:border-destructive"],
        "danger zone and validation state",
    ),
    style_token(
        "destructive foreground",
        UiExtractedStyleTokenKind::Text,
        "--destructive-foreground",
        &["text-destructive-foreground"],
        "text on destructive action surfaces",
    ),
    style_token(
        "success",
        UiExtractedStyleTokenKind::Status,
        "--success",
        &["text-success", "bg-success"],
        "verified, paid, completed, and positive receipt states",
    ),
    style_token(
        "warning",
        UiExtractedStyleTokenKind::Status,
        "--warning",
        &["text-warning", "bg-warning"],
        "pending, budget, and caution states",
    ),
    style_token(
        "info",
        UiExtractedStyleTokenKind::Status,
        "--info",
        &["text-info", "bg-info"],
        "neutral informational and policy reference states",
    ),
    style_token(
        "input",
        UiExtractedStyleTokenKind::Surface,
        "--input",
        &["border-input", "bg-input"],
        "input border and disabled input fill",
    ),
    style_token(
        "ring",
        UiExtractedStyleTokenKind::Border,
        "--ring",
        &["border-ring", "focus-visible:ring-ring/50"],
        "keyboard focus and validation ring",
    ),
    style_token(
        "chart 1",
        UiExtractedStyleTokenKind::Status,
        "--chart-1",
        &["bg-chart-1", "text-chart-1"],
        "first chart series color",
    ),
    style_token(
        "chart 2",
        UiExtractedStyleTokenKind::Status,
        "--chart-2",
        &["bg-chart-2", "text-chart-2"],
        "second chart series color",
    ),
    style_token(
        "chart 3",
        UiExtractedStyleTokenKind::Status,
        "--chart-3",
        &["bg-chart-3", "text-chart-3"],
        "third chart series color",
    ),
    style_token(
        "chart 4",
        UiExtractedStyleTokenKind::Status,
        "--chart-4",
        &["bg-chart-4", "text-chart-4"],
        "fourth chart series color",
    ),
    style_token(
        "chart 5",
        UiExtractedStyleTokenKind::Status,
        "--chart-5",
        &["bg-chart-5", "text-chart-5"],
        "fifth chart series color",
    ),
];

const fn style_token(
    name: &'static str,
    kind: UiExtractedStyleTokenKind,
    css_var: &'static str,
    class_names: &'static [&'static str],
    role: &'static str,
) -> UiExtractedStyleToken {
    UiExtractedStyleToken {
        name,
        kind,
        css_var,
        class_names,
        role,
    }
}
