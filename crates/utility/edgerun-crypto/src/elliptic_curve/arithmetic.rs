//! Elliptic curve arithmetic traits.

use crate::elliptic_curve::{
    ops::{Invert, LinearCombination, MulByGenerator, Reduce, ShrAssign},
    point::AffineCoordinates,
    scalar::{FromUintUnchecked, IsHigh},
    Curve, FieldBytes, PrimeCurve, ScalarPrimitive,
};
use crate::subtle::{ConditionallySelectable, ConstantTimeEq, CtOption};
use crate::zeroize::DefaultIsZeroes;
use core::fmt::Debug;

/// Elliptic curve with an arithmetic implementation.
pub trait CurveArithmetic: Curve {
    /// Elliptic curve point in affine coordinates.
    type AffinePoint: 'static
        + AffineCoordinates<FieldRepr = FieldBytes<Self>>
        + Copy
        + ConditionallySelectable
        + ConstantTimeEq
        + Debug
        + Default
        + DefaultIsZeroes
        + Eq
        + PartialEq
        + Sized
        + Send
        + Sync;

    /// Elliptic curve point in projective coordinates.
    ///
    /// Note: the following bounds are provided by [`crate::group::Group`]:
    /// - `'static`
    /// - [`Copy`]
    /// - [`Clone`]
    /// - [`Debug`]
    /// - [`Eq`]
    /// - [`Sized`]
    /// - [`Send`]
    /// - [`Sync`]
    type ProjectivePoint: ConditionallySelectable
        + ConstantTimeEq
        + Default
        + DefaultIsZeroes
        + From<Self::AffinePoint>
        + Into<Self::AffinePoint>
        + LinearCombination
        + MulByGenerator
        + crate::group::Curve<AffineRepr = Self::AffinePoint>
        + crate::group::Group<Scalar = Self::Scalar>;

    /// Scalar field modulo this curve's order.
    ///
    /// Note: the following bounds are provided by [`crate::ff::Field`]:
    /// - `'static`
    /// - [`Copy`]
    /// - [`Clone`]
    /// - [`ConditionallySelectable`]
    /// - [`ConstantTimeEq`]
    /// - [`Debug`]
    /// - [`Default`]
    /// - [`Send`]
    /// - [`Sync`]
    type Scalar: AsRef<Self::Scalar>
        + DefaultIsZeroes
        + From<ScalarPrimitive<Self>>
        + FromUintUnchecked<Uint = Self::Uint>
        + Into<FieldBytes<Self>>
        + Into<ScalarPrimitive<Self>>
        + Into<Self::Uint>
        + Invert<Output = CtOption<Self::Scalar>>
        + IsHigh
        + PartialOrd
        + Reduce<Self::Uint, Bytes = FieldBytes<Self>>
        + ShrAssign<usize>
        + crate::ff::Field
        + crate::ff::PrimeField<Repr = FieldBytes<Self>>;
}

/// Prime order elliptic curve with projective arithmetic implementation.
pub trait PrimeCurveArithmetic:
    PrimeCurve + CurveArithmetic<ProjectivePoint = Self::CurveGroup>
{
    /// Prime order elliptic curve group.
    type CurveGroup: crate::group::prime::PrimeCurve<
        Affine = <Self as CurveArithmetic>::AffinePoint,
    >;
}
