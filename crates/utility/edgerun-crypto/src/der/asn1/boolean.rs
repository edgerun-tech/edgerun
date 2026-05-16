//! ASN.1 `BOOLEAN` support.

use crate::der::{
    DecodeValue, EncodeValue, Error, ErrorKind, FixedTag, Header, Length, Reader, Result, Tag,
    Writer, asn1::AnyRef, ord::OrdIsValueOrd,
};

/// Byte used to encode `true` in ASN.1 DER. From X.690 Section 11.1:
///
/// > If the encoding represents the boolean value TRUE, its single contents
/// > octet shall have all eight bits set to one.
const TRUE_OCTET: u8 = 0b11111111;

/// Byte used to encode `false` in ASN.1 DER.
const FALSE_OCTET: u8 = 0b00000000;

impl<'a> DecodeValue<'a> for bool {
    fn decode_value<R: Reader<'a>>(reader: &mut R, header: Header) -> Result<Self> {
        if header.length != Length::ONE {
            return Err(reader.error(ErrorKind::Length { tag: Self::TAG }));
        }

        match reader.read_byte()? {
            FALSE_OCTET => Ok(false),
            TRUE_OCTET => Ok(true),
            _ => Err(Self::TAG.non_canonical_error()),
        }
    }
}

impl EncodeValue for bool {
    fn value_len(&self) -> Result<Length> {
        Ok(Length::ONE)
    }

    fn encode_value(&self, writer: &mut impl Writer) -> Result<()> {
        writer.write_byte(if *self { TRUE_OCTET } else { FALSE_OCTET })
    }
}

impl FixedTag for bool {
    const TAG: Tag = Tag::Boolean;
}

impl OrdIsValueOrd for bool {}

impl TryFrom<AnyRef<'_>> for bool {
    type Error = Error;

    fn try_from(any: AnyRef<'_>) -> Result<bool> {
        any.try_into()
    }
}

#[cfg(test)]
mod tests;
