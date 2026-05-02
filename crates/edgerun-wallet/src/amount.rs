//! DecimalAmount — fixed-precision decimal arithmetic without floats.
//!
//! Stores signless decimal as integer mantissa + scale.
//! Parses from string only (no exponent notation, no NaN/inf).
//! Rejects negative values for public money amounts.

use core::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecimalAmount {
    /// Unsigned mantissa (the digits without decimal point).
    mantissa: u128,
    /// Number of decimal places (scale).
    scale: u8,
}

impl DecimalAmount {
    /// Parse from string. Rejects exponents, NaN, inf, negative values.
    /// Returns None if the string is not a valid positive decimal.
    pub fn parse(input: &str) -> Option<Self> {
        let s = input.trim();
        if s.is_empty() {
            return None;
        }

        // Reject negative
        if s.starts_with('-') || s.starts_with('+') {
            return None;
        }

        // Reject exponent notation
        if s.contains('e') || s.contains('E') {
            return None;
        }

        let (int_part, frac_part) = if let Some(pos) = s.find('.') {
            let (a, b) = s.split_at(pos);
            (a, &b[1..])
        } else {
            (s, "")
        };

        // Validate digits only
        if !int_part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        if !frac_part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }

        // Remove leading zeros from int part for clean mantissa
        let int_clean: alloc::string::String = int_part.trim_start_matches('0').into();
        let int_clean = if int_clean.is_empty() { "0" } else { &int_clean };

        let scale = frac_part.len() as u8;
        if scale > 38 {
            return None; // scale too large
        }

        // Build mantissa: int_part + frac_part (no decimal point)
        let mantissa_str = alloc::format!("{}{}", int_clean, frac_part);
        let mantissa_str = mantissa_str.trim_start_matches('0');
        let mantissa_str = if mantissa_str.is_empty() { "0" } else { mantissa_str };

        let mantissa: u128 = mantissa_str.parse().ok()?;

        Some(Self { mantissa, scale })
    }

    /// Create from integer (scale = 0).
    pub fn from_integer(val: u128) -> Self {
        Self { mantissa: val, scale: 0 }
    }

    /// Get the mantissa (raw digits).
    pub fn mantissa(&self) -> u128 {
        self.mantissa
    }

    /// Get the scale (decimal places).
    pub fn scale(&self) -> u8 {
        self.scale
    }

    /// Canonical format: "123.456" with trailing zeros preserved from original parse.
    pub fn to_canonical_string(&self) -> alloc::string::String {
        if self.scale == 0 {
            return alloc::format!("{}", self.mantissa);
        }

        let mantissa_str = alloc::format!("{}", self.mantissa);
        let scale = self.scale as usize;

        if mantissa_str.len() > scale {
            let (int_part, frac_part) = mantissa_str.split_at(mantissa_str.len() - scale);
            alloc::format!("{}.{}", int_part, frac_part)
        } else {
            // Need leading zeros
            let needed = scale - mantissa_str.len();
            let mut result = alloc::string::String::with_capacity(scale + 2 + needed);
            result.push('0');
            result.push('.');
            for _ in 0..needed {
                result.push('0');
            }
            result.push_str(&mantissa_str);
            result
        }
    }

    /// Normalize to a target scale (adds trailing zeros or truncates).
    /// Truncation is lossy — for display only.
    pub fn normalize(&self, target_scale: u8) -> Option<Self> {
        if self.scale == target_scale {
            return Some(*self);
        }

        if target_scale > self.scale {
            // Add trailing zeros
            let extra = (target_scale - self.scale) as u32;
            let pow = 10u128.checked_pow(extra)?;
            let mantissa = self.mantissa.checked_mul(pow)?;
            Some(Self { mantissa, scale: target_scale })
        } else {
            // Truncate (lossy)
            let reduce = (self.scale - target_scale) as u32;
            let pow = 10u128.checked_pow(reduce)?;
            let mantissa = self.mantissa / pow;
            Some(Self { mantissa, scale: target_scale })
        }
    }

    /// Checked addition. Returns None on overflow.
    pub fn checked_add(&self, other: &Self) -> Option<Self> {
        let max_scale = self.scale.max(other.scale);
        let a = self.normalize(max_scale)?;
        let b = other.normalize(max_scale)?;
        let mantissa = a.mantissa.checked_add(b.mantissa)?;
        Some(Self { mantissa, scale: max_scale })
    }

    /// Checked subtraction. Returns None if result would be negative.
    pub fn checked_sub(&self, other: &Self) -> Option<Self> {
        let max_scale = self.scale.max(other.scale);
        let a = self.normalize(max_scale)?;
        let b = other.normalize(max_scale)?;
        if b.mantissa > a.mantissa {
            return None; // Would be negative
        }
        let mantissa = a.mantissa.checked_sub(b.mantissa)?;
        Some(Self { mantissa, scale: max_scale })
    }

    /// Compare two amounts (normalize to same scale first).
    pub fn compare(&self, other: &Self) -> core::cmp::Ordering {
        let max_scale = self.scale.max(other.scale);
        let a = self.normalize(max_scale).unwrap();
        let b = other.normalize(max_scale).unwrap();
        a.mantissa.cmp(&b.mantissa)
    }

    /// Returns true if this amount is zero.
    pub fn is_zero(&self) -> bool {
        self.mantissa == 0
    }
}

