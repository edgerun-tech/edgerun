//! ASN.1 `REAL` support.

// TODO(tarcieri): checked arithmetic
#![allow(
    clippy::cast_lossless,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects
)]

use crate::der::{
    BytesRef, DecodeValue, EncodeValue, FixedTag, Header, Length, Reader, Result, StrRef, Tag,
    Writer,
};

use super::integer::uint::strip_leading_zeroes;

impl<'a> DecodeValue<'a> for f64 {
    fn decode_value<R: Reader<'a>>(reader: &mut R, header: Header) -> Result<Self> {
        let bytes = BytesRef::decode_value(reader, header)?.as_slice();

        if header.length == Length::ZERO {
            Ok(0.0)
        } else if is_nth_bit_one::<7>(bytes) {
            // Binary encoding from section 8.5.7 applies
            let sign: u64 = u64::from(is_nth_bit_one::<6>(bytes));

            // Section 8.5.7.2: Check the base -- the DER specs say that only base 2 should be supported in DER
            let base = mnth_bits_to_u8::<5, 4>(bytes);

            if base != 0 {
                // Real related error: base is not DER compliant (base encoded in enum)
                return Err(Tag::Real.value_error());
            }

            // Section 8.5.7.3
            let scaling_factor = mnth_bits_to_u8::<3, 2>(bytes);

            // Section 8.5.7.4
            let mantissa_start;
            let exponent = match mnth_bits_to_u8::<1, 0>(bytes) {
                0 => {
                    mantissa_start = 2;
                    let ebytes = (i16::from_be_bytes([0x0, bytes[1]])).to_be_bytes();
                    u64::from_be_bytes([0x0, 0x0, 0x0, 0x0, 0x0, 0x0, ebytes[0], ebytes[1]])
                }
                1 => {
                    mantissa_start = 3;
                    let ebytes = (i16::from_be_bytes([bytes[1], bytes[2]])).to_be_bytes();
                    u64::from_be_bytes([0x0, 0x0, 0x0, 0x0, 0x0, 0x0, ebytes[0], ebytes[1]])
                }
                _ => {
                    // Real related error: encoded exponent cannot be represented on an IEEE-754 double
                    return Err(Tag::Real.value_error());
                }
            };
            // Section 8.5.7.5: Read the remaining bytes for the mantissa
            let mut n_bytes = [0x0; 8];
            for (pos, byte) in bytes[mantissa_start..].iter().rev().enumerate() {
                n_bytes[7 - pos] = *byte;
            }
            let n = u64::from_be_bytes(n_bytes);
            // Multiply byt 2^F corresponds to just a left shift
            let mantissa = n << scaling_factor;
            // Create the f64
            Ok(encode_f64(sign, exponent, mantissa))
        } else if is_nth_bit_one::<6>(bytes) {
            // This either a special value, or it's the value minus zero is encoded, section 8.5.9 applies
            match mnth_bits_to_u8::<1, 0>(bytes) {
                0 => Ok(f64::INFINITY),
                1 => Ok(f64::NEG_INFINITY),
                2 => Ok(f64::NAN),
                3 => Ok(-0.0_f64),
                _ => Err(Tag::Real.value_error()),
            }
        } else {
            let astr = StrRef::from_bytes(&bytes[1..])?;
            match astr.inner.parse::<f64>() {
                Ok(val) => Ok(val),
                // Real related error: encoding not supported or malformed
                Err(_) => Err(Tag::Real.value_error()),
            }
        }
    }
}

impl EncodeValue for f64 {
    fn value_len(&self) -> Result<Length> {
        if self.is_sign_positive() && (*self) < f64::MIN_POSITIVE {
            // Zero: positive yet smaller than the minimum positive number
            Ok(Length::ZERO)
        } else if self.is_nan()
            || self.is_infinite()
            || (self.is_sign_negative() && -self < f64::MIN_POSITIVE)
        {
            // NaN, infinite (positive or negative), or negative zero (negative but its negative is less than the min positive number)
            Ok(Length::ONE)
        } else {
            // The length is that of the first octets plus those needed for the exponent plus those needed for the mantissa
            let (_sign, exponent, mantissa) = decode_f64(*self);

            let exponent_len = if exponent == 0 {
                // Section 8.5.7.4: there must be at least one octet for exponent encoding
                // But, if the exponent is zero, it'll be skipped, so we make sure force it to 1
                Length::ONE
            } else {
                let ebytes = exponent.to_be_bytes();
                Length::try_from(strip_leading_zeroes(&ebytes).len())?
            };

            let mantissa_len = if mantissa == 0 {
                Length::ONE
            } else {
                let mbytes = mantissa.to_be_bytes();
                Length::try_from(strip_leading_zeroes(&mbytes).len())?
            };

            exponent_len + mantissa_len + Length::ONE
        }
    }

