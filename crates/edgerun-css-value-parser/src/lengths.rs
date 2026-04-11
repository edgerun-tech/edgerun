//! CSS length, angle, time, frequency, and resolution unit parsing.
//!
//! Supports all 30 CSS length units from css_values.proto plus angle,
//! time, frequency, and resolution units.

/// CSS length unit — matches `LengthUnit` enum from css_values.proto.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LengthUnit {
    // Absolute lengths
    Px,
    Cm, Mm, Q, In, Pc, Pt,
    // Relative lengths
    Em, Rem, Ex, Rex, Cap, Rcap, Ch, Rch, Ic, Ric, Lh, Rlh,
    // Viewport units
    Vw, Svw, Lvw, Dvw,
    Vh, Svh, Lvh, Dvh,
    Vi, Svi, Lvi, Dvi,
    Vb, Svb, Lvb, Dvb,
    Vmin, Svmin, Lvmin, Dvmin,
    Vmax, Svmax, Lvmax, Dvmax,
    // Grid/fraction
    Fr,
    // Angle units
    Deg, Grad, Rad, Turn,
    // Time units
    S, Ms,
    // Frequency units
    Hz, Khz,
    // Resolution units
    Dpi, Dpcm, Dppx,
    // Special
    X,
}

impl LengthUnit {
    /// Returns true if this unit is absolute (doesn't depend on context).
    pub fn is_absolute(self) -> bool {
        matches!(self,
            LengthUnit::Px | LengthUnit::Cm | LengthUnit::Mm | LengthUnit::Q
            | LengthUnit::In | LengthUnit::Pc | LengthUnit::Pt
            | LengthUnit::Deg | LengthUnit::Grad | LengthUnit::Rad | LengthUnit::Turn
            | LengthUnit::S | LengthUnit::Ms
            | LengthUnit::Hz | LengthUnit::Khz
            | LengthUnit::Dpi | LengthUnit::Dpcm | LengthUnit::Dppx
            | LengthUnit::X
        )
    }

    /// Returns the CSS suffix string for this unit (e.g. "px", "em", "deg").
    pub fn suffix(self) -> &'static str {
        match self {
            LengthUnit::Px => "px", LengthUnit::Cm => "cm", LengthUnit::Mm => "mm",
            LengthUnit::Q => "q", LengthUnit::In => "in", LengthUnit::Pc => "pc",
            LengthUnit::Pt => "pt",
            LengthUnit::Em => "em", LengthUnit::Rem => "rem", LengthUnit::Ex => "ex",
            LengthUnit::Rex => "rex", LengthUnit::Cap => "cap", LengthUnit::Rcap => "rcap",
            LengthUnit::Ch => "ch", LengthUnit::Rch => "rch", LengthUnit::Ic => "ic",
            LengthUnit::Ric => "ric", LengthUnit::Lh => "lh", LengthUnit::Rlh => "rlh",
            LengthUnit::Vw => "vw", LengthUnit::Svw => "svw", LengthUnit::Lvw => "lvw",
            LengthUnit::Dvw => "dvw", LengthUnit::Vh => "vh", LengthUnit::Svh => "svh",
            LengthUnit::Lvh => "lvh", LengthUnit::Dvh => "dvh",
            LengthUnit::Vi => "vi", LengthUnit::Svi => "svi", LengthUnit::Lvi => "lvi",
            LengthUnit::Dvi => "dvi", LengthUnit::Vb => "vb", LengthUnit::Svb => "svb",
            LengthUnit::Lvb => "lvb", LengthUnit::Dvb => "dvb",
            LengthUnit::Vmin => "vmin", LengthUnit::Svmin => "svmin",
            LengthUnit::Lvmin => "lvmin", LengthUnit::Dvmin => "dvmin",
            LengthUnit::Vmax => "vmax", LengthUnit::Svmax => "svmax",
            LengthUnit::Lvmax => "lvmax", LengthUnit::Dvmax => "dvmax",
            LengthUnit::Fr => "fr",
            LengthUnit::Deg => "deg", LengthUnit::Grad => "grad",
            LengthUnit::Rad => "rad", LengthUnit::Turn => "turn",
            LengthUnit::S => "s", LengthUnit::Ms => "ms",
            LengthUnit::Hz => "hz", LengthUnit::Khz => "khz",
            LengthUnit::Dpi => "dpi", LengthUnit::Dpcm => "dpcm",
            LengthUnit::Dppx => "dppx", LengthUnit::X => "x",
        }
    }
}

/// A CSS length value (number + unit).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Length {
    pub value: f64,
    pub unit: LengthUnit,
}

