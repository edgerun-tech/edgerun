//! CSS color parsing — named colors, hex, rgb(), rgba(), hsl(), hsla(),
//! hwb(), currentColor, transparent, and modern color notation.

extern crate alloc;
use alloc::format;

use edgerun_color::{CssColor, NamedColor};

/// A parsed hex color with alpha info.
#[derive(Debug, Clone, Copy)]
pub struct HexColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub alpha: f64,
}

/// Parse a CSS color string into a `CssColor`.
///
/// Supports:
/// - Named colors: `red`, `blue`, `rebeccapurple`
/// - Hex colors: `#f00`, `#ff0000`, `#f00a`, `#ff0000aa`
/// - rgb(): `rgb(255, 0, 0)`, `rgb(100%, 0%, 0%)`
/// - rgba(): `rgba(255, 0, 0, 0.5)`
/// - hsl(): `hsl(0, 100%, 50%)`
/// - hsla(): `hsla(0, 100%, 50%, 0.5)`
/// - hwb(): `hwb(0 0% 0%)`
/// - Special: `currentColor`, `transparent`
/// - Color functions: `oklch()`, `oklab()`, `lab()`, `lch()`, `color()`
///
/// Also handles the case where a named color was parsed as a keyword
/// (e.g. "red" → `CssKeyword::Red`), by converting known keyword colors
/// to their `CssColor::Named` equivalents.
pub fn parse_color(input: &str) -> Option<CssColor> {
    let input = input.trim();

    // Special keywords
    if input.eq_ignore_ascii_case("transparent") {
        return Some(CssColor::Transparent);
    }
    if input.eq_ignore_ascii_case("currentcolor") {
        return Some(CssColor::CurrentColor);
    }

    // Hex colors: #RGB, #RGBA, #RRGGBB, #RRGGBBAA
    if let Some(hex) = parse_hex_color(input) {
        return Some(CssColor::Rgb { r: hex.r, g: hex.g, b: hex.b, alpha: hex.alpha });
    }

    // rgb() / rgba()
    if let Some(color) = parse_rgb(input) {
        return Some(color);
    }

    // hsl() / hsla()
    if let Some(color) = parse_hsl(input) {
        return Some(color);
    }

    // hwb()
    if let Some(color) = parse_hwb(input) {
        return Some(color);
    }

    // lab()
    if let Some(color) = parse_lab(input) {
        return Some(color);
    }

    // lch()
    if let Some(color) = parse_lch(input) {
        return Some(color);
    }

    // oklch()
    if let Some(color) = parse_oklch(input) {
        return Some(color);
    }

    // oklab()
    if let Some(color) = parse_oklab(input) {
        return Some(color);
    }

    // Named colors
    if let Some(name) = NamedColor::from_name(&input.to_lowercase()) {
        return Some(CssColor::Named(name));
    }

    None
}

/// Parse a hex color: `#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`
pub fn parse_hex_color(input: &str) -> Option<HexColor> {
    if !input.starts_with('#') {
        return None;
    }
    let hex = &input[1..];
    let len = hex.len();

    let (r, g, b, a) = match len {
        3 => {
            // #RGB → expand to #RRGGBB
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()? as f64 / 255.0;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()? as f64 / 255.0;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()? as f64 / 255.0;
            (r, g, b, 1.0)
        }
        4 => {
            // #RGBA → expand to #RRGGBBAA
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()? as f64 / 255.0;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()? as f64 / 255.0;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()? as f64 / 255.0;
            let a = u8::from_str_radix(&hex[3..4].repeat(2), 16).ok()? as f64 / 255.0;
            (r, g, b, a)
        }
        6 => {
            // #RRGGBB
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
            (r, g, b, 1.0)
        }
        8 => {
            // #RRGGBBAA
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()? as f64 / 255.0;
            (r, g, b, a)
        }
        _ => return None,
    };

    Some(HexColor { r, g, b, alpha: a })
}

/// Parse rgb() or rgba(): `rgb(255, 0, 0)`, `rgb(100%, 0%, 0%)`, `rgba(255, 0, 0, 0.5)`
fn parse_rgb(input: &str) -> Option<CssColor> {
    let lower = input.to_lowercase();
    let is_rgba = lower.starts_with("rgba(");
    if !lower.starts_with("rgb(") && !is_rgba {
        return None;
    }
    if !input.ends_with(')') {
        return None;
    }
    let inner = if is_rgba {
        &input[5..input.len() - 1]
    } else {
        &input[4..input.len() - 1]
    };
    let inner = inner.trim();

    // Try modern syntax: space-separated with / alpha
    // rgb(255 0 0 / 0.5)
    if let Some(slash_pos) = inner.find('/') {
        let color_part = inner[..slash_pos].trim();
        let alpha_part = inner[slash_pos + 1..].trim();
        let alpha = alpha_part.parse::<f64>().ok()?;
        return parse_rgb_color_values(color_part, Some(alpha));
    }

    // Legacy syntax: comma-separated rgb(r, g, b) or rgba(r, g, b, a)
    let parts: alloc::vec::Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
    match parts.len() {
        3 => parse_rgb_color_values(inner, None),
        4 => {
            let color_parts = format!("{}, {}, {}", parts[0], parts[1], parts[2]);
            let alpha = parts[3].parse::<f64>().ok()?;
            parse_rgb_color_values(&color_parts, Some(alpha))
        }
        _ => None,
    }
}

