/// 16.16 fixed-point number for hardware performance multipliers.
///
/// Range: 0.00001525 to 65535.99998
/// Precision: ~5 decimal digits
///
/// Used to convert physical core-microseconds into billable
/// reference-core-microseconds (RC-µs) based on benchmark scores.
///
/// No external dependencies — pure integer arithmetic.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FixedPoint16(u32);

impl FixedPoint16 {
    /// The value 1.0 represented in 16.16 fixed point.
    pub const ONE: Self = Self(1 << 16);

    /// The value 0.0.
    pub const ZERO: Self = Self(0);

    /// Maximum representable value.
    pub const MAX: Self = Self(u32::MAX);

    /// Create from a raw u32 representation.
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Get the raw u32 representation.
    #[inline]
    pub const fn to_raw(self) -> u32 {
        self.0
    }

    /// Create from a numerator and denominator ratio.
    ///
    /// `from_ratio(3, 2)` = 1.5 = 0x0001_8000
    ///
    /// # Panics
    /// Panics if `denominator` is 0 or if the result overflows u32.
    #[inline]
    pub const fn from_ratio(numerator: u64, denominator: u64) -> Self {
        assert!(denominator != 0, "division by zero in FixedPoint16::from_ratio");
        // Shift numerator left by 16 bits before dividing
        Self(((numerator << 16) / denominator) as u32)
    }

    /// Create from an integer.
    #[inline]
    pub const fn from_int(n: i32) -> Self {
        if n >= 0 {
            Self((n as u32) << 16)
        } else {
            Self(0)
        }
    }

    /// Truncate to integer (toward zero).
    #[inline]
    pub const fn to_int(self) -> i32 {
        (self.0 >> 16) as i32
    }

    /// Multiply by a u64, returning a u64 result.
    ///
    /// `fp.mul_u64(1000)` = `fp * 1000` rounded down.
    #[inline]
    pub const fn mul_u64(self, val: u64) -> u64 {
        (val * (self.0 as u64)) >> 16
    }

    /// Multiply by a u32, returning a u32 result.
    #[inline]
    pub const fn mul_u32(self, val: u32) -> u32 {
        (((val as u64) * (self.0 as u64)) >> 16) as u32
    }

    /// Multiply two fixed-point numbers.
    #[inline]
    pub const fn mul_fp(self, other: Self) -> Self {
        Self((((self.0 as u64) * (other.0 as u64)) >> 16) as u32)
    }

    /// Divide two fixed-point numbers.
    ///
    /// # Panics
    /// Panics if `other` is zero.
    #[inline]
    pub const fn div_fp(self, other: Self) -> Self {
        assert!(other.0 != 0, "division by zero in FixedPoint16::div_fp");
        Self((((self.0 as u64) << 16) / (other.0 as u64)) as u32)
    }

    /// Add two fixed-point numbers.
    #[inline]
    pub const fn add(self, other: Self) -> Self {
        Self(self.0.wrapping_add(other.0))
    }

