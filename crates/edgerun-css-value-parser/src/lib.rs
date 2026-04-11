//! Layer G1: CSS Value Type Parser
//!
//! Converts CSS value strings (e.g. `"32px"`, `"red"`, `"calc(100% - 20px)"`)
//! into strongly-typed Rust values that behavioral crates can consume.
//!
//! ```
//! use edgerun_css_value_parser::{parse_css_value, CssValue, LengthUnit};
//!
//! let v = parse_css_value("32px").unwrap();
//! assert!(matches!(v, CssValue::Length { value: 32.0, unit: LengthUnit::Px }));
//!
//! let v = parse_css_value("red").unwrap();
//! assert!(matches!(v, CssValue::Color(_)));
//!
//! let v = parse_css_value("calc(100% - 20px)").unwrap();
//! assert!(matches!(v, CssValue::Calc(_)));
//! ```
#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod lengths;
mod colors;
mod keywords;
mod calc;

pub use lengths::{LengthUnit, Length, parse_length, parse_angle, parse_time, parse_frequency, parse_resolution};
pub use colors::{parse_color, parse_hex_color, HexColor};
pub use keywords::{CssKeyword, parse_keyword, parse_css_wide_keyword};
pub use calc::{CalcExpr, CalcTerm, CalcFactor, parse_calc};

use edgerun_color::CssColor;

/// A parsed CSS value.
///
/// This is the main type that behavioral crates should consume.
/// Each variant maps to a specific CSS value type from the spec.
#[derive(Debug, Clone, PartialEq)]
pub enum CssValue {
    /// A length with unit: `32px`, `10em`, `1.5rem`
    Length { value: f64, unit: LengthUnit },
    /// A percentage: `50%`
    Percentage(f64),
    /// A plain number: `1.5`, `0`, `-10`
    Number(f64),
    /// A color: named, hex, rgb(), hsl(), hwb(), currentColor, transparent
    Color(CssColor),
    /// A CSS keyword: `auto`, `none`, `bold`, `center`, etc.
    Keyword(CssKeyword),
    /// A CSS-wide keyword: `inherit`, `initial`, `unset`, `revert`
    WideKeyword(keywords::CssWideKeyword),
    /// A calc() expression: `calc(100% - 20px)`
    Calc(CalcExpr),
    /// A var() reference: `var(--my-prop)`
    Var { name: alloc::string::String, fallback: Option<alloc::string::String> },
    /// A url() reference: `url("font.woff2")`
    Url(alloc::string::String),
    /// A function call: `min()`, `max()`, `clamp()`, `rgb()` (unparsed), etc.
    Function { name: alloc::string::String, args: alloc::vec::Vec<CssValue> },
    /// An identifier that doesn't match any known type: `sans-serif`, `flex`
    Ident(alloc::string::String),
    /// A quoted string: `"Helvetica Neue"`
    String(alloc::string::String),
}

/// Parse a CSS value string into a typed `CssValue`.
///
/// This is the main entry point. It tries parsers in order:
/// 1. CSS-wide keywords (inherit, initial, unset, revert)
/// 2. Lengths (32px, 10em, 1.5rem)
/// 3. Angles (90deg, 1.5turn)
/// 4. Times (2s, 500ms)
/// 5. Frequencies (60hz, 1khz)
/// 6. Resolutions (96dpi, 2dppx)
/// 7. Percentages (50%)
/// 8. Plain numbers (1.5, 0, -10)
/// 9. Colors (red, #ff0000, rgb(255,0,0), hsl(0,100%,50%), currentColor, transparent)
/// 10. calc() expressions
/// 11. var() references
/// 12. url() references
/// 13. Known keywords (auto, none, bold, center, etc.)
/// 14. Fallback: return as Ident
pub fn parse_css_value(input: &str) -> Option<CssValue> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    // 1. CSS-wide keywords
    if let Some(kw) = parse_css_wide_keyword(input) {
        return Some(CssValue::WideKeyword(kw));
    }

    // 2-6. Dimensional values (length, angle, time, frequency, resolution)
    if let Some(length) = parse_length(input) {
        return Some(CssValue::Length { value: length.value, unit: length.unit });
    }
    if let Some(dim) = parse_angle(input) {
        return Some(CssValue::Length { value: dim.value, unit: dim.unit });
    }
    if let Some(dim) = parse_time(input) {
        return Some(CssValue::Length { value: dim.value, unit: dim.unit });
    }
    if let Some(dim) = parse_frequency(input) {
        return Some(CssValue::Length { value: dim.value, unit: dim.unit });
    }
    if let Some(dim) = parse_resolution(input) {
        return Some(CssValue::Length { value: dim.value, unit: dim.unit });
    }

    // 7. Percentage
    if let Some(pct) = parse_percentage(input) {
        return Some(CssValue::Percentage(pct));
    }

    // 8. Plain number
    if let Some(num) = parse_number(input) {
        return Some(CssValue::Number(num));
    }

    // 9. Colors
    if let Some(color) = parse_color(input) {
        return Some(CssValue::Color(color));
    }

    // 10. calc()
    if let Some(calc) = parse_calc(input) {
        return Some(CssValue::Calc(calc));
    }

    // 11. var()
    if let Some(var) = parse_var(input) {
        return Some(CssValue::Var { name: var.0, fallback: var.1 });
    }

    // 12. url()
    if let Some(url) = parse_url(input) {
        return Some(CssValue::Url(url));
    }

    // 13. Known keywords
    if let Some(kw) = parse_keyword(input) {
        return Some(CssValue::Keyword(kw));
    }

    // 14. Identifier fallback
    Some(CssValue::Ident(input.into()))
}