impl fmt::Debug for DecimalAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DecimalAmount({}, scale={})", self.to_canonical_string(), self.scale)
    }
}

impl fmt::Display for DecimalAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_canonical_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let a = DecimalAmount::parse("100.00").unwrap();
        assert_eq!(a.mantissa(), 10000);
        assert_eq!(a.scale(), 2);
        assert_eq!(a.to_canonical_string(), "100.00");
    }

    #[test]
    fn parse_small() {
        let a = DecimalAmount::parse("0.12345678").unwrap();
        assert_eq!(a.mantissa(), 12345678);
        assert_eq!(a.scale(), 8);
    }

    #[test]
    fn parse_integer() {
        let a = DecimalAmount::parse("42").unwrap();
        assert_eq!(a.mantissa(), 42);
        assert_eq!(a.scale(), 0);
        assert_eq!(a.to_canonical_string(), "42");
    }

    #[test]
    fn parse_zero() {
        let a = DecimalAmount::parse("0").unwrap();
        assert!(a.is_zero());
        assert_eq!(a.to_canonical_string(), "0");
    }

    #[test]
    fn parse_leading_zeros() {
        let a = DecimalAmount::parse("000.500").unwrap();
        assert_eq!(a.mantissa(), 500);
        assert_eq!(a.scale(), 3);
        assert_eq!(a.to_canonical_string(), "0.500");
    }

    #[test]
    fn reject_negative() {
        assert!(DecimalAmount::parse("-1.0").is_none());
    }

    #[test]
    fn reject_exponent() {
        assert!(DecimalAmount::parse("1e5").is_none());
        assert!(DecimalAmount::parse("1E-3").is_none());
    }

    #[test]
    fn reject_nan_inf() {
        assert!(DecimalAmount::parse("NaN").is_none());
        assert!(DecimalAmount::parse("inf").is_none());
    }

    #[test]
    fn add_same_scale() {
        let a = DecimalAmount::parse("100.00").unwrap();
        let b = DecimalAmount::parse("50.00").unwrap();
        let sum = a.checked_add(&b).unwrap();
        assert_eq!(sum.to_canonical_string(), "150.00");
    }

    #[test]
    fn add_diff_scales() {
        let a = DecimalAmount::parse("100.5").unwrap(); // scale 1
        let b = DecimalAmount::parse("0.25").unwrap();   // scale 2
        let sum = a.checked_add(&b).unwrap();
        assert_eq!(sum.to_canonical_string(), "100.75");
    }

    #[test]
    fn sub() {
        let a = DecimalAmount::parse("100.00").unwrap();
        let b = DecimalAmount::parse("25.50").unwrap();
        let diff = a.checked_sub(&b).unwrap();
        assert_eq!(diff.to_canonical_string(), "74.50");
    }

    #[test]
    fn sub_negative_returns_none() {
        let a = DecimalAmount::parse("10.00").unwrap();
        let b = DecimalAmount::parse("20.00").unwrap();
        assert!(a.checked_sub(&b).is_none());
    }

    #[test]
    fn compare() {
        let a = DecimalAmount::parse("100.00").unwrap();
        let b = DecimalAmount::parse("99.999").unwrap();
        assert_eq!(a.compare(&b), core::cmp::Ordering::Greater);
        assert_eq!(b.compare(&a), core::cmp::Ordering::Less);
        assert_eq!(a.compare(&a), core::cmp::Ordering::Equal);
    }

    #[test]
    fn normalize_up() {
        let a = DecimalAmount::parse("1.5").unwrap(); // scale 1
        let norm = a.normalize(3).unwrap();
        assert_eq!(norm.mantissa(), 1500);
        assert_eq!(norm.scale(), 3);
        assert_eq!(norm.to_canonical_string(), "1.500");
    }

    #[test]
    fn normalize_down_lossy() {
        let a = DecimalAmount::parse("1.5559").unwrap();
        let norm = a.normalize(2).unwrap();
        assert_eq!(norm.to_canonical_string(), "1.55");
    }
}
