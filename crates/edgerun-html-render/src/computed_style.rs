//! Computes typed `ComputedStyle` from raw CSS declarations.
//!
//! This bridges the gap between `css_parser::Declarations` (BTreeMap<String, String>)
//! and `edgerun_layout::render_object::ComputedStyle` by using the value parser.

#![no_std]
extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use edgerun_css_value_parser::{parse_css_value, CssValue, LengthUnit};
use edgerun_layout::render_object::{
    Color, ComputedStyle, FontSelection, FormattingContext, PositionType,
};

use crate::css_parser::Declarations;

/// Default ComputedStyle used when no CSS declarations apply.
pub fn default_style() -> ComputedStyle {
    ComputedStyle {
        formatting_context: FormattingContext::Inline,
        position: PositionType::Static,
        opacity: 1.0,
        z_index: None,
        background_color: None,
        border_color: Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
        border_width: 0.0,
        color: Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
        font: FontSelection {
            family: "sans-serif".into(),
            weight: 400,
            size: 16.0,
            line_height: 20.0,
        },
        has_transform: false,
        will_change: Vec::new(),
    }
}

/// Parse CSS declarations into a typed ComputedStyle.
///
/// Inherited values (from parent) are used for relative units (em, rem, %).
/// If a property is not declared, the default value is used.
pub fn compute_style(decls: &Declarations, inherited: &ComputedStyle) -> ComputedStyle {
    let mut style = inherited.clone();

    // Display / formatting context
    if let Some(v) = decls.get("display") {
        style.formatting_context = parse_display(v);
    }

    // Position
    if let Some(v) = decls.get("position") {
        style.position = parse_position(v);
    }

    // Opacity
    if let Some(v) = decls.get("opacity") {
        if let CssValue::Number(n) = parse_css_value(v).unwrap_or(CssValue::Number(1.0)) {
            style.opacity = n.clamp(0.0, 1.0);
        }
    }

    // Z-index
    if let Some(v) = decls.get("z-index") {
        style.z_index = parse_z_index(v);
    }

    // Color
    if let Some(v) = decls.get("color") {
        if let Some(c) = parse_color_value(v) {
            style.color = c;
        }
    }

    // Background color
    if let Some(v) = decls.get("background-color") {
        style.background_color = parse_color_value(v);
    }

    // Border color
    if let Some(v) = decls.get("border-color") {
        if let Some(c) = parse_color_value(v) {
            style.border_color = c;
        }
    }

    // Border width
    if let Some(v) = decls.get("border-width") {
        style.border_width = parse_border_width(v);
    }

    // Font size
    if let Some(v) = decls.get("font-size") {
        style.font.size = parse_font_size(v, inherited.font.size);
    }

    // Font weight
    if let Some(v) = decls.get("font-weight") {
        style.font.weight = parse_font_weight(v);
    }

    // Font family
    if let Some(v) = decls.get("font-family") {
        style.font.family = parse_font_family(v);
    }

    // Line height
    if let Some(v) = decls.get("line-height") {
        style.font.line_height = parse_line_height(v, style.font.size);
    }

    style
}

/// Parse a `display` value into a FormattingContext.
fn parse_display(value: &str) -> FormattingContext {
    match value.trim().to_lowercase().as_str() {
        "block" => FormattingContext::Block,
        "inline" => FormattingContext::Inline,
        "flex" => FormattingContext::Flex,
        "grid" => FormattingContext::Grid,
        "table" => FormattingContext::Table,
        "ruby" => FormattingContext::Ruby,
        "list-item" => FormattingContext::List,
        "none" => FormattingContext::None,
        "contents" => FormattingContext::Contents,
        _ => FormattingContext::Inline,
    }
}

/// Parse a `position` value.
fn parse_position(value: &str) -> PositionType {
    match value.trim().to_lowercase().as_str() {
        "static" => PositionType::Static,
        "relative" => PositionType::Relative,
        "absolute" => PositionType::Absolute,
        "fixed" => PositionType::Fixed,
        "sticky" => PositionType::Sticky,
        _ => PositionType::Static,
    }
}

/// Parse a `z-index` value.
fn parse_z_index(value: &str) -> Option<i64> {
    match value.trim().to_lowercase().as_str() {
        "auto" => None,
        _ => {
            if let CssValue::Number(n) = parse_css_value(value).unwrap_or(CssValue::Number(0.0)) {
                return Some(n as i64);
            }
            None
        }
    }
}

