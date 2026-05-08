//! Support for the SEC1 `Elliptic-Curve-Point-to-Octet-String` and
//! `Octet-String-to-Elliptic-Curve-Point` encoding algorithms.
//!
//! Described in [SEC1: Elliptic Curve Cryptography] (Version 2.0) section 2.3.3 (p.10).
//!
//! [SEC1: Elliptic Curve Cryptography]: https://www.secg.org/sec1-v2.pdf

use crate::base16ct::HexDisplay;
use crate::sec1::{Error, Result};
use core::{
    cmp::Ordering,
    fmt::{self, Debug},
    hash::{Hash, Hasher},
    ops::Add,
    str,
};
use generic_array::{
    typenum::{U1, U24, U28, U32, U48, U66},
    ArrayLength, GenericArray,
};

#[cfg(feature = "elliptic_curve_alloc")]
use alloc::boxed::Box;

#[cfg(feature = "elliptic_curve_serde")]
use serdect::serde::{de, ser, Deserialize, Serialize};

use crate::subtle::{Choice, ConditionallySelectable};

use crate::zeroize::Zeroize;

/// Trait for supported modulus sizes which precomputes the typenums for
/// various point encodings so they don't need to be included as bounds.
// TODO(tarcieri): replace this all with const generic expressions.
pub trait ModulusSize: 'static + ArrayLength<u8> + Copy + Debug {
    /// Size of a compressed point for the given elliptic curve when encoded
    /// using the SEC1 `Elliptic-Curve-Point-to-Octet-String` algorithm
    /// (including leading `0x02` or `0x03` tag byte).
    type CompressedPointSize: 'static + ArrayLength<u8> + Copy + Debug;

    /// Size of an uncompressed point for the given elliptic curve when encoded
    /// using the SEC1 `Elliptic-Curve-Point-to-Octet-String` algorithm
    /// (including leading `0x04` tag byte).
    type UncompressedPointSize: 'static + ArrayLength<u8> + Copy + Debug;

    /// Size of an untagged point for given elliptic curve, i.e. size of two
    /// serialized base field elements.
    type UntaggedPointSize: 'static + ArrayLength<u8> + Copy + Debug;
}

macro_rules! impl_modulus_size {
    ($($size:ty),+) => {
        $(impl ModulusSize for $size {
            type CompressedPointSize = <$size as Add<U1>>::Output;
            type UncompressedPointSize = <Self::UntaggedPointSize as Add<U1>>::Output;
            type UntaggedPointSize = <$size as Add>::Output;
        })+
    }
}

impl_modulus_size!(U24, U28, U32, U48, U66);

/// SEC1 encoded curve point.
///
/// This type is an enum over the compressed and uncompressed encodings,
/// useful for cases where either encoding can be supported, or conversions
/// between the two forms.
#[derive(Clone, Default)]
pub struct EncodedPoint<Size>
where
    Size: ModulusSize,
{
    bytes: GenericArray<u8, Size::UncompressedPointSize>,
}

