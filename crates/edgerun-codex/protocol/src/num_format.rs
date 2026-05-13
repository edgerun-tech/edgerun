fn locale_separator() -> char {
    let Some(locale) = edgerun_locale::get_locale() else {
        return ',';
    };
    let locale = locale.to_ascii_lowercase();
    let language = locale
        .split(['-', '_', '.'])
        .next()
        .unwrap_or(locale.as_str());

    if locale.starts_with("de-ch") || locale.starts_with("it-ch") || locale.starts_with("fr-ch") {
        '\''
    } else if matches!(language, "fr" | "ru" | "uk" | "cs" | "sk" | "pl") {
        ' '
    } else if matches!(
        language,
        "de" | "es" | "it" | "nl" | "pt" | "da" | "no" | "sv" | "fi"
    ) {
        '.'
    } else {
        ','
    }
}

fn format_with_separator(n: i64, separator: char) -> String {
    let negative = n < 0;
    let mut value = n.unsigned_abs();
    let mut groups = Vec::new();

    loop {
        groups.push(value % 1000);
        value /= 1000;
        if value == 0 {
            break;
        }
    }

    let mut out = String::new();
    if negative {
        out.push('-');
    }

    let mut iter = groups.iter().rev();
    if let Some(first) = iter.next() {
        out.push_str(&first.to_string());
    }
    for group in iter {
        out.push(separator);
        out.push_str(&format!("{group:03}"));
    }
    out
}

/// Format an i64 with locale-aware digit separators.
pub fn format_with_separators(n: i64) -> String {
    format_with_separator(n, locale_separator())
}

fn format_scaled(n: i64, scale: i64, frac_digits: u32, separator: char) -> String {
    let factor = 10_i64.pow(frac_digits);
    let rounded = ((n as f64 / scale as f64) * factor as f64).round() as i64;
    let whole = rounded / factor;
    let frac = rounded.abs() % factor;

    if frac_digits == 0 {
        return format_with_separator(whole, separator);
    }

    format!(
        "{}.{:0width$}",
        format_with_separator(whole, separator),
        frac,
        width = frac_digits as usize
    )
}

fn format_si_suffix_with_separator(n: i64, separator: char) -> String {
    let n = n.max(0);
    if n < 1000 {
        return format_with_separator(n, separator);
    }

    const UNITS: [(i64, &str); 3] = [(1_000, "K"), (1_000_000, "M"), (1_000_000_000, "G")];
    let f = n as f64;
    for &(scale, suffix) in &UNITS {
        if (100.0 * f / scale as f64).round() < 1000.0 {
            return format!("{}{}", format_scaled(n, scale, 2, separator), suffix);
        } else if (10.0 * f / scale as f64).round() < 1000.0 {
            return format!("{}{}", format_scaled(n, scale, 1, separator), suffix);
        } else if (f / scale as f64).round() < 1000.0 {
            return format!("{}{}", format_scaled(n, scale, 0, separator), suffix);
        }
    }

    format!(
        "{}G",
        format_with_separator(((n as f64) / 1e9).round() as i64, separator)
    )
}

/// Format token counts to 3 significant figures, using base-10 SI suffixes.
pub fn format_si_suffix(n: i64) -> String {
    format_si_suffix_with_separator(n, locale_separator())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separators() {
        assert_eq!(format_with_separator(0, ','), "0");
        assert_eq!(format_with_separator(999, ','), "999");
        assert_eq!(format_with_separator(1_234_567, ','), "1,234,567");
        assert_eq!(format_with_separator(-1_234_567, '.'), "-1.234.567");
    }

    #[test]
    fn kmg() {
        let fmt = |n: i64| format_si_suffix_with_separator(n, ',');
        assert_eq!(fmt(0), "0");
        assert_eq!(fmt(999), "999");
        assert_eq!(fmt(1_000), "1.00K");
        assert_eq!(fmt(1_200), "1.20K");
        assert_eq!(fmt(10_000), "10.0K");
        assert_eq!(fmt(100_000), "100K");
        assert_eq!(fmt(999_500), "1.00M");
        assert_eq!(fmt(1_000_000), "1.00M");
        assert_eq!(fmt(1_234_000), "1.23M");
        assert_eq!(fmt(12_345_678), "12.3M");
        assert_eq!(fmt(999_950_000), "1.00G");
        assert_eq!(fmt(1_000_000_000), "1.00G");
        assert_eq!(fmt(1_234_000_000), "1.23G");
        assert_eq!(fmt(1_234_000_000_000), "1,234G");
    }
}
