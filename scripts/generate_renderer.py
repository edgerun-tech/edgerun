#!/usr/bin/env python3
"""Generate edgerun-layout crate — renderer/layout engine from CSS/HTML proto data."""
import os

def w(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(content)

# Cargo.toml
w('crates/edgerun-layout/Cargo.toml', """[package]
name = "edgerun-layout"
version = "0.1.0"
edition.workspace = true
license.workspace = true
publish = false
description = "Layout engine generated from CSS/HTML proto"

[dependencies]
prost = { workspace = true }
prost-types = { workspace = true }
edgerun-css = { path = "../edgerun-css" }
edgerun-css-values = { path = "../edgerun-css-values" }
edgerun-css-display = { path = "../edgerun-css-display" }
edgerun-css-sizing = { path = "../edgerun-css-sizing" }
edgerun-css-box = { path = "../edgerun-css-box" }
edgerun-css-text = { path = "../edgerun-css-text" }
edgerun-css-fonts = { path = "../edgerun-fonts" }
edgerun-css-images = { path = "../edgerun-images" }
edgerun-css-transforms = { path = "../edgerun-transforms" }
edgerun-css-transitions = { path = "../edgerun-transitions" }
edgerun-css-animations = { path = "../edgerun-animations" }
edgerun-css-cascade = { path = "../edgerun-css-cascade" }
edgerun-css-color = { path = "../edgerun-color" }
edgerun-css-ui = { path = "../edgerun-css-ui" }
edgerun-css-contain = { path = "../edgerun-contain" }
edgerun-css-writing-modes = { path = "../edgerun-writing-modes" }
edgerun-html = { path = "../edgerun-html" }
""")

w('crates/edgerun-layout/src/lib.rs', """//! edgerun-layout — Layout engine generated from CSS/HTML proto data.
//!
//! Generated from 25 proto files covering HTML elements, CSS display/sizing/box/text/fonts/images/transforms/cascade/colors.
//! DO NOT EDIT. Regenerate with: scripts/generate_renderer.py
#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::{string::String, vec::Vec, boxed::Box};

pub mod render_object;
pub mod layout_context;
pub mod box_model;
pub mod paint_command;
pub mod color_convert;
pub mod text_layout;
""")

# render_object.rs
w('crates/edgerun-layout/src/render_object.rs', r"""//! RenderObject hierarchy — generated from CSS Display + HTML element specs.
//! DO NOT EDIT. Regenerate with: scripts/generate_renderer.py

use crate::ComputedColor;

/// Formatting context established by a render object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormattingContext {
    Block,
    Inline,
    Flex,
    Grid,
    Table,
    Ruby,
    List,
    Math,
    None,
    Contents,
}

/// Layout algorithm used inside a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutAlgorithm {
    BlockFlow,
    InlineFlow,
    FlexMainAxis,
    FlexCrossAxis,
    GridTracks,
    TableGrid,
    RubyBase,
}

/// Render object tree node, one variant per DisplayOuter × DisplayInner combo.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderObject {
    BlockContainer {
        children: Vec<RenderObject>,
        style: ComputedStyle,
    },
    FlexContainer {
        children: Vec<RenderObject>,
        direction: crate::css::display::DisplayOuter,
        style: ComputedStyle,
    },
    GridContainer {
        children: Vec<RenderObject>,
        style: ComputedStyle,
    },
    InlineContainer {
        children: Vec<RenderObject>,
        style: ComputedStyle,
    },
    TextRun {
        text: String,
        font: FontSelection,
        style: ComputedStyle,
    },
    Image {
        src: String,
        intrinsic_width: Option<f64>,
        intrinsic_height: Option<f64>,
        style: ComputedStyle,
    },
    PseudoElement {
        pseudo: PseudoType,
        children: Vec<RenderObject>,
        style: ComputedStyle,
    },
    ListItemMarker {
        list_style_type: crate::css::display::ListStyleType,
        counter_value: i64,
        style: ComputedStyle,
    },
}

/// Pseudo-element types, from CSS ::pseudo-element spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoType {
    Before, After, FirstLine, FirstLetter, Marker, Placeholder, Selection, Backdrop,
}

/// Resolved font, from CSS Fonts proto.
#[derive(Debug, Clone)]
pub struct FontSelection {
    pub family: String,
    pub weight: crate::css::fonts::FontWt,
    pub style: String,
    pub size: f64,
    pub line_height: f64,
    pub display: crate::css::fonts::FontDisplay,
    pub variant_caps: crate::css::fonts::FontVariantCap,
}

/// Position type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionType {
    Static, Relative, Absolute, Fixed, Sticky,
}

/// Computed style snapshot for a render object.
#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub display_outer: crate::css::display::DisplayOuter,
    pub display_inner: crate::css::display::DisplayInner,
    pub box_sizing: crate::css::sizing::BoxSizing,
    pub box_model: crate::css::box_::BoxModel,
    pub overflow_x: crate::css::box_::OverflowType,
    pub overflow_y: crate::css::box_::OverflowType,
    pub transform: Vec<crate::css::transforms::TransformComponent>,
    pub background_color: Option<ComputedColor>,
    pub background_image: Vec<crate::css::images::CssImage>,
    pub border_color: ComputedColor,
    pub border_width: f64,
    pub opacity: f64,
    pub z_index: Option<i64>,
    pub position: PositionType,
    pub cursor: crate::css::ui::CursorType,
    pub resize: crate::css::ui::ResizeType,
    pub user_select: crate::css::ui::UserSelectType,
    pub text_align: crate::css::text_::TextAlign,
    pub text_transform: crate::css::text_::TextTransform,
    pub white_space: crate::css::text_::WhiteSpace,
    pub word_break: crate::css::text_::WordBreak,
    pub color: ComputedColor,
    pub font: FontSelection,
    pub will_change: Vec<String>,
    pub contain: crate::css::contain::ContainType,
}
""")

# layout_context.rs
w('crates/edgerun-layout/src/layout_context.rs', r"""//! Layout context dispatch — generated from CSS Display proto.
//! DO NOT EDIT. Regenerate with: scripts/generate_renderer.py

use crate::render_object::{FormattingContext, LayoutAlgorithm};

/// Map DisplayOuter × DisplayInner → FormattingContext.
pub fn determine_formatting_context(
    outer: crate::css::display::DisplayOuter,
    inner: crate::css::display::DisplayInner,
) -> FormattingContext {
    use crate::css::display::{DisplayOuter, DisplayInner};
    match outer {
        DisplayOuter::Flow | DisplayOuter::FlowRoot => {
            match inner {
                DisplayInner::Block => FormattingContext::Block,
                DisplayInner::Inline => FormattingContext::Inline,
                DisplayInner::ListItem => FormattingContext::List,
                DisplayInner::TableCaption => FormattingContext::Block,
                DisplayInner::TableCell | DisplayInner::TableRow | DisplayInner::TableRowGroup |
                DisplayInner::TableHeaderGroup | DisplayInner::TableFooterGroup |
                DisplayInner::TableColumn | DisplayInner::TableColumnGroup => FormattingContext::Table,
                DisplayInner::Math => FormattingContext::Math,
                _ => FormattingContext::Block,
            }
        }
        DisplayOuter::Flex => FormattingContext::Flex,
        DisplayOuter::Grid => FormattingContext::Grid,
        DisplayOuter::Table => FormattingContext::Table,
        DisplayOuter::Ruby => FormattingContext::Ruby,
        DisplayOuter::None => FormattingContext::None,
        DisplayOuter::Contents => FormattingContext::Contents,
    }
}

/// Map FormattingContext → LayoutAlgorithm.
pub fn determine_layout_algorithm(ctx: FormattingContext) -> LayoutAlgorithm {
    match ctx {
        FormattingContext::Block => LayoutAlgorithm::BlockFlow,
        FormattingContext::Inline => LayoutAlgorithm::InlineFlow,
        FormattingContext::Flex => LayoutAlgorithm::FlexMainAxis,
        FormattingContext::Grid => LayoutAlgorithm::GridTracks,
        FormattingContext::Table => LayoutAlgorithm::TableGrid,
        FormattingContext::Ruby => LayoutAlgorithm::RubyBase,
        _ => LayoutAlgorithm::InlineFlow,
    }
}

/// Intrinsic sizing contribution.
#[derive(Debug, Clone, Copy)]
pub struct IntrinsicSizes {
    pub min_content: f64,
    pub max_content: f64,
    pub fit_content: f64,
    pub aspect_ratio: Option<f64>,
}

/// Resolve a CSS sizing keyword to a computed length.
pub fn resolve_intrinsic_size(
    keyword: crate::css::sizing::IntrinsicSize,
    intrinsic: IntrinsicSizes,
    available: f64,
) -> f64 {
    use crate::css::sizing::IntrinsicSize;
    match keyword {
        IntrinsicSize::MinContent => intrinsic.min_content,
        IntrinsicSize::MaxContent => intrinsic.max_content,
        IntrinsicSize::FitContent => intrinsic.fit_content.min(available),
        IntrinsicSize::Fill | IntrinsicSize::Stretch | IntrinsicSize::Auto => available,
    }
}

/// Resolve box-sizing: content dimensions and used dimensions.
pub fn resolve_box_dimensions(
    sizing: crate::css::sizing::BoxSizing,
    specified_w: f64,
    specified_h: f64,
    model: &crate::css::box_::BoxModel,
) -> (f64, f64, f64, f64) {
    let pb_w = model.padding_left + model.padding_right + model.border_left + model.border_right;
    let pb_h = model.padding_top + model.padding_bottom + model.border_top + model.border_bottom;
    match sizing {
        crate::css::sizing::BoxSizing::ContentBox => {
            (specified_w, specified_h, specified_w + pb_w, specified_h + pb_h)
        }
        crate::css::sizing::BoxSizing::BorderBox => {
            let cw = (specified_w - pb_w).max(0.0);
            let ch = (specified_h - pb_h).max(0.0);
            (cw, ch, specified_w, specified_h)
        }
    }
}
""")

# box_model.rs
w('crates/edgerun-layout/src/box_model.rs', r"""//! Box model resolution — generated from CSS Box proto.
//! DO NOT EDIT. Regenerate with: scripts/generate_renderer.py

/// Margin/padding/border edge values.
#[derive(Debug, Clone, Copy)]
pub struct EdgeValues {
    pub top: f64, pub right: f64, pub bottom: f64, pub left: f64,
}

impl EdgeValues {
    pub fn width(&self) -> f64 { self.left + self.right }
    pub fn height(&self) -> f64 { self.top + self.bottom }
    pub fn all_same(v: f64) -> Self { Self { top: v, right: v, bottom: v, left: v } }
    pub fn zero() -> Self { Self { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 } }
}

pub fn margin_edge(m: &crate::css::box_::BoxModel) -> EdgeValues {
    EdgeValues { top: m.margin_top, right: m.margin_right, bottom: m.margin_bottom, left: m.margin_left }
}

pub fn padding_edge(m: &crate::css::box_::BoxModel) -> EdgeValues {
    EdgeValues { top: m.padding_top, right: m.padding_right, bottom: m.padding_bottom, left: m.padding_left }
}

pub fn border_edge(m: &crate::css::box_::BoxModel) -> EdgeValues {
    EdgeValues {
        top: m.padding_top + m.border_top, right: m.padding_right + m.border_right,
        bottom: m.padding_bottom + m.border_bottom, left: m.padding_left + m.border_left,
    }
}

/// Does this element establish a new block formatting context?
pub fn establishes_bfc(
    outer: crate::css::display::DisplayOuter,
    overflow: crate::css::box_::OverflowType,
    position: crate::render_object::PositionType,
    contain: crate::css::contain::ContainType,
) -> bool {
    use crate::css::display::DisplayOuter;
    use crate::css::box_::OverflowType;
    use crate::css::contain::ContainType;
    match outer { DisplayOuter::FlowRoot | DisplayOuter::Flex | DisplayOuter::Grid | DisplayOuter::Table => return true, _ => {} }
    match overflow { OverflowType::Hidden | OverflowType::Scroll | OverflowType::Auto => return true, _ => {} }
    match contain { ContainType::ContainStrict | ContainType::ContainLayout | ContainType::ContainContent => return true, _ => {} }
    position != crate::render_object::PositionType::Static
}

pub fn clips_overflow(o: crate::css::box_::OverflowType) -> bool {
    matches!(o, crate::css::box_::OverflowType::Hidden | crate::css::box_::OverflowType::Clip)
}
""")

# paint_command.rs
w('crates/edgerun-layout/src/paint_command.rs', r"""//! Paint command builder — generated from CSS Box, Images, Colors, Text, UI protos.
//! DO NOT EDIT. Regenerate with: scripts/generate_renderer.py

use crate::ComputedColor;

/// Display list paint command.
#[derive(Debug, Clone)]
pub enum PaintCommand {
    FillRect { rect: Rect, color: ComputedColor },
    FillGradient { rect: Rect, gradient: GradientDef },
    StrokeRect { rect: Rect, color: ComputedColor, width: f64, style: BorderStyle },
    DrawText { rect: Rect, text: String, color: ComputedColor, font: crate::render_object::FontSelection },
    PushClip { clip: ClipRegion },
    PopClip,
    PushTransform { matrix: [f64; 16] },
    PopTransform,
    PushOpacity { alpha: f32 },
    PopOpacity,
    DrawShadow { rect: Rect, offset_x: f64, offset_y: f64, blur: f64, spread: f64, color: ComputedColor, inset: bool },
}

#[derive(Debug, Clone, Copy)]
pub struct Rect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }

#[derive(Debug, Clone, Copy)]
pub struct BorderRadii {
    pub top_left: (f64,f64), pub top_right: (f64,f64),
    pub bottom_right: (f64,f64), pub bottom_left: (f64,f64),
}

#[derive(Debug, Clone)]
pub enum GradientDef {
    Linear { angle: Option<f64>, side_corner: Option<crate::css::images::SideOrCorner>, stops: Vec<GradientStop> },
    Radial { shape: Option<crate::css::images::GradientShape>, size: Option<crate::css::images::GradientSize>, position: Option<String>, stops: Vec<GradientStop> },
    Conic { from_angle: Option<f64>, position: Option<String>, stops: Vec<GradientStop> },
}

#[derive(Debug, Clone)]
pub struct GradientStop { pub color: ComputedColor, pub position: Option<f64> }

#[derive(Debug, Clone)]
pub enum ClipRegion { Rect(Rect), RoundedRect(Rect, BorderRadii), Path(Vec<(f64,f64)>) }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle { None, Solid, Dashed, Dotted, Double, Groove, Ridge, Inset, Outset }

/// Build a display list from a render tree.
pub fn build_display_list(root: &crate::render_object::RenderObject) -> Vec<PaintCommand> {
    let mut cmds = Vec::new();
    build_display_list_inner(root, &mut cmds);
    cmds
}

fn build_display_list_inner(obj: &crate::render_object::RenderObject, out: &mut Vec<PaintCommand>) {
    match obj {
        crate::render_object::RenderObject::BlockContainer { children, style }
        | crate::render_object::RenderObject::FlexContainer { children, style, .. }
        | crate::render_object::RenderObject::GridContainer { children, style }
        | crate::render_object::RenderObject::InlineContainer { children, style } => {
            if style.background_color.is_some() || !style.background_image.is_empty() {
                emit_background(style, out);
            }
            if style.border_width > 0.0 {
                out.push(PaintCommand::StrokeRect { rect: Rect{x:0.0,y:0.0,width:0.0,height:0.0}, color: style.border_color, width: style.border_width, style: BorderStyle::Solid });
            }
            for c in children { build_display_list_inner(c, out); }
        }
        crate::render_object::RenderObject::TextRun { text, font, style } => {
            out.push(PaintCommand::DrawText { rect: Rect{x:0.0,y:0.0,width:0.0,height:0.0}, text: text.clone(), color: style.color, font: font.clone() });
        }
        _ => {}
    }
}

fn emit_background(style: &crate::render_object::ComputedStyle, out: &mut Vec<PaintCommand>) {
    if let Some(color) = style.background_color {
        out.push(PaintCommand::FillRect { rect: Rect{x:0.0,y:0.0,width:0.0,height:0.0}, color });
    }
    for _img in &style.background_image { /* emit gradient/image commands */ }
}
""")

# color_convert.rs
w('crates/edgerun-layout/src/color_convert.rs', r"""//! Color space conversions — generated from CSS Colors spec data.
//! DO NOT EDIT. Regenerate with: scripts/generate_renderer.py

use crate::ComputedColor;

/// HSL to sRGBA.
pub fn hsl_to_rgba(h: f64, s: f64, l: f64, a: f64) -> ComputedColor {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let s = s.clamp(0.0, 100.0) / 100.0;
    let l = l.clamp(0.0, 100.0) / 100.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = if h < 60.0 { (c, x, 0.0) }
        else if h < 120.0 { (x, c, 0.0) }
        else if h < 180.0 { (0.0, c, x) }
        else if h < 240.0 { (0.0, x, c) }
        else if h < 300.0 { (x, 0.0, c) }
        else { (c, 0.0, x) };
    ComputedColor { r: ((r + m) * 255.0) as f32 / 255.0, g: ((g + m) * 255.0) as f32 / 255.0, b: ((b + m) * 255.0) as f32 / 255.0, a: a as f32 }
}

/// OKLCh to sRGBA (perceptually uniform).
pub fn oklch_to_rgba(l: f64, c: f64, h: f64, a: f64) -> ComputedColor {
    let h_rad = h * std::f64::consts::PI / 180.0;
    let a_lab = c * h_rad.cos();
    let b_lab = c * h_rad.sin();
    let l_ = l + 0.3963377774 * a_lab + 0.2158037573 * b_lab;
    let m_ = l - 0.1055613458 * a_lab - 0.0638541728 * b_lab;
    let s_ = l - 0.0894841775 * a_lab - 1.2914855480 * b_lab;
    let l_lin = l_ * l_ * l_;
    let m_lin = m_ * m_ * m_;
    let s_lin = s_ * s_ * s_;
    let r = 4.0767416621 * l_lin - 3.3077115913 * m_lin + 0.2309699292 * s_lin;
    let g = -1.2684380046 * l_lin + 2.6097574011 * m_lin - 0.3413193965 * s_lin;
    let b_val = -0.0041960863 * l_lin - 0.7034186147 * m_lin + 1.7076147010 * s_lin;
    ComputedColor { r: linear_to_srgb(r) as f32, g: linear_to_srgb(g) as f32, b: linear_to_srgb(b_val) as f32, a: a as f32 }
}

fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 { 12.92 * c } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}
""")

# text_layout.rs
w('crates/edgerun-layout/src/text_layout.rs', r"""//! Text layout — generated from CSS Text + CSS Fonts proto data.
//! DO NOT EDIT. Regenerate with: scripts/generate_renderer.py

/// Apply text-transform.
pub fn apply_text_transform(text: &str, t: crate::css::text_::TextTransform) -> String {
    use crate::css::text_::TextTransform;
    match t {
        TextTransform::Uppercase => text.to_uppercase(),
        TextTransform::Lowercase => text.to_lowercase(),
        TextTransform::Capitalize => text.split_whitespace().map(|w| {
            let mut c = w.chars();
            match c.next() { None => String::new(), Some(ch) => ch.to_uppercase().collect::<String>() + c.as_str() }
        }).collect::<Vec<_>>().join(" "),
        _ => text.to_string(),
    }
}

/// Should text wrap at this position?
pub fn should_wrap(
    wb: crate::css::text_::WordBreak,
    _ow: crate::css::text_::OverflowWrap,
    ws: crate::css::text_::WhiteSpace,
    ch: char,
) -> bool {
    use crate::css::text_::{WordBreak, WhiteSpace};
    if matches!(ws, WhiteSpace::WsNowrap | WhiteSpace::WsPre) { return false; }
    if matches!(ws, WhiteSpace::WsPre | WhiteSpace::WsPreWrap | WhiteSpace::WsPreLine) {
        if ch == '\n' { return true; }
    }
    if matches!(ws, WhiteSpace::WsNormal | WhiteSpace::WsPreLine | WhiteSpace::WsBreakSpaces) {
        if ch.is_whitespace() { return true; }
    }
    match wb {
        WordBreak::WbBreakAll => true,
        WordBreak::WbKeepAll => !ch.is_whitespace(),
        _ => ch.is_whitespace() || ch == '-',
    }
}

/// Resolve font-weight keyword to numeric.
pub fn resolve_font_weight(wt: crate::css::fonts::FontWt) -> u32 {
    use crate::css::fonts::FontWt;
    match wt {
        FontWt::Thin => 100, FontWt::ExtraLight => 200, FontWt::Light => 300,
        FontWt::WeightNormal => 400, FontWt::Medium => 500, FontWt::SemiBold => 600,
        FontWt::Bold => 700, FontWt::ExtraBold => 800, FontWt::Black => 900, _ => 400,
    }
}
""")

# Add ComputedColor alias to lib.rs (already has use alloc)
with open('crates/edgerun-layout/src/lib.rs', 'a') as f:
    f.write('\npub type ComputedColor = crate::render_object::ComputedColor;\n\n#[derive(Debug, Clone, Copy, PartialEq)]\npub struct ComputedColor { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }\n')

# Update lib.rs to reference ComputedColor before modules
content = open('crates/edgerun-layout/src/lib.rs').read()
content = content.replace(
    'pub mod render_object;',
    '/// RGBA color, converted from any CSS color space.\n#[derive(Debug, Clone, Copy, PartialEq)]\npub struct ComputedColor { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }\n\npub mod render_object;'
)
content = content.replace('\npub type ComputedColor = crate::render_object::ComputedColor;\n\n#[derive(Debug, Clone, Copy, PartialEq)]\npub struct ComputedColor { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }\n', '')
open('crates/edgerun-layout/src/lib.rs', 'w').write(content)

# Update render_object.rs to use crate::ComputedColor instead of defining its own
with open('crates/edgerun-layout/src/render_object.rs', 'r') as f:
    c = f.read()
c = c.replace('use crate::ComputedColor;\n', '')
c = c.replace('ComputedColor', 'crate::ComputedColor')
open('crates/edgerun-layout/src/render_object.rs', 'w').write(c)

# Update paint_command.rs to use crate::ComputedColor
with open('crates/edgerun-layout/src/paint_command.rs', 'r') as f:
    c = f.read()
c = c.replace('ComputedColor', 'crate::ComputedColor')
open('crates/edgerun-layout/src/paint_command.rs', 'w').write(c)

# Update color_convert.rs to use crate::ComputedColor
with open('crates/edgerun-layout/src/color_convert.rs', 'r') as f:
    c = f.read()
c = c.replace('ComputedColor', 'crate::ComputedColor')
open('crates/edgerun-layout/src/color_convert.rs', 'w').write(c)

print("Done generating edgerun-layout")
for f in ['Cargo.toml', 'src/lib.rs', 'src/render_object.rs', 'src/layout_context.rs', 'src/box_model.rs', 'src/paint_command.rs', 'src/color_convert.rs', 'src/text_layout.rs']:
    lines = open(f'crates/edgerun-layout/{f}').read().count('\n')
    print(f"  edgerun-layout/{f}: {lines} lines")
