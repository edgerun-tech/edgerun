use super::{Color4, UiRect, UiResolvedTheme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignItems {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JustifyContent {
    Start,
    Center,
    End,
    Between,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiColorToken {
    Bg,
    Sidebar,
    Topbar,
    Panel,
    Row,
    Active,
    Composer,
    Text,
    Muted,
    Border,
    Accent,
    AccentText,
    Success,
    Warning,
    Danger,
    Info,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UiStyleColor {
    Semantic(UiColorToken),
    Fixed(Color4),
}

impl UiStyleColor {
    pub const fn fixed(color: Color4) -> Self {
        Self::Fixed(color)
    }

    pub const fn semantic(token: UiColorToken) -> Self {
        Self::Semantic(token)
    }

    pub const fn resolve(self, theme: UiResolvedTheme) -> Color4 {
        let colors = theme.colors;
        match self {
            Self::Fixed(color) => color,
            Self::Semantic(UiColorToken::Bg) => colors.bg,
            Self::Semantic(UiColorToken::Sidebar) => colors.sidebar,
            Self::Semantic(UiColorToken::Topbar) => colors.topbar,
            Self::Semantic(UiColorToken::Panel) => colors.panel,
            Self::Semantic(UiColorToken::Row) => colors.row,
            Self::Semantic(UiColorToken::Active) => colors.active,
            Self::Semantic(UiColorToken::Composer) => colors.composer,
            Self::Semantic(UiColorToken::Text) => colors.text,
            Self::Semantic(UiColorToken::Muted) => colors.muted,
            Self::Semantic(UiColorToken::Border) => colors.border,
            Self::Semantic(UiColorToken::Accent) => colors.accent,
            Self::Semantic(UiColorToken::AccentText) => colors.accent_text,
            Self::Semantic(UiColorToken::Success) => colors.success,
            Self::Semantic(UiColorToken::Warning) => colors.warning,
            Self::Semantic(UiColorToken::Danger) => colors.danger,
            Self::Semantic(UiColorToken::Info) => colors.info,
        }
    }
}

const SPACING_UNIT: f32 = 4.0;
const FILL_PARENT: f32 = -1.0;

#[derive(Clone, Debug)]
pub struct UiStyle {
    pub direction: Axis,
    pub gap: f32,
    pub padding: [f32; 4],
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub grow: bool,
    pub grid_cols: Option<u16>,
    pub col_span: u16,
    pub align: AlignItems,
    pub justify: JustifyContent,
    pub bg: Option<UiStyleColor>,
    pub text: UiStyleColor,
    pub border: bool,
    pub radius: f32,
    pub truncate: bool,
    pub clip: bool,
    pub disabled: bool,
    pub loading: bool,
}

impl Default for UiStyle {
    fn default() -> Self {
        Self {
            direction: Axis::Vertical,
            gap: 0.0,
            padding: [0.0; 4],
            width: None,
            height: None,
            grow: false,
            grid_cols: None,
            col_span: 1,
            align: AlignItems::Stretch,
            justify: JustifyContent::Start,
            bg: None,
            text: UiStyleColor::semantic(UiColorToken::Text),
            border: false,
            radius: 0.0,
            truncate: false,
            clip: false,
            disabled: false,
            loading: false,
        }
    }
}

impl UiStyle {
    pub fn parse(classes: &str) -> Self {
        Self::parse_impl(classes, None)
    }

    pub fn parse_for_width(classes: &str, width: f32) -> Self {
        Self::parse_impl(classes, Some(width))
    }

    fn parse_impl(classes: &str, width: Option<f32>) -> Self {
        let mut style = Self::default();
        for class in classes.split_whitespace() {
            if let Some(class) = responsive_class(class, width) {
                style.apply_class(class);
            }
        }
        style
    }

    fn apply_class(&mut self, class: &str) {
        if self.apply_layout_class(class)
            || self.apply_radius_class(class)
            || self.apply_color_class(class)
            || self.apply_grid_class(class)
            || self.apply_spacing_class(class)
            || self.apply_size_class(class)
        {
            return;
        }
    }

    fn apply_layout_class(&mut self, class: &str) -> bool {
        match class {
            "row" | "flex-row" => self.direction = Axis::Horizontal,
            "col" | "column" | "flex-col" => self.direction = Axis::Vertical,
            "flex-1" | "grow" => self.grow = true,
            "truncate" => self.truncate = true,
            "overflow-hidden" | "overflow-clip" => self.clip = true,
            "disabled" => self.disabled = true,
            "loading" => self.loading = true,
            "border" | "border-border" | "border-input" | "border-dashed" | "ring" | "ring-1"
            | "ring-foreground/10" => self.border = true,
            "items-start" => self.align = AlignItems::Start,
            "items-center" => self.align = AlignItems::Center,
            "items-end" => self.align = AlignItems::End,
            "items-stretch" => self.align = AlignItems::Stretch,
            "justify-start" => self.justify = JustifyContent::Start,
            "justify-center" => self.justify = JustifyContent::Center,
            "justify-end" => self.justify = JustifyContent::End,
            "justify-between" => self.justify = JustifyContent::Between,
            _ => return false,
        }
        true
    }

    fn apply_grid_class(&mut self, class: &str) -> bool {
        if class == "grid" {
            self.grid_cols = Some(self.grid_cols.unwrap_or(1));
            return true;
        }
        if let Some(columns) = class
            .strip_prefix("grid-cols-")
            .and_then(|value| value.parse::<u16>().ok())
        {
            self.grid_cols = Some(columns.max(1));
            return true;
        }
        if let Some(span) = class
            .strip_prefix("col-span-")
            .and_then(|value| value.parse::<u16>().ok())
        {
            self.col_span = span.max(1);
            return true;
        }
        false
    }

    fn apply_radius_class(&mut self, class: &str) -> bool {
        self.radius = match class {
            "rounded" | "rounded-md" => 8.0,
            "rounded-sm" => 4.0,
            "rounded-lg" => 12.0,
            "rounded-xl" => 16.0,
            "rounded-2xl" => 20.0,
            "rounded-4xl" => 999.0,
            "rounded-full" => 999.0,
            _ => return false,
        };
        true
    }

    fn apply_color_class(&mut self, class: &str) -> bool {
        if let Some(color) = semantic_bg_color(class) {
            self.bg = Some(color);
            return true;
        }
        if let Some(color) = semantic_text_color(class) {
            self.text = color;
            return true;
        }
        if let Some(color) = class.strip_prefix("bg-").and_then(tailwind_class_color) {
            self.bg = Some(UiStyleColor::fixed(color));
            return true;
        }
        if let Some(color) = class.strip_prefix("text-").and_then(tailwind_class_color) {
            self.text = UiStyleColor::fixed(color);
            return true;
        }
        false
    }

    fn apply_spacing_class(&mut self, class: &str) -> bool {
        if let Some(value) = class.strip_prefix("gap-").and_then(spacing_value) {
            self.gap = value;
        } else if let Some(value) = class.strip_prefix("p-").and_then(spacing_value) {
            self.padding = [value; 4];
        } else if let Some(value) = class.strip_prefix("px-").and_then(spacing_value) {
            self.padding[1] = value;
            self.padding[3] = value;
        } else if let Some(value) = class.strip_prefix("py-").and_then(spacing_value) {
            self.padding[0] = value;
            self.padding[2] = value;
        } else {
            return false;
        }
        true
    }

    fn apply_size_class(&mut self, class: &str) -> bool {
        if class == "w-full" {
            self.width = Some(FILL_PARENT);
        } else if class == "h-full" {
            self.height = Some(FILL_PARENT);
        } else if let Some(value) = class.strip_prefix("w-").and_then(size_value) {
            self.width = Some(value);
        } else if let Some(value) = class.strip_prefix("h-").and_then(size_value) {
            self.height = Some(value);
        } else if let Some(value) = class.strip_prefix("size-").and_then(size_value) {
            self.width = Some(value);
            self.height = Some(value);
        } else {
            return false;
        }
        true
    }

    pub(super) fn layout_rect(&self, bounds: UiRect) -> UiRect {
        UiRect {
            x: bounds.x,
            y: bounds.y,
            w: match self.width {
                Some(value) if value >= 0.0 => value.min(bounds.w),
                _ => bounds.w,
            },
            h: match self.height {
                Some(value) if value >= 0.0 => value.min(bounds.h),
                _ => bounds.h,
            },
        }
    }
}

fn responsive_class<'a>(class: &'a str, width: Option<f32>) -> Option<&'a str> {
    let Some((prefix, base)) = class.split_once(':') else {
        return Some(class);
    };
    let min_width = match prefix {
        "sm" => 640.0,
        "md" => 768.0,
        "lg" => 1024.0,
        "xl" => 1280.0,
        _ => return None,
    };
    if width.unwrap_or(0.0) >= min_width {
        Some(base)
    } else {
        None
    }
}

fn spacing_value(value: &str) -> Option<f32> {
    value.parse::<f32>().ok().map(|n| n * SPACING_UNIT)
}

fn size_value(value: &str) -> Option<f32> {
    match value {
        "full" => Some(FILL_PARENT),
        _ => value.parse::<f32>().ok().map(|n| n * SPACING_UNIT),
    }
}

fn semantic_bg_color(class: &str) -> Option<UiStyleColor> {
    match class {
        "bg-bg" | "bg-background" => Some(UiStyleColor::semantic(UiColorToken::Bg)),
        "bg-sidebar" => Some(UiStyleColor::semantic(UiColorToken::Sidebar)),
        "bg-topbar" => Some(UiStyleColor::semantic(UiColorToken::Topbar)),
        "bg-panel" | "bg-card" | "bg-popover" => Some(UiStyleColor::semantic(UiColorToken::Panel)),
        "bg-row" | "bg-muted" | "bg-secondary" => Some(UiStyleColor::semantic(UiColorToken::Row)),
        "bg-active" => Some(UiStyleColor::semantic(UiColorToken::Active)),
        "bg-accent" => Some(UiStyleColor::semantic(UiColorToken::Accent)),
        "bg-composer" | "bg-input" => Some(UiStyleColor::semantic(UiColorToken::Composer)),
        "bg-primary" => Some(UiStyleColor::semantic(UiColorToken::Accent)),
        "bg-chart-1" => Some(UiStyleColor::semantic(UiColorToken::Accent)),
        "bg-chart-2" => Some(UiStyleColor::semantic(UiColorToken::Success)),
        "bg-chart-3" => Some(UiStyleColor::semantic(UiColorToken::Warning)),
        "bg-chart-4" => Some(UiStyleColor::semantic(UiColorToken::Info)),
        "bg-chart-5" => Some(UiStyleColor::semantic(UiColorToken::Danger)),
        _ => None,
    }
}

fn semantic_text_color(class: &str) -> Option<UiStyleColor> {
    match class {
        "text-primary"
        | "text-text"
        | "text-foreground"
        | "text-card-foreground"
        | "text-popover-foreground" => Some(UiStyleColor::semantic(UiColorToken::Text)),
        "text-muted" | "text-muted-foreground" | "text-secondary-foreground" => {
            Some(UiStyleColor::semantic(UiColorToken::Muted))
        }
        "text-accent" => Some(UiStyleColor::semantic(UiColorToken::Accent)),
        "text-primary-foreground" | "text-accent-foreground" => {
            Some(UiStyleColor::semantic(UiColorToken::AccentText))
        }
        "text-background" => Some(UiStyleColor::semantic(UiColorToken::Bg)),
        "text-green" => Some(UiStyleColor::semantic(UiColorToken::Success)),
        "text-violet" => Some(UiStyleColor::semantic(UiColorToken::Info)),
        "text-amber" => Some(UiStyleColor::semantic(UiColorToken::Warning)),
        "text-chart-1" => Some(UiStyleColor::semantic(UiColorToken::Accent)),
        "text-chart-2" => Some(UiStyleColor::semantic(UiColorToken::Success)),
        "text-chart-3" => Some(UiStyleColor::semantic(UiColorToken::Warning)),
        "text-chart-4" => Some(UiStyleColor::semantic(UiColorToken::Info)),
        "text-chart-5" => Some(UiStyleColor::semantic(UiColorToken::Danger)),
        "text-danger" | "text-destructive" | "text-destructive-foreground" => {
            Some(UiStyleColor::semantic(UiColorToken::Danger))
        }
        _ => None,
    }
}

fn tailwind_class_color(value: &str) -> Option<Color4> {
    let color = match value {
        "slate-50" => crate::TAILWIND.slate_50,
        "slate-400" => crate::TAILWIND.slate_400,
        "slate-700" => crate::TAILWIND.slate_700,
        "slate-800" => crate::TAILWIND.slate_800,
        "slate-900" => crate::TAILWIND.slate_900,
        "slate-950" => crate::TAILWIND.slate_950,
        "sky-50" => crate::TAILWIND.sky_50,
        "cyan-600" => crate::TAILWIND.cyan_600,
        "emerald-500" => crate::TAILWIND.emerald_500,
        "amber-500" => crate::TAILWIND.amber_500,
        "violet-500" => crate::TAILWIND.violet_500,
        "rose-600" => crate::TAILWIND.rose_600,
        "black" => crate::TAILWIND.black,
        _ => return None,
    };
    Some(Color4::from_color(color))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_layout_spacing_and_size_classes_independently() {
        let style =
            UiStyle::parse("row flex-1 truncate border rounded-lg gap-3 px-4 py-2 w-64 h-full");

        assert_eq!(style.direction, Axis::Horizontal);
        assert!(style.grow);
        assert!(style.truncate);
        assert!(style.border);
        assert_eq!(style.radius, 12.0);
        assert_eq!(style.gap, 12.0);
        assert_eq!(style.padding, [8.0, 16.0, 8.0, 16.0]);
        assert_eq!(style.width, Some(256.0));
        assert_eq!(style.height, Some(FILL_PARENT));
        assert_eq!(style.grid_cols, None);
        assert_eq!(style.col_span, 1);
        assert_eq!(style.align, AlignItems::Stretch);
        assert_eq!(style.justify, JustifyContent::Start);
    }

    #[test]
    fn parses_alignment_classes() {
        let style = UiStyle::parse("row items-center justify-between");

        assert_eq!(style.direction, Axis::Horizontal);
        assert_eq!(style.align, AlignItems::Center);
        assert_eq!(style.justify, JustifyContent::Between);
    }

    #[test]
    fn parses_grid_span_class() {
        let style = UiStyle::parse("grid grid-cols-4 col-span-3");

        assert_eq!(style.grid_cols, Some(4));
        assert_eq!(style.col_span, 3);
    }

    #[test]
    fn applies_responsive_classes_at_matching_widths() {
        let narrow = UiStyle::parse_for_width("grid grid-cols-1 md:grid-cols-4 md:gap-4", 640.0);
        let wide = UiStyle::parse_for_width("grid grid-cols-1 md:grid-cols-4 md:gap-4", 900.0);

        assert_eq!(narrow.grid_cols, Some(1));
        assert_eq!(narrow.gap, 0.0);
        assert_eq!(wide.grid_cols, Some(4));
        assert_eq!(wide.gap, 16.0);
    }

    #[test]
    fn resolves_semantic_and_tailwind_color_classes_from_shared_palette() {
        let semantic = UiStyle::parse("bg-panel text-muted");
        assert_eq!(
            semantic.bg,
            Some(UiStyleColor::semantic(UiColorToken::Panel))
        );
        assert_eq!(semantic.text, UiStyleColor::semantic(UiColorToken::Muted));
        assert_eq!(
            semantic
                .bg
                .expect("semantic background")
                .resolve(UiResolvedTheme::default()),
            UiResolvedTheme::default().colors.panel
        );

        let tailwind = UiStyle::parse("bg-slate-900 text-cyan-600");
        assert_eq!(
            tailwind.bg,
            Some(UiStyleColor::fixed(Color4::from_color(
                crate::TAILWIND.slate_900
            )))
        );
        assert_eq!(
            tailwind.text,
            UiStyleColor::fixed(Color4::from_color(crate::TAILWIND.cyan_600))
        );
    }

    #[test]
    fn parses_extracted_shadcn_aliases() {
        let style = UiStyle::parse(
            "rounded-2xl border-dashed ring-1 bg-card text-card-foreground text-muted-foreground",
        );

        assert!(style.border);
        assert_eq!(style.radius, 20.0);
        assert_eq!(style.bg, Some(UiStyleColor::semantic(UiColorToken::Panel)));
        assert_eq!(style.text, UiStyleColor::semantic(UiColorToken::Muted));

        let button = UiStyle::parse("bg-primary text-primary-foreground");
        assert_eq!(
            button.bg,
            Some(UiStyleColor::semantic(UiColorToken::Accent))
        );
        assert_eq!(
            button.text,
            UiStyleColor::semantic(UiColorToken::AccentText)
        );

        let icon = UiStyle::parse("rounded-4xl bg-secondary text-background");
        assert_eq!(icon.radius, 999.0);
        assert_eq!(icon.bg, Some(UiStyleColor::semantic(UiColorToken::Row)));
        assert_eq!(icon.text, UiStyleColor::semantic(UiColorToken::Bg));

        let selected = UiStyle::parse("bg-accent text-accent-foreground");
        assert_eq!(
            selected.bg,
            Some(UiStyleColor::semantic(UiColorToken::Accent))
        );
        assert_eq!(
            selected.text,
            UiStyleColor::semantic(UiColorToken::AccentText)
        );

        let chart = UiStyle::parse("bg-chart-2 text-chart-5");
        assert_eq!(
            chart.bg,
            Some(UiStyleColor::semantic(UiColorToken::Success))
        );
        assert_eq!(chart.text, UiStyleColor::semantic(UiColorToken::Danger));
    }

    #[test]
    fn unknown_classes_are_ignored_without_mutating_defaults() {
        let style = UiStyle::parse("hover:bg-slate-900 made-up-class");
        assert_eq!(style.direction, Axis::Vertical);
        assert_eq!(style.bg, None);
        assert_eq!(style.text, UiStyleColor::semantic(UiColorToken::Text));
        assert_eq!(style.width, None);
        assert_eq!(style.height, None);
    }

    #[test]
    fn parses_overflow_clip_classes() {
        assert!(UiStyle::parse("overflow-hidden").clip);
        assert!(UiStyle::parse("overflow-clip").clip);
        assert!(!UiStyle::parse("overflow-visible").clip);
    }
}
