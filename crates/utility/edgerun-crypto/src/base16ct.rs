use core::fmt;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    InvalidEncoding,
    InvalidLength,
}

pub struct HexDisplay<'a>(pub &'a [u8]);

impl fmt::Display for HexDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(self, f)
    }
}

impl fmt::UpperHex for HexDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut hex = [0u8; 2];
        for &byte in self.0 {
            f.write_str(upper::encode_str(&[byte], &mut hex).map_err(|_| fmt::Error)?)?;
        }
        Ok(())
    }
}

impl fmt::LowerHex for HexDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut hex = [0u8; 2];
        for &byte in self.0 {
            f.write_str(lower::encode_str(&[byte], &mut hex).map_err(|_| fmt::Error)?)?;
        }
        Ok(())
    }
}

#[inline(always)]
pub fn decoded_len(bytes: &[u8]) -> Result<usize> {
    if bytes.len() & 1 == 0 {
        Ok(bytes.len() / 2)
    } else {
        Err(Error::InvalidLength)
    }
}

#[inline(always)]
pub fn encoded_len(bytes: &[u8]) -> usize {
    bytes.len() * 2
}

fn decode_inner<'a>(
    src: &[u8],
    dst: &'a mut [u8],
    decode_nibble: impl Fn(u8) -> u16,
) -> Result<&'a [u8]> {
    let dst = dst
        .get_mut(..decoded_len(src)?)
        .ok_or(Error::InvalidLength)?;

    let mut err = 0u16;
    for (src, dst) in src.chunks_exact(2).zip(dst.iter_mut()) {
        let byte = (decode_nibble(src[0]) << 4) | decode_nibble(src[1]);
        err |= byte >> 8;
        *dst = byte as u8;
    }

    if err == 0 {
        Ok(dst)
    } else {
        Err(Error::InvalidEncoding)
    }
}

pub mod lower {
    use super::{Error, decode_inner, encoded_len};

    pub fn decode<'a>(src: impl AsRef<[u8]>, dst: &'a mut [u8]) -> Result<&'a [u8], Error> {
        decode_inner(src.as_ref(), dst, decode_nibble)
    }

    pub fn encode<'a>(src: &[u8], dst: &'a mut [u8]) -> Result<&'a [u8], Error> {
        let dst = dst
            .get_mut(..encoded_len(src))
            .ok_or(Error::InvalidLength)?;
        for (src, dst) in src.iter().zip(dst.chunks_exact_mut(2)) {
            dst[0] = encode_nibble(src >> 4);
            dst[1] = encode_nibble(src & 0x0f);
        }
        Ok(dst)
    }

    pub fn encode_str<'a>(src: &[u8], dst: &'a mut [u8]) -> Result<&'a str, Error> {
        encode(src, dst).map(|r| unsafe { core::str::from_utf8_unchecked(r) })
    }

    #[inline(always)]
    fn decode_nibble(src: u8) -> u16 {
        let byte = src as i16;
        let mut ret: i16 = -1;
        ret += (((0x2fi16 - byte) & (byte - 0x3a)) >> 8) & (byte - 47);
        ret += (((0x60i16 - byte) & (byte - 0x67)) >> 8) & (byte - 86);
        ret as u16
    }

    #[inline(always)]
    fn encode_nibble(src: u8) -> u8 {
        let mut ret = src as i16 + 0x30;
        ret += ((0x39i16 - ret) >> 8) & (0x61i16 - 0x3a);
        ret as u8
    }
}

pub mod upper {
    use super::{Error, decode_inner, encoded_len};

    pub fn decode<'a>(src: impl AsRef<[u8]>, dst: &'a mut [u8]) -> Result<&'a [u8], Error> {
        decode_inner(src.as_ref(), dst, decode_nibble)
    }

    pub fn encode<'a>(src: &[u8], dst: &'a mut [u8]) -> Result<&'a [u8], Error> {
        let dst = dst
            .get_mut(..encoded_len(src))
            .ok_or(Error::InvalidLength)?;
        for (src, dst) in src.iter().zip(dst.chunks_exact_mut(2)) {
            dst[0] = encode_nibble(src >> 4);
            dst[1] = encode_nibble(src & 0x0f);
        }
        Ok(dst)
    }

    pub fn encode_str<'a>(src: &[u8], dst: &'a mut [u8]) -> Result<&'a str, Error> {
        encode(src, dst).map(|r| unsafe { core::str::from_utf8_unchecked(r) })
    }

    #[inline(always)]
    fn decode_nibble(src: u8) -> u16 {
        let byte = src as i16;
        let mut ret: i16 = -1;
        ret += (((0x2fi16 - byte) & (byte - 0x3a)) >> 8) & (byte - 47);
        ret += (((0x40i16 - byte) & (byte - 0x47)) >> 8) & (byte - 54);
        ret as u16
    }

    #[inline(always)]
    fn encode_nibble(src: u8) -> u8 {
        let mut ret = src as i16 + 0x30;
        ret += ((0x39i16 - ret) >> 8) & (0x41i16 - 0x3a);
        ret as u8
    }
}

pub mod mixed {
    use super::{Error, decode_inner};

    pub fn decode<'a>(src: impl AsRef<[u8]>, dst: &'a mut [u8]) -> Result<&'a [u8], Error> {
        decode_inner(src.as_ref(), dst, decode_nibble)
    }

    #[inline(always)]
    fn decode_nibble(src: u8) -> u16 {
        let byte = src as i16;
        let mut ret: i16 = -1;
        ret += (((0x2fi16 - byte) & (byte - 0x3a)) >> 8) & (byte - 47);
        ret += (((0x40i16 - byte) & (byte - 0x47)) >> 8) & (byte - 54);
        ret += (((0x60i16 - byte) & (byte - 0x67)) >> 8) & (byte - 86);
        ret as u16
    }
}