impl Length {
    /// Convert this length to pixels. Returns None for relative units.
    ///
    /// Conversion factors (CSS spec):
    /// - 1in = 96px
    /// - 1cm = 96/2.54 px ≈ 37.8px
    /// - 1mm = 96/25.4 px ≈ 3.78px
    /// - 1Q = 96/101.6 px ≈ 0.945px
    /// - 1pc = 16px (1pc = 1/6 in = 96/6 px)
    /// - 1pt = 96/72 px ≈ 1.333px
    /// - 1px = 1px
    pub fn to_px(self) -> Option<f64> {
        match self.unit {
            LengthUnit::Px => Some(self.value),
            LengthUnit::In => Some(self.value * 96.0),
            LengthUnit::Cm => Some(self.value * 96.0 / 2.54),
            LengthUnit::Mm => Some(self.value * 96.0 / 25.4),
            LengthUnit::Q => Some(self.value * 96.0 / 101.6),
            LengthUnit::Pc => Some(self.value * 16.0),
            LengthUnit::Pt => Some(self.value * 96.0 / 72.0),
            // Relative units need context — return None
            _ => None,
        }
    }
}

/// Parse a CSS length string (e.g. `"32px"`, `"1.5em"`, `"100vh"`) into a `Length`.
pub fn parse_length(input: &str) -> Option<Length> {
    let input = input.trim();
    let (value_str, unit) = split_dimension(input)?;
    let value = value_str.trim().parse::<f64>().ok()?;
    Some(Length { value, unit })
}

/// Parse a CSS angle: `"90deg"`, `"1.5turn"`
pub fn parse_angle(input: &str) -> Option<Length> {
    let input = input.trim();
    let (value_str, unit) = split_dimension(input)?;
    if !matches!(unit, LengthUnit::Deg | LengthUnit::Grad | LengthUnit::Rad | LengthUnit::Turn) {
        return None;
    }
    let value = value_str.trim().parse::<f64>().ok()?;
    Some(Length { value, unit })
}

/// Parse a CSS time: `"2s"`, `"500ms"`
pub fn parse_time(input: &str) -> Option<Length> {
    let input = input.trim();
    let (value_str, unit) = split_dimension(input)?;
    if !matches!(unit, LengthUnit::S | LengthUnit::Ms) {
        return None;
    }
    let value = value_str.trim().parse::<f64>().ok()?;
    Some(Length { value, unit })
}

/// Parse a CSS frequency: `"60hz"`, `"1khz"`
pub fn parse_frequency(input: &str) -> Option<Length> {
    let input = input.trim();
    let (value_str, unit) = split_dimension(input)?;
    if !matches!(unit, LengthUnit::Hz | LengthUnit::Khz) {
        return None;
    }
    let value = value_str.trim().parse::<f64>().ok()?;
    Some(Length { value, unit })
}

/// Parse a CSS resolution: `"96dpi"`, `"2dppx"`
pub fn parse_resolution(input: &str) -> Option<Length> {
    let input = input.trim();
    let (value_str, unit) = split_dimension(input)?;
    if !matches!(unit, LengthUnit::Dpi | LengthUnit::Dpcm | LengthUnit::Dppx) {
        return None;
    }
    let value = value_str.trim().parse::<f64>().ok()?;
    Some(Length { value, unit })
}

/// Match a CSS unit at the start of the string, returning (unit, suffix_len).
/// This matches ONLY the unit — no number prefix. Used by calc parser.
pub(crate) fn match_unit(input: &str) -> Option<(LengthUnit, usize)> {
    // Try longest unit names first to avoid ambiguity (e.g. "dvh" before "dv", "vh" before "v")
    const UNITS: &[(LengthUnit, &str)] = &[
        (LengthUnit::Dvmin, "dvmin"), (LengthUnit::Dvmax, "dvmax"),
        (LengthUnit::Lvmin, "lvmin"), (LengthUnit::Lvmax, "lvmax"),
        (LengthUnit::Svmin, "svmin"), (LengthUnit::Svmax, "svmax"),
        (LengthUnit::Rcap, "rcap"), (LengthUnit::Rch, "rch"), (LengthUnit::Ric, "ric"),
        (LengthUnit::Rlh, "rlh"), (LengthUnit::Rex, "rex"),
        (LengthUnit::Dpcm, "dpcm"), (LengthUnit::Dppx, "dppx"),
        (LengthUnit::Svw, "svw"), (LengthUnit::Svh, "svh"), (LengthUnit::Svi, "svi"),
        (LengthUnit::Svb, "svb"), (LengthUnit::Lvw, "lvw"), (LengthUnit::Lvh, "lvh"),
        (LengthUnit::Lvi, "lvi"), (LengthUnit::Lvb, "lvb"),
        (LengthUnit::Dvw, "dvw"), (LengthUnit::Dvh, "dvh"), (LengthUnit::Dvi, "dvi"),
        (LengthUnit::Dvb, "dvb"),
        (LengthUnit::Vmin, "vmin"), (LengthUnit::Vmax, "vmax"),
        (LengthUnit::Grad, "grad"), (LengthUnit::Turn, "turn"),
        (LengthUnit::Khz, "khz"), (LengthUnit::Dpi, "dpi"),
        (LengthUnit::Cap, "cap"), (LengthUnit::Rem, "rem"),
        (LengthUnit::Vw, "vw"), (LengthUnit::Vh, "vh"),
        (LengthUnit::Vi, "vi"), (LengthUnit::Vb, "vb"),
        (LengthUnit::Em, "em"), (LengthUnit::Ex, "ex"),
        (LengthUnit::Ch, "ch"), (LengthUnit::Ic, "ic"),
        (LengthUnit::Lh, "lh"),
        (LengthUnit::Px, "px"), (LengthUnit::Cm, "cm"),
        (LengthUnit::Mm, "mm"), (LengthUnit::Pt, "pt"),
        (LengthUnit::Pc, "pc"), (LengthUnit::Q, "q"),
        (LengthUnit::In, "in"), (LengthUnit::Fr, "fr"),
        (LengthUnit::Deg, "deg"), (LengthUnit::Rad, "rad"),
        (LengthUnit::S, "s"), (LengthUnit::Ms, "ms"),
        (LengthUnit::Hz, "hz"), (LengthUnit::X, "x"),
    ];

    for &(unit, suffix) in UNITS {
        if input.starts_with(suffix) {
            return Some((unit, suffix.len()));
        }
    }
    None
}

