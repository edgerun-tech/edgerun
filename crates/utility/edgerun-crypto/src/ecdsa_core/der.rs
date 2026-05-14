//! Support for ASN.1 DER-encoded ECDSA signatures as specified in
//! [RFC5912 Appendix A].
//!
//! [RFC5912 Appendix A]: https://www.rfc-editor.org/rfc/rfc5912#appendix-A

use crate::der::{Decode, Encode, FixedTag, Length, Reader, Tag, Writer, asn1::UintRef};
use crate::ecdsa_core::{Error, Result};
use crate::elliptic_curve::{
    FieldBytesSize, PrimeCurve,
    consts::U9,
    generic_array::{ArrayLength, GenericArray, typenum::Unsigned},
};
use core::{
    fmt::{self, Debug},
    ops::{Add, Range},
};

#[cfg(feature = "p256_ecdsa_alloc")]
use {
    crate::signature::SignatureEncoding,
    crate::spki::{SignatureBitStringEncoding, der::asn1::BitString},
    alloc::{boxed::Box, vec::Vec},
};

/// Maximum overhead of an ASN.1 DER-encoded ECDSA signature for a given curve:
/// 9-bytes.
///
/// Includes 3-byte ASN.1 DER header:
///
/// - 1-byte: ASN.1 `SEQUENCE` tag (0x30)
/// - 2-byte: length
///
/// ...followed by two ASN.1 `INTEGER` values, which each have a header whose
/// maximum length is the following:
///
/// - 1-byte: ASN.1 `INTEGER` tag (0x02)
/// - 1-byte: length
/// - 1-byte: zero to indicate value is positive (`INTEGER` is signed)
pub type MaxOverhead = U9;

/// Maximum size of an ASN.1 DER encoded signature for the given elliptic curve.
pub type MaxSize<C> = <<FieldBytesSize<C> as Add>::Output as Add<MaxOverhead>>::Output;

/// Byte array containing a serialized ASN.1 signature
type SignatureBytes<C> = GenericArray<u8, MaxSize<C>>;

/// ASN.1 DER-encoded signature as specified in [RFC5912 Appendix A]:
///
/// ```text
/// ECDSA-Sig-Value ::= SEQUENCE {
///   r  INTEGER,
///   s  INTEGER
/// }
/// ```ignore
///
/// [RFC5912 Appendix A]: https://www.rfc-editor.org/rfc/rfc5912#appendix-A
pub struct Signature<C>
where
    C: PrimeCurve,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    /// ASN.1 DER-encoded signature data
    bytes: SignatureBytes<C>,

    /// Range of the `r` value within the signature
    r_range: Range<usize>,

    /// Range of the `s` value within the signature
    s_range: Range<usize>,
}

#[allow(clippy::len_without_is_empty)]
impl<C> Signature<C>
where
    C: PrimeCurve,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    /// Parse signature from DER-encoded bytes.
    pub fn from_bytes(input: &[u8]) -> Result<Self> {
        let (r, s) = decode_der(input).map_err(|_| Error::new())?;

        if r.as_bytes().len() > C::FieldBytesSize::USIZE
            || s.as_bytes().len() > C::FieldBytesSize::USIZE
        {
            return Err(Error::new());
        }

        let r_range = find_scalar_range(input, r.as_bytes())?;
        let s_range = find_scalar_range(input, s.as_bytes())?;

        if s_range.end != input.len() {
            return Err(Error::new());
        }

        let mut bytes = SignatureBytes::<C>::default();
        bytes[..s_range.end].copy_from_slice(input);

        Ok(Signature {
            bytes,
            r_range,
            s_range,
        })
    }

    /// Create an ASN.1 DER encoded signature from big endian `r` and `s` scalar
    /// components.
    pub(crate) fn from_components(r: &[u8], s: &[u8]) -> crate::der::Result<Self> {
        let r = UintRef::new(r)?;
        let s = UintRef::new(s)?;

        let mut bytes = SignatureBytes::<C>::default();
        let mut writer = crate::der::SliceWriter::new(&mut bytes);

        writer.sequence((r.encoded_len()? + s.encoded_len()?)?, |seq| {
            seq.encode(&r)?;
            seq.encode(&s)
        })?;

        writer
            .finish()?
            .try_into()
            .map_err(|_| Tag::Sequence.value_error())
    }

    /// Borrow this signature as a byte slice
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes.as_slice()[..self.len()]
    }

    /// Serialize this signature as a boxed byte slice
    #[cfg(feature = "p256_ecdsa_alloc")]
    pub fn to_bytes(&self) -> Box<[u8]> {
        self.as_bytes().to_vec().into_boxed_slice()
    }

    /// Get the length of the signature in bytes
    pub fn len(&self) -> usize {
        self.s_range.end
    }

    /// Get the `r` component of the signature (leading zeros removed)
    pub(crate) fn r(&self) -> &[u8] {
        &self.bytes[self.r_range.clone()]
    }

    /// Get the `s` component of the signature (leading zeros removed)
    pub(crate) fn s(&self) -> &[u8] {
        &self.bytes[self.s_range.clone()]
    }
}

impl<C> AsRef<[u8]> for Signature<C>
where
    C: PrimeCurve,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<C> Clone for Signature<C>