/// Parse a color from a CSS value string.
fn parse_color_value(value: &str) -> Option<Color> {
    let cv = parse_css_value(value)?;
    match cv {
        CssValue::Color(c) => Some(css_color_to_rgba(&c)),
        _ => None,
    }
}

/// Convert edgerun_color::CssColor to layout Color (f32 RGBA).
fn css_color_to_rgba(c: &edgerun_color::CssColor) -> Color {
    use edgerun_color::{CssColor, NamedColor};

    // Resolve named colors to rgba
    if let CssColor::Named(name) = c {
        // NamedColor → RGBA via edgerun-rasterizer's color_lut mapping
        // For now, use a reasonable default for common colors
        return named_color_to_rgba(name);
    }

    if let CssColor::Rgb { r, g, b, alpha } = c {
        return Color {
            r: *r as f32,
            g: *g as f32,
            b: *b as f32,
            a: *alpha as f32,
        };
    }

    // For HSL/other color spaces, convert to sRGB via edgerun-layout
    // Fall back to black for now
    Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    }
}

/// Convert NamedColor to RGBA.
fn named_color_to_rgba(name: &NamedColor) -> Color {
    use edgerun_color::NamedColor;

    // Common colors
    let (r, g, b) = match name {
        NamedColor::Red => (1.0, 0.0, 0.0),
        NamedColor::Green => (0.0, 0.5, 0.0),
        NamedColor::Blue => (0.0, 0.0, 1.0),
        NamedColor::Black => (0.0, 0.0, 0.0),
        NamedColor::White => (1.0, 1.0, 1.0),
        NamedColor::Transparent => (0.0, 0.0, 0.0),
        NamedColor::Orange => (1.0, 0.65, 0.0),
        NamedColor::Yellow => (1.0, 1.0, 0.0),
        NamedColor::Purple => (0.5, 0.0, 0.5),
        NamedColor::Pink => (1.0, 0.75, 0.8),
        NamedColor::Gray | NamedColor::Grey => (0.5, 0.5, 0.5),
        NamedColor::Darkgray | NamedColor::Darkgrey => (0.33, 0.33, 0.33),
        NamedColor::Lightgray | NamedColor::Lightgrey => (0.83, 0.83, 0.83),
        NamedColor::Navy => (0.0, 0.0, 0.5),
        NamedColor::Teal => (0.0, 0.5, 0.5),
        NamedColor::Maroon => (0.5, 0.0, 0.0),
        NamedColor::Olive => (0.5, 0.5, 0.0),
        NamedColor::Aqua | NamedColor::Cyan => (0.0, 1.0, 1.0),
        NamedColor::Fuchsia | NamedColor::Magenta => (1.0, 0.0, 1.0),
        NamedColor::Silver => (0.75, 0.75, 0.75),
        NamedColor::Lime => (0.0, 1.0, 0.0),
        NamedColor::Rebeccapurple => (0.4, 0.2, 0.6),
        NamedColor::Coral => (1.0, 0.5, 0.31),
        NamedColor::Tomato => (1.0, 0.39, 0.28),
        NamedColor::Gold => (1.0, 0.84, 0.0),
        NamedColor::Crimson => (0.86, 0.08, 0.24),
        NamedColor::Chocolate => (0.82, 0.41, 0.12),
        NamedColor::Firebrick => (0.7, 0.13, 0.13),
        NamedColor::Indigo => (0.29, 0.0, 0.51),
        NamedColor::Salmon => (0.98, 0.5, 0.45),
        NamedColor::Sandybrown => (0.96, 0.64, 0.38),
        NamedColor::Turquoise => (0.25, 0.88, 0.82),
        NamedColor::Violet => (0.93, 0.51, 0.93),
        NamedColor::Wheat => (0.96, 0.87, 0.7),
        NamedColor::Peru => (0.8, 0.52, 0.25),
        NamedColor::Plum => (0.87, 0.63, 0.87),
        NamedColor::Orchid => (0.85, 0.44, 0.84),
        NamedColor::Tan => (0.82, 0.71, 0.55),
        NamedColor::Sienna => (0.63, 0.32, 0.18),
        _ => (0.5, 0.5, 0.5), // fallback gray
    };

    let alpha = if *name == NamedColor::Transparent { 0.0 } else { 1.0 };

    Color { r, g, b, a: alpha }
}

