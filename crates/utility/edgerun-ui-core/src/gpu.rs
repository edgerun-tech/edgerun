//! Shared GPU UI scene primitives and a small native OpenGL renderer.
//!
//! The scene types are platform neutral and are intended to be consumed by both
//! native OpenGL/EGL/SDL hosts and browser WebGL hosts. The native GL renderer
//! below is only one backend for the scene.

use std::string::String;
use std::vec::Vec;

#[cfg(feature = "fontdue-text")]
use std::collections::HashMap;

pub mod components;
pub mod style;
pub use components::{
    bar_chart, field, menu_item, metric_card, panel_header, slider, text_area, transaction_row,
    BarChart, Field, MenuItem, MetricCard, PanelHeader, Slider, TextArea, TransactionRow, UiGrid,
    UiStack,
};
pub use style::{AlignItems, Axis, JustifyContent, UiStyle};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color4 {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color4 {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn from_color(color: crate::Color) -> Self {
        Self::rgba(
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
            color.a as f32 / 255.0,
        )
    }

    pub const fn from_color_alpha(color: crate::Color, a: f32) -> Self {
        Self::rgba(
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
            a,
        )
    }

    pub const fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RectMode {
    Fill,
    Shadow,
    Border,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
    pub color: Color4,
    pub mode: RectMode,
    pub shadow: f32,
}

impl GpuRect {
    pub const fn fill(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            color,
            mode: RectMode::Fill,
            shadow: 0.0,
        }
    }

    pub const fn shadow(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        color: Color4,
        shadow: f32,
    ) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            color,
            mode: RectMode::Shadow,
            shadow,
        }
    }

    pub const fn border(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            color,
            mode: RectMode::Border,
            shadow: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GpuScene {
    pub clear: Color4,
    rects: Vec<GpuRect>,
    hits: Vec<GpuHit>,
    #[cfg(feature = "fontdue-text")]
    text_quads: Vec<TextQuad>,
}

impl GpuScene {
    pub fn new(clear: Color4) -> Self {
        Self {
            clear,
            rects: Vec::new(),
            hits: Vec::new(),
            #[cfg(feature = "fontdue-text")]
            text_quads: Vec::new(),
        }
    }

    pub fn clear_rects(&mut self) {
        self.rects.clear();
        self.hits.clear();
        #[cfg(feature = "fontdue-text")]
        self.text_quads.clear();
    }

    pub fn push_rect(&mut self, rect: GpuRect) {
        self.rects.push(rect);
    }

    pub fn push_hit(&mut self, hit: GpuHit) {
        self.hits.push(hit);
    }

    pub fn push_text(&mut self, mut x: f32, y: f32, text: &str, scale: f32, color: Color4) {
        let cell = scale.max(1.0);
        let step = cell * 6.0;
        let start_x = x;
        for ch in text.chars() {
            match ch {
                '\n' => {
                    x = start_x;
                }
                '\r' => {}
                ' ' => x += step,
                _ => {
                    let glyph = glyph5x7(ch);
                    for (row, bits) in glyph.iter().copied().enumerate() {
                        for col in 0..5 {
                            if ((bits >> (4 - col)) & 1) == 0 {
                                continue;
                            }
                            self.push_rect(GpuRect::fill(
                                x + col as f32 * cell,
                                y + row as f32 * cell,
                                cell,
                                cell,
                                0.0,
                                color,
                            ));
                        }
                    }
                    x += step;
                }
            }
        }
    }

    pub fn rects(&self) -> &[GpuRect] {
        &self.rects
    }

    pub fn hits(&self) -> &[GpuHit] {
        &self.hits
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<GpuHit> {
        self.hits
            .iter()
            .rev()
            .copied()
            .find(|hit| hit.contains(x, y))
    }

    pub fn apply_color_scheme(&mut self, scheme: UiColorScheme) {
        if scheme == UiColorScheme::Dark {
            return;
        }
        let from = SchemePalette::dark();
        let to = scheme.palette();
        self.clear = remap_scheme_color(self.clear, from, to);
        for rect in &mut self.rects {
            rect.color = remap_scheme_color(rect.color, from, to);
        }
        #[cfg(feature = "fontdue-text")]
        for quad in &mut self.text_quads {
            quad.color = remap_scheme_color(quad.color, from, to);
        }
    }

    #[cfg(feature = "fontdue-text")]
    pub fn push_font_text(&mut self, atlas: &FontAtlas, x: f32, y: f32, text: &str, color: Color4) {
        atlas.layout_text(self, x, y, text, color);
    }

    #[cfg(feature = "fontdue-text")]
    pub fn text_quads(&self) -> &[TextQuad] {
        &self.text_quads
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiColorScheme {
    #[default]
    Dark,
    Light,
    Terminal,
}

impl UiColorScheme {
    pub const fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Light,
            2 => Self::Terminal,
            _ => Self::Dark,
        }
    }

    pub const fn code(self) -> u32 {
        match self {
            Self::Dark => 0,
            Self::Light => 1,
            Self::Terminal => 2,
        }
    }

    fn palette(self) -> SchemePalette {
        match self {
            Self::Dark => SchemePalette::dark(),
            Self::Light => SchemePalette::light(),
            Self::Terminal => SchemePalette::terminal(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitKind {
    Contact,
    Composer,
    Send,
    Button,
    Tab,
    Toggle,
    ListRow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuHit {
    pub kind: HitKind,
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl GpuHit {
    pub const fn new(kind: HitKind, id: u32, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            kind,
            id,
            x,
            y,
            w,
            h,
        }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.w && y <= self.y + self.h
    }
}

#[cfg(feature = "fontdue-text")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextQuad {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub color: Color4,
}

#[cfg(feature = "fontdue-text")]
#[derive(Clone, Copy, Debug)]
struct AtlasGlyph {
    uv: [f32; 4],
    size: [f32; 2],
    bearing: [f32; 2],
    advance: f32,
}

#[cfg(feature = "fontdue-text")]
#[derive(Clone, Debug)]
pub struct FontAtlas {
    pub width: u32,
    pub height: u32,
    pub alpha: Vec<u8>,
    glyphs: HashMap<char, AtlasGlyph>,
    px: f32,
}

#[cfg(feature = "fontdue-text")]
impl FontAtlas {
    pub fn from_font_bytes(bytes: &[u8], px: f32) -> Result<Self, String> {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|_| "font parse failed".to_string())?;
        Ok(Self::build(&font, &ascii_chars(), px))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_inter(px: f32) -> Result<Self, String> {
        let path = crate::font::find_best_ui_font().ok_or_else(|| {
            "Inter font not found; set EDGE_UI_FONT=/path/to/Inter.ttf".to_string()
        })?;
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        Self::from_font_bytes(&bytes, px)
    }

    fn build(font: &fontdue::Font, chars: &[char], px: f32) -> Self {
        let width = 1024u32;
        let height = 1024u32;
        let mut alpha = vec![0u8; (width * height) as usize];
        let mut glyphs = HashMap::new();
        let mut x = 2u32;
        let mut y = 2u32;
        let mut row_h = 0u32;

        for &ch in chars {
            let (metrics, bitmap) = font.rasterize(ch, px);
            if metrics.width == 0 || metrics.height == 0 {
                glyphs.insert(
                    ch,
                    AtlasGlyph {
                        uv: [0.0; 4],
                        size: [0.0, 0.0],
                        bearing: [metrics.xmin as f32, metrics.ymin as f32],
                        advance: metrics.advance_width,
                    },
                );
                continue;
            }

            let gw = metrics.width as u32;
            let gh = metrics.height as u32;
            if x + gw + 2 >= width {
                x = 2;
                y += row_h + 2;
                row_h = 0;
            }
            if y + gh + 2 >= height {
                break;
            }

            for gy in 0..gh {
                for gx in 0..gw {
                    alpha[((y + gy) * width + x + gx) as usize] = bitmap[(gy * gw + gx) as usize];
                }
            }

            glyphs.insert(
                ch,
                AtlasGlyph {
                    uv: [
                        x as f32 / width as f32,
                        y as f32 / height as f32,
                        (x + gw) as f32 / width as f32,
                        (y + gh) as f32 / height as f32,
                    ],
                    size: [gw as f32, gh as f32],
                    bearing: [metrics.xmin as f32, metrics.ymin as f32],
                    advance: metrics.advance_width,
                },
            );
            x += gw + 2;
            row_h = row_h.max(gh);
        }

        Self {
            width,
            height,
            alpha,
            glyphs,
            px,
        }
    }

    fn layout_text(&self, scene: &mut GpuScene, mut x: f32, y: f32, text: &str, color: Color4) {
        let baseline = y + self.px * 0.82;
        for ch in text.chars() {
            if ch == '\n' {
                continue;
            }
            let Some(glyph) = self
                .glyphs
                .get(&ch)
                .or_else(|| self.glyphs.get(&ch.to_ascii_uppercase()))
            else {
                x += self.px * 0.32;
                continue;
            };
            if glyph.size[0] > 0.0 && glyph.size[1] > 0.0 {
                scene.text_quads.push(TextQuad {
                    x: x + glyph.bearing[0],
                    y: baseline - glyph.bearing[1] - glyph.size[1],
                    w: glyph.size[0],
                    h: glyph.size[1],
                    u0: glyph.uv[0],
                    v0: glyph.uv[1],
                    u1: glyph.uv[2],
                    v1: glyph.uv[3],
                    color,
                });
            }
            x += glyph.advance.max(self.px * 0.28);
        }
    }

    pub fn text_width(&self, text: &str) -> f32 {
        text.chars()
            .map(|ch| {
                self.glyphs
                    .get(&ch)
                    .or_else(|| self.glyphs.get(&ch.to_ascii_uppercase()))
                    .map(|glyph| glyph.advance.max(self.px * 0.28))
                    .unwrap_or(self.px * 0.32)
            })
            .sum()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ChatShellMetrics {
    pub sidebar_w: f32,
    pub topbar_h: f32,
    pub composer_h: f32,
    pub pad: f32,
}

impl Default for ChatShellMetrics {
    fn default() -> Self {
        Self {
            sidebar_w: 284.0,
            topbar_h: 58.0,
            composer_h: 86.0,
            pad: 18.0,
        }
    }
}

pub struct UiPainter<'a, 'font> {
    scene: &'a mut GpuScene,
    #[cfg(feature = "fontdue-text")]
    atlas: Option<&'font FontAtlas>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl UiRect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn inset(self, dx: f32, dy: f32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            w: (self.w - dx * 2.0).max(0.0),
            h: (self.h - dy * 2.0).max(0.0),
        }
    }

    pub fn right(self, w: f32) -> Self {
        Self {
            x: self.x + self.w - w,
            y: self.y,
            w,
            h: self.h,
        }
    }

    pub fn bottom(self, h: f32) -> Self {
        Self {
            x: self.x,
            y: self.y + self.h - h,
            w: self.w,
            h,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Clone, Debug)]
pub enum UiNodeKind {
    Row,
    Column,
    Grid {
        columns: u16,
    },
    Card,
    Text(String),
    Badge {
        label: String,
        color: Color4,
    },
    Button {
        label: String,
        id: u32,
        style: ButtonStyle,
    },
    PanelHeader {
        title: String,
        subtitle: String,
        action: Option<(String, u32)>,
    },
    MetricCard {
        title: String,
        value: String,
        detail: String,
        progress: Option<f32>,
        accent: Color4,
    },
    Field {
        label: String,
        value: String,
        helper: String,
        focused: bool,
    },
    TextArea {
        label: String,
        value: String,
        focused: bool,
    },
    Slider {
        label: String,
        value: f32,
        min_label: String,
        max_label: String,
        accent: Color4,
        id: u32,
    },
    BarChart {
        title: String,
        subtitle: String,
        labels: Vec<String>,
        values: Vec<f32>,
        accent: Color4,
    },
    TransactionRow {
        title: String,
        subtitle: String,
        date: String,
        amount: String,
        positive: bool,
        id: u32,
    },
    MenuItem {
        label: String,
        detail: String,
        badge: String,
        selected: bool,
        accent: Color4,
        id: u32,
    },
    Divider,
    Spacer,
}

#[derive(Clone, Debug)]
pub struct UiNode {
    kind: UiNodeKind,
    style: UiStyle,
    children: Vec<UiNode>,
}

impl UiNode {
    pub fn row(classes: &str) -> Self {
        let mut style = UiStyle::parse(classes);
        style.direction = Axis::Horizontal;
        Self {
            kind: UiNodeKind::Row,
            style,
            children: Vec::new(),
        }
    }

    pub fn column(classes: &str) -> Self {
        let mut style = UiStyle::parse(classes);
        style.direction = Axis::Vertical;
        Self {
            kind: UiNodeKind::Column,
            style,
            children: Vec::new(),
        }
    }

    pub fn grid(classes: &str, columns: u16) -> Self {
        Self {
            kind: UiNodeKind::Grid {
                columns: columns.max(1),
            },
            style: UiStyle::parse(classes),
            children: Vec::new(),
        }
    }

    pub fn grid_auto(classes: &str) -> Self {
        let style = UiStyle::parse(classes);
        Self {
            kind: UiNodeKind::Grid {
                columns: style.grid_cols.unwrap_or(1),
            },
            style,
            children: Vec::new(),
        }
    }

    pub fn card(classes: &str) -> Self {
        Self {
            kind: UiNodeKind::Card,
            style: UiStyle::parse(classes),
            children: Vec::new(),
        }
    }

    pub fn text(value: &str) -> Self {
        Self {
            kind: UiNodeKind::Text(value.to_string()),
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn badge(label: &str, color: Color4) -> Self {
        Self {
            kind: UiNodeKind::Badge {
                label: label.to_string(),
                color,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn button(label: &str, id: u32, style: ButtonStyle) -> Self {
        Self {
            kind: UiNodeKind::Button {
                label: label.to_string(),
                id,
                style,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn panel_header(title: &str) -> Self {
        Self {
            kind: UiNodeKind::PanelHeader {
                title: title.to_string(),
                subtitle: String::new(),
                action: None,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn metric_card(title: &str, value: &str) -> Self {
        Self {
            kind: UiNodeKind::MetricCard {
                title: title.to_string(),
                value: value.to_string(),
                detail: String::new(),
                progress: None,
                accent: palette::ACCENT,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn field(label: &str, value: &str) -> Self {
        Self {
            kind: UiNodeKind::Field {
                label: label.to_string(),
                value: value.to_string(),
                helper: String::new(),
                focused: false,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn text_area(label: &str, value: &str) -> Self {
        Self {
            kind: UiNodeKind::TextArea {
                label: label.to_string(),
                value: value.to_string(),
                focused: false,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn slider(label: &str, value: f32, id: u32) -> Self {
        Self {
            kind: UiNodeKind::Slider {
                label: label.to_string(),
                value,
                min_label: String::new(),
                max_label: String::new(),
                accent: palette::ACCENT,
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn bar_chart(
        title: &str,
        labels: impl IntoIterator<Item = String>,
        values: impl IntoIterator<Item = f32>,
    ) -> Self {
        Self {
            kind: UiNodeKind::BarChart {
                title: title.to_string(),
                subtitle: String::new(),
                labels: labels.into_iter().collect(),
                values: values.into_iter().collect(),
                accent: palette::ACCENT,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn bar_chart_labels(title: &str, labels: &[&str], values: &[f32]) -> Self {
        Self::bar_chart(
            title,
            labels.iter().copied().map(str::to_string),
            values.iter().copied(),
        )
    }

    pub fn transaction_row(title: &str, amount: &str, id: u32) -> Self {
        Self {
            kind: UiNodeKind::TransactionRow {
                title: title.to_string(),
                subtitle: String::new(),
                date: String::new(),
                amount: amount.to_string(),
                positive: false,
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn menu_item(label: &str, id: u32) -> Self {
        Self {
            kind: UiNodeKind::MenuItem {
                label: label.to_string(),
                detail: String::new(),
                badge: String::new(),
                selected: false,
                accent: palette::ACCENT,
                id,
            },
            style: UiStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn divider(classes: &str) -> Self {
        Self {
            kind: UiNodeKind::Divider,
            style: UiStyle::parse(classes),
            children: Vec::new(),
        }
    }

    pub fn spacer(classes: &str) -> Self {
        Self {
            kind: UiNodeKind::Spacer,
            style: UiStyle::parse(classes),
            children: Vec::new(),
        }
    }

    pub fn class(mut self, classes: &str) -> Self {
        let mut parsed = UiStyle::parse(classes);
        if matches!(self.kind, UiNodeKind::Row) {
            parsed.direction = Axis::Horizontal;
        }
        if let UiNodeKind::Grid { columns } = &mut self.kind {
            if let Some(grid_cols) = parsed.grid_cols {
                *columns = grid_cols;
            }
        }
        if self.style.col_span > 1 && parsed.col_span == 1 {
            parsed.col_span = self.style.col_span;
        }
        self.style = parsed;
        self
    }

    pub fn span(mut self, span: u16) -> Self {
        self.style.col_span = span.max(1);
        self
    }

    pub fn child(mut self, child: UiNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = UiNode>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn when(self, condition: bool, child: UiNode) -> Self {
        if condition {
            self.child(child)
        } else {
            self
        }
    }

    pub fn detail(mut self, value: &str) -> Self {
        match &mut self.kind {
            UiNodeKind::MetricCard { detail, .. } | UiNodeKind::MenuItem { detail, .. } => {
                *detail = value.to_string()
            }
            UiNodeKind::Field { helper, .. } => *helper = value.to_string(),
            UiNodeKind::TextArea { value: text, .. } => *text = value.to_string(),
            UiNodeKind::TransactionRow { subtitle, .. }
            | UiNodeKind::BarChart { subtitle, .. }
            | UiNodeKind::PanelHeader { subtitle, .. } => *subtitle = value.to_string(),
            _ => {}
        }
        self
    }

    pub fn action(mut self, label: &str, id: u32) -> Self {
        if let UiNodeKind::PanelHeader { action, .. } = &mut self.kind {
            *action = Some((label.to_string(), id));
        }
        self
    }

    pub fn badge_text(mut self, value: &str) -> Self {
        if let UiNodeKind::MenuItem { badge, .. } = &mut self.kind {
            *badge = value.to_string();
        }
        self
    }

    pub fn progress(mut self, value: f32) -> Self {
        if let UiNodeKind::MetricCard { progress, .. } = &mut self.kind {
            *progress = Some(value);
        }
        self
    }

    pub fn accent(mut self, color: Color4) -> Self {
        match &mut self.kind {
            UiNodeKind::MetricCard { accent, .. }
            | UiNodeKind::Slider { accent, .. }
            | UiNodeKind::BarChart { accent, .. }
            | UiNodeKind::MenuItem { accent, .. } => *accent = color,
            UiNodeKind::Badge {
                color: badge_color, ..
            } => *badge_color = color,
            _ => {}
        }
        self
    }

    pub fn range_labels(mut self, min_label: &str, max_label: &str) -> Self {
        if let UiNodeKind::Slider {
            min_label: min_node_label,
            max_label: max_node_label,
            ..
        } = &mut self.kind
        {
            *min_node_label = min_label.to_string();
            *max_node_label = max_label.to_string();
        }
        self
    }

    pub fn date(mut self, date: &str) -> Self {
        if let UiNodeKind::TransactionRow {
            date: node_date, ..
        } = &mut self.kind
        {
            *node_date = date.to_string();
        }
        self
    }

    pub fn positive(mut self, positive: bool) -> Self {
        if let UiNodeKind::TransactionRow {
            positive: node_positive,
            ..
        } = &mut self.kind
        {
            *node_positive = positive;
        }
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        match &mut self.kind {
            UiNodeKind::Field {
                focused: node_focused,
                ..
            }
            | UiNodeKind::TextArea {
                focused: node_focused,
                ..
            } => *node_focused = focused,
            _ => {}
        }
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        if let UiNodeKind::MenuItem {
            selected: node_selected,
            ..
        } = &mut self.kind
        {
            *node_selected = selected;
        }
        self
    }

    pub fn render(&self, ui: &mut UiPainter<'_, '_>, bounds: UiRect) {
        let rect = self.style.layout_rect(bounds);
        match &self.kind {
            UiNodeKind::Text(value) => {
                ui.bounded_label(rect.x, rect.y, rect.w, value, 2.0, self.style.text);
            }
            UiNodeKind::Badge { label, color } => {
                ui.badge(rect.x, rect.y, label, *color);
            }
            UiNodeKind::Button { label, id, style } => {
                let h = self.style.height.unwrap_or(34.0).min(rect.h);
                ui.button(
                    UiRect::new(rect.x, rect.y, rect.w, h),
                    label,
                    *style,
                    *id,
                    true,
                );
            }
            UiNodeKind::PanelHeader {
                title,
                subtitle,
                action,
            } => {
                self::components::panel_header(
                    ui,
                    rect,
                    self::components::PanelHeader {
                        title,
                        subtitle,
                        action: action.as_ref().map(|(label, id)| (label.as_str(), *id)),
                    },
                );
            }
            UiNodeKind::MetricCard {
                title,
                value,
                detail,
                progress,
                accent,
            } => {
                self::components::metric_card(
                    ui,
                    rect,
                    self::components::MetricCard {
                        title,
                        value,
                        detail,
                        progress: *progress,
                        accent: *accent,
                    },
                );
            }
            UiNodeKind::Field {
                label,
                value,
                helper,
                focused,
            } => {
                self::components::field(
                    ui,
                    rect,
                    self::components::Field {
                        label,
                        value,
                        helper,
                        focused: *focused,
                    },
                );
            }
            UiNodeKind::TextArea {
                label,
                value,
                focused,
            } => {
                self::components::text_area(
                    ui,
                    rect,
                    self::components::TextArea {
                        label,
                        value,
                        focused: *focused,
                    },
                );
            }
            UiNodeKind::Slider {
                label,
                value,
                min_label,
                max_label,
                accent,
                id,
            } => {
                self::components::slider(
                    ui,
                    rect,
                    self::components::Slider {
                        label,
                        value: *value,
                        min_label,
                        max_label,
                        accent: *accent,
                        id: *id,
                    },
                );
            }
            UiNodeKind::BarChart {
                title,
                subtitle,
                labels,
                values,
                accent,
            } => {
                let label_refs = labels.iter().map(String::as_str).collect::<Vec<_>>();
                self::components::bar_chart(
                    ui,
                    rect,
                    self::components::BarChart {
                        title,
                        subtitle,
                        labels: &label_refs,
                        values,
                        accent: *accent,
                    },
                );
            }
            UiNodeKind::TransactionRow {
                title,
                subtitle,
                date,
                amount,
                positive,
                id,
            } => {
                self::components::transaction_row(
                    ui,
                    rect,
                    self::components::TransactionRow {
                        title,
                        subtitle,
                        date,
                        amount,
                        positive: *positive,
                        id: *id,
                    },
                );
            }
            UiNodeKind::MenuItem {
                label,
                detail,
                badge,
                selected,
                accent,
                id,
            } => {
                self::components::menu_item(
                    ui,
                    rect,
                    self::components::MenuItem {
                        label,
                        detail,
                        badge,
                        selected: *selected,
                        accent: *accent,
                        id: *id,
                    },
                );
            }
            UiNodeKind::Divider => {
                let axis = if rect.w >= rect.h {
                    Axis::Horizontal
                } else {
                    Axis::Vertical
                };
                ui.divider(rect.x, rect.y, rect.w.max(rect.h), axis);
            }
            UiNodeKind::Spacer => {}
            UiNodeKind::Row | UiNodeKind::Column | UiNodeKind::Grid { .. } | UiNodeKind::Card => {
                if let Some(bg) = self.style.bg {
                    if matches!(self.kind, UiNodeKind::Card) {
                        ui.card(rect.x, rect.y, rect.w, rect.h, self.style.radius, bg);
                    } else {
                        ui.scene.push_rect(GpuRect::fill(
                            rect.x,
                            rect.y,
                            rect.w,
                            rect.h,
                            self.style.radius,
                            bg,
                        ));
                        if self.style.border {
                            ui.scene.push_rect(GpuRect::border(
                                rect.x,
                                rect.y,
                                rect.w,
                                rect.h,
                                self.style.radius,
                                palette::BORDER,
                            ));
                        }
                    }
                }
                if let UiNodeKind::Grid { columns } = &self.kind {
                    render_grid_children(ui, rect, &self.style, *columns, &self.children);
                } else {
                    render_children(ui, rect, &self.style, &self.children);
                }
            }
        }
    }
}

pub fn row(classes: &str) -> UiNode {
    UiNode::row(classes)
}

pub fn column(classes: &str) -> UiNode {
    UiNode::column(classes)
}

pub fn grid(classes: &str, columns: u16) -> UiNode {
    UiNode::grid(classes, columns)
}

pub fn grid_auto(classes: &str) -> UiNode {
    UiNode::grid_auto(classes)
}

pub fn card(classes: &str) -> UiNode {
    UiNode::card(classes)
}

pub fn text(value: &str) -> UiNode {
    UiNode::text(value)
}

pub fn badge(label: &str, color: Color4) -> UiNode {
    UiNode::badge(label, color)
}

pub fn button(label: &str, id: u32, style: ButtonStyle) -> UiNode {
    UiNode::button(label, id, style)
}

pub fn header(title: &str) -> UiNode {
    UiNode::panel_header(title)
}

pub fn metric(title: &str, value: &str) -> UiNode {
    UiNode::metric_card(title, value)
}

pub fn field_node(label: &str, value: &str) -> UiNode {
    UiNode::field(label, value)
}

pub fn text_area_node(label: &str, value: &str) -> UiNode {
    UiNode::text_area(label, value)
}

pub fn slider_node(label: &str, value: f32, id: u32) -> UiNode {
    UiNode::slider(label, value, id)
}

pub fn bar_chart_node(
    title: &str,
    labels: impl IntoIterator<Item = String>,
    values: impl IntoIterator<Item = f32>,
) -> UiNode {
    UiNode::bar_chart(title, labels, values)
}

pub fn bar_chart_labels(title: &str, labels: &[&str], values: &[f32]) -> UiNode {
    UiNode::bar_chart_labels(title, labels, values)
}

pub fn transaction_node(title: &str, amount: &str, id: u32) -> UiNode {
    UiNode::transaction_row(title, amount, id)
}

pub fn menu_item_node(label: &str, id: u32) -> UiNode {
    UiNode::menu_item(label, id)
}

pub fn divider(classes: &str) -> UiNode {
    UiNode::divider(classes)
}

pub fn spacer(classes: &str) -> UiNode {
    UiNode::spacer(classes)
}

#[macro_export]
macro_rules! gpu_ui {
    ($node:expr) => {
        $node
    };
    ($node:expr, [ $($child:expr),* $(,)? ]) => {{
        let mut node = $node;
        $(
            node = node.child($child);
        )*
        node
    }};
}

impl<'a, 'font> UiPainter<'a, 'font> {
    pub fn new(scene: &'a mut GpuScene) -> Self {
        Self {
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas: None,
        }
    }

    #[cfg(feature = "fontdue-text")]
    pub fn with_font(scene: &'a mut GpuScene, atlas: &'font FontAtlas) -> Self {
        Self {
            scene,
            atlas: Some(atlas),
        }
    }

    pub fn label(&mut self, x: f32, y: f32, text: &str, scale: f32, color: Color4) {
        push_label(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            text,
            scale,
            color,
        );
    }

    pub fn bounded_label(
        &mut self,
        x: f32,
        y: f32,
        max_w: f32,
        text: &str,
        scale: f32,
        color: Color4,
    ) {
        push_bounded_label(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            max_w,
            text,
            scale,
            color,
        );
    }

    pub fn panel(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) {
        panel(self.scene, x, y, w, h, radius, color);
    }

    pub fn card(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) {
        soft_card(self.scene, x, y, w, h, radius, color);
    }

    pub fn fill_rect(&mut self, rect: UiRect, radius: f32, color: Color4) {
        self.scene
            .push_rect(GpuRect::fill(rect.x, rect.y, rect.w, rect.h, radius, color));
    }

    pub fn border_rect(&mut self, rect: UiRect, radius: f32, color: Color4) {
        self.scene.push_rect(GpuRect::border(
            rect.x, rect.y, rect.w, rect.h, radius, color,
        ));
    }

    pub fn pill(&mut self, x: f32, y: f32, w: f32, label: &str, color: Color4) {
        draw_pill(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            w,
            label,
            color,
        );
    }

    pub fn hit(&mut self, kind: HitKind, id: u32, x: f32, y: f32, w: f32, h: f32) {
        self.scene.push_hit(GpuHit::new(kind, id, x, y, w, h));
    }

    pub fn divider(&mut self, x: f32, y: f32, w: f32, axis: Axis) {
        match axis {
            Axis::Horizontal => {
                self.scene
                    .push_rect(GpuRect::fill(x, y, w, 1.0, 0.0, palette::BORDER));
            }
            Axis::Vertical => {
                self.scene
                    .push_rect(GpuRect::fill(x, y, 1.0, w, 0.0, palette::BORDER));
            }
        }
    }

    pub fn badge(&mut self, x: f32, y: f32, label: &str, color: Color4) -> f32 {
        let w = component_label_width(
            label,
            2.0,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
        ) + 18.0;
        self.scene
            .push_rect(GpuRect::fill(x, y, w, 22.0, 11.0, color.with_alpha(0.16)));
        self.scene
            .push_rect(GpuRect::border(x, y, w, 22.0, 11.0, color.with_alpha(0.42)));
        self.bounded_label(x + 9.0, y + 5.0, w - 18.0, label, 2.0, color);
        w
    }

    pub fn avatar(&mut self, x: f32, y: f32, size: f32, label: &str, color: Color4, online: bool) {
        let radius = size * 0.5;
        self.scene.push_rect(GpuRect::fill(
            x,
            y,
            size,
            size,
            radius,
            color.with_alpha(0.22),
        ));
        self.scene.push_rect(GpuRect::border(
            x,
            y,
            size,
            size,
            radius,
            color.with_alpha(0.54),
        ));
        self.bounded_label(
            x + size * 0.34,
            y + size * 0.30,
            size * 0.42,
            contact_initial(label),
            2.0,
            color,
        );
        self.status_dot(x + size - 8.0, y + size - 8.0, online);
    }

    pub fn status_dot(&mut self, x: f32, y: f32, online: bool) {
        self.scene.push_rect(GpuRect::fill(
            x,
            y,
            8.0,
            8.0,
            4.0,
            if online {
                palette::GREEN
            } else {
                palette::MUTED
            },
        ));
    }

    pub fn button(&mut self, rect: UiRect, label: &str, style: ButtonStyle, id: u32, active: bool) {
        self.hit(HitKind::Button, id, rect.x, rect.y, rect.w, rect.h);
        let (fill, border, text) = match style {
            ButtonStyle::Primary => (palette::ACCENT, palette::ACCENT, palette::ACCENT_TEXT),
            ButtonStyle::Secondary => (palette::ROW, palette::BORDER, palette::TEXT),
            ButtonStyle::Ghost => (
                palette::PANEL.with_alpha(0.0),
                palette::BORDER,
                palette::MUTED,
            ),
            ButtonStyle::Danger => (
                palette::DANGER.with_alpha(0.18),
                palette::DANGER,
                palette::DANGER,
            ),
        };
        let fill = if active {
            fill
        } else {
            fill.with_alpha(fill.a * 0.74)
        };
        self.scene
            .push_rect(GpuRect::fill(rect.x, rect.y, rect.w, rect.h, 10.0, fill));
        self.scene.push_rect(GpuRect::border(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            10.0,
            border.with_alpha(if active { 0.72 } else { 0.42 }),
        ));
        let label_w = component_label_width(
            label,
            2.0,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
        );
        self.bounded_label(
            rect.x + ((rect.w - label_w) * 0.5).max(10.0),
            rect.y + (rect.h - 14.0) * 0.5,
            (rect.w - 20.0).max(0.0),
            label,
            2.0,
            text,
        );
    }

    pub fn icon_button(&mut self, rect: UiRect, glyph: &str, id: u32, active: bool) {
        self.button(rect, glyph, ButtonStyle::Secondary, id, active);
    }

    pub fn input_field(&mut self, rect: UiRect, placeholder: &str, focused: bool) {
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            12.0,
            palette::COMPOSER,
        ));
        self.scene.push_rect(GpuRect::border(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            12.0,
            if focused {
                palette::ACCENT
            } else {
                palette::BORDER
            },
        ));
        self.bounded_label(
            rect.x + 16.0,
            rect.y + 13.0,
            (rect.w - 32.0).max(0.0),
            placeholder,
            2.0,
            palette::MUTED,
        );
    }

    pub fn toggle(&mut self, x: f32, y: f32, on: bool, id: u32) {
        self.hit(HitKind::Toggle, id, x, y, 46.0, 24.0);
        let color = if on { palette::GREEN } else { palette::MUTED };
        self.scene.push_rect(GpuRect::fill(
            x,
            y,
            46.0,
            24.0,
            12.0,
            color.with_alpha(0.18),
        ));
        self.scene.push_rect(GpuRect::border(
            x,
            y,
            46.0,
            24.0,
            12.0,
            color.with_alpha(0.54),
        ));
        let knob_x = if on { x + 24.0 } else { x + 4.0 };
        self.scene
            .push_rect(GpuRect::fill(knob_x, y + 4.0, 16.0, 16.0, 8.0, color));
    }

    pub fn progress_bar(&mut self, rect: UiRect, fraction: f32, color: Color4) {
        let value = fraction.clamp(0.0, 1.0);
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            rect.h * 0.5,
            palette::ROW,
        ));
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w * value,
            rect.h,
            rect.h * 0.5,
            color,
        ));
    }

    pub fn segmented_tabs(&mut self, rect: UiRect, labels: &[&str], selected: usize, base_id: u32) {
        if labels.is_empty() {
            return;
        }
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            12.0,
            palette::ROW,
        ));
        self.scene.push_rect(GpuRect::border(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            12.0,
            palette::BORDER,
        ));
        let item_w = rect.w / labels.len() as f32;
        for (index, label) in labels.iter().enumerate() {
            let x = rect.x + index as f32 * item_w;
            let active = index == selected;
            self.hit(
                HitKind::Tab,
                base_id + index as u32,
                x,
                rect.y,
                item_w,
                rect.h,
            );
            if active {
                self.scene.push_rect(GpuRect::fill(
                    x + 3.0,
                    rect.y + 3.0,
                    item_w - 6.0,
                    rect.h - 6.0,
                    9.0,
                    palette::ACTIVE_ROW,
                ));
            }
            let label_w = component_label_width(
                label,
                2.0,
                #[cfg(feature = "fontdue-text")]
                self.atlas,
            );
            self.bounded_label(
                x + ((item_w - label_w) * 0.5).max(8.0),
                rect.y + (rect.h - 14.0) * 0.5,
                (item_w - 16.0).max(0.0),
                label,
                2.0,
                if active {
                    palette::TEXT
                } else {
                    palette::MUTED
                },
            );
        }
    }

    pub fn list_row(&mut self, rect: UiRect, title: &str, detail: &str, accent: Color4, id: u32) {
        self.hit(HitKind::ListRow, id, rect.x, rect.y, rect.w, rect.h);
        self.card(rect.x, rect.y, rect.w, rect.h, 10.0, palette::ROW);
        self.scene
            .push_rect(GpuRect::fill(rect.x, rect.y, 3.0, rect.h, 2.0, accent));
        self.bounded_label(
            rect.x + 16.0,
            rect.y + 10.0,
            (rect.w - 32.0).max(0.0),
            title,
            2.0,
            palette::TEXT,
        );
        self.bounded_label(
            rect.x + 16.0,
            rect.y + 31.0,
            (rect.w - 32.0).max(0.0),
            detail,
            2.0,
            palette::MUTED,
        );
    }

    pub fn scrollbar(&mut self, rect: UiRect, visible_fraction: f32, offset_fraction: f32) {
        let visible = visible_fraction.clamp(0.08, 1.0);
        let offset = offset_fraction.clamp(0.0, 1.0);
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            rect.w * 0.5,
            palette::ROW.with_alpha(0.42),
        ));
        let thumb_h = rect.h * visible;
        let thumb_y = rect.y + (rect.h - thumb_h) * offset;
        self.scene.push_rect(GpuRect::fill(
            rect.x,
            thumb_y,
            rect.w,
            thumb_h,
            rect.w * 0.5,
            palette::MUTED.with_alpha(0.74),
        ));
    }

    pub fn contact_row(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        contact: &UnifiedContact<'_>,
        selected: bool,
        id: u32,
    ) {
        self.hit(HitKind::Contact, id, x, y, w, 56.0);
        draw_contact_row(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            w,
            contact,
            selected,
        );
    }

    pub fn message_bubble(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        role: &str,
        body: &str,
        fill: Color4,
        accent: Color4,
    ) -> f32 {
        draw_message(
            self.scene,
            #[cfg(feature = "fontdue-text")]
            self.atlas,
            x,
            y,
            w,
            role,
            body,
            fill,
            accent,
        )
    }

    pub fn composer(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        placeholder: &str,
        active: bool,
        chips: &[(&str, Color4)],
    ) {
        self.hit(HitKind::Composer, 0, x, y, w, h);
        self.card(x, y, w, h, 16.0, palette::COMPOSER);
        self.scene.push_rect(GpuRect::border(
            x,
            y,
            w,
            h,
            16.0,
            if active {
                palette::ACCENT
            } else {
                palette::BORDER
            },
        ));
        self.bounded_label(
            x + 22.0,
            y + 24.0,
            (w - 96.0).max(0.0),
            placeholder,
            2.0,
            palette::MUTED,
        );

        let mut chip_x = x + 20.0;
        for (label, color) in chips.iter().copied() {
            let chip_w = component_label_width(
                label,
                2.0,
                #[cfg(feature = "fontdue-text")]
                self.atlas,
            ) + 24.0;
            if chip_x + chip_w > x + w - 78.0 {
                break;
            }
            self.pill(chip_x, y + h - 34.0, chip_w, label, color);
            chip_x += chip_w + 10.0;
        }

        self.hit(HitKind::Send, 0, x + w - 58.0, y + h - 56.0, 40.0, 38.0);
        self.scene.push_rect(GpuRect::fill(
            x + w - 58.0,
            y + h - 56.0,
            40.0,
            38.0,
            12.0,
            palette::ACCENT,
        ));
        self.bounded_label(
            x + w - 47.0,
            y + h - 45.0,
            24.0,
            ">",
            3.0,
            palette::ACCENT_TEXT,
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnifiedContactKind {
    Person,
    CodexClient,
    Node,
}

#[derive(Clone, Copy, Debug)]
pub struct UnifiedContact<'a> {
    pub name: &'a str,
    pub detail: &'a str,
    pub kind: UnifiedContactKind,
    pub unread: u16,
    pub online: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct UnifiedMessage<'a> {
    pub author: &'a str,
    pub body: &'a str,
    pub outgoing: bool,
    pub accent: Color4,
}

#[derive(Clone, Debug)]
pub struct UnifiedChatState<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub contacts: &'a [UnifiedContact<'a>],
    pub selected_contact: usize,
    pub messages: &'a [UnifiedMessage<'a>],
    pub composer_placeholder: &'a str,
    pub connected: bool,
}

impl<'a> UnifiedChatState<'a> {
    pub const fn empty() -> Self {
        Self {
            title: "EdgeRun Chat",
            subtitle: "contact book",
            contacts: &[],
            selected_contact: 0,
            messages: &[],
            composer_placeholder: "Select a contact to start a thread...",
            connected: false,
        }
    }
}

pub fn build_unified_chat_shell(scene: &mut GpuScene, width: f32, height: f32) {
    let state = UnifiedChatState::empty();
    build_unified_chat_shell_impl(
        scene,
        width,
        height,
        &state,
        #[cfg(feature = "fontdue-text")]
        None,
    );
}

#[cfg(feature = "fontdue-text")]
pub fn build_unified_chat_shell_with_font(
    scene: &mut GpuScene,
    atlas: &FontAtlas,
    width: f32,
    height: f32,
    state: &UnifiedChatState<'_>,
) {
    build_unified_chat_shell_impl(scene, width, height, state, Some(atlas));
}

fn build_unified_chat_shell_impl(
    scene: &mut GpuScene,
    width: f32,
    height: f32,
    state: &UnifiedChatState<'_>,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) {
    scene.clear = palette::BG;
    scene.clear_rects();
    let mut ui = UiPainter {
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
    };
    let m = ChatShellMetrics::default();
    let w = width.max(360.0);
    let h = height.max(320.0);
    let sidebar_w = if w < 760.0 { 0.0 } else { m.sidebar_w + 28.0 };
    let main_x = sidebar_w;
    let main_w = w - sidebar_w;

    if sidebar_w > 0.0 {
        ui.panel(0.0, 0.0, sidebar_w, h, 0.0, palette::SIDEBAR);
        ui.scene.push_rect(GpuRect::fill(
            0.0,
            0.0,
            sidebar_w,
            4.0,
            0.0,
            palette::ACCENT,
        ));
        ui.bounded_label(
            24.0,
            20.0,
            (sidebar_w - 138.0).max(0.0),
            state.title,
            3.0,
            palette::TEXT,
        );
        ui.bounded_label(
            24.0,
            50.0,
            (sidebar_w - 48.0).max(0.0),
            state.subtitle,
            2.0,
            palette::MUTED,
        );
        ui.pill(
            sidebar_w - 104.0,
            22.0,
            78.0,
            if state.connected { "relay" } else { "local" },
            if state.connected {
                palette::GREEN
            } else {
                palette::AMBER
            },
        );
        ui.scene.push_rect(GpuRect::fill(
            sidebar_w - 1.0,
            0.0,
            1.0,
            h,
            0.0,
            palette::BORDER,
        ));

        for (index, contact) in state.contacts.iter().enumerate() {
            let y = 88.0 + index as f32 * 68.0;
            let selected = index == state.selected_contact;
            ui.contact_row(16.0, y, sidebar_w - 32.0, contact, selected, index as u32);
        }
    }

    ui.panel(main_x, 0.0, main_w, m.topbar_h, 0.0, palette::TOPBAR);
    let active = state
        .contacts
        .get(state.selected_contact)
        .or_else(|| state.contacts.first());
    let active_name = active.map(|contact| contact.name).unwrap_or("Unified chat");
    let active_detail = active
        .map(|contact| contact.detail)
        .unwrap_or("contact thread");
    let title_w = if main_w > 620.0 {
        190.0
    } else {
        (main_w - 40.0).max(0.0)
    };
    ui.bounded_label(
        main_x + 20.0,
        14.0,
        title_w,
        active_name,
        2.0,
        palette::TEXT,
    );
    ui.bounded_label(
        main_x + 20.0,
        35.0,
        title_w,
        active_detail,
        2.0,
        palette::MUTED,
    );
    let tabs_w = 226.0_f32.min((main_w - 390.0).max(0.0));
    if tabs_w > 160.0 {
        ui.segmented_tabs(
            UiRect::new(main_x + 220.0, 13.0, tabs_w, 32.0),
            &["chat", "proofs", "files"],
            0,
            20,
        );
    }
    ui.pill(
        main_x + main_w - 322.0,
        15.0,
        132.0,
        "recipient sealed",
        palette::GREEN,
    );
    ui.pill(
        main_x + main_w - 178.0,
        15.0,
        154.0,
        "identity routed",
        palette::ACCENT,
    );
    ui.scene.push_rect(GpuRect::fill(
        main_x,
        m.topbar_h - 1.0,
        main_w,
        1.0,
        0.0,
        palette::BORDER,
    ));

    let transcript_top = m.topbar_h + m.pad;
    let transcript_x = main_x + m.pad;
    let transcript_w = main_w - m.pad * 2.0;
    if state.contacts.is_empty() {
        let empty_w = transcript_w.clamp(260.0, 520.0);
        let empty_h = 126.0;
        let empty_x = transcript_x + (transcript_w - empty_w) * 0.5;
        let empty_y = transcript_top + 46.0;
        ui.card(empty_x, empty_y, empty_w, empty_h, 12.0, palette::PANEL);
        ui.bounded_label(
            empty_x + 22.0,
            empty_y + 24.0,
            empty_w - 44.0,
            "No contacts yet",
            3.0,
            palette::TEXT,
        );
        ui.bounded_label(
            empty_x + 22.0,
            empty_y + 62.0,
            empty_w - 44.0,
            "Connect a contact book or receive an identity-routed contact to show threads here.",
            2.0,
            palette::MUTED,
        );
        let composer = (
            main_x + m.pad,
            h - m.composer_h - m.pad,
            main_w - m.pad * 2.0,
            m.composer_h,
        );
        ui.composer(
            composer.0,
            composer.1,
            composer.2,
            composer.3,
            state.composer_placeholder,
            false,
            &[],
        );
        return;
    }

    let rail_w = if transcript_w > 820.0 { 220.0 } else { 0.0 };
    let message_area_w = transcript_w - if rail_w > 0.0 { rail_w + 18.0 } else { 0.0 };
    let message_w = (message_area_w * 0.82).clamp(220.0, 760.0);
    let transcript_limit = h - m.composer_h - m.pad * 2.0;
    let mut message_y = transcript_top;

    for message in state.messages {
        let x = if message.outgoing {
            transcript_x + message_area_w - message_w
        } else {
            transcript_x
        };
        let fill = if message.outgoing {
            palette::USER
        } else {
            palette::ASSISTANT
        };
        let height = estimate_message_height(
            message.body,
            (message_w - 42.0).max(120.0),
            4,
            #[cfg(feature = "fontdue-text")]
            ui.atlas,
        );
        if message_y + height > transcript_limit {
            break;
        }
        let drawn = ui.message_bubble(
            x,
            message_y,
            message_w,
            message.author,
            message.body,
            fill,
            message.accent,
        );
        message_y += drawn + 16.0;
    }

    if rail_w > 0.0 {
        let rail_x = transcript_x + transcript_w - rail_w;
        ui.card(rail_x, transcript_top, rail_w, 220.0, 12.0, palette::PANEL);
        ui.bounded_label(
            rail_x + 16.0,
            transcript_top + 18.0,
            rail_w - 32.0,
            "Thread policy",
            2.0,
            palette::TEXT,
        );
        ui.pill(
            rail_x + 16.0,
            transcript_top + 50.0,
            132.0,
            "2 recipients",
            palette::ACCENT,
        );
        ui.pill(
            rail_x + 16.0,
            transcript_top + 84.0,
            158.0,
            "codex tools scoped",
            palette::VIOLET,
        );
        ui.pill(
            rail_x + 16.0,
            transcript_top + 118.0,
            140.0,
            "relay admitted",
            palette::GREEN,
        );
        ui.divider(
            rail_x + 16.0,
            transcript_top + 152.0,
            rail_w - 32.0,
            Axis::Horizontal,
        );
        ui.bounded_label(
            rail_x + 16.0,
            transcript_top + 166.0,
            rail_w - 88.0,
            "route health",
            2.0,
            palette::MUTED,
        );
        ui.progress_bar(
            UiRect::new(rail_x + 16.0, transcript_top + 190.0, rail_w - 32.0, 8.0),
            if state.connected { 0.86 } else { 0.38 },
            if state.connected {
                palette::GREEN
            } else {
                palette::AMBER
            },
        );
        ui.toggle(
            rail_x + rail_w - 66.0,
            transcript_top + 162.0,
            state.connected,
            44,
        );
        row("row bg-row border rounded-md p-2 gap-2")
            .child(text("policy").class("w-14 text-muted truncate"))
            .child(text("scoped").class("flex-1 text-green truncate"))
            .child(button("open", 55, ButtonStyle::Ghost).class("w-16 h-8"))
            .render(
                &mut ui,
                UiRect::new(rail_x + 16.0, transcript_top + 206.0, rail_w - 32.0, 42.0),
            );
    }
    if state.messages.len() > 2 {
        ui.scrollbar(
            UiRect::new(
                transcript_x + message_area_w + 6.0,
                transcript_top,
                6.0,
                (transcript_limit - transcript_top).max(80.0),
            ),
            0.72,
            0.0,
        );
    }

    let composer = (
        main_x + m.pad,
        h - m.composer_h - m.pad,
        main_w - m.pad * 2.0,
        m.composer_h,
    );
    ui.composer(
        composer.0,
        composer.1,
        composer.2,
        composer.3,
        state.composer_placeholder,
        state.connected,
        &[
            ("encrypted", palette::GREEN),
            ("contact", palette::ACCENT),
            ("codex tools", palette::VIOLET),
        ],
    );
}

fn push_label(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    text: &str,
    scale: f32,
    color: Color4,
) {
    #[cfg(feature = "fontdue-text")]
    if let Some(atlas) = atlas {
        scene.push_font_text(atlas, x, y, text, color);
        return;
    }
    scene.push_text(x, y, text, scale, color);
}

fn push_bounded_label(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    max_w: f32,
    text: &str,
    scale: f32,
    color: Color4,
) {
    if max_w <= 0.0 {
        return;
    }
    let label = truncate_label_to_width(
        text,
        max_w,
        scale,
        #[cfg(feature = "fontdue-text")]
        atlas,
    );
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x,
        y,
        &label,
        scale,
        color,
    );
}

fn draw_pill(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    color: Color4,
) {
    scene.push_rect(GpuRect::fill(x, y, w, 28.0, 14.0, color.with_alpha(0.14)));
    scene.push_rect(GpuRect::border(x, y, w, 28.0, 14.0, color.with_alpha(0.46)));
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 12.0,
        y + 7.0,
        (w - 24.0).max(0.0),
        label,
        2.0,
        color,
    );
}

fn draw_contact_row(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    w: f32,
    contact: &UnifiedContact<'_>,
    selected: bool,
) {
    soft_card(
        scene,
        x,
        y,
        w,
        56.0,
        10.0,
        if selected {
            palette::ACTIVE_ROW
        } else {
            palette::ROW
        },
    );
    let accent = match contact.kind {
        UnifiedContactKind::Person => palette::ACCENT,
        UnifiedContactKind::CodexClient => palette::VIOLET,
        UnifiedContactKind::Node => palette::GREEN,
    };
    scene.push_rect(GpuRect::fill(
        x + 14.0,
        y + 13.0,
        30.0,
        30.0,
        15.0,
        accent.with_alpha(0.22),
    ));
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 23.0,
        y + 19.0,
        10.0,
        contact_initial(contact.name),
        2.0,
        accent,
    );
    scene.push_rect(GpuRect::fill(
        x + 38.0,
        y + 36.0,
        8.0,
        8.0,
        4.0,
        if contact.online {
            palette::GREEN
        } else {
            palette::MUTED
        },
    ));
    let unread_w = if contact.unread > 0 { 50.0 } else { 0.0 };
    let text_w = (w - 72.0 - unread_w).max(0.0);
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 56.0,
        y + 11.0,
        text_w,
        contact.name,
        2.0,
        if selected {
            palette::TEXT
        } else {
            palette::MUTED
        },
    );
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 56.0,
        y + 32.0,
        text_w,
        contact.detail,
        2.0,
        accent,
    );
    if contact.unread > 0 {
        let label = if contact.unread > 9 { "9+" } else { "new" };
        draw_pill(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            x + w - 58.0,
            y + 14.0,
            42.0,
            label,
            palette::AMBER,
        );
    }
}

fn contact_initial(name: &str) -> &str {
    name.get(0..1).unwrap_or("?")
}

fn draw_message(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    w: f32,
    role: &str,
    body: &str,
    fill: Color4,
    accent: Color4,
) -> f32 {
    let body_w = (w - 42.0).max(120.0);
    let lines = wrap_lines(
        body,
        body_w,
        4,
        #[cfg(feature = "fontdue-text")]
        atlas,
    );
    let h = 56.0 + lines.len() as f32 * 22.0;
    soft_card(scene, x, y, w, h, 14.0, fill);
    scene.push_rect(GpuRect::fill(x, y, 4.0, h, 2.0, accent));
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 22.0,
        y + 16.0,
        (w - 44.0).max(0.0),
        role,
        2.0,
        accent,
    );
    for (index, line) in lines.iter().enumerate() {
        push_bounded_label(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            x + 22.0,
            y + 42.0 + index as f32 * 22.0,
            body_w,
            line,
            2.0,
            palette::TEXT,
        );
    }
    h
}

fn estimate_message_height(
    text: &str,
    max_width: f32,
    max_lines: usize,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> f32 {
    56.0 + wrap_lines(
        text,
        max_width,
        max_lines,
        #[cfg(feature = "fontdue-text")]
        atlas,
    )
    .len() as f32
        * 22.0
}

fn wrap_lines(
    text: &str,
    max_width: f32,
    max_lines: usize,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if measure_label_width(
            &candidate,
            2.0,
            #[cfg(feature = "fontdue-text")]
            atlas,
        ) <= max_width
        {
            current = candidate;
            continue;
        }

        if !current.is_empty() {
            lines.push(current);
        }
        current = word.to_string();

        if lines.len() + 1 >= max_lines {
            break;
        }
    }

    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    let consumed_words = lines
        .iter()
        .map(|line| line.split_whitespace().count())
        .sum::<usize>();
    let total_words = text.split_whitespace().count();
    if consumed_words < total_words {
        if let Some(last) = lines.last_mut() {
            while !last.is_empty()
                && measure_label_width(
                    &format!("{last}..."),
                    2.0,
                    #[cfg(feature = "fontdue-text")]
                    atlas,
                ) > max_width
            {
                last.pop();
            }
            last.push_str("...");
        }
    }

    lines
}

fn measure_label_width(
    text: &str,
    scale: f32,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> f32 {
    #[cfg(feature = "fontdue-text")]
    if let Some(atlas) = atlas {
        return atlas.text_width(text);
    }
    text.chars().count() as f32 * scale.max(1.0) * 6.0
}

fn truncate_label_to_width(
    text: &str,
    max_width: f32,
    scale: f32,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> String {
    if measure_label_width(
        text,
        scale,
        #[cfg(feature = "fontdue-text")]
        atlas,
    ) <= max_width
    {
        return text.to_string();
    }

    let ellipsis = "...";
    let ellipsis_w = measure_label_width(
        ellipsis,
        scale,
        #[cfg(feature = "fontdue-text")]
        atlas,
    );
    if ellipsis_w > max_width {
        return String::new();
    }

    let mut out = String::new();
    for ch in text.chars() {
        out.push(ch);
        let candidate = format!("{out}{ellipsis}");
        if measure_label_width(
            &candidate,
            scale,
            #[cfg(feature = "fontdue-text")]
            atlas,
        ) > max_width
        {
            out.pop();
            break;
        }
    }
    out.push_str(ellipsis);
    out
}

fn component_label_width(
    text: &str,
    scale: f32,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> f32 {
    measure_label_width(
        text,
        scale,
        #[cfg(feature = "fontdue-text")]
        atlas,
    )
}

fn render_children(ui: &mut UiPainter<'_, '_>, rect: UiRect, style: &UiStyle, children: &[UiNode]) {
    if children.is_empty() {
        return;
    }
    let content = UiRect {
        x: rect.x + style.padding[3],
        y: rect.y + style.padding[0],
        w: (rect.w - style.padding[1] - style.padding[3]).max(0.0),
        h: (rect.h - style.padding[0] - style.padding[2]).max(0.0),
    };
    let main_available = match style.direction {
        Axis::Horizontal => content.w,
        Axis::Vertical => content.h,
    };
    let mut fixed = 0.0;
    let mut grow_count = 0usize;
    for child in children {
        if child.style.grow {
            grow_count += 1;
            continue;
        }
        fixed += child_main_size(child, style.direction);
    }
    let gap_total = style.gap * children.len().saturating_sub(1) as f32;
    let grow_size = if grow_count > 0 {
        ((main_available - fixed - gap_total).max(0.0)) / grow_count as f32
    } else {
        0.0
    };
    let mut main_sizes = Vec::with_capacity(children.len());
    for child in children {
        main_sizes.push(if child.style.grow {
            grow_size
        } else {
            child_main_size(child, style.direction)
        });
    }
    let main_sum: f32 = main_sizes.iter().sum();
    let mut gap = style.gap;
    let slack = (main_available - main_sum - gap_total).max(0.0);
    let start_offset = if grow_count > 0 {
        0.0
    } else {
        match style.justify {
            JustifyContent::Start | JustifyContent::Between => 0.0,
            JustifyContent::Center => slack * 0.5,
            JustifyContent::End => slack,
        }
    };
    if grow_count == 0 && matches!(style.justify, JustifyContent::Between) && children.len() > 1 {
        gap = (main_available - main_sum).max(0.0) / children.len().saturating_sub(1) as f32;
    }

    let mut cursor = match style.direction {
        Axis::Horizontal => content.x + start_offset,
        Axis::Vertical => content.y + start_offset,
    };
    for (child, main) in children.iter().zip(main_sizes) {
        let child_rect = match style.direction {
            Axis::Horizontal => {
                let cross =
                    aligned_cross(content.y, content.h, child, Axis::Horizontal, style.align);
                UiRect::new(
                    cursor,
                    cross.0,
                    main.max(0.0).min(content.x + content.w - cursor),
                    cross.1,
                )
            }
            Axis::Vertical => {
                let cross = aligned_cross(content.x, content.w, child, Axis::Vertical, style.align);
                UiRect::new(
                    cross.0,
                    cursor,
                    cross.1,
                    main.max(0.0).min(content.y + content.h - cursor),
                )
            }
        };
        child.render(ui, child_rect);
        cursor += main + gap;
    }
}

fn aligned_cross(
    content_start: f32,
    content_size: f32,
    child: &UiNode,
    parent_axis: Axis,
    align: AlignItems,
) -> (f32, f32) {
    let explicit = match parent_axis {
        Axis::Horizontal => child.style.height,
        Axis::Vertical => child.style.width,
    };
    let size = match explicit {
        Some(value) if value >= 0.0 => value.min(content_size),
        _ if matches!(align, AlignItems::Stretch) => content_size,
        _ => child_cross_size(child, parent_axis).min(content_size),
    }
    .max(0.0);
    let offset = match align {
        AlignItems::Start | AlignItems::Stretch => 0.0,
        AlignItems::Center => (content_size - size).max(0.0) * 0.5,
        AlignItems::End => (content_size - size).max(0.0),
    };
    (content_start + offset, size)
}

fn render_grid_children(
    ui: &mut UiPainter<'_, '_>,
    rect: UiRect,
    style: &UiStyle,
    columns: u16,
    children: &[UiNode],
) {
    if children.is_empty() {
        return;
    }
    let content = UiRect {
        x: rect.x + style.padding[3],
        y: rect.y + style.padding[0],
        w: (rect.w - style.padding[1] - style.padding[3]).max(0.0),
        h: (rect.h - style.padding[0] - style.padding[2]).max(0.0),
    };
    let columns = columns.max(1) as usize;
    let gap = style.gap;
    let track_w = ((content.w - gap * columns.saturating_sub(1) as f32) / columns as f32).max(0.0);
    let mut row_y = content.y;
    let mut index = 0usize;

    while index < children.len() && row_y < content.y + content.h {
        let row_start = index;
        let mut row_col = 0usize;
        let mut row_h = 0.0_f32;
        while index < children.len() {
            let child = &children[index];
            let span = child.style.col_span.max(1).min(columns as u16) as usize;
            if row_col > 0 && row_col + span > columns {
                break;
            }
            row_h = row_h.max(child_main_size(child, Axis::Vertical));
            row_col += span;
            index += 1;
            if row_col >= columns {
                break;
            }
        }

        let mut col = 0usize;
        for child in &children[row_start..index] {
            let span = child.style.col_span.max(1).min(columns as u16) as usize;
            let w = track_w * span as f32 + gap * span.saturating_sub(1) as f32;
            let h = child_main_size(child, Axis::Vertical).min(content.y + content.h - row_y);
            let x = content.x + col as f32 * (track_w + gap);
            child.render(ui, UiRect::new(x, row_y, w, h));
            col += span;
        }
        row_y += row_h + gap;
    }
}

fn child_main_size(child: &UiNode, axis: Axis) -> f32 {
    match axis {
        Axis::Horizontal => child.style.width.unwrap_or_else(|| intrinsic_width(child)),
        Axis::Vertical => child
            .style
            .height
            .unwrap_or_else(|| intrinsic_height(child)),
    }
}

fn child_cross_size(child: &UiNode, parent_axis: Axis) -> f32 {
    match parent_axis {
        Axis::Horizontal => child
            .style
            .height
            .unwrap_or_else(|| intrinsic_height(child)),
        Axis::Vertical => child.style.width.unwrap_or_else(|| intrinsic_width(child)),
    }
}

fn intrinsic_width(child: &UiNode) -> f32 {
    match &child.kind {
        UiNodeKind::Text(value) => (value.chars().count() as f32 * 8.0 + 2.0).clamp(24.0, 220.0),
        UiNodeKind::Badge { label, .. } => {
            (label.chars().count() as f32 * 8.0 + 22.0).clamp(36.0, 180.0)
        }
        UiNodeKind::Button { label, .. } => (label.chars().count() as f32 * 8.0 + 28.0).max(44.0),
        UiNodeKind::PanelHeader { .. } => 280.0,
        UiNodeKind::MetricCard { .. } => 220.0,
        UiNodeKind::Field { .. } => 240.0,
        UiNodeKind::TextArea { .. } => 280.0,
        UiNodeKind::Slider { .. } => 240.0,
        UiNodeKind::BarChart { .. } => 320.0,
        UiNodeKind::TransactionRow { .. } => 320.0,
        UiNodeKind::MenuItem { .. } => 220.0,
        UiNodeKind::Divider => 1.0,
        UiNodeKind::Spacer => child.style.width.unwrap_or(12.0),
        _ => child.style.width.unwrap_or(120.0),
    }
}

fn intrinsic_height(child: &UiNode) -> f32 {
    match child.kind {
        UiNodeKind::Text(_) => 20.0,
        UiNodeKind::Badge { .. } => 22.0,
        UiNodeKind::Button { .. } => 34.0,
        UiNodeKind::PanelHeader { .. } => 44.0,
        UiNodeKind::MetricCard { .. } => 132.0,
        UiNodeKind::Field { .. } => 92.0,
        UiNodeKind::TextArea { .. } => 132.0,
        UiNodeKind::Slider { .. } => 78.0,
        UiNodeKind::BarChart { .. } => 180.0,
        UiNodeKind::TransactionRow { .. } => 58.0,
        UiNodeKind::MenuItem { .. } => 58.0,
        UiNodeKind::Divider => 1.0,
        UiNodeKind::Spacer => child.style.height.unwrap_or(12.0),
        _ => child.style.height.unwrap_or(44.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_node_builder_renders_component_tree() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-panel border rounded-md p-4 gap-3")
                .children([
                    row("gap-2 h-10")
                        .child(text("Dashboard").class("flex-1 text-text truncate"))
                        .child(badge("sealed", palette::GREEN)),
                    metric("Relay balance", "$0.00")
                        .detail("pending setup")
                        .progress(0.32)
                        .class("h-32"),
                    field_node("Endpoint", "nodes.edgerun.tech")
                        .detail("identity-routed relay")
                        .focused(true),
                    menu_item_node("Payments", 42)
                        .detail("proof-backed receipts")
                        .badge_text("new")
                        .selected(true),
                    button("Open", 43, ButtonStyle::Secondary).class("h-8"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 360.0, 420.0));
        }

        assert!(scene.rects().len() > 20);
        assert!(scene.hits().len() >= 2);
    }

    #[test]
    fn ui_node_builder_renders_dashboard_primitives() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            let months = ["Jan", "Feb", "Mar", "Apr"];
            let values = [0.42, 0.72, 0.55, 0.91];
            column("bg-panel border rounded-md p-4 gap-3")
                .children([
                    header("Contribution History")
                        .detail("Last 4 months")
                        .action("View", 50),
                    bar_chart_labels("Relay Receipts", &months, &values).class("h-44"),
                    slider_node("Payout threshold", 0.62, 51)
                        .range_labels("$50", "$10,000")
                        .accent(palette::GREEN),
                    text_area_node("Notes", "Route budget and admission notes").focused(true),
                    transaction_node("Stripe payout", "+$4,200.00", 52)
                        .detail("Income")
                        .date("Today")
                        .positive(true),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 420.0, 620.0));
        }

        assert!(scene.rects().len() > 35);
        assert!(scene.hits().len() >= 3);
    }

    #[test]
    fn gpu_ui_macro_composes_nested_trees() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            gpu_ui!(
                column("bg-panel border rounded-md p-4 gap-3"),
                [
                    gpu_ui!(
                        row("gap-2 h-10"),
                        [
                            text("Components").class("flex-1 text-text truncate"),
                            badge("rust", palette::ACCENT)
                        ]
                    ),
                    metric("Storage", "128 MB").detail("verified cache"),
                    button("Run", 70, ButtonStyle::Primary).class("h-8")
                ]
            )
            .render(&mut ui, UiRect::new(0.0, 0.0, 320.0, 260.0));
        }

        assert!(scene.rects().len() > 15);
        assert_eq!(scene.hits().len(), 1);
    }

    #[test]
    fn grid_node_places_spanned_dashboard_cards() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            let labels = ["A", "B", "C"];
            let values = [0.25, 0.5, 0.75];
            grid("bg-panel border rounded-md p-3 gap-3", 4)
                .children([
                    metric("Balance", "$42.00")
                        .progress(0.4)
                        .span(2)
                        .class("h-32"),
                    field_node("Relay", "nodes.edgerun.tech")
                        .focused(true)
                        .span(2),
                    bar_chart_labels("Activity", &labels, &values)
                        .span(4)
                        .class("h-44"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 640.0, 420.0));
        }

        assert!(scene.rects().len() > 30);
        assert!(scene.hits().is_empty());
    }

    #[test]
    fn grid_auto_reads_columns_from_classes() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            grid_auto("grid grid-cols-3 bg-panel border rounded-md p-3 gap-3")
                .children([
                    metric("One", "1").class("h-24"),
                    metric("Two", "2").class("h-24"),
                    metric("Three", "3").class("h-24"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 480.0, 160.0));
        }

        assert!(scene.rects().len() > 20);
    }

    #[test]
    fn alignment_classes_control_child_placement() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            row("bg-panel border rounded-md p-2 items-center justify-between")
                .children([
                    text("Left").class("w-12"),
                    button("Right", 80, ButtonStyle::Secondary).class("w-20 h-8"),
                ])
                .render(&mut ui, UiRect::new(0.0, 0.0, 320.0, 80.0));
        }

        let hit = scene
            .hits()
            .iter()
            .find(|hit| hit.id == 80)
            .expect("button hit");
        assert!(hit.x > 220.0);
        assert!((hit.y - 24.0).abs() < 0.1);
    }

    #[test]
    fn column_children_stretch_by_default() {
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            column("bg-panel border rounded-md p-2 gap-2")
                .child(button("Wide", 81, ButtonStyle::Secondary).class("h-8"))
                .render(&mut ui, UiRect::new(0.0, 0.0, 220.0, 80.0));
        }

        let hit = scene
            .hits()
            .iter()
            .find(|hit| hit.id == 81)
            .expect("button hit");
        assert!(hit.w > 190.0);
    }

    #[test]
    fn conditional_children_do_not_allocate_placeholder_layout() {
        let mut hidden = row("gap-2")
            .when(false, text("hidden"))
            .child(text("visible"));
        let shown = row("gap-2").when(true, text("visible"));

        assert_eq!(hidden.children.len(), 1);
        assert_eq!(shown.children.len(), 1);
        hidden = hidden.child(text("second"));
        assert_eq!(hidden.children.len(), 2);
    }
}

#[cfg(feature = "fontdue-text")]
fn ascii_chars() -> Vec<char> {
    (32u8..=126u8).map(char::from).collect()
}

fn soft_card(scene: &mut GpuScene, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) {
    scene.push_rect(GpuRect::shadow(
        x,
        y + 4.0,
        w,
        h,
        radius,
        Color4::rgba(0.0, 0.0, 0.0, 0.24),
        10.0,
    ));
    panel(scene, x, y, w, h, radius, color);
}

fn panel(scene: &mut GpuScene, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color4) {
    scene.push_rect(GpuRect::fill(x, y, w, h, radius, color));
    scene.push_rect(GpuRect::border(x, y, w, h, radius, palette::BORDER));
}

pub mod palette {
    use super::Color4;

    pub const BG: Color4 = Color4::from_color(crate::EDGERUN_DARK.bg);
    pub const SIDEBAR: Color4 = Color4::from_color_alpha(crate::EDGERUN_DARK.panel, 0.98);
    pub const TOPBAR: Color4 = Color4::from_color_alpha(crate::EDGERUN_DARK.panel, 0.96);
    pub const ROW: Color4 = Color4::from_color_alpha(crate::EDGERUN_DARK.panel_2, 0.74);
    pub const ACTIVE_ROW: Color4 = Color4::from_color_alpha(crate::EDGERUN_DARK.accent, 0.42);
    pub const PANEL: Color4 = Color4::from_color_alpha(crate::EDGERUN_DARK.panel, 0.94);
    pub const ASSISTANT: Color4 = Color4::from_color_alpha(crate::EDGERUN_DARK.panel_2, 0.96);
    pub const USER: Color4 = Color4::from_color_alpha(crate::EDGERUN_DARK.accent, 0.34);
    pub const COMPOSER: Color4 = Color4::from_color_alpha(crate::EDGERUN_DARK.panel, 0.98);
    pub const BORDER: Color4 = Color4::from_color_alpha(crate::EDGERUN_DARK.border, 0.48);
    pub const ACCENT: Color4 = Color4::from_color(crate::EDGERUN_DARK.accent);
    pub const GREEN: Color4 = Color4::from_color(crate::TAILWIND.emerald_500);
    pub const VIOLET: Color4 = Color4::from_color(crate::TAILWIND.violet_500);
    pub const AMBER: Color4 = Color4::from_color(crate::TAILWIND.amber_500);
    pub const DANGER: Color4 = Color4::from_color(crate::EDGERUN_DARK.danger);
    pub const ACCENT_TEXT: Color4 = Color4::from_color(crate::EDGERUN_DARK.accent_text);
    pub const TEXT: Color4 = Color4::from_color(crate::EDGERUN_DARK.text);
    pub const MUTED: Color4 = Color4::from_color(crate::EDGERUN_DARK.muted);
}

#[derive(Clone, Copy, Debug)]
struct SchemePalette {
    bg: Color4,
    sidebar: Color4,
    topbar: Color4,
    row: Color4,
    active_row: Color4,
    panel: Color4,
    assistant: Color4,
    user: Color4,
    composer: Color4,
    border: Color4,
    accent: Color4,
    green: Color4,
    violet: Color4,
    amber: Color4,
    danger: Color4,
    accent_text: Color4,
    text: Color4,
    muted: Color4,
}

impl SchemePalette {
    const fn dark() -> Self {
        Self {
            bg: palette::BG,
            sidebar: palette::SIDEBAR,
            topbar: palette::TOPBAR,
            row: palette::ROW,
            active_row: palette::ACTIVE_ROW,
            panel: palette::PANEL,
            assistant: palette::ASSISTANT,
            user: palette::USER,
            composer: palette::COMPOSER,
            border: palette::BORDER,
            accent: palette::ACCENT,
            green: palette::GREEN,
            violet: palette::VIOLET,
            amber: palette::AMBER,
            danger: palette::DANGER,
            accent_text: palette::ACCENT_TEXT,
            text: palette::TEXT,
            muted: palette::MUTED,
        }
    }

    const fn light() -> Self {
        Self {
            bg: Color4::rgba(0.972, 0.980, 0.988, 1.0),
            sidebar: Color4::rgba(1.000, 1.000, 1.000, 0.98),
            topbar: Color4::rgba(1.000, 1.000, 1.000, 0.96),
            row: Color4::rgba(0.945, 0.960, 0.975, 0.90),
            active_row: Color4::rgba(0.055, 0.455, 0.565, 0.16),
            panel: Color4::rgba(1.000, 1.000, 1.000, 0.96),
            assistant: Color4::rgba(0.945, 0.960, 0.975, 0.96),
            user: Color4::rgba(0.055, 0.455, 0.565, 0.16),
            composer: Color4::rgba(1.000, 1.000, 1.000, 0.98),
            border: Color4::rgba(0.580, 0.640, 0.720, 0.42),
            accent: Color4::rgba(0.035, 0.455, 0.565, 1.0),
            green: Color4::rgba(0.020, 0.520, 0.370, 1.0),
            violet: Color4::rgba(0.430, 0.250, 0.760, 1.0),
            amber: Color4::rgba(0.710, 0.390, 0.000, 1.0),
            danger: Color4::rgba(0.760, 0.070, 0.235, 1.0),
            accent_text: Color4::rgba(0.960, 0.990, 1.000, 1.0),
            text: Color4::rgba(0.060, 0.090, 0.160, 1.0),
            muted: Color4::rgba(0.390, 0.455, 0.550, 1.0),
        }
    }

    const fn terminal() -> Self {
        Self {
            bg: Color4::rgba(0.000, 0.050, 0.035, 1.0),
            sidebar: Color4::rgba(0.000, 0.075, 0.055, 0.98),
            topbar: Color4::rgba(0.000, 0.070, 0.050, 0.96),
            row: Color4::rgba(0.000, 0.135, 0.095, 0.74),
            active_row: Color4::rgba(0.160, 0.980, 0.620, 0.22),
            panel: Color4::rgba(0.000, 0.095, 0.070, 0.94),
            assistant: Color4::rgba(0.000, 0.120, 0.085, 0.96),
            user: Color4::rgba(0.160, 0.980, 0.620, 0.18),
            composer: Color4::rgba(0.000, 0.100, 0.075, 0.98),
            border: Color4::rgba(0.160, 0.980, 0.620, 0.36),
            accent: Color4::rgba(0.160, 0.980, 0.620, 1.0),
            green: Color4::rgba(0.160, 0.980, 0.620, 1.0),
            violet: Color4::rgba(0.500, 0.840, 1.000, 1.0),
            amber: Color4::rgba(0.980, 0.780, 0.260, 1.0),
            danger: Color4::rgba(1.000, 0.330, 0.430, 1.0),
            accent_text: Color4::rgba(0.000, 0.050, 0.035, 1.0),
            text: Color4::rgba(0.800, 1.000, 0.890, 1.0),
            muted: Color4::rgba(0.430, 0.760, 0.600, 1.0),
        }
    }
}

fn remap_scheme_color(color: Color4, from: SchemePalette, to: SchemePalette) -> Color4 {
    let pairs = [
        (from.bg, to.bg),
        (from.sidebar, to.sidebar),
        (from.topbar, to.topbar),
        (from.row, to.row),
        (from.active_row, to.active_row),
        (from.panel, to.panel),
        (from.assistant, to.assistant),
        (from.user, to.user),
        (from.composer, to.composer),
        (from.border, to.border),
        (from.accent, to.accent),
        (from.green, to.green),
        (from.violet, to.violet),
        (from.amber, to.amber),
        (from.danger, to.danger),
        (from.accent_text, to.accent_text),
        (from.text, to.text),
        (from.muted, to.muted),
    ];
    for (source, target) in pairs {
        if same_color(color, source) {
            return target.with_alpha(color.a);
        }
    }
    for (source, target) in pairs {
        if same_rgb(color, source) {
            return target.with_alpha(color.a);
        }
    }
    color
}

fn same_color(a: Color4, b: Color4) -> bool {
    same_rgb(a, b) && close_f32(a.a, b.a)
}

fn same_rgb(a: Color4, b: Color4) -> bool {
    close_f32(a.r, b.r) && close_f32(a.g, b.g) && close_f32(a.b, b.b)
}

fn close_f32(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.001
}

fn glyph5x7(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        ':' => [
            0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000,
        ],
        '+' => [
            0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '/' => [
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ],
        '>' => [
            0b10000, 0b01000, 0b00100, 0b00010, 0b00100, 0b01000, 0b10000,
        ],
        _ => [
            0b11111, 0b10001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100,
        ],
    }
}

pub mod webgl2 {
    pub const VERT: &str = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
uniform vec2 u_screen;
uniform vec4 u_rect;
out vec2 v_local;
out vec2 v_size;
void main() {
    vec2 px = u_rect.xy + a_pos * u_rect.zw;
    vec2 ndc = vec2(px.x / u_screen.x * 2.0 - 1.0, 1.0 - px.y / u_screen.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
    v_local = a_pos * u_rect.zw;
    v_size = u_rect.zw;
}
"#;

    pub const FRAG: &str = r#"#version 300 es
precision highp float;
in vec2 v_local;
in vec2 v_size;
out vec4 out_color;
uniform vec4 u_color;
uniform float u_radius;
uniform float u_shadow;
uniform int u_mode;
float rounded_box(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + vec2(r);
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}
void main() {
    vec2 p = v_local - v_size * 0.5;
    float d = rounded_box(p, v_size * 0.5, u_radius);
    float aa = max(fwidth(d), 0.75);
    float alpha = 1.0 - smoothstep(0.0, aa, d);
    if (u_mode == 1) {
        float sd = rounded_box(p - vec2(0.0, -u_shadow * 0.18), v_size * 0.5, u_radius + u_shadow * 0.35);
        float blur = max(u_shadow, 1.0);
        alpha = 1.0 - smoothstep(-blur, blur, sd);
        out_color = vec4(u_color.rgb, u_color.a * alpha * 0.28);
    } else if (u_mode == 2) {
        float inner = rounded_box(p, v_size * 0.5 - vec2(1.25), max(u_radius - 1.25, 0.0));
        float border = (1.0 - smoothstep(0.0, aa, d)) * smoothstep(0.0, aa, inner);
        out_color = vec4(u_color.rgb, u_color.a * border);
    } else {
        out_color = vec4(u_color.rgb, u_color.a * alpha);
    }
}
"#;
}

#[cfg(all(feature = "gpu-gl", not(target_arch = "wasm32")))]
pub mod gl {
    use super::{Color4, GpuRect, GpuScene, RectMode};
    #[cfg(feature = "fontdue-text")]
    use super::{FontAtlas, TextQuad};
    use std::ffi::{CStr, CString};
    use std::os::raw::{c_char, c_int, c_void};
    use std::ptr;

    const GL_COLOR_BUFFER_BIT: u32 = 0x0000_4000;
    const GL_ARRAY_BUFFER: u32 = 0x8892;
    const GL_DYNAMIC_DRAW: u32 = 0x88E8;
    const GL_FLOAT: u32 = 0x1406;
    const GL_FALSE: u8 = 0;
    const GL_TRIANGLES: u32 = 0x0004;
    const GL_VERTEX_SHADER: u32 = 0x8B31;
    const GL_FRAGMENT_SHADER: u32 = 0x8B30;
    const GL_COMPILE_STATUS: u32 = 0x8B81;
    const GL_LINK_STATUS: u32 = 0x8B82;
    const GL_BLEND: u32 = 0x0BE2;
    const GL_SRC_ALPHA: u32 = 0x0302;
    const GL_ONE_MINUS_SRC_ALPHA: u32 = 0x0303;
    #[cfg(feature = "fontdue-text")]
    const GL_TEXTURE_2D: u32 = 0x0DE1;
    #[cfg(feature = "fontdue-text")]
    const GL_TEXTURE0: u32 = 0x84C0;
    #[cfg(feature = "fontdue-text")]
    const GL_RED: u32 = 0x1903;
    #[cfg(feature = "fontdue-text")]
    const GL_R8: u32 = 0x8229;
    #[cfg(feature = "fontdue-text")]
    const GL_UNSIGNED_BYTE: u32 = 0x1401;
    #[cfg(feature = "fontdue-text")]
    const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
    #[cfg(feature = "fontdue-text")]
    const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
    #[cfg(feature = "fontdue-text")]
    const GL_TEXTURE_WRAP_S: u32 = 0x2802;
    #[cfg(feature = "fontdue-text")]
    const GL_TEXTURE_WRAP_T: u32 = 0x2803;
    #[cfg(feature = "fontdue-text")]
    const GL_LINEAR: i32 = 0x2601;
    #[cfg(feature = "fontdue-text")]
    const GL_CLAMP_TO_EDGE: i32 = 0x812F;
    #[cfg(feature = "fontdue-text")]
    const GL_UNPACK_ALIGNMENT: u32 = 0x0CF5;

    #[link(name = "GL")]
    unsafe extern "C" {
        fn glViewport(x: c_int, y: c_int, width: c_int, height: c_int);
        fn glClearColor(r: f32, g: f32, b: f32, a: f32);
        fn glClear(mask: u32);
        fn glEnable(cap: u32);
        fn glBlendFunc(sfactor: u32, dfactor: u32);
        fn glCreateShader(shader_type: u32) -> u32;
        fn glShaderSource(
            shader: u32,
            count: c_int,
            string: *const *const c_char,
            length: *const c_int,
        );
        fn glCompileShader(shader: u32);
        fn glGetShaderiv(shader: u32, pname: u32, params: *mut c_int);
        fn glGetShaderInfoLog(
            shader: u32,
            buf_size: c_int,
            length: *mut c_int,
            info_log: *mut c_char,
        );
        fn glDeleteShader(shader: u32);
        fn glCreateProgram() -> u32;
        fn glAttachShader(program: u32, shader: u32);
        fn glLinkProgram(program: u32);
        fn glGetProgramiv(program: u32, pname: u32, params: *mut c_int);
        fn glGetProgramInfoLog(
            program: u32,
            buf_size: c_int,
            length: *mut c_int,
            info_log: *mut c_char,
        );
        fn glUseProgram(program: u32);
        fn glGetUniformLocation(program: u32, name: *const c_char) -> c_int;
        fn glUniform2f(location: c_int, v0: f32, v1: f32);
        fn glUniform4f(location: c_int, v0: f32, v1: f32, v2: f32, v3: f32);
        fn glUniform1f(location: c_int, v0: f32);
        fn glUniform1i(location: c_int, v0: c_int);
        fn glGenVertexArrays(n: c_int, arrays: *mut u32);
        fn glBindVertexArray(array: u32);
        fn glGenBuffers(n: c_int, buffers: *mut u32);
        fn glBindBuffer(target: u32, buffer: u32);
        fn glBufferData(target: u32, size: isize, data: *const c_void, usage: u32);
        fn glEnableVertexAttribArray(index: u32);
        fn glVertexAttribPointer(
            index: u32,
            size: c_int,
            ty: u32,
            normalized: u8,
            stride: c_int,
            pointer: *const c_void,
        );
        fn glDrawArrays(mode: u32, first: c_int, count: c_int);
        fn glDeleteBuffers(n: c_int, buffers: *const u32);
        fn glDeleteVertexArrays(n: c_int, arrays: *const u32);
        fn glDeleteProgram(program: u32);
        #[cfg(feature = "fontdue-text")]
        fn glGenTextures(n: c_int, textures: *mut u32);
        #[cfg(feature = "fontdue-text")]
        fn glBindTexture(target: u32, texture: u32);
        #[cfg(feature = "fontdue-text")]
        fn glTexParameteri(target: u32, pname: u32, param: c_int);
        #[cfg(feature = "fontdue-text")]
        fn glTexImage2D(
            target: u32,
            level: c_int,
            internalformat: c_int,
            width: c_int,
            height: c_int,
            border: c_int,
            format: u32,
            ty: u32,
            pixels: *const c_void,
        );
        #[cfg(feature = "fontdue-text")]
        fn glActiveTexture(texture: u32);
        #[cfg(feature = "fontdue-text")]
        fn glPixelStorei(pname: u32, param: c_int);
        #[cfg(feature = "fontdue-text")]
        fn glDeleteTextures(n: c_int, textures: *const u32);
    }

    const VERT: &str = r#"#version 330 core
layout(location = 0) in vec2 a_pos;
uniform vec2 u_screen;
uniform vec4 u_rect;
out vec2 v_local;
out vec2 v_size;
void main() {
    vec2 px = u_rect.xy + a_pos * u_rect.zw;
    vec2 ndc = vec2(px.x / u_screen.x * 2.0 - 1.0, 1.0 - px.y / u_screen.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
    v_local = a_pos * u_rect.zw;
    v_size = u_rect.zw;
}
"#;

    #[cfg(feature = "fontdue-text")]
    const TEXT_VERT: &str = r#"#version 330 core
layout(location = 0) in vec4 a_data;
uniform vec2 u_screen;
out vec2 v_uv;
void main() {
    vec2 px = a_data.xy;
    vec2 ndc = vec2(px.x / u_screen.x * 2.0 - 1.0, 1.0 - px.y / u_screen.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
    v_uv = a_data.zw;
}
"#;

    #[cfg(feature = "fontdue-text")]
    const TEXT_FRAG: &str = r#"#version 330 core
in vec2 v_uv;
out vec4 out_color;
uniform sampler2D u_tex;
uniform vec4 u_color;
void main() {
    float a = texture(u_tex, v_uv).r;
    out_color = vec4(u_color.rgb, u_color.a * a);
}
"#;

    const FRAG: &str = r#"#version 330 core
in vec2 v_local;
in vec2 v_size;
out vec4 out_color;
uniform vec4 u_color;
uniform float u_radius;
uniform float u_shadow;
uniform int u_mode;
float rounded_box(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + vec2(r);
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}
void main() {
    vec2 p = v_local - v_size * 0.5;
    float d = rounded_box(p, v_size * 0.5, u_radius);
    float aa = max(fwidth(d), 0.75);
    float alpha = 1.0 - smoothstep(0.0, aa, d);
    if (u_mode == 1) {
        float sd = rounded_box(p - vec2(0.0, -u_shadow * 0.18), v_size * 0.5, u_radius + u_shadow * 0.35);
        float blur = max(u_shadow, 1.0);
        alpha = 1.0 - smoothstep(-blur, blur, sd);
        out_color = vec4(u_color.rgb, u_color.a * alpha * 0.28);
    } else if (u_mode == 2) {
        float inner = rounded_box(p, v_size * 0.5 - vec2(1.25), max(u_radius - 1.25, 0.0));
        float border = (1.0 - smoothstep(0.0, aa, d)) * smoothstep(0.0, aa, inner);
        out_color = vec4(u_color.rgb, u_color.a * border);
    } else {
        out_color = vec4(u_color.rgb, u_color.a * alpha);
    }
}
"#;

    pub struct GlRenderer {
        program: u32,
        vao: u32,
        vbo: u32,
        u_screen: c_int,
        u_rect: c_int,
        u_color: c_int,
        u_radius: c_int,
        u_mode: c_int,
        u_shadow: c_int,
        #[cfg(feature = "fontdue-text")]
        text: Option<TextRenderer>,
    }

    impl GlRenderer {
        /// Create a renderer for the current OpenGL context.
        ///
        /// The caller must create and make current the GL context before calling
        /// this constructor.
        pub unsafe fn new_current_context() -> Result<Self, String> {
            let program = unsafe { glCreateProgram() };
            let vs = compile_shader(GL_VERTEX_SHADER, VERT)?;
            let fs = compile_shader(GL_FRAGMENT_SHADER, FRAG)?;
            unsafe {
                glAttachShader(program, vs);
                glAttachShader(program, fs);
                glLinkProgram(program);
                glDeleteShader(vs);
                glDeleteShader(fs);
            }
            check_program(program)?;

            let mut vao = 0;
            let mut vbo = 0;
            let verts: [f32; 12] = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0];
            unsafe {
                glGenVertexArrays(1, &mut vao);
                glBindVertexArray(vao);
                glGenBuffers(1, &mut vbo);
                glBindBuffer(GL_ARRAY_BUFFER, vbo);
                glBufferData(
                    GL_ARRAY_BUFFER,
                    (verts.len() * 4) as isize,
                    verts.as_ptr().cast(),
                    GL_DYNAMIC_DRAW,
                );
                glEnableVertexAttribArray(0);
                glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, 2 * 4, ptr::null());
            }

            Ok(Self {
                program,
                vao,
                vbo,
                u_screen: uniform(program, "u_screen"),
                u_rect: uniform(program, "u_rect"),
                u_color: uniform(program, "u_color"),
                u_radius: uniform(program, "u_radius"),
                u_mode: uniform(program, "u_mode"),
                u_shadow: uniform(program, "u_shadow"),
                #[cfg(feature = "fontdue-text")]
                text: None,
            })
        }

        #[cfg(feature = "fontdue-text")]
        pub unsafe fn new_current_context_with_font(atlas: &FontAtlas) -> Result<Self, String> {
            let mut renderer = unsafe { Self::new_current_context()? };
            renderer.text = Some(TextRenderer::new(atlas)?);
            Ok(renderer)
        }

        pub fn render(&self, width: i32, height: i32, scene: &GpuScene) {
            let clear = scene.clear;
            unsafe {
                glViewport(0, 0, width, height);
                glClearColor(clear.r, clear.g, clear.b, clear.a);
                glClear(GL_COLOR_BUFFER_BIT);
                glEnable(GL_BLEND);
                glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
                glUseProgram(self.program);
                glBindVertexArray(self.vao);
                glUniform2f(self.u_screen, width as f32, height as f32);
            }
            for rect in scene.rects() {
                self.draw_rect(*rect);
            }
            #[cfg(feature = "fontdue-text")]
            if let Some(text) = &self.text {
                text.render(width, height, scene.text_quads());
            }
        }

        fn draw_rect(&self, rect: GpuRect) {
            let mode = match rect.mode {
                RectMode::Fill => 0,
                RectMode::Shadow => 1,
                RectMode::Border => 2,
            };
            let Color4 { r, g, b, a } = rect.color;
            unsafe {
                glUniform4f(self.u_rect, rect.x, rect.y, rect.w, rect.h);
                glUniform4f(self.u_color, r, g, b, a);
                glUniform1f(self.u_radius, rect.radius);
                glUniform1i(self.u_mode, mode);
                glUniform1f(self.u_shadow, rect.shadow);
                glDrawArrays(GL_TRIANGLES, 0, 6);
            }
        }
    }

    #[cfg(feature = "fontdue-text")]
    struct TextRenderer {
        program: u32,
        vao: u32,
        vbo: u32,
        texture: u32,
        u_screen: c_int,
        u_color: c_int,
        u_tex: c_int,
    }

    #[cfg(feature = "fontdue-text")]
    impl TextRenderer {
        fn new(atlas: &FontAtlas) -> Result<Self, String> {
            let program = unsafe { glCreateProgram() };
            let vs = compile_shader(GL_VERTEX_SHADER, TEXT_VERT)?;
            let fs = compile_shader(GL_FRAGMENT_SHADER, TEXT_FRAG)?;
            unsafe {
                glAttachShader(program, vs);
                glAttachShader(program, fs);
                glLinkProgram(program);
                glDeleteShader(vs);
                glDeleteShader(fs);
            }
            check_program(program)?;

            let mut vao = 0;
            let mut vbo = 0;
            unsafe {
                glGenVertexArrays(1, &mut vao);
                glBindVertexArray(vao);
                glGenBuffers(1, &mut vbo);
                glBindBuffer(GL_ARRAY_BUFFER, vbo);
                glEnableVertexAttribArray(0);
                glVertexAttribPointer(0, 4, GL_FLOAT, GL_FALSE, 4 * 4, ptr::null());
            }

            let mut texture = 0;
            unsafe {
                glGenTextures(1, &mut texture);
                glBindTexture(GL_TEXTURE_2D, texture);
                glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
                glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
                glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
                glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
                glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
                glTexImage2D(
                    GL_TEXTURE_2D,
                    0,
                    GL_R8 as i32,
                    atlas.width as i32,
                    atlas.height as i32,
                    0,
                    GL_RED,
                    GL_UNSIGNED_BYTE,
                    atlas.alpha.as_ptr().cast(),
                );
            }

            Ok(Self {
                program,
                vao,
                vbo,
                texture,
                u_screen: uniform(program, "u_screen"),
                u_color: uniform(program, "u_color"),
                u_tex: uniform(program, "u_tex"),
            })
        }

        fn render(&self, width: i32, height: i32, quads: &[TextQuad]) {
            unsafe {
                glUseProgram(self.program);
                glBindVertexArray(self.vao);
                glActiveTexture(GL_TEXTURE0);
                glBindTexture(GL_TEXTURE_2D, self.texture);
                glUniform1i(self.u_tex, 0);
                glUniform2f(self.u_screen, width as f32, height as f32);
            }
            for quad in quads {
                self.draw_quad(*quad);
            }
        }

        fn draw_quad(&self, q: TextQuad) {
            let verts: [f32; 24] = [
                q.x,
                q.y,
                q.u0,
                q.v0,
                q.x + q.w,
                q.y,
                q.u1,
                q.v0,
                q.x + q.w,
                q.y + q.h,
                q.u1,
                q.v1,
                q.x,
                q.y,
                q.u0,
                q.v0,
                q.x + q.w,
                q.y + q.h,
                q.u1,
                q.v1,
                q.x,
                q.y + q.h,
                q.u0,
                q.v1,
            ];
            let Color4 { r, g, b, a } = q.color;
            unsafe {
                glUniform4f(self.u_color, r, g, b, a);
                glBindBuffer(GL_ARRAY_BUFFER, self.vbo);
                glBufferData(
                    GL_ARRAY_BUFFER,
                    (verts.len() * 4) as isize,
                    verts.as_ptr().cast(),
                    GL_DYNAMIC_DRAW,
                );
                glDrawArrays(GL_TRIANGLES, 0, 6);
            }
        }
    }

    #[cfg(feature = "fontdue-text")]
    impl Drop for TextRenderer {
        fn drop(&mut self) {
            unsafe {
                glDeleteTextures(1, &self.texture);
                glDeleteBuffers(1, &self.vbo);
                glDeleteVertexArrays(1, &self.vao);
                glDeleteProgram(self.program);
            }
        }
    }

    impl Drop for GlRenderer {
        fn drop(&mut self) {
            unsafe {
                glDeleteBuffers(1, &self.vbo);
                glDeleteVertexArrays(1, &self.vao);
                glDeleteProgram(self.program);
            }
        }
    }

    fn compile_shader(kind: u32, source: &str) -> Result<u32, String> {
        let shader = unsafe { glCreateShader(kind) };
        let c_src = CString::new(source).map_err(|error| error.to_string())?;
        let ptr = c_src.as_ptr();
        unsafe {
            glShaderSource(shader, 1, &ptr, ptr::null());
            glCompileShader(shader);
        }
        let mut ok = 0;
        unsafe {
            glGetShaderiv(shader, GL_COMPILE_STATUS, &mut ok);
        }
        if ok == 0 {
            let log = shader_log(shader);
            unsafe {
                glDeleteShader(shader);
            }
            Err(log)
        } else {
            Ok(shader)
        }
    }

    fn check_program(program: u32) -> Result<(), String> {
        let mut ok = 0;
        unsafe {
            glGetProgramiv(program, GL_LINK_STATUS, &mut ok);
        }
        if ok == 0 {
            Err(program_log(program))
        } else {
            Ok(())
        }
    }

    fn shader_log(shader: u32) -> String {
        let mut buf = vec![0i8; 2048];
        let mut len = 0;
        unsafe {
            glGetShaderInfoLog(shader, buf.len() as i32, &mut len, buf.as_mut_ptr());
            CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
        }
    }

    fn program_log(program: u32) -> String {
        let mut buf = vec![0i8; 2048];
        let mut len = 0;
        unsafe {
            glGetProgramInfoLog(program, buf.len() as i32, &mut len, buf.as_mut_ptr());
            CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
        }
    }

    fn uniform(program: u32, name: &str) -> i32 {
        let Ok(c) = CString::new(name) else {
            return -1;
        };
        unsafe { glGetUniformLocation(program, c.as_ptr()) }
    }
}
