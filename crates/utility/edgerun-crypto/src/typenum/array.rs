use core::ops::{Add, Div, Mul, Sub};

use super::*;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "scale_info", derive(scale_info::TypeInfo))]
pub struct ATerm;

impl TypeArray for ATerm {}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "scale_info", derive(scale_info::TypeInfo))]
pub struct TArr<V, A> {
    first: V,
    rest: A,
}

impl<V, A> TypeArray for TArr<V, A> {}

#[macro_export]
macro_rules! tarr {
    () => ( $crate::ATerm );
    ($n:ty) => ( $crate::TArr<$n, $crate::ATerm> );
    ($n:ty,) => ( $crate::TArr<$n, $crate::ATerm> );
    ($n:ty, $($tail:ty),+) => ( $crate::TArr<$n, tarr![$($tail),+]> );
    ($n:ty, $($tail:ty),+,) => ( $crate::TArr<$n, tarr![$($tail),+]> );
    ($n:ty | $rest:ty) => ( $crate::TArr<$n, $rest> );
    ($n:ty, $($tail:ty),+ | $rest:ty) => ( $crate::TArr<$n, tarr![$($tail),+ | $rest]> );
}

// ---------------------------------------------------------------------------------------
// Length

impl Len for ATerm {
    type Output = U0;
    #[inline]
    fn len(&self) -> Self::Output {
        UTerm
    }
}

impl<V, A> Len for TArr<V, A>
where
    A: Len,
    Length<A>: Add<B1>,
    Sum<Length<A>, B1>: Unsigned,
{
    type Output = Add1<Length<A>>;
    #[inline]
    fn len(&self) -> Self::Output {
        self.rest.len() + B1
    }
}

// ---------------------------------------------------------------------------------------
// FoldAdd

const _: () = {
    pub struct Null;
    impl<T> Add<T> for Null {
        type Output = T;
        fn add(self, rhs: T) -> Self::Output {
            rhs
        }
    }

    impl FoldAdd for ATerm {
        type Output = Null;
    }
};

impl<V, A> FoldAdd for TArr<V, A>
where
    A: FoldAdd,
    FoldSum<A>: Add<V>,
{
    type Output = Sum<FoldSum<A>, V>;
}

// ---------------------------------------------------------------------------------------
// FoldMul

const _: () = {
    pub struct Null;
    impl<T> Mul<T> for Null {
        type Output = T;
        fn mul(self, rhs: T) -> Self::Output {
            rhs
        }
    }

    impl FoldMul for ATerm {
        type Output = Null;
    }
};

impl<V, A> FoldMul for TArr<V, A>
where
    A: FoldMul,
    FoldProd<A>: Mul<V>,
{
    type Output = Prod<FoldProd<A>, V>;
}

// ---------------------------------------------------------------------------------------
// Add arrays
// Note that two arrays are only addable if they are the same length.

impl Add<ATerm> for ATerm {
    type Output = ATerm;
    #[inline]
    fn add(self, _: ATerm) -> Self::Output {
        ATerm
    }
}

impl<Al, Vl, Ar, Vr> Add<TArr<Vr, Ar>> for TArr<Vl, Al>
where
    Al: Add<Ar>,
    Vl: Add<Vr>,
{
    type Output = TArr<Sum<Vl, Vr>, Sum<Al, Ar>>;
    #[inline]
    fn add(self, rhs: TArr<Vr, Ar>) -> Self::Output {
        TArr {
            first: self.first + rhs.first,
            rest: self.rest + rhs.rest,
        }
    }
}

// ---------------------------------------------------------------------------------------
// Subtract arrays
// Note that two arrays are only subtractable if they are the same length.

impl Sub<ATerm> for ATerm {
    type Output = ATerm;
    #[inline]
    fn sub(self, _: ATerm) -> Self::Output {
        ATerm
    }
}

impl<Vl, Al, Vr, Ar> Sub<TArr<Vr, Ar>> for TArr<Vl, Al>
where
    Vl: Sub<Vr>,
    Al: Sub<Ar>,
{
    type Output = TArr<Diff<Vl, Vr>, Diff<Al, Ar>>;
    #[inline]
    fn sub(self, rhs: TArr<Vr, Ar>) -> Self::Output {
        TArr {
            first: self.first - rhs.first,
            rest: self.rest - rhs.rest,
        }
    }
}

// ---------------------------------------------------------------------------------------
// Multiply an array by a scalar

impl<Rhs> Mul<Rhs> for ATerm {
    type Output = ATerm;
    #[inline]
    fn mul(self, _: Rhs) -> Self::Output {
        ATerm
    }
}

impl<V, A, Rhs> Mul<Rhs> for TArr<V, A>
where
    V: Mul<Rhs>,
    A: Mul<Rhs>,
    Rhs: Copy,
{
    type Output = TArr<Prod<V, Rhs>, Prod<A, Rhs>>;
    #[inline]
    fn mul(self, rhs: Rhs) -> Self::Output {
        TArr {
            first: self.first * rhs,
            rest: self.rest * rhs,
        }
    }
}