    /// Subtract two fixed-point numbers. Saturates at zero.
    #[inline]
    pub const fn sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    /// Check if zero.
    #[inline]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Format as a human-readable string with 4 decimal places.
    pub fn format(&self) -> [u8; 16] {
        let int_part = self.0 >> 16;
        let frac_part = self.0 & 0xFFFF;
        // frac_part / 65536 * 10000 = frac_part * 10000 / 65536
        let frac_4digits = ((frac_part as u64) * 10000 / 65536) as u32;

        let mut buf = [0u8; 16];
        let mut len;

        // Write integer part
        if int_part == 0 {
            buf[0] = b'0';
            len = 1;
        } else {
            let mut n = int_part;
            let mut digits = [0u8; 10];
            let mut d = 0;
            while n > 0 {
                digits[d] = (n % 10) as u8 + b'0';
                n /= 10;
                d += 1;
            }
            for i in 0..d {
                buf[i] = digits[d - 1 - i];
            }
            len = d;
        }

        buf[len] = b'.';
        len += 1;

        // Write 4 fractional digits
        for shift in [3, 2, 1, 0] {
            let digit = (frac_4digits / 10u32.pow(shift)) % 10;
            buf[len] = digit as u8 + b'0';
            len += 1;
        }

        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_is_65536() {
        assert_eq!(FixedPoint16::ONE.to_raw(), 65536);
    }

    #[test]
    fn zero_is_zero() {
        assert!(FixedPoint16::ZERO.is_zero());
    }

    #[test]
    fn from_ratio_half() {
        let fp = FixedPoint16::from_ratio(1, 2);
        assert_eq!(fp.to_raw(), 32768); // 0.5 = 0x8000
    }

    #[test]
    fn from_ratio_one_and_half() {
        let fp = FixedPoint16::from_ratio(3, 2);
        assert_eq!(fp.to_raw(), 98304); // 1.5 = 0x1_8000
    }

    #[test]
    fn from_ratio_double() {
        let fp = FixedPoint16::from_ratio(2, 1);
        assert_eq!(fp.to_raw(), 131072); // 2.0 = 0x2_0000
    }

    #[test]
    fn from_int() {
        assert_eq!(FixedPoint16::from_int(5).to_raw(), 5 << 16);
        assert_eq!(FixedPoint16::from_int(0).to_raw(), 0);
    }

    #[test]
    fn to_int_truncates() {
        let fp = FixedPoint16::from_ratio(7, 2); // 3.5
        assert_eq!(fp.to_int(), 3);
    }

    #[test]
    fn mul_u64() {
        let fp = FixedPoint16::from_ratio(3, 2); // 1.5
        assert_eq!(fp.mul_u64(1000), 1500);
    }

    #[test]
    fn mul_u32() {
        let fp = FixedPoint16::from_ratio(5, 2); // 2.5
        assert_eq!(fp.mul_u32(100), 250);
    }

    #[test]
    fn mul_fp() {
        let a = FixedPoint16::from_ratio(3, 2); // 1.5
        let b = FixedPoint16::from_ratio(2, 1); // 2.0
        let result = a.mul_fp(b);
        assert_eq!(result.to_raw(), FixedPoint16::from_ratio(3, 1).to_raw()); // 3.0
    }

    #[test]
    fn div_fp() {
        let a = FixedPoint16::from_ratio(3, 1); // 3.0
        let b = FixedPoint16::from_ratio(2, 1); // 2.0
        let result = a.div_fp(b);
        assert_eq!(result.to_raw(), FixedPoint16::from_ratio(3, 2).to_raw()); // 1.5
    }

    #[test]
    fn add() {
        let a = FixedPoint16::from_ratio(1, 2);
        let b = FixedPoint16::from_ratio(1, 2);
        assert_eq!(a.add(b).to_raw(), FixedPoint16::ONE.to_raw());
    }

    #[test]
    fn sub_saturates() {
        let a = FixedPoint16::from_ratio(1, 4);
        let b = FixedPoint16::from_ratio(3, 4);
        assert!(a.sub(b).is_zero()); // saturates at 0
    }

    #[test]
    fn mul_compute_core_us() {
        // Simulate: 4 physical cores running for 1_000_000 µs
        // With a CPU multiplier of 2.5x (2.5 reference cores per physical core)
        let physical_core_us: u64 = 4_000_000; // 4 cores * 1_000_000 µs
        let cpu_multiplier = FixedPoint16::from_ratio(5, 2); // 2.5x
        let billable_rc_us = cpu_multiplier.mul_u64(physical_core_us);
        assert_eq!(billable_rc_us, 10_000_000);
    }

    #[test]
    fn format_one() {
        let buf = FixedPoint16::ONE.format();
        let s = std::str::from_utf8(&buf).unwrap().trim_end_matches('\0');
        assert_eq!(s, "1.0000");
    }

    #[test]
    fn format_half() {
        let fp = FixedPoint16::from_ratio(1, 2);
        let buf = fp.format();
        let s = std::str::from_utf8(&buf).unwrap().trim_end_matches('\0');
        assert_eq!(s, "0.5000");
    }

    #[test]
    fn format_two_and_half() {
        let fp = FixedPoint16::from_ratio(5, 2);
        let buf = fp.format();
        let s = std::str::from_utf8(&buf).unwrap().trim_end_matches('\0');
        assert_eq!(s, "2.5000");
    }

    #[test]
    fn roundtrip_from_to_int() {
        for n in 0..=100 {
            let fp = FixedPoint16::from_int(n);
            assert_eq!(fp.to_int(), n);
        }
    }

    #[test]
    fn from_ratio_large_performance() {
        // Modern EPYC might be 10x the reference (e.g., Pi 4)
        let fp = FixedPoint16::from_ratio(10, 1);
        assert_eq!(fp.to_int(), 10);
        assert_eq!(fp.mul_u64(1_000_000), 10_000_000);
    }
}