fn parse_rgb_color_values(input: &str, alpha: Option<f64>) -> Option<CssColor> {
    let parts: alloc::vec::Vec<&str> = input.split(',').map(|s| s.trim()).collect();
    if parts.len() != 3 {
        // Try space-separated
        let parts: alloc::vec::Vec<&str> = input.split_whitespace().collect();
        if parts.len() != 3 {
            return None;
        }
        let r = parse_rgb_component(parts[0])?;
        let g = parse_rgb_component(parts[1])?;
        let b = parse_rgb_component(parts[2])?;
        return Some(CssColor::Rgb { r, g, b, alpha: alpha.unwrap_or(1.0) });
    }

    let r = parse_rgb_component(parts[0])?;
    let g = parse_rgb_component(parts[1])?;
    let b = parse_rgb_component(parts[2])?;
    Some(CssColor::Rgb { r, g, b, alpha: alpha.unwrap_or(1.0) })
}

fn parse_rgb_component(s: &str) -> Option<f64> {
    if s.ends_with('%') {
        let v = s[..s.len() - 1].trim().parse::<f64>().ok()?;
        Some((v / 100.0).clamp(0.0, 1.0))
    } else {
        // Integer 0-255
        let v = s.parse::<f64>().ok()?;
        Some((v / 255.0).clamp(0.0, 1.0))
    }
}

/// Parse hsl() or hsla(): `hsl(0, 100%, 50%)`, `hsla(0, 100%, 50%, 0.5)`
fn parse_hsl(input: &str) -> Option<CssColor> {
    let lower = input.to_lowercase();
    let is_hsla = lower.starts_with("hsla(");
    if !lower.starts_with("hsl(") && !is_hsla {
        return None;
    }
    if !input.ends_with(')') {
        return None;
    }
    let inner = if is_hsla {
        &input[5..input.len() - 1]
    } else {
        &input[4..input.len() - 1]
    };
    let inner = inner.trim();

    // Modern syntax: hsl(0 100% 50% / 0.5)
    if let Some(slash_pos) = inner.find('/') {
        let color_part = inner[..slash_pos].trim();
        let alpha_part = inner[slash_pos + 1..].trim();
        let alpha = alpha_part.parse::<f64>().ok()?;
        return parse_hsl_components(color_part, Some(alpha));
    }

    // Legacy: hsl(h, s%, l%) or hsla(h, s%, l%, a)
    let parts: alloc::vec::Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
    match parts.len() {
        3 => parse_hsl_components(inner, None),
        4 => {
            let color_part = format!("{}, {}, {}", parts[0], parts[1], parts[2]);
            let alpha = parts[3].parse::<f64>().ok()?;
            parse_hsl_components(&color_part, Some(alpha))
        }
        _ => None,
    }
}

fn parse_hsl_components(input: &str, alpha: Option<f64>) -> Option<CssColor> {
    let parts: alloc::vec::Vec<&str> = input.split([',', ' ']).map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.len() != 3 {
        return None;
    }

    let h = parts[0].trim_end_matches("deg").parse::<f64>().ok()?;
    let s_str = parts[1];
    let l_str = parts[2];

    if !s_str.ends_with('%') || !l_str.ends_with('%') {
        return None;
    }
    let s = s_str[..s_str.len() - 1].trim().parse::<f64>().ok()? / 100.0;
    let l = l_str[..l_str.len() - 1].trim().parse::<f64>().ok()? / 100.0;

    Some(CssColor::Hsl { h, s, l, alpha: alpha.unwrap_or(1.0) })
}

