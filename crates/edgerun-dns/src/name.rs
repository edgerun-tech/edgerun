//! DNS domain name validation (RFC 1035 §2.3.1, RFC 2181 §11).

use alloc::{boxed::Box, format, string::{String, ToString}, vec, vec::Vec};
/// Error returned when a domain name fails validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameError {
    /// Domain name exceeds 253 characters (excluding trailing dot).
    ///
    /// - `len`: The actual length of the domain name.
    TooLong {
        /// The actual length of the domain name.
        len: usize,
    },
    /// A label exceeds 63 characters.
    ///
    /// - `label`: The label that exceeds the limit.
    LabelTooLong {
        /// The label that exceeds the 63-character limit.
        label: String,
    },
    /// A label starts or ends with a hyphen.
    ///
    /// - `label`: The label with invalid hyphen placement.
    InvalidHyphen {
        /// The label with invalid hyphen placement.
        label: String,
    },
    /// A label contains characters other than letters, digits, or hyphens.
    ///
    /// - `label`: The label containing invalid characters.
    InvalidCharacter {
        /// The label containing invalid characters.
        label: String,
    },
    /// Domain name is empty.
    Empty,
}

impl core::fmt::Display for NameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooLong { len } => {
                write!(f, "domain name too long: {} characters (max 253)", len)
            }
            Self::LabelTooLong { label } => {
                write!(f, "label too long: '{}' (max 63 characters)", label)
            }
            Self::InvalidHyphen { label } => {
                write!(f, "invalid hyphen placement in label: '{}'", label)
            }
            Self::InvalidCharacter { label } => {
                write!(f, "invalid character in label: '{}'", label)
            }
            Self::Empty => write!(f, "domain name is empty"),
        }
    }
}

impl core::error::Error for NameError {}

/// Validate a domain name against RFC 1035 rules.
///
/// Accepts names with or without trailing dot. The `@` symbol is accepted
/// as a shorthand for the zone origin (treated as empty/relative).
///
/// # Errors
/// Returns `NameError` if the name violates any DNS naming constraint.
pub fn validate_name(name: &str) -> Result<(), NameError> {
    // Accept "@" as zone origin shorthand
    if name == "@" || name.is_empty() {
        return Ok(());
    }

    // Strip trailing dot for length checks
    let name = name.strip_suffix('.').unwrap_or(name);

    if name.is_empty() {
        return Err(NameError::Empty);
    }

    if name.len() > 253 {
        return Err(NameError::TooLong { len: name.len() });
    }

    for label in name.split('.') {
        if label.is_empty() {
            continue; // empty label from double-dot or trailing dot
        }

        if label.len() > 63 {
            return Err(NameError::LabelTooLong {
                label: label.to_string(),
            });
        }

        if label.starts_with('-') || label.ends_with('-') {
            return Err(NameError::InvalidHyphen {
                label: label.to_string(),
            });
        }

        // Accept ASCII letters, digits, hyphens
        // Also accept punycode (xn--) which uses only these chars
        for ch in label.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '-' {
                return Err(NameError::InvalidCharacter {
                    label: label.to_string(),
                });
            }
        }
    }

    Ok(())
}

/// Normalize a domain name for internal use: lowercase, strip trailing dots.
pub fn normalize_name(name: &str) -> String {
    name.trim_end_matches('.').to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_names() {
        assert!(validate_name("example.com").is_ok());
        assert!(validate_name("www.example.com").is_ok());
        assert!(validate_name("example.com.").is_ok());
        assert!(validate_name("@").is_ok());
        assert!(validate_name("").is_ok());
        assert!(validate_name("a-b-c.example.com").is_ok());
        assert!(validate_name("xn--nxasmq5b.example.com").is_ok()); // punycode
        assert!(validate_name("123.456.789").is_ok()); // numeric (technically valid in zone files)
    }

    #[test]
    fn test_invalid_names() {
        // Hyphen at start/end
        assert!(validate_name("-example.com").is_err());
        assert!(validate_name("example-.com").is_err());

        // Label too long (64 chars)
        assert!(validate_name(&format!("{}.com", "a".repeat(64))).is_err());

        // Name too long
        assert!(validate_name(&"a".repeat(254)).is_err());

        // Invalid characters
        assert!(validate_name("exam ple.com").is_err()); // space
        assert!(validate_name("exam_ple.com").is_err()); // underscore (common but not RFC-compliant)
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize_name("WWW.EXAMPLE.COM"), "www.example.com");
        assert_eq!(normalize_name("www.example.com."), "www.example.com");
        assert_eq!(normalize_name("www.example.com..."), "www.example.com");
    }
}