#[allow(clippy::len_without_is_empty)]
impl<Size> EncodedPoint<Size>
where
    Size: ModulusSize,
{
    /// Decode elliptic curve point (compressed or uncompressed) from the
    /// `Elliptic-Curve-Point-to-Octet-String` encoding described in
    /// SEC 1: Elliptic Curve Cryptography (Version 2.0) section
    /// 2.3.3 (page 10).
    ///
    /// <http://www.secg.org/sec1-v2.pdf>
    pub fn from_bytes(input: impl AsRef<[u8]>) -> Result<Self> {
        let input = input.as_ref();

        // Validate tag
        let tag = input
            .first()
            .cloned()
            .ok_or(Error::PointEncoding)
            .and_then(Tag::from_u8)?;

        // Validate length
        let expected_len = tag.message_len(Size::to_usize());

        if input.len() != expected_len {
            return Err(Error::PointEncoding);
        }

        let mut bytes = GenericArray::default();
        bytes[..expected_len].copy_from_slice(input);
        Ok(Self { bytes })
    }

    /// Decode elliptic curve point from raw uncompressed coordinates, i.e.
    /// encoded as the concatenated `x || y` coordinates with no leading SEC1
    /// tag byte (which would otherwise be `0x04` for an uncompressed point).
    pub fn from_untagged_bytes(bytes: &GenericArray<u8, Size::UntaggedPointSize>) -> Self {
        let (x, y) = bytes.split_at(Size::to_usize());
        Self::from_affine_coordinates(x.into(), y.into(), false)
    }

    /// Encode an elliptic curve point from big endian serialized coordinates
    /// (with optional point compression)
    pub fn from_affine_coordinates(
        x: &GenericArray<u8, Size>,
        y: &GenericArray<u8, Size>,
        compress: bool,
    ) -> Self {
        let tag = if compress {
            Tag::compress_y(y.as_slice())
        } else {
            Tag::Uncompressed
        };

        let mut bytes = GenericArray::default();
        bytes[0] = tag.into();
        bytes[1..(Size::to_usize() + 1)].copy_from_slice(x);

        if !compress {
            bytes[(Size::to_usize() + 1)..].copy_from_slice(y);
        }

        Self { bytes }
    }

    /// Return [`EncodedPoint`] representing the additive identity
    /// (a.k.a. point at infinity)
    pub fn identity() -> Self {
        Self::default()
    }

    /// Get the length of the encoded point in bytes
    pub fn len(&self) -> usize {
        self.tag().message_len(Size::to_usize())
    }

    /// Get byte slice containing the serialized [`EncodedPoint`].
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len()]
    }

    /// Get boxed byte slice containing the serialized [`EncodedPoint`]
    #[cfg(feature = "elliptic_curve_alloc")]
    pub fn to_bytes(&self) -> Box<[u8]> {
        self.as_bytes().to_vec().into_boxed_slice()
    }

    /// Is this [`EncodedPoint`] compact?
    pub fn is_compact(&self) -> bool {
        self.tag().is_compact()
    }

    /// Is this [`EncodedPoint`] compressed?
    pub fn is_compressed(&self) -> bool {
        self.tag().is_compressed()
    }

    /// Is this [`EncodedPoint`] the additive identity? (a.k.a. point at infinity)
    pub fn is_identity(&self) -> bool {
        self.tag().is_identity()
    }

    /// Compress this [`EncodedPoint`], returning a new [`EncodedPoint`].
    pub fn compress(&self) -> Self {
        match self.coordinates() {
            Coordinates::Compressed { .. }
            | Coordinates::Compact { .. }
            | Coordinates::Identity => self.clone(),
            Coordinates::Uncompressed { x, y } => Self::from_affine_coordinates(x, y, true),
        }
    }

    /// Get the SEC1 tag for this [`EncodedPoint`]
    pub fn tag(&self) -> Tag {
        // Tag is ensured valid by the constructor
        Tag::from_u8(self.bytes[0]).expect("invalid tag")
    }

    /// Get the [`Coordinates`] for this [`EncodedPoint`].
    #[inline]
    pub fn coordinates(&self) -> Coordinates<'_, Size> {
        if self.is_identity() {
            return Coordinates::Identity;
        }

        let (x, y) = self.bytes[1..].split_at(Size::to_usize());

        if self.is_compressed() {
            Coordinates::Compressed {
                x: x.into(),
                y_is_odd: self.tag() as u8 & 1 == 1,
            }
        } else if self.is_compact() {
            Coordinates::Compact { x: x.into() }
        } else {
            Coordinates::Uncompressed {
                x: x.into(),
                y: y.into(),
            }
        }
    }

    /// Get the x-coordinate for this [`EncodedPoint`].
    ///
    /// Returns `None` if this point is the identity point.
    pub fn x(&self) -> Option<&GenericArray<u8, Size>> {
        match self.coordinates() {
            Coordinates::Identity => None,
            Coordinates::Compressed { x, .. } => Some(x),
            Coordinates::Uncompressed { x, .. } => Some(x),
            Coordinates::Compact { x } => Some(x),
        }
    }

    /// Get the y-coordinate for this [`EncodedPoint`].
    ///
    /// Returns `None` if this point is compressed or the identity point.
    pub fn y(&self) -> Option<&GenericArray<u8, Size>> {
        match self.coordinates() {
            Coordinates::Compressed { .. } | Coordinates::Identity => None,
            Coordinates::Uncompressed { y, .. } => Some(y),
            Coordinates::Compact { .. } => None,
        }
    }
}