/// Parse a percentage value: `50%` → `0.5`
fn parse_percentage(input: &str) -> Option<f64> {
    let input = input.trim();
    if let Some(rest) = input.strip_suffix('%') {
        let rest = rest.trim();
        if let Ok(v) = rest.parse::<f64>() {
            return Some(v / 100.0);
        }
    }
    None
}

/// Parse a plain number (no unit).
fn parse_number(input: &str) -> Option<f64> {
    let input = input.trim();
    // Must not end with a letter (that would be a dimension)
    if input.chars().last().map_or(false, |c| c.is_ascii_alphabetic()) {
        return None;
    }
    input.parse::<f64>().ok()
}

/// Parse a var() reference: `var(--my-prop)` or `var(--my-prop, fallback)`
fn parse_var(input: &str) -> Option<(alloc::string::String, Option<alloc::string::String>)> {
    let input = input.trim();
    if !input.starts_with("var(") || !input.ends_with(')') {
        return None;
    }
    let inner = &input[4..input.len() - 1]; // between var( and )
    let inner = inner.trim();
    if !inner.starts_with("--") {
        return None;
    }
    // Split on comma for fallback
    let parts: alloc::vec::Vec<&str> = inner.splitn(2, ',').collect();
    let name = parts[0].trim().into();
    let fallback = parts.get(1).map(|s| s.trim().into());
    Some((name, fallback))
}