/// Parse hwb(): `hwb(0 0% 0%)`
fn parse_hwb(input: &str) -> Option<CssColor> {
    let lower = input.to_lowercase();
    if !lower.starts_with("hwb(") || !input.ends_with(')') {
        return None;
    }
    let inner = &input[4..input.len() - 1].trim();

    let parts: alloc::vec::Vec<&str> = inner.split([',', ' ']).map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.len() < 3 {
        return None;
    }

    let h = parts[0].trim_end_matches("deg").parse::<f64>().ok()?;

    // Whitness and blackness should end with %
    let w = if parts[1].ends_with('%') {
        parts[1][..parts[1].len() - 1].trim().parse::<f64>().ok()? / 100.0
    } else {
        parts[1].parse::<f64>().ok()?
    };
    let b = if parts[2].ends_with('%') {
        parts[2][..parts[2].len() - 1].trim().parse::<f64>().ok()? / 100.0
    } else {
        parts[2].parse::<f64>().ok()?
    };

    let alpha = parts.get(3).and_then(|s| {
        let s = s.trim();
        if s.ends_with('%') {
            Some(s[..s.len() - 1].trim().parse::<f64>().ok()? / 100.0)
        } else {
            s.parse::<f64>().ok()
        }
    });

    Some(CssColor::Hwb { h, w, b, alpha: alpha.unwrap_or(1.0) })
}

/// Parse lab(): `lab(50% 40 -50)`
fn parse_lab(input: &str) -> Option<CssColor> {
    let lower = input.to_lowercase();
    if !lower.starts_with("lab(") || !input.ends_with(')') {
        return None;
    }
    let inner = &input[4..input.len() - 1].trim();

    let parts: alloc::vec::Vec<&str> = inner.split([',', ' ']).map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.len() < 3 {
        return None;
    }

    let l = if parts[0].ends_with('%') {
        parts[0][..parts[0].len() - 1].trim().parse::<f64>().ok()?
    } else {
        parts[0].parse::<f64>().ok()?
    };
    let a = parts[1].parse::<f64>().ok()?;
    let b = parts[2].parse::<f64>().ok()?;
    let alpha = parts.get(3).and_then(|s| s.parse::<f64>().ok());

    Some(CssColor::Lab { l, a, b, alpha: alpha.unwrap_or(1.0) })
}

/// Parse lch(): `lch(50% 50 30)`
fn parse_lch(input: &str) -> Option<CssColor> {
    let lower = input.to_lowercase();
    if !lower.starts_with("lch(") || !input.ends_with(')') {
        return None;
    }
    let inner = &input[4..input.len() - 1].trim();

    let parts: alloc::vec::Vec<&str> = inner.split([',', ' ']).map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.len() < 3 {
        return None;
    }

    let l = if parts[0].ends_with('%') {
        parts[0][..parts[0].len() - 1].trim().parse::<f64>().ok()?
    } else {
        parts[0].parse::<f64>().ok()?
    };
    let c = parts[1].parse::<f64>().ok()?;
    let h = parts[2].parse::<f64>().ok()?;
    let alpha = parts.get(3).and_then(|s| s.parse::<f64>().ok());

    Some(CssColor::Lch { l, c, h, alpha: alpha.unwrap_or(1.0) })
}

/// Parse oklch(): `oklch(0.5 0.2 30)`
fn parse_oklch(input: &str) -> Option<CssColor> {
    let lower = input.to_lowercase();
    if !lower.starts_with("oklch(") || !input.ends_with(')') {
        return None;
    }
    let inner = &input[7..input.len() - 1].trim();

    let parts: alloc::vec::Vec<&str> = inner.split([',', ' ']).map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.len() < 3 {
        return None;
    }

    let l = parts[0].parse::<f64>().ok()?;
    let c = parts[1].parse::<f64>().ok()?;
    let h = parts[2].parse::<f64>().ok()?;
    let alpha = parts.get(3).and_then(|s| s.parse::<f64>().ok());

    Some(CssColor::Oklch { l, c, h, alpha: alpha.unwrap_or(1.0) })
}