/// Parse border-width: thin=1, medium=3, thick=5, or numeric.
fn parse_border_width(value: &str) -> f64 {
    match value.trim().to_lowercase().as_str() {
        "thin" => 1.0,
        "medium" => 3.0,
        "thick" => 5.0,
        _ => {
            if let CssValue::Length { value, unit } = parse_css_value(value).unwrap_or(CssValue::Number(0.0)) {
                if unit == LengthUnit::Px {
                    return value;
                }
                // em/rem relative to default font size
                if unit == LengthUnit::Em || unit == LengthUnit::Rem {
                    return value * 16.0;
                }
            }
            0.0
        }
    }
}

/// Parse font-size with inherited parent size for relative units.
fn parse_font_size(value: &str, inherited_size: f64) -> f64 {
    let v = value.trim();
    match v.to_lowercase().as_str() {
        "xx-small" => 9.0,
        "x-small" => 10.0,
        "small" => 13.0,
        "medium" => 16.0,
        "large" => 18.0,
        "x-large" => 24.0,
        "xx-large" => 32.0,
        "xxx-large" => 48.0,
        "smaller" => inherited_size * 0.833,
        "larger" => inherited_size * 1.2,
        _ => {
            if let CssValue::Length { value, unit } = parse_css_value(v).unwrap_or(CssValue::Number(16.0)) {
                resolve_length(value, unit, inherited_size)
            } else if let CssValue::Percentage(pct) = parse_css_value(v).unwrap_or(CssValue::Percentage(1.0)) {
                pct * inherited_size
            } else {
                16.0
            }
        }
    }
}

/// Resolve a length with a unit to pixels, using inherited size for relative units.
fn resolve_length(value: f64, unit: LengthUnit, inherited_size: f64) -> f64 {
    match unit {
        LengthUnit::Em | LengthUnit::Rem => value * inherited_size,
        LengthUnit::Px => value,
        LengthUnit::Pt => value * 96.0 / 72.0,
        LengthUnit::In => value * 96.0,
        LengthUnit::Cm => value * 96.0 / 2.54,
        LengthUnit::Mm => value * 96.0 / 25.4,
        LengthUnit::Q => value * 96.0 / 101.6,
        LengthUnit::Pc => value * 16.0,
        LengthUnit::Ex | LengthUnit::Ch => value * inherited_size * 0.5, // approximate
        // Viewport units — no viewport info, fall back to inherited
        _ => value,
    }
}

/// Parse font-weight: 100-900, normal=400, bold=700.
fn parse_font_weight(value: &str) -> u32 {
    match value.trim().to_lowercase().as_str() {
        "normal" => 400,
        "bold" => 700,
        "bolder" => 700,
        "lighter" => 300,
        _ => {
            if let CssValue::Number(n) = parse_css_value(value.trim()).unwrap_or(CssValue::Number(400.0)) {
                return (n as u32).clamp(100, 900);
            }
            400
        }
    }
}

/// Parse font-family: strip quotes from comma-separated list.
fn parse_font_family(value: &str) -> String {
    // Take first font from the list
    let first = value.split(',').next().unwrap_or(value).trim();
    // Strip quotes
    let first = first.trim_matches('"').trim_matches('\'');
    if first.is_empty() {
        "sans-serif".into()
    } else {
        first.into()
    }
}