/// Split a dimension string into (numeric_part, unit).
/// E.g. "32px" → ("32", LengthUnit::Px)
pub(crate) fn split_dimension(input: &str) -> Option<(&str, LengthUnit)> {
    // Find the split point between number and unit
    let bytes = input.as_bytes();

    // Skip sign
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        i += 1;
    }

    // Skip digits and decimal point
    let mut has_dot = false;
    let mut has_digit = false;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            has_digit = true;
            i += 1;
        } else if bytes[i] == b'.' && !has_dot {
            has_dot = true;
            i += 1;
        } else {
            break;
        }
    }

    if !has_digit {
        return None;
    }

    let unit_str = &input[i..];

    let unit = match unit_str {
        // Length units — try longest matches first to avoid ambiguity
        "px" => LengthUnit::Px, "em" => LengthUnit::Em, "rem" => LengthUnit::Rem,
        "ex" => LengthUnit::Ex, "rex" => LengthUnit::Rex, "cap" => LengthUnit::Cap,
        "rcap" => LengthUnit::Rcap, "ch" => LengthUnit::Ch, "rch" => LengthUnit::Rch,
        "ic" => LengthUnit::Ic, "ric" => LengthUnit::Ric, "lh" => LengthUnit::Lh,
        "rlh" => LengthUnit::Rlh,
        "vw" => LengthUnit::Vw, "svw" => LengthUnit::Svw, "lvw" => LengthUnit::Lvw,
        "dvw" => LengthUnit::Dvw,
        "vh" => LengthUnit::Vh, "svh" => LengthUnit::Svh, "lvh" => LengthUnit::Lvh,
        "dvh" => LengthUnit::Dvh,
        "vi" => LengthUnit::Vi, "svi" => LengthUnit::Svi, "lvi" => LengthUnit::Lvi,
        "dvi" => LengthUnit::Dvi,
        "vb" => LengthUnit::Vb, "svb" => LengthUnit::Svb, "lvb" => LengthUnit::Lvb,
        "dvb" => LengthUnit::Dvb,
        "vmin" => LengthUnit::Vmin, "svmin" => LengthUnit::Svmin,
        "lvmin" => LengthUnit::Lvmin, "dvmin" => LengthUnit::Dvmin,
        "vmax" => LengthUnit::Vmax, "svmax" => LengthUnit::Svmax,
        "lvmax" => LengthUnit::Lvmax, "dvmax" => LengthUnit::Dvmax,
        "cm" => LengthUnit::Cm, "mm" => LengthUnit::Mm, "q" => LengthUnit::Q,
        "in" => LengthUnit::In, "pc" => LengthUnit::Pc, "pt" => LengthUnit::Pt,
        "fr" => LengthUnit::Fr,
        // Angle units
        "deg" => LengthUnit::Deg, "grad" => LengthUnit::Grad,
        "rad" => LengthUnit::Rad, "turn" => LengthUnit::Turn,
        // Time units
        "s" => LengthUnit::S, "ms" => LengthUnit::Ms,
        // Frequency units
        "hz" => LengthUnit::Hz, "khz" => LengthUnit::Khz,
        // Resolution units
        "dpi" => LengthUnit::Dpi, "dpcm" => LengthUnit::Dpcm, "dppx" => LengthUnit::Dppx,
        // Special
        "x" => LengthUnit::X,
        _ => return None,
    };

    Some((&input[..i], unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_dimension_simple() {
        let result = split_dimension("10px");
        assert!(result.is_some());
        let (num, unit) = result.unwrap();
        assert_eq!(num, "10");
        assert_eq!(unit, LengthUnit::Px);
    }

    #[test]
    fn test_match_unit_px() {
        let result = match_unit("px");
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, LengthUnit::Px);
    }

    #[test]
    fn test_match_unit_with_trailing() {
        let result = match_unit("px + 5px");
        assert!(result.is_some());
        let (unit, len) = result.unwrap();
        assert_eq!(unit, LengthUnit::Px);
        assert_eq!(len, 2);
    }

    #[test]
    fn test_parse_simple_px() {
        let l = parse_length("32px").unwrap();
        assert_eq!(l.value, 32.0);
        assert_eq!(l.unit, LengthUnit::Px);
    }

    #[test]
    fn test_parse_float_em() {
        let l = parse_length("1.5em").unwrap();
        assert_eq!(l.value, 1.5);
        assert_eq!(l.unit, LengthUnit::Em);
    }

    #[test]
    fn test_parse_negative() {
        let l = parse_length("-10px").unwrap();
        assert_eq!(l.value, -10.0);
    }

    #[test]
    fn test_parse_zero() {
        let l = parse_length("0px").unwrap();
        assert_eq!(l.value, 0.0);
    }

    #[test]
    fn test_parse_vh() {
        let l = parse_length("100vh").unwrap();
        assert_eq!(l.unit, LengthUnit::Vh);
    }

    #[test]
    fn test_parse_dvh() {
        let l = parse_length("50dvh").unwrap();
        assert_eq!(l.unit, LengthUnit::Dvh);
    }

    #[test]
    fn test_parse_rem() {
        let l = parse_length("2rem").unwrap();
        assert_eq!(l.unit, LengthUnit::Rem);
    }

    #[test]
    fn test_parse_angle_deg() {
        let a = parse_angle("90deg").unwrap();
        assert_eq!(a.unit, LengthUnit::Deg);
        assert_eq!(a.value, 90.0);
    }

    #[test]
    fn test_parse_time_s() {
        let t = parse_time("2s").unwrap();
        assert_eq!(t.unit, LengthUnit::S);
        assert_eq!(t.value, 2.0);
    }

    #[test]
    fn test_parse_time_ms() {
        let t = parse_time("500ms").unwrap();
        assert_eq!(t.unit, LengthUnit::Ms);
        assert_eq!(t.value, 500.0);
    }

    #[test]
    fn test_parse_frequency() {
        let f = parse_frequency("60hz").unwrap();
        assert_eq!(f.unit, LengthUnit::Hz);
    }

    #[test]
    fn test_parse_resolution() {
        let r = parse_resolution("96dpi").unwrap();
        assert_eq!(r.unit, LengthUnit::Dpi);
    }

    #[test]
    fn test_to_px_absolute() {
        assert!((Length { value: 1.0, unit: LengthUnit::In }.to_px().unwrap() - 96.0).abs() < 0.01);
        assert!((Length { value: 2.54, unit: LengthUnit::Cm }.to_px().unwrap() - 96.0).abs() < 0.1);
        assert!((Length { value: 72.0, unit: LengthUnit::Pt }.to_px().unwrap() - 96.0).abs() < 0.01);
        assert!((Length { value: 6.0, unit: LengthUnit::Pc }.to_px().unwrap() - 96.0).abs() < 0.01);
    }

    #[test]
    fn test_to_px_relative_returns_none() {
        assert!(Length { value: 1.0, unit: LengthUnit::Em }.to_px().is_none());
        assert!(Length { value: 1.0, unit: LengthUnit::Vw }.to_px().is_none());
    }

    #[test]
    fn test_is_absolute() {
        assert!(LengthUnit::Px.is_absolute());
        assert!(LengthUnit::In.is_absolute());
        assert!(LengthUnit::Deg.is_absolute());
        assert!(!LengthUnit::Em.is_absolute());
        assert!(!LengthUnit::Vh.is_absolute());
    }

    #[test]
    fn test_suffix() {
        assert_eq!(LengthUnit::Px.suffix(), "px");
        assert_eq!(LengthUnit::Em.suffix(), "em");
        assert_eq!(LengthUnit::Dvh.suffix(), "dvh");
        assert_eq!(LengthUnit::Deg.suffix(), "deg");
    }

    #[test]
    fn test_invalid_no_unit() {
        assert!(parse_length("32").is_none());
    }

    #[test]
    fn test_invalid_no_number() {
        assert!(parse_length("px").is_none());
    }
}
