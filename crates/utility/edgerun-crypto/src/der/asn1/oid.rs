//! ASN.1 `OBJECT IDENTIFIER`

use crate::const_oid::ObjectIdentifier;
use crate::der::{
    DecodeValue, EncodeValue, Error, FixedTag, Header, Length, Reader, Result, Tag, Tagged, Writer,
    asn1::AnyRef, ord::OrdIsValueOrd,
};

#[cfg(feature = "alloc")]
use super::Any;

impl<'a> DecodeValue<'a> for ObjectIdentifier {
    fn decode_value<R: Reader<'a>>(reader: &mut R, header: Header) -> Result<Self> {
        let mut buf = [0u8; ObjectIdentifier::MAX_SIZE];
        let slice = buf
            .get_mut(..header.length.try_into()?)
            .ok_or_else(|| Self::TAG.length_error())?;

        let actual_len = reader.read_into(slice)?.len();
        debug_assert_eq!(actual_len, header.length.try_into()?);
        Ok(Self::from_bytes(slice)?)
    }
}

impl EncodeValue for ObjectIdentifier {
    fn value_len(&self) -> Result<Length> {
        Length::try_from(self.as_bytes().len())
    }

    fn encode_value(&self, writer: &mut impl Writer) -> Result<()> {
        writer.write(self.as_bytes())
    }
}

impl FixedTag for ObjectIdentifier {
    const TAG: Tag = Tag::ObjectIdentifier;
}

impl OrdIsValueOrd for ObjectIdentifier {}

impl<'a> From<&'a ObjectIdentifier> for AnyRef<'a> {
    fn from(oid: &'a ObjectIdentifier) -> AnyRef<'a> {
        // Note: ensuring an infallible conversion is possible relies on the
        // invariant that `crate::const_oid::MAX_LEN <= Length::max()`.
        //
        // The `length()` test below ensures this is the case.
        let value = oid
            .as_bytes()
            .try_into()
            .expect("OID length invariant violated");

        AnyRef::from_tag_and_value(Tag::ObjectIdentifier, value)
    }
}

#[cfg(feature = "alloc")]
impl From<ObjectIdentifier> for Any {
    fn from(oid: ObjectIdentifier) -> Any {
        AnyRef::from(&oid).into()
    }
}

impl TryFrom<AnyRef<'_>> for ObjectIdentifier {
    type Error = Error;

    fn try_from(any: AnyRef<'_>) -> Result<ObjectIdentifier> {
        any.tag().assert_eq(Tag::ObjectIdentifier)?;
        Ok(ObjectIdentifier::from_bytes(any.value())?)
    }
}

#[cfg(test)]
mod tests;
