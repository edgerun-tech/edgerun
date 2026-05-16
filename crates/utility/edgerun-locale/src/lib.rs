#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
const LOCALE_ENV_KEYS: &[&str] = &["LC_ALL", "LC_MESSAGES", "LANG"];

/// Return the current process locale in a form accepted by ICU parsers.
///
/// The value is discovered from the conventional Unix locale environment
/// variables and normalized from forms like `en_US.UTF-8` to `en-US`.
#[cfg(feature = "std")]
pub fn get_locale() -> Option<String> {
    LOCALE_ENV_KEYS
        .iter()
        .filter_map(|key| std::env::var(key).ok())
        .find_map(|value| normalize_locale(&value))
}

#[cfg(feature = "std")]
fn normalize_locale(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || matches!(value, "C" | "POSIX") {
        return None;
    }

    let without_modifier = value.split('@').next().unwrap_or(value);
    let without_codeset = without_modifier
        .split('.')
        .next()
        .unwrap_or(without_modifier);
    let normalized = without_codeset.replace('_', "-");

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_locale;

    #[test]
    fn normalizes_common_locale_forms() {
        assert_eq!(normalize_locale("en_US.UTF-8"), Some("en-US".to_string()));
        assert_eq!(normalize_locale("th_TH"), Some("th-TH".to_string()));
        assert_eq!(normalize_locale("fr-FR@euro"), Some("fr-FR".to_string()));
    }

    #[test]
    fn ignores_empty_and_posix_locales() {
        assert_eq!(normalize_locale(""), None);
        assert_eq!(normalize_locale("C"), None);
        assert_eq!(normalize_locale("POSIX"), None);
    }
}
