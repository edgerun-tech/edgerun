//! Support for ECDSA signatures with low-S normalization.

use crate::ecdsa_core::Signature;
use crate::elliptic_curve::PrimeCurve;

/// ECDSA signature with low-S normalization applied.
#[derive(Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct NormalizedSignature<C: PrimeCurve> {
    inner: Signature<C>,
}