/// Parse oklab(): `oklab(0.5 0.1 -0.05)`
fn parse_oklab(input: &str) -> Option<CssColor> {
    let lower = input.to_lowercase();
    if !lower.starts_with("oklab(") || !input.ends_with(')') {
        return None;
    }
    let inner = &input[7..input.len() - 1].trim();

    let parts: alloc::vec::Vec<&str> = inner.split([',', ' ']).map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.len() < 3 {
        return None;
    }

    let l = parts[0].parse::<f64>().ok()?;
    let a = parts[1].parse::<f64>().ok()?;
    let b = parts[2].parse::<f64>().ok()?;
    let alpha = parts.get(3).and_then(|s| s.parse::<f64>().ok());

    Some(CssColor::Oklab { l, a, b, alpha: alpha.unwrap_or(1.0) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_color() {
        let c = parse_color("red").unwrap();
        assert!(matches!(c, CssColor::Named(NamedColor::Red)));
    }

    #[test]
    fn test_named_color_case_insensitive() {
        let c = parse_color("Red").unwrap();
        assert!(matches!(c, CssColor::Named(NamedColor::Red)));
    }

    #[test]
    fn test_hex_short_3() {
        let c = parse_color("#f00").unwrap();
        if let CssColor::Rgb { r, g, b, alpha } = c {
            assert!((r - 1.0).abs() < 0.01);
            assert!(g < 0.01);
            assert!(b < 0.01);
            assert!((alpha - 1.0).abs() < 0.01);
        } else {
            panic!("Expected Rgb");
        }
    }

    #[test]
    fn test_hex_long_6() {
        let c = parse_color("#ff0000").unwrap();
        if let CssColor::Rgb { r, g, b, alpha } = c {
            assert!((r - 1.0).abs() < 0.01);
            assert!(g < 0.01);
            assert!(b < 0.01);
        } else {
            panic!("Expected Rgb");
        }
    }

    #[test]
    fn test_hex_8_with_alpha() {
        let c = parse_color("#ff000080").unwrap();
        if let CssColor::Rgb { alpha, .. } = c {
            assert!((alpha - 0.502).abs() < 0.01);
        } else {
            panic!("Expected Rgb");
        }
    }

    #[test]
    fn test_rgb() {
        let c = parse_color("rgb(255, 0, 0)").unwrap();
        if let CssColor::Rgb { r, g, b, alpha } = c {
            assert!((r - 1.0).abs() < 0.01);
            assert!(g < 0.01);
            assert!(b < 0.01);
            assert!((alpha - 1.0).abs() < 0.01);
        } else {
            panic!("Expected Rgb");
        }
    }

    #[test]
    fn test_rgb_percentage() {
        let c = parse_color("rgb(100%, 50%, 0%)").unwrap();
        if let CssColor::Rgb { r, g, b, .. } = c {
            assert!((r - 1.0).abs() < 0.01);
            assert!((g - 0.5).abs() < 0.01);
            assert!(b < 0.01);
        } else {
            panic!("Expected Rgb");
        }
    }

    #[test]
    fn test_rgba() {
        let c = parse_color("rgba(255, 0, 0, 0.5)").unwrap();
        if let CssColor::Rgb { alpha, .. } = c {
            assert!((alpha - 0.5).abs() < 0.001);
        } else {
            panic!("Expected Rgb");
        }
    }

    #[test]
    fn test_rgb_modern_syntax() {
        let c = parse_color("rgb(255 0 0 / 0.5)").unwrap();
        if let CssColor::Rgb { alpha, .. } = c {
            assert!((alpha - 0.5).abs() < 0.001);
        } else {
            panic!("Expected Rgb");
        }
    }

    #[test]
    fn test_hsl() {
        let c = parse_color("hsl(0, 100%, 50%)").unwrap();
        if let CssColor::Hsl { h, s, l, alpha } = c {
            assert!((h - 0.0).abs() < 0.01);
            assert!((s - 1.0).abs() < 0.01);
            assert!((l - 0.5).abs() < 0.01);
            assert!((alpha - 1.0).abs() < 0.01);
        } else {
            panic!("Expected Hsl");
        }
    }

    #[test]
    fn test_hsla() {
        let c = parse_color("hsla(120, 100%, 25%, 0.8)").unwrap();
        if let CssColor::Hsl { h, s, l, alpha } = c {
            assert!((h - 120.0).abs() < 0.01);
            assert!((s - 1.0).abs() < 0.01);
            assert!((l - 0.25).abs() < 0.01);
            assert!((alpha - 0.8).abs() < 0.01);
        } else {
            panic!("Expected Hsl");
        }
    }

    #[test]
    fn test_hsl_modern_syntax() {
        let c = parse_color("hsl(0 100% 50% / 0.5)").unwrap();
        if let CssColor::Hsl { alpha, .. } = c {
            assert!((alpha - 0.5).abs() < 0.001);
        } else {
            panic!("Expected Hsl");
        }
    }

    #[test]
    fn test_hwb() {
        let c = parse_color("hwb(0 0% 0%)").unwrap();
        assert!(matches!(c, CssColor::Hwb { .. }));
    }

    #[test]
    fn test_currentcolor() {
        let c = parse_color("currentColor").unwrap();
        assert!(matches!(c, CssColor::CurrentColor));
    }

    #[test]
    fn test_transparent() {
        let c = parse_color("transparent").unwrap();
        assert!(matches!(c, CssColor::Transparent));
    }

    #[test]
    fn test_invalid_color() {
        assert!(parse_color("notacolor").is_none());
        assert!(parse_color("#GGG").is_none());
        assert!(parse_color("rgb(256, 0, 0)").is_some()); // clamped to 1.0
    }
}