impl<Size> AsRef<[u8]> for EncodedPoint<Size>
where
    Size: ModulusSize,
{
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<Size> ConditionallySelectable for EncodedPoint<Size>
where
    Size: ModulusSize,
    <Size::UncompressedPointSize as ArrayLength<u8>>::ArrayType: Copy,
{
    fn conditional_select(a: &Self, b: &Self, choice: Choice) -> Self {
        let mut bytes = GenericArray::default();

        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::conditional_select(&a.bytes[i], &b.bytes[i], choice);
        }

        Self { bytes }
    }
}

impl<Size> Copy for EncodedPoint<Size>
where
    Size: ModulusSize,
    <Size::UncompressedPointSize as ArrayLength<u8>>::ArrayType: Copy,
{
}

impl<Size> Debug for EncodedPoint<Size>
where
    Size: ModulusSize,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EncodedPoint({:?})", self.coordinates())
    }
}

impl<Size: ModulusSize> Eq for EncodedPoint<Size> {}

impl<Size> PartialEq for EncodedPoint<Size>
where
    Size: ModulusSize,
{
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<Size> Hash for EncodedPoint<Size>
where
    Size: ModulusSize,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state)
    }
}

impl<Size: ModulusSize> PartialOrd for EncodedPoint<Size>
where
    Size: ModulusSize,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Size: ModulusSize> Ord for EncodedPoint<Size>
where
    Size: ModulusSize,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

impl<Size: ModulusSize> TryFrom<&[u8]> for EncodedPoint<Size>
where
    Size: ModulusSize,
{
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        Self::from_bytes(bytes)
    }
}

impl<Size> Zeroize for EncodedPoint<Size>
where
    Size: ModulusSize,
{
    fn zeroize(&mut self) {
        self.bytes.zeroize();
        *self = Self::identity();
    }
}

impl<Size> fmt::Display for EncodedPoint<Size>
where
    Size: ModulusSize,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", self)
    }
}

impl<Size> fmt::LowerHex for EncodedPoint<Size>
where
    Size: ModulusSize,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", HexDisplay(self.as_bytes()))
    }
}

impl<Size> fmt::UpperHex for EncodedPoint<Size>
where
    Size: ModulusSize,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", HexDisplay(self.as_bytes()))
    }
}

/// Decode a SEC1-encoded point from hexadecimal.
///
/// Upper and lower case hexadecimal are both accepted, however mixed case is
/// rejected.
impl<Size> str::FromStr for EncodedPoint<Size>
where
    Size: ModulusSize,
{
    type Err = Error;

    fn from_str(hex: &str) -> Result<Self> {
        let mut buf = GenericArray::<u8, Size::UncompressedPointSize>::default();
        crate::base16ct::mixed::decode(hex, &mut buf)
            .map_err(|_| Error::PointEncoding)
            .and_then(Self::from_bytes)
    }
}

#[cfg(feature = "elliptic_curve_serde")]
impl<Size> Serialize for EncodedPoint<Size>
where
    Size: ModulusSize,
{
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serdect::slice::serialize_hex_upper_or_bin(&self.as_bytes(), serializer)
    }
}

#[cfg(feature = "elliptic_curve_serde")]
impl<'de, Size> Deserialize<'de> for EncodedPoint<Size>
where
    Size: ModulusSize,
{
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let bytes = serdect::slice::deserialize_hex_or_bin_vec(deserializer)?;
        Self::from_bytes(bytes).map_err(de::Error::custom)
    }
}