/// Parse line-height: number * font_size, length, or normal.
fn parse_line_height(value: &str, font_size: f64) -> f64 {
    let v = value.trim();
    if v.to_lowercase() == "normal" {
        return font_size * 1.2;
    }
    // Unitless number: multiply font_size
    if let CssValue::Number(n) = parse_css_value(v).unwrap_or(CssValue::Number(1.2)) {
        return n * font_size;
    }
    // Length
    if let CssValue::Length { value, unit } = parse_css_value(v).unwrap_or(CssValue::Number(font_size)) {
        return resolve_length(value, unit, font_size);
    }
    font_size * 1.2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_style() {
        let s = default_style();
        assert_eq!(s.font.size, 16.0);
        assert_eq!(s.font.weight, 400);
        assert_eq!(s.opacity, 1.0);
        assert_eq!(s.position, PositionType::Static);
    }

    #[test]
    fn test_compute_font_size_px() {
        let mut decls = Declarations::new();
        decls.insert("font-size".into(), "24px".into());
        let parent = default_style();
        let style = compute_style(&decls, &parent);
        assert!((style.font.size - 24.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_font_size_em() {
        let mut decls = Declarations::new();
        decls.insert("font-size".into(), "1.5em".into());
        let mut parent = default_style();
        parent.font.size = 20.0;
        let style = compute_style(&decls, &parent);
        assert!((style.font.size - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_font_size_keyword() {
        let mut decls = Declarations::new();
        decls.insert("font-size".into(), "x-large".into());
        let parent = default_style();
        let style = compute_style(&decls, &parent);
        assert!((style.font.size - 24.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_color_named() {
        let mut decls = Declarations::new();
        decls.insert("color".into(), "red".into());
        let parent = default_style();
        let style = compute_style(&decls, &parent);
        assert!((style.color.r - 1.0).abs() < 0.01);
        assert!(style.color.g < 0.01);
    }

    #[test]
    fn test_compute_color_hex() {
        let mut decls = Declarations::new();
        decls.insert("color".into(), "#00ff00".into());
        let parent = default_style();
        let style = compute_style(&decls, &parent);
        assert!((style.color.g - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_background_color() {
        let mut decls = Declarations::new();
        decls.insert("background-color".into(), "orange".into());
        let parent = default_style();
        let style = compute_style(&decls, &parent);
        assert!(style.background_color.is_some());
        let bg = style.background_color.unwrap();
        assert!((bg.r - 1.0).abs() < 0.01);
        assert!((bg.g - 0.65).abs() < 0.01);
    }

    #[test]
    fn test_compute_font_weight() {
        let mut decls = Declarations::new();
        decls.insert("font-weight".into(), "bold".into());
        let parent = default_style();
        let style = compute_style(&decls, &parent);
        assert_eq!(style.font.weight, 700);
    }

    #[test]
    fn test_compute_font_weight_numeric() {
        let mut decls = Declarations::new();
        decls.insert("font-weight".into(), "300".into());
        let parent = default_style();
        let style = compute_style(&decls, &parent);
        assert_eq!(style.font.weight, 300);
    }

    #[test]
    fn test_compute_font_family() {
        let mut decls = Declarations::new();
        decls.insert("font-family".into(), "'Helvetica Neue', Arial, sans-serif".into());
        let parent = default_style();
        let style = compute_style(&decls, &parent);
        assert_eq!(style.font.family, "Helvetica Neue");
    }

    #[test]
    fn test_compute_opacity() {
        let mut decls = Declarations::new();
        decls.insert("opacity".into(), "0.75".into());
        let parent = default_style();
        let style = compute_style(&decls, &parent);
        assert!((style.opacity - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_compute_position() {
        let mut decls = Declarations::new();
        decls.insert("position".into(), "absolute".into());
        let parent = default_style();
        let style = compute_style(&decls, &parent);
        assert_eq!(style.position, PositionType::Absolute);
    }

    #[test]
    fn test_compute_z_index() {
        let mut decls = Declarations::new();
        decls.insert("z-index".into(), "10".into());
        let parent = default_style();
        let style = compute_style(&decls, &parent);
        assert_eq!(style.z_index, Some(10));
    }

    #[test]
    fn test_compute_z_index_auto() {
        let mut decls = Declarations::new();
        decls.insert("z-index".into(), "auto".into());
        let parent = default_style();
        let style = compute_style(&decls, &parent);
        assert_eq!(style.z_index, None);
    }

    #[test]
    fn test_compute_border_width() {
        let mut decls = Declarations::new();
        decls.insert("border-width".into(), "2px".into());
        let parent = default_style();
        let style = compute_style(&decls, &parent);
        assert!((style.border_width - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_line_height_unitless() {
        let mut decls = Declarations::new();
        decls.insert("line-height".into(), "1.5".into());
        let parent = default_style();
        let style = compute_style(&decls, &parent);
        assert!((style.font.line_height - 24.0).abs() < 0.01); // 1.5 * 16
    }

    #[test]
    fn test_compute_multiple_properties() {
        let mut decls = Declarations::new();
        decls.insert("font-size".into(), "20px".into());
        decls.insert("color".into(), "blue".into());
        decls.insert("font-weight".into(), "700".into());
        decls.insert("background-color".into(), "#ffff00".into());

        let parent = default_style();
        let style = compute_style(&decls, &parent);

        assert!((style.font.size - 20.0).abs() < 0.01);
        assert_eq!(style.font.weight, 700);
        assert!((style.color.b - 1.0).abs() < 0.01);
        assert!(style.background_color.is_some());
    }
}
