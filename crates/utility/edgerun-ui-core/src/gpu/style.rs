use super::{palette, Color4, UiRect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
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
    pub col_span: u16,
    pub bg: Option<Color4>,
    pub text: Color4,
    pub border: bool,
    pub radius: f32,
    pub truncate: bool,
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
            col_span: 1,
            bg: None,
            text: palette::TEXT,
            border: false,
            radius: 0.0,
            truncate: false,
        }
    }
}

impl UiStyle {
    pub fn parse(classes: &str) -> Self {
        let mut style = Self::default();
        for class in classes.split_whitespace() {
            style.apply_class(class);
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
            "border" => self.border = true,
            _ => return false,
        }
        true
    }

    fn apply_grid_class(&mut self, class: &str) -> bool {
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
            self.bg = Some(color);
            return true;
        }
        if let Some(color) = class.strip_prefix("text-").and_then(tailwind_class_color) {
            self.text = color;
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

fn spacing_value(value: &str) -> Option<f32> {
    value.parse::<f32>().ok().map(|n| n * SPACING_UNIT)
}

fn size_value(value: &str) -> Option<f32> {
    match value {
        "full" => Some(FILL_PARENT),
        _ => value.parse::<f32>().ok().map(|n| n * SPACING_UNIT),
    }
}

fn semantic_bg_color(class: &str) -> Option<Color4> {
    match class {
        "bg-bg" => Some(palette::BG),
        "bg-sidebar" => Some(palette::SIDEBAR),
        "bg-topbar" => Some(palette::TOPBAR),
        "bg-panel" => Some(palette::PANEL),
        "bg-row" => Some(palette::ROW),
        "bg-active" => Some(palette::ACTIVE_ROW),
        "bg-composer" => Some(palette::COMPOSER),
        "bg-accent" => Some(palette::ACCENT),
        _ => None,
    }
}

fn semantic_text_color(class: &str) -> Option<Color4> {
    match class {
        "text-primary" | "text-text" => Some(palette::TEXT),
        "text-muted" => Some(palette::MUTED),
        "text-accent" => Some(palette::ACCENT),
        "text-green" => Some(palette::GREEN),
        "text-violet" => Some(palette::VIOLET),
        "text-amber" => Some(palette::AMBER),
        "text-danger" => Some(palette::DANGER),
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
        assert_eq!(style.col_span, 1);
    }

    #[test]
    fn parses_grid_span_class() {
        let style = UiStyle::parse("col-span-3");

        assert_eq!(style.col_span, 3);
    }

    #[test]
    fn resolves_semantic_and_tailwind_color_classes_from_shared_palette() {
        let semantic = UiStyle::parse("bg-panel text-muted");
        assert_eq!(semantic.bg, Some(palette::PANEL));
        assert_eq!(semantic.text, palette::MUTED);

        let tailwind = UiStyle::parse("bg-slate-900 text-cyan-600");
        assert_eq!(
            tailwind.bg,
            Some(Color4::from_color(crate::TAILWIND.slate_900))
        );
        assert_eq!(tailwind.text, Color4::from_color(crate::TAILWIND.cyan_600));
    }

    #[test]
    fn unknown_classes_are_ignored_without_mutating_defaults() {
        let style = UiStyle::parse("hover:bg-slate-900 made-up-class");
        assert_eq!(style.direction, Axis::Vertical);
        assert_eq!(style.bg, None);
        assert_eq!(style.text, palette::TEXT);
        assert_eq!(style.width, None);
        assert_eq!(style.height, None);
    }
}