/// Enum representing the coordinates of either compressed or uncompressed
/// SEC1-encoded elliptic curve points.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Coordinates<'a, Size: ModulusSize> {
    /// Identity point (a.k.a. point at infinity)
    Identity,

    /// Compact curve point
    Compact {
        /// x-coordinate
        x: &'a GenericArray<u8, Size>,
    },

    /// Compressed curve point
    Compressed {
        /// x-coordinate
        x: &'a GenericArray<u8, Size>,

        /// Is the y-coordinate odd?
        y_is_odd: bool,
    },

    /// Uncompressed curve point
    Uncompressed {
        /// x-coordinate
        x: &'a GenericArray<u8, Size>,

        /// y-coordinate
        y: &'a GenericArray<u8, Size>,
    },
}

impl<'a, Size: ModulusSize> Coordinates<'a, Size> {
    /// Get the tag octet needed to encode this set of [`Coordinates`]
    pub fn tag(&self) -> Tag {
        match self {
            Coordinates::Compact { .. } => Tag::Compact,
            Coordinates::Compressed { y_is_odd, .. } => {
                if *y_is_odd {
                    Tag::CompressedOddY
                } else {
                    Tag::CompressedEvenY
                }
            }
            Coordinates::Identity => Tag::Identity,
            Coordinates::Uncompressed { .. } => Tag::Uncompressed,
        }
    }
}

/// Tag byte used by the `Elliptic-Curve-Point-to-Octet-String` encoding.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Tag {
    /// Identity point (`0x00`)
    Identity = 0,

    /// Compressed point with even y-coordinate (`0x02`)
    CompressedEvenY = 2,

    /// Compressed point with odd y-coordinate (`0x03`)
    CompressedOddY = 3,

    /// Uncompressed point (`0x04`)
    Uncompressed = 4,

    /// Compact point (`0x05`)
    Compact = 5,
}

impl Tag {
    /// Parse a tag value from a byte
    pub fn from_u8(byte: u8) -> Result<Self> {
        match byte {
            0 => Ok(Tag::Identity),
            2 => Ok(Tag::CompressedEvenY),
            3 => Ok(Tag::CompressedOddY),
            4 => Ok(Tag::Uncompressed),
            5 => Ok(Tag::Compact),
            _ => Err(Error::PointEncoding),
        }
    }

    /// Is this point compact?
    pub fn is_compact(self) -> bool {
        matches!(self, Tag::Compact)
    }

    /// Is this point compressed?
    pub fn is_compressed(self) -> bool {
        matches!(self, Tag::CompressedEvenY | Tag::CompressedOddY)
    }

    /// Is this point the identity point?
    pub fn is_identity(self) -> bool {
        self == Tag::Identity
    }

    /// Compute the expected total message length for a message prefixed
    /// with this tag (including the tag byte), given the field element size
    /// (in bytes) for a particular elliptic curve.
    pub fn message_len(self, field_element_size: usize) -> usize {
        1 + match self {
            Tag::Identity => 0,
            Tag::CompressedEvenY | Tag::CompressedOddY => field_element_size,
            Tag::Uncompressed => field_element_size * 2,
            Tag::Compact => field_element_size,
        }
    }

    /// Compress the given y-coordinate, returning a `Tag::Compressed*` value
    fn compress_y(y: &[u8]) -> Self {
        // Is the y-coordinate odd in the SEC1 sense: `self mod 2 == 1`?
        if y.as_ref().last().expect("empty y-coordinate") & 1 == 1 {
            Tag::CompressedOddY
        } else {
            Tag::CompressedEvenY
        }
    }
}

impl TryFrom<u8> for Tag {
    type Error = Error;

    fn try_from(byte: u8) -> Result<Self> {
        Self::from_u8(byte)
    }
}

impl From<Tag> for u8 {
    fn from(tag: Tag) -> u8 {
        tag as u8
    }
}