/// Parse a url() reference: `url("font.woff2")` or `url(font.woff2)`
fn parse_url(input: &str) -> Option<alloc::string::String> {
    let input = input.trim();
    if !input.starts_with("url(") || !input.ends_with(')') {
        return None;
    }
    let inner = &input[4..input.len() - 1];
    let inner = inner.trim();
    // Strip optional quotes
    let inner = inner.strip_prefix('"').unwrap_or(inner);
    let inner = inner.strip_prefix('\'').unwrap_or(inner);
    let inner = inner.strip_suffix('"').unwrap_or(inner);
    let inner = inner.strip_suffix('\'').unwrap_or(inner);
    if inner.is_empty() {
        return None;
    }
    Some(inner.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_color::NamedColor;

    // --- parse_css_value: Lengths ---

    #[test]
    fn test_parse_px() {
        let v = parse_css_value("32px").unwrap();
        assert!(matches!(v, CssValue::Length { value: 32.0, unit: LengthUnit::Px }));
    }

    #[test]
    fn test_parse_em() {
        let v = parse_css_value("1.5em").unwrap();
        assert!(matches!(v, CssValue::Length { value: 1.5, unit: LengthUnit::Em }));
    }

    #[test]
    fn test_parse_rem() {
        let v = parse_css_value("2rem").unwrap();
        assert!(matches!(v, CssValue::Length { value: 2.0, unit: LengthUnit::Rem }));
    }

    #[test]
    fn test_parse_vh() {
        let v = parse_css_value("100vh").unwrap();
        assert!(matches!(v, CssValue::Length { value: 100.0, unit: LengthUnit::Vh }));
    }

    #[test]
    fn test_parse_negative_length() {
        let v = parse_css_value("-10px").unwrap();
        assert!(matches!(v, CssValue::Length { value: -10.0, unit: LengthUnit::Px }));
    }

    #[test]
    fn test_parse_zero_px() {
        let v = parse_css_value("0px").unwrap();
        assert!(matches!(v, CssValue::Length { value: 0.0, unit: LengthUnit::Px }));
    }

    // --- parse_css_value: Angles ---

    #[test]
    fn test_parse_deg() {
        let v = parse_css_value("90deg").unwrap();
        assert!(matches!(v, CssValue::Length { value: 90.0, unit: LengthUnit::Deg }));
    }

    #[test]
    fn test_parse_turn() {
        let v = parse_css_value("0.5turn").unwrap();
        assert!(matches!(v, CssValue::Length { value: 0.5, unit: LengthUnit::Turn }));
    }

    // --- parse_css_value: Times ---

    #[test]
    fn test_parse_seconds() {
        let v = parse_css_value("2s").unwrap();
        assert!(matches!(v, CssValue::Length { value: 2.0, unit: LengthUnit::S }));
    }

    #[test]
    fn test_parse_ms() {
        let v = parse_css_value("500ms").unwrap();
        assert!(matches!(v, CssValue::Length { value: 500.0, unit: LengthUnit::Ms }));
    }

    // --- parse_css_value: Percentages ---

    #[test]
    fn test_parse_percentage() {
        let v = parse_css_value("50%").unwrap();
        assert!(matches!(v, CssValue::Percentage(0.5)));
    }

    #[test]
    fn test_parse_percentage_100() {
        let v = parse_css_value("100%").unwrap();
        assert!(matches!(v, CssValue::Percentage(1.0)));
    }

    #[test]
    fn test_parse_percentage_0() {
        let v = parse_css_value("0%").unwrap();
        assert!(matches!(v, CssValue::Percentage(0.0)));
    }

    // --- parse_css_value: Numbers ---

    #[test]
    fn test_parse_number() {
        let v = parse_css_value("1.5").unwrap();
        assert!(matches!(v, CssValue::Number(1.5)));
    }

    #[test]
    fn test_parse_number_zero() {
        let v = parse_css_value("0").unwrap();
        assert!(matches!(v, CssValue::Number(0.0)));
    }

    #[test]
    fn test_parse_negative_number() {
        let v = parse_css_value("-5").unwrap();
        assert!(matches!(v, CssValue::Number(-5.0)));
    }

    // --- parse_css_value: Colors ---

    #[test]
    fn test_parse_named_color() {
        let v = parse_css_value("red").unwrap();
        assert!(matches!(v, CssValue::Color(CssColor::Named(NamedColor::Red))));
    }

    #[test]
    fn test_parse_hex_color_short() {
        let v = parse_css_value("#f00").unwrap();
        assert!(matches!(v, CssValue::Color(CssColor::Rgb { r: 1.0, g: 0.0, b: 0.0, alpha: 1.0 })));
    }

    #[test]
    fn test_parse_hex_color_long() {
        let v = parse_css_value("#ff0000").unwrap();
        assert!(matches!(v, CssValue::Color(CssColor::Rgb { r: 1.0, g: 0.0, b: 0.0, alpha: 1.0 })));
    }

    #[test]
    fn test_parse_hex_color_with_alpha() {
        let v = parse_css_value("#ff000080").unwrap();
        assert!(matches!(v, CssValue::Color(CssColor::Rgb { r: 1.0, g: 0.0, b: 0.0, alpha: _ })));
        if let CssValue::Color(CssColor::Rgb { alpha, .. }) = v {
            assert!((alpha - 0.502).abs() < 0.01);
        }
    }

    #[test]
    fn test_parse_rgb() {
        let v = parse_css_value("rgb(255, 0, 0)").unwrap();
        assert!(matches!(v, CssValue::Color(CssColor::Rgb { r: 1.0, g: 0.0, b: 0.0, alpha: 1.0 })));
    }

    #[test]
    fn test_parse_rgb_percentage() {
        let v = parse_css_value("rgb(100%, 0%, 0%)").unwrap();
        assert!(matches!(v, CssValue::Color(CssColor::Rgb { r: 1.0, g: 0.0, b: 0.0, alpha: 1.0 })));
    }

    #[test]
    fn test_parse_rgba() {
        let v = parse_css_value("rgba(255, 0, 0, 0.5)").unwrap();
        if let CssValue::Color(CssColor::Rgb { alpha, .. }) = v {
            assert!((alpha - 0.5).abs() < 0.001);
        } else {
            panic!("Expected Rgb color");
        }
    }

    #[test]
    fn test_parse_hsl() {
        let v = parse_css_value("hsl(0, 100%, 50%)").unwrap();
        assert!(matches!(v, CssValue::Color(CssColor::Hsl { .. })));
    }

    #[test]
    fn test_parse_currentcolor() {
        let v = parse_css_value("currentColor").unwrap();
        assert!(matches!(v, CssValue::Color(CssColor::CurrentColor)));
    }

    #[test]
    fn test_parse_transparent() {
        let v = parse_css_value("transparent").unwrap();
        assert!(matches!(v, CssValue::Color(CssColor::Transparent)));
    }

    // --- parse_css_value: Keywords ---

    #[test]
    fn test_parse_keyword_auto() {
        let v = parse_css_value("auto").unwrap();
        assert!(matches!(v, CssValue::Keyword(CssKeyword::Auto)));
    }

    #[test]
    fn test_parse_keyword_none() {
        let v = parse_css_value("none").unwrap();
        assert!(matches!(v, CssValue::Keyword(CssKeyword::None)));
    }

    #[test]
    fn test_parse_keyword_bold() {
        let v = parse_css_value("bold").unwrap();
        assert!(matches!(v, CssValue::Keyword(CssKeyword::Bold)));
    }

    // --- parse_css_value: Wide Keywords ---

    #[test]
    fn test_parse_inherit() {
        let v = parse_css_value("inherit").unwrap();
        assert!(matches!(v, CssValue::WideKeyword(keywords::CssWideKeyword::Inherit)));
    }

    #[test]
    fn test_parse_initial() {
        let v = parse_css_value("initial").unwrap();
        assert!(matches!(v, CssValue::WideKeyword(keywords::CssWideKeyword::Initial)));
    }

    #[test]
    fn test_parse_unset() {
        let v = parse_css_value("unset").unwrap();
        assert!(matches!(v, CssValue::WideKeyword(keywords::CssWideKeyword::Unset)));
    }

    // --- parse_css_value: calc() ---

    #[test]
    fn test_parse_calc_simple() {
        let v = parse_css_value("calc(100% - 20px)").unwrap();
        assert!(matches!(v, CssValue::Calc(_)));
    }

    #[test]
    fn test_parse_calc_addition() {
        let v = parse_css_value("calc(10px + 5px)").unwrap();
        assert!(matches!(v, CssValue::Calc(_)));
    }

    // --- parse_css_value: var() ---

    #[test]
    fn test_parse_var() {
        let v = parse_css_value("var(--my-color)").unwrap();
        assert!(matches!(v, CssValue::Var { .. }));
        if let CssValue::Var { name, fallback } = v {
            assert_eq!(name, "--my-color");
            assert!(fallback.is_none());
        }
    }

    #[test]
    fn test_parse_var_with_fallback() {
        let v = parse_css_value("var(--bg, #fff)").unwrap();
        if let CssValue::Var { name, fallback } = v {
            assert_eq!(name, "--bg");
            assert_eq!(fallback.unwrap(), "#fff");
        } else {
            panic!("Expected Var");
        }
    }

    // --- parse_css_value: url() ---

    #[test]
    fn test_parse_url_quoted() {
        let v = parse_css_value("url(\"font.woff2\")").unwrap();
        assert!(matches!(v, CssValue::Url(_)));
        if let CssValue::Url(u) = v {
            assert_eq!(u, "font.woff2");
        }
    }

    #[test]
    fn test_parse_url_unquoted() {
        let v = parse_css_value("url(font.woff2)").unwrap();
        if let CssValue::Url(u) = v {
            assert_eq!(u, "font.woff2");
        } else {
            panic!("Expected Url");
        }
    }

    // --- parse_css_value: Ident ---

    #[test]
    fn test_parse_ident() {
        let v = parse_css_value("sans-serif").unwrap();
        assert!(matches!(v, CssValue::Ident(_)));
        if let CssValue::Ident(s) = v {
            assert_eq!(s, "sans-serif");
        }
    }

    #[test]
    fn test_parse_flex_ident() {
        let v = parse_css_value("flex").unwrap();
        assert!(matches!(v, CssValue::Keyword(CssKeyword::Flex)));
    }

    // --- parse_css_value: Empty ---

    #[test]
    fn test_parse_empty() {
        assert!(parse_css_value("").is_none());
        assert!(parse_css_value("   ").is_none());
    }

    // --- Length conversion ---

    #[test]
    fn test_length_to_px() {
        let px = Length { value: 1.0, unit: LengthUnit::In }.to_px().unwrap();
        assert!((px - 96.0).abs() < 0.01);

        let pt = Length { value: 72.0, unit: LengthUnit::Pt }.to_px().unwrap();
        assert!((pt - 96.0).abs() < 0.01);

        let cm = Length { value: 2.54, unit: LengthUnit::Cm }.to_px().unwrap();
        assert!((cm - 96.0).abs() < 1.0);
    }

    #[test]
    fn test_length_is_absolute() {
        assert!(LengthUnit::Px.is_absolute());
        assert!(LengthUnit::In.is_absolute());
        assert!(LengthUnit::Pt.is_absolute());
        assert!(!LengthUnit::Em.is_absolute());
        assert!(!LengthUnit::Vw.is_absolute());
    }
}