    fn encode_value(&self, writer: &mut impl Writer) -> Result<()> {
        // Check if special value
        // Encode zero first, if it's zero
        // Special value from section 8.5.9 if non zero
        if self.is_nan()
            || self.is_infinite()
            || (self.is_sign_negative() && -self < f64::MIN_POSITIVE)
            || (self.is_sign_positive() && (*self) < f64::MIN_POSITIVE)
        {
            if self.is_sign_positive() && (*self) < f64::MIN_POSITIVE {
                // Zero
                return Ok(());
            } else if self.is_nan() {
                // Not a number
                writer.write_byte(0b0100_0010)?;
            } else if self.is_infinite() {
                if self.is_sign_negative() {
                    // Negative infinity
                    writer.write_byte(0b0100_0001)?;
                } else {
                    // Plus infinity
                    writer.write_byte(0b0100_0000)?;
                }
            } else {
                // Minus zero
                writer.write_byte(0b0100_0011)?;
            }
        } else {
            // Always use binary encoding, set bit 8 to 1
            let mut first_byte = 0b1000_0000;

            if self.is_sign_negative() {
                // Section 8.5.7.1: set bit 7 to 1 if negative
                first_byte |= 0b0100_0000;
            }

            // Bits 6 and 5 are set to 0 to specify that binary encoding is used
            //
            // NOTE: the scaling factor is only used to align the implicit point of the mantissa.
            // This is unnecessary in DER because the base is 2, and therefore necessarily aligned.
            // Therefore, we do not modify the mantissa in anyway after this function call, which
            // already adds the implicit one of the IEEE 754 representation.
            let (_sign, exponent, mantissa) = decode_f64(*self);

            // Encode the exponent as two's complement on 16 bits and remove the bias
            let exponent_bytes = exponent.to_be_bytes();
            let ebytes = strip_leading_zeroes(&exponent_bytes);

            match ebytes.len() {
                0 | 1 => {}
                2 => first_byte |= 0b0000_0001,
                3 => first_byte |= 0b0000_0010,
                _ => {
                    // TODO: support multi octet exponent encoding?
                    return Err(Tag::Real.value_error());
                }
            }

            writer.write_byte(first_byte)?;

            // Encode both bytes or just the last one, handled by encode_bytes directly
            // Rust already encodes the data as two's complement, so no further processing is needed
            writer.write(ebytes)?;

            // Now, encode the mantissa as unsigned binary number
            let mantissa_bytes = mantissa.to_be_bytes();
            let mbytes = strip_leading_zeroes(&mantissa_bytes);
            writer.write(mbytes)?;
        }

        Ok(())
    }
}

impl FixedTag for f64 {
    const TAG: Tag = Tag::Real;
}

/// Is the N-th bit 1 in the first octet?
/// NOTE: this function is zero indexed
pub(crate) fn is_nth_bit_one<const N: usize>(bytes: &[u8]) -> bool {
    if N < 8 {
        bytes
            .first()
            .map(|byte| byte & (1 << N) != 0)
            .unwrap_or(false)
    } else {
        false
    }
}

/// Convert bits M, N into a u8, in the first octet only
pub(crate) fn mnth_bits_to_u8<const M: usize, const N: usize>(bytes: &[u8]) -> u8 {
    let bit_m = is_nth_bit_one::<M>(bytes);
    let bit_n = is_nth_bit_one::<N>(bytes);
    (bit_m as u8) << 1 | bit_n as u8
}

/// Decode an f64 as its sign, exponent, and mantissa in u64 and in that order, using bit shifts and masks.
/// Note: this function **removes** the 1023 bias from the exponent and adds the implicit 1
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn decode_f64(f: f64) -> (u64, u64, u64) {
    let bits = f.to_bits();
    let sign = bits >> 63;
    let exponent = bits >> 52 & 0x7ff;
    let exponent_bytes_no_bias = (exponent as i16 - 1023).to_be_bytes();
    let exponent_no_bias = u64::from_be_bytes([
        0x0,
        0x0,
        0x0,
        0x0,
        0x0,
        0x0,
        exponent_bytes_no_bias[0],
        exponent_bytes_no_bias[1],
    ]);
    let mantissa = bits & 0xfffffffffffff;
    (sign, exponent_no_bias, mantissa + 1)
}

/// Encode an f64 from its sign, exponent (**without** the 1023 bias), and (mantissa - 1) using bit shifts as received by ASN1
pub(crate) fn encode_f64(sign: u64, exponent: u64, mantissa: u64) -> f64 {
    // Add the bias to the exponent
    let exponent_with_bias =
        (i16::from_be_bytes([exponent.to_be_bytes()[6], exponent.to_be_bytes()[7]]) + 1023) as u64;
    let bits = sign << 63 | exponent_with_bias << 52 | (mantissa - 1);
    f64::from_bits(bits)
}
