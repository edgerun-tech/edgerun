pub mod point_arithmetic;

mod affine;
#[cfg(feature = "primeorder_dev")]
mod dev;
mod field;
mod projective;

pub use crate::elliptic_curve;
pub use crate::elliptic_curve::{
    Field, FieldBytes, PrimeCurve, PrimeField, generic_array, point::Double,
};
pub use crate::primeorder::{affine::AffinePoint, projective::ProjectivePoint};

use crate::elliptic_curve::CurveArithmetic;

/// Parameters for elliptic curves of prime order which can be described by the
/// short Weierstrass equation.
pub trait PrimeCurveParams:
    PrimeCurve
    + CurveArithmetic
    + CurveArithmetic<AffinePoint = AffinePoint<Self>>
    + CurveArithmetic<ProjectivePoint = ProjectivePoint<Self>>
{
    /// Base field element type.
    // TODO(tarcieri): add `Invert` bound
    type FieldElement: PrimeField<Repr = FieldBytes<Self>>;

    /// [Point arithmetic](point_arithmetic) implementation, might be optimized for this specific curve
    type PointArithmetic: point_arithmetic::PointArithmetic<Self>;

    /// Coefficient `a` in the curve equation.
    const EQUATION_A: Self::FieldElement;

    /// Coefficient `b` in the curve equation.
    const EQUATION_B: Self::FieldElement;

    /// Generator point's affine coordinates: (x, y).
    const GENERATOR: (Self::FieldElement, Self::FieldElement);
}