impl Mul<ATerm> for Z0 {
    type Output = ATerm;
    #[inline]
    fn mul(self, _: ATerm) -> Self::Output {
        ATerm
    }
}

impl<U> Mul<ATerm> for PInt<U>
where
    U: Unsigned + NonZero,
{
    type Output = ATerm;
    #[inline]
    fn mul(self, _: ATerm) -> Self::Output {
        ATerm
    }
}

impl<U> Mul<ATerm> for NInt<U>
where
    U: Unsigned + NonZero,
{
    type Output = ATerm;
    #[inline]
    fn mul(self, _: ATerm) -> Self::Output {
        ATerm
    }
}

impl<V, A> Mul<TArr<V, A>> for Z0
where
    Z0: Mul<A>,
{
    type Output = TArr<Z0, Prod<Z0, A>>;
    #[inline]
    fn mul(self, rhs: TArr<V, A>) -> Self::Output {
        TArr {
            first: Z0,
            rest: self * rhs.rest,
        }
    }
}

impl<V, A, U> Mul<TArr<V, A>> for PInt<U>
where
    U: Unsigned + NonZero,
    PInt<U>: Mul<A> + Mul<V>,
{
    type Output = TArr<Prod<PInt<U>, V>, Prod<PInt<U>, A>>;
    #[inline]
    fn mul(self, rhs: TArr<V, A>) -> Self::Output {
        TArr {
            first: self * rhs.first,
            rest: self * rhs.rest,
        }
    }
}

impl<V, A, U> Mul<TArr<V, A>> for NInt<U>
where
    U: Unsigned + NonZero,
    NInt<U>: Mul<A> + Mul<V>,
{
    type Output = TArr<Prod<NInt<U>, V>, Prod<NInt<U>, A>>;
    #[inline]
    fn mul(self, rhs: TArr<V, A>) -> Self::Output {
        TArr {
            first: self * rhs.first,
            rest: self * rhs.rest,
        }
    }
}

// ---------------------------------------------------------------------------------------
// Divide an array by a scalar

impl<Rhs> Div<Rhs> for ATerm {
    type Output = ATerm;
    #[inline]
    fn div(self, _: Rhs) -> Self::Output {
        ATerm
    }
}

impl<V, A, Rhs> Div<Rhs> for TArr<V, A>
where
    V: Div<Rhs>,
    A: Div<Rhs>,
    Rhs: Copy,
{
    type Output = TArr<Quot<V, Rhs>, Quot<A, Rhs>>;
    #[inline]
    fn div(self, rhs: Rhs) -> Self::Output {
        TArr {
            first: self.first / rhs,
            rest: self.rest / rhs,
        }
    }
}

// ---------------------------------------------------------------------------------------
// Partial Divide an array by a scalar

impl<Rhs> PartialDiv<Rhs> for ATerm {
    type Output = ATerm;
    #[inline]
    fn partial_div(self, _: Rhs) -> Self::Output {
        ATerm
    }
}

impl<V, A, Rhs> PartialDiv<Rhs> for TArr<V, A>
where
    V: PartialDiv<Rhs>,
    A: PartialDiv<Rhs>,
    Rhs: Copy,
{
    type Output = TArr<PartialQuot<V, Rhs>, PartialQuot<A, Rhs>>;
    #[inline]
    fn partial_div(self, rhs: Rhs) -> Self::Output {
        TArr {
            first: self.first.partial_div(rhs),
            rest: self.rest.partial_div(rhs),
        }
    }
}

// ---------------------------------------------------------------------------------------
// Modulo an array by a scalar
use core::ops::Rem;

impl<Rhs> Rem<Rhs> for ATerm {
    type Output = ATerm;
    #[inline]
    fn rem(self, _: Rhs) -> Self::Output {
        ATerm
    }
}

impl<V, A, Rhs> Rem<Rhs> for TArr<V, A>
where
    V: Rem<Rhs>,
    A: Rem<Rhs>,
    Rhs: Copy,
{
    type Output = TArr<Mod<V, Rhs>, Mod<A, Rhs>>;
    #[inline]
    fn rem(self, rhs: Rhs) -> Self::Output {
        TArr {
            first: self.first % rhs,
            rest: self.rest % rhs,
        }
    }
}

// ---------------------------------------------------------------------------------------
// Negate an array
use core::ops::Neg;

impl Neg for ATerm {
    type Output = ATerm;
    #[inline]
    fn neg(self) -> Self::Output {
        ATerm
    }
}

impl<V, A> Neg for TArr<V, A>
where
    V: Neg,
    A: Neg,
{
    type Output = TArr<Negate<V>, Negate<A>>;
    #[inline]
    fn neg(self) -> Self::Output {
        TArr {
            first: -self.first,
            rest: -self.rest,
        }
    }
}