where
    C: PrimeCurve,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    fn clone(&self) -> Self {
        Self {
            bytes: self.bytes.clone(),
            r_range: self.r_range.clone(),
            s_range: self.s_range.clone(),
        }
    }
}

impl<C> Debug for Signature<C>
where
    C: PrimeCurve,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ecdsa::der::Signature<{:?}>(", C::default())?;

        for &byte in self.as_ref() {
            write!(f, "{:02X}", byte)?;
        }

        write!(f, ")")
    }
}

impl<'a, C> Decode<'a> for Signature<C>
where
    C: PrimeCurve,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    fn decode<R: Reader<'a>>(reader: &mut R) -> crate::der::Result<Self> {
        let header = reader.peek_header()?;
        header.tag.assert_eq(Tag::Sequence)?;

        let mut buf = SignatureBytes::<C>::default();
        let len = (header.encoded_len()? + header.length)?;
        let slice = buf
            .get_mut(..usize::try_from(len)?)
            .ok_or_else(|| reader.error(Tag::Sequence.length_error().kind()))?;

        reader.read_into(slice)?;
        Self::from_bytes(slice).map_err(|_| Tag::Integer.value_error())
    }
}

impl<C> Encode for Signature<C>
where
    C: PrimeCurve,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    fn encoded_len(&self) -> crate::der::Result<Length> {
        Length::try_from(self.len())
    }

    fn encode(&self, writer: &mut impl Writer) -> crate::der::Result<()> {
        writer.write(self.as_bytes())
    }
}

impl<C> FixedTag for Signature<C>
where
    C: PrimeCurve,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    const TAG: Tag = Tag::Sequence;
}

impl<C> From<crate::ecdsa_core::Signature<C>> for Signature<C>
where
    C: PrimeCurve,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    fn from(sig: crate::ecdsa_core::Signature<C>) -> Signature<C> {
        sig.to_der()
    }
}

impl<C> TryFrom<&[u8]> for Signature<C>
where
    C: PrimeCurve,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    type Error = Error;

    fn try_from(input: &[u8]) -> Result<Self> {
        Self::from_bytes(input)
    }
}

impl<C> TryFrom<Signature<C>> for crate::ecdsa_core::Signature<C>
where
    C: PrimeCurve,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    type Error = Error;

    fn try_from(sig: Signature<C>) -> Result<super::Signature<C>> {
        let mut bytes = super::SignatureBytes::<C>::default();
        let r_begin = C::FieldBytesSize::USIZE.saturating_sub(sig.r().len());
        let s_begin = bytes.len().saturating_sub(sig.s().len());
        bytes[r_begin..C::FieldBytesSize::USIZE].copy_from_slice(sig.r());
        bytes[s_begin..].copy_from_slice(sig.s());
        Self::try_from(bytes.as_slice())
    }
}

#[cfg(feature = "p256_ecdsa_alloc")]
impl<C> From<Signature<C>> for Box<[u8]>
where
    C: PrimeCurve,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    fn from(signature: Signature<C>) -> Box<[u8]> {
        signature.to_vec().into_boxed_slice()
    }
}

#[cfg(feature = "p256_ecdsa_alloc")]
impl<C> SignatureEncoding for Signature<C>
where
    C: PrimeCurve,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    type Repr = Box<[u8]>;

    fn to_vec(&self) -> Vec<u8> {
        self.as_bytes().into()
    }
}

#[cfg(feature = "p256_ecdsa_alloc")]
impl<C> SignatureBitStringEncoding for Signature<C>
where
    C: PrimeCurve,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    fn to_bitstring(&self) -> crate::der::Result<BitString> {
        BitString::new(0, self.to_vec())
    }
}


/// Decode the `r` and `s` components of a DER-encoded ECDSA signature.
fn decode_der(der_bytes: &[u8]) -> crate::der::Result<(UintRef<'_>, UintRef<'_>)> {
    let mut reader = crate::der::SliceReader::new(der_bytes)?;
    let header = crate::der::Header::decode(&mut reader)?;
    header.tag.assert_eq(Tag::Sequence)?;

    let ret = reader.read_nested(header.length, |reader| {
        let r = UintRef::decode(reader)?;
        let s = UintRef::decode(reader)?;
        Ok((r, s))
    })?;

    reader.finish(ret)
}

/// Locate the range within a slice at which a particular subslice is located
fn find_scalar_range(outer: &[u8], inner: &[u8]) -> Result<Range<usize>> {
    let outer_start = outer.as_ptr() as usize;
    let inner_start = inner.as_ptr() as usize;
    let start = inner_start
        .checked_sub(outer_start)
        .ok_or_else(Error::new)?;
    let end = start.checked_add(inner.len()).ok_or_else(Error::new)?;
    Ok(Range { start, end })
}

#[cfg(all(feature = "p256_ecdsa_digest", feature = "p256_ecdsa_hazmat"))]
impl<C> crate::signature::PrehashSignature for Signature<C>
where
    C: PrimeCurve + crate::ecdsa_core::hazmat::DigestPrimitive,
    MaxSize<C>: ArrayLength<u8>,
    <FieldBytesSize<C> as Add>::Output: Add<MaxOverhead> + ArrayLength<u8>,
{
    type Digest = C::Digest;
}
